use std::fmt::Debug;
use std::path::{Path, PathBuf};

use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{Expr, Mod, Stmt, StmtIf, StmtImport, StmtImportFrom};
use thiserror::Error;

use crate::dependencies::import::NormalizedImport;
use crate::python::{error::ParsingError, parsing::parse_python_source};
use crate::resolvers::PackageResolutionError;
use crate::{exclusion, filesystem};

#[derive(Error, Debug)]
pub enum ImportParseError {
    #[error("Failed to parse project imports.\nFile: {file}\nFailure: {source}")]
    Parsing {
        file: String,
        #[source]
        source: ParsingError,
    },
    #[error("Failed to parse project imports.\n{0}")]
    Filesystem(#[from] filesystem::FileSystemError),
    #[error("Failed to build exclude patterns.\n{0}")]
    ExclusionPatterns(#[from] exclusion::PathExclusionError),
    #[error("Package resolution error.\n{0}")]
    PackageResolution(#[from] PackageResolutionError),
}

pub type Result<T> = std::result::Result<T, ImportParseError>;

pub struct ImportVisitor {
    file_mod_path: Option<String>,
    is_package: bool,
    ignore_type_checking_imports: bool,
    pub normalized_imports: Vec<NormalizedImport>,
}

impl ImportVisitor {
    pub fn new(
        file_mod_path: Option<String>,
        is_package: bool,
        ignore_type_checking_imports: bool,
    ) -> Self {
        ImportVisitor {
            file_mod_path,
            is_package,
            ignore_type_checking_imports,
            normalized_imports: Default::default(),
        }
    }

    fn normalize_absolute_import(
        &mut self,
        import_statement: &StmtImport,
    ) -> Vec<NormalizedImport> {
        let mut normalized_imports = vec![];

        for alias in &import_statement.names {
            let import = NormalizedImport {
                module_path: alias.name.to_string(),
                alias_path: None,
                alias_offset: alias.range.start(),
                import_offset: import_statement.range.start(),
                is_absolute: true,
            };
            normalized_imports.push(import);
        }
        normalized_imports
    }

    fn normalize_import_from(
        &mut self,
        import_statement: &StmtImportFrom,
    ) -> Vec<NormalizedImport> {
        let mut normalized_imports = vec![];

        let import_depth: usize = import_statement.level.try_into().unwrap();
        let num_paths_to_strip = if self.is_package {
            import_depth.saturating_sub(1)
        } else {
            import_depth
        };

        let mod_path = match &self.file_mod_path {
            Some(mod_path) => mod_path,
            None => "",
        };
        // If our current file mod path is None, we are not within the source root
        // so we assume that relative imports are also not within the source root
        if mod_path.is_empty() && import_depth > 0 {
            return normalized_imports;
        };

        let base_path_parts: Vec<&str> = mod_path.split('.').collect();
        let base_path_parts = if num_paths_to_strip > 0 {
            base_path_parts[..base_path_parts.len() - num_paths_to_strip].to_vec()
        } else {
            base_path_parts
        };

        let base_mod_path = if let Some(ref module) = import_statement.module {
            if import_depth > 0 {
                // For relative imports (level > 0), adjust the base module path

                // base_mod_path becomes the current file's mod path
                // minus the paths_to_strip (due to level of import)
                // plus the module we are importing from
                if base_path_parts.is_empty() {
                    module.to_string()
                } else {
                    format!("{}.{}", base_path_parts.join("."), module)
                }
            } else {
                module.to_string()
            }
        } else {
            // We are importing from the current package ('.') or a parent ('..' or more)
            // We have already stripped parts from the current file's mod path based on the import depth,
            // so we just need to join the remaining parts with a '.'
            if base_path_parts.is_empty() {
                // This means we are looking at a current package import outside of a source root
                return normalized_imports;
            }
            base_path_parts.join(".")
        };

        for name in &import_statement.names {
            let global_mod_path = format!("{}.{}", base_mod_path, name.name.as_str());
            let import = NormalizedImport {
                module_path: global_mod_path,
                alias_path: Some(name.asname.as_ref().unwrap_or(&name.name).to_string()),
                alias_offset: name.range.start(),
                import_offset: import_statement.range.start(),
                is_absolute: false,
            };

            normalized_imports.push(import);
        }

        normalized_imports
    }

    fn should_ignore_if_statement(&mut self, node: &StmtIf) -> bool {
        let id = match node.test.as_ref() {
            Expr::Name(ref name) => Some(name.id.as_str()),
            // This will match a single-level attribute access in cases like:
            // import typing as t; if t.TYPE_CHECKING: ...
            Expr::Attribute(ref attribute) => Some(attribute.attr.as_str()),
            _ => None,
        };
        id.unwrap_or_default() == "TYPE_CHECKING" && self.ignore_type_checking_imports
    }

    fn visit_stmt_import(&mut self, node: &StmtImport) {
        let stmt_imports = self.normalize_absolute_import(node);
        self.normalized_imports.extend(stmt_imports);
    }

    fn visit_stmt_import_from(&mut self, node: &StmtImportFrom) {
        let stmt_imports = self.normalize_import_from(node);
        self.normalized_imports.extend(stmt_imports);
    }
}

impl StatementVisitor<'_> for ImportVisitor {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import(statement) => self.visit_stmt_import(statement),
            Stmt::ImportFrom(statement) => self.visit_stmt_import_from(statement),
            Stmt::If(statement) => {
                if !self.should_ignore_if_statement(statement) {
                    walk_stmt(self, stmt)
                }
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

struct StringImportVisitor<'a> {
    source_roots: &'a [PathBuf],
    pub normalized_imports: Vec<NormalizedImport>,
}

impl<'a> StringImportVisitor<'a> {
    fn new(source_roots: &'a [PathBuf]) -> Self {
        StringImportVisitor {
            source_roots,
            normalized_imports: vec![],
        }
    }
}

impl Visitor<'_> for StringImportVisitor<'_> {
    fn visit_string_literal(&mut self, string_literal: &ruff_python_ast::StringLiteral) {
        // DEFAULT python-infer-string-imports-min-dots is 2
        if string_literal.value.chars().filter(|&c| c == '.').count() < 2 {
            return;
        }

        let resolved_module =
            filesystem::module_to_file_path(self.source_roots, &string_literal.value, true);
        if resolved_module.is_some() {
            self.normalized_imports.push(NormalizedImport {
                module_path: string_literal.value.to_string(),
                alias_path: None,
                alias_offset: string_literal.range.start(),
                import_offset: string_literal.range.start(),
                is_absolute: true,
            });
        }
    }
}

pub fn get_normalized_imports_from_ast<P: AsRef<Path>>(
    source_roots: &[PathBuf],
    file_path: P,
    file_ast: &Mod,
    ignore_type_checking_imports: bool,
    include_string_imports: bool,
) -> Result<Vec<NormalizedImport>> {
    let is_package = file_path
        .as_ref()
        .to_string_lossy()
        .ends_with("__init__.py");
    let file_mod_path: Option<String> =
        filesystem::file_to_module_path(source_roots, file_path.as_ref()).ok();
    let mut import_visitor =
        ImportVisitor::new(file_mod_path, is_package, ignore_type_checking_imports);
    let mut string_import_visitor = StringImportVisitor::new(source_roots);

    match file_ast {
        Mod::Module(ref module) => {
            import_visitor.visit_body(&module.body);
            if include_string_imports {
                string_import_visitor.visit_body(&module.body);
            }
        }
        Mod::Expression(_) => (), // should error
    };

    if include_string_imports {
        let mut result_imports = Vec::with_capacity(
            import_visitor.normalized_imports.len()
                + string_import_visitor.normalized_imports.len(),
        );
        result_imports.extend(import_visitor.normalized_imports);
        result_imports.extend(string_import_visitor.normalized_imports);

        Ok(result_imports)
    } else {
        Ok(import_visitor.normalized_imports)
    }
}

pub fn get_normalized_imports<P: AsRef<Path>>(
    source_roots: &[PathBuf],
    file_path: P,
    file_contents: &str,
    ignore_type_checking_imports: bool,
    include_string_imports: bool,
) -> Result<Vec<NormalizedImport>> {
    let file_ast = parse_python_source(file_contents).map_err(|err| ImportParseError::Parsing {
        file: file_path.as_ref().to_string_lossy().to_string(),
        source: err,
    })?;
    get_normalized_imports_from_ast(
        source_roots,
        file_path,
        &file_ast,
        ignore_type_checking_imports,
        include_string_imports,
    )
}
