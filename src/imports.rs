use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

use pyo3::conversion::IntoPy;
use pyo3::PyObject;

use once_cell::sync::Lazy;
use regex::Regex;

use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{Expr, Mod, Stmt, StmtIf, StmtImport, StmtImportFrom};
use ruff_source_file::Locator;
use thiserror::Error;

use crate::parsing::py_ast::parse_python_source;
use crate::{exclusion, filesystem, parsing};

#[derive(Error, Debug)]
pub enum ImportParseError {
    #[error("Failed to parse project imports.\nFile: {file}\nFailure: {source}")]
    Parsing {
        file: String,
        #[source]
        source: parsing::ParsingError,
    },
    #[error("Failed to parse project imports.\n{0}")]
    Filesystem(#[from] filesystem::FileSystemError),
    #[error("Failed to check if path is excluded.\n{0}")]
    Exclusion(#[from] exclusion::PathExclusionError),
}

pub type Result<T> = std::result::Result<T, ImportParseError>;

/// An import with a normalized module path and located line number
#[derive(Debug)]
pub struct NormalizedImport {
    pub module_path: String,
    pub line_no: usize,
}

impl NormalizedImport {
    pub fn top_level_module_name(&self) -> &str {
        self.module_path
            .split('.')
            .next()
            .expect("Normalized import module path is empty")
    }
}

#[derive(Debug, Default)]
pub struct NormalizedImports {
    pub imports: Vec<NormalizedImport>,
    pub directive_ignored_imports: Vec<NormalizedImport>,
}

impl NormalizedImports {
    fn extend_imports(&mut self, other: NormalizedImports) {
        self.imports.extend(other.imports);
        self.directive_ignored_imports
            .extend(other.directive_ignored_imports);
    }
}

impl IntoPy<PyObject> for NormalizedImport {
    fn into_py(self, py: pyo3::prelude::Python<'_>) -> PyObject {
        (self.module_path, self.line_no).into_py(py)
    }
}

pub type IgnoreDirectives = HashMap<usize, Vec<String>>;

static TACH_IGNORE_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| Regex::new(r"# *tach-ignore(?:\([^)]*\))?((?:\s+[\w.]+)*)\s*$").unwrap());

fn get_ignore_directives(file_content: &str) -> IgnoreDirectives {
    let mut ignores: IgnoreDirectives = HashMap::new();

    for (lineno, line) in file_content.lines().enumerate() {
        let normal_lineno = lineno + 1;
        if let Some(captures) = TACH_IGNORE_REGEX.captures(line) {
            let ignored_modules = captures.get(1).map_or("", |m| m.as_str());
            let modules: Vec<String> = if ignored_modules.is_empty() {
                Vec::new()
            } else {
                ignored_modules
                    .split_whitespace()
                    .map(|module| module.to_string())
                    .collect()
            };

            if line.starts_with('#') {
                ignores.insert(normal_lineno + 1, modules);
            } else {
                ignores.insert(normal_lineno, modules);
            }
        }
    }

    ignores
}

pub struct ImportVisitor<'a> {
    file_mod_path: Option<String>,
    locator: Locator<'a>,
    is_package: bool,
    ignore_directives: IgnoreDirectives,
    ignore_type_checking_imports: bool,
    pub normalized_imports: NormalizedImports,
}

impl<'a> ImportVisitor<'a> {
    pub fn new(
        file_mod_path: Option<String>,
        locator: Locator<'a>,
        is_package: bool,
        ignore_directives: IgnoreDirectives,
        ignore_type_checking_imports: bool,
    ) -> Self {
        ImportVisitor {
            file_mod_path,
            locator,
            is_package,
            ignore_directives,
            ignore_type_checking_imports,
            normalized_imports: Default::default(),
        }
    }

    fn normalize_absolute_import(&self, import_statement: &StmtImport) -> NormalizedImports {
        let mut normalized_imports = NormalizedImports::default();
        let line_no = self
            .locator
            .compute_line_index(import_statement.range.start())
            .get();
        let ignored_modules: Option<&Vec<String>> = self.ignore_directives.get(&line_no);

        if let Some(ignored) = ignored_modules {
            if ignored.is_empty() {
                // Blanket ignore of current import - add all to directive_ignored_imports
                normalized_imports.directive_ignored_imports.extend(
                    import_statement.names.iter().map(|alias| NormalizedImport {
                        module_path: alias.name.to_string(),
                        line_no: self
                            .locator
                            .compute_line_index(alias.range.start())
                            .get()
                            .try_into()
                            .unwrap(),
                    }),
                );
                return normalized_imports;
            }
        }

        for alias in &import_statement.names {
            let import = NormalizedImport {
                module_path: alias.name.to_string(),
                line_no: self
                    .locator
                    .compute_line_index(alias.range.start())
                    .get()
                    .try_into()
                    .unwrap(),
            };

            if let Some(ignored) = ignored_modules {
                if ignored.contains(alias.name.as_ref()) {
                    normalized_imports.directive_ignored_imports.push(import);
                } else {
                    normalized_imports.imports.push(import);
                }
            } else {
                normalized_imports.imports.push(import);
            }
        }
        normalized_imports
    }

    fn normalize_import_from(&self, import_statement: &StmtImportFrom) -> NormalizedImports {
        let mut normalized_imports = NormalizedImports::default();

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

        let line_no = self
            .locator
            .compute_line_index(import_statement.range.start())
            .get();
        let ignored_modules: Option<&Vec<String>> = self.ignore_directives.get(&line_no);

        if let Some(ignored) = ignored_modules {
            if ignored.is_empty() {
                // Blanket ignore - add all imports to directive_ignored_imports
                for name in &import_statement.names {
                    let global_mod_path = format!("{}.{}", base_mod_path, name.name.as_str());
                    normalized_imports
                        .directive_ignored_imports
                        .push(NormalizedImport {
                            module_path: global_mod_path,
                            line_no: self
                                .locator
                                .compute_line_index(name.range.start())
                                .get()
                                .try_into()
                                .unwrap(),
                        });
                }
                return normalized_imports;
            }
        }

        for name in &import_statement.names {
            let global_mod_path = format!("{}.{}", base_mod_path, name.name.as_str());
            let import = NormalizedImport {
                module_path: global_mod_path,
                line_no: self
                    .locator
                    .compute_line_index(name.range.start())
                    .get()
                    .try_into()
                    .unwrap(),
            };

            if let Some(ignored) = ignored_modules {
                if ignored.contains(
                    &name
                        .asname
                        .as_deref()
                        .unwrap_or(name.name.as_ref())
                        .to_string(),
                ) {
                    normalized_imports.directive_ignored_imports.push(import);
                } else {
                    normalized_imports.imports.push(import);
                }
            } else {
                normalized_imports.imports.push(import);
            }
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
        self.normalized_imports
            .extend_imports(self.normalize_absolute_import(node))
    }

    fn visit_stmt_import_from(&mut self, node: &StmtImportFrom) {
        self.normalized_imports
            .extend_imports(self.normalize_import_from(node))
    }
}

impl<'a> StatementVisitor<'a> for ImportVisitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
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
    locator: Locator<'a>,
    pub normalized_imports: Vec<NormalizedImport>,
}

impl<'a> StringImportVisitor<'a> {
    fn new(source_roots: &'a [PathBuf], locator: Locator<'a>) -> Self {
        StringImportVisitor {
            source_roots,
            locator,
            normalized_imports: vec![],
        }
    }
}

impl<'a> Visitor<'a> for StringImportVisitor<'a> {
    fn visit_string_literal(&mut self, string_literal: &'a ruff_python_ast::StringLiteral) {
        // DEFAULT python-infer-string-imports-min-dots is 2
        if string_literal.value.chars().filter(|&c| c == '.').count() < 2 {
            return;
        }

        let resolved_module =
            filesystem::module_to_file_path(self.source_roots, &string_literal.value, true);
        if resolved_module.is_some() {
            self.normalized_imports.push(NormalizedImport {
                module_path: string_literal.value.to_string(),
                line_no: self
                    .locator
                    .compute_line_index(string_literal.range.start())
                    .get(),
            });
        }
    }
}

/// Source Roots here are assumed to be absolute paths
pub fn is_project_import<P: AsRef<Path>>(source_roots: &[P], mod_path: &str) -> Result<bool> {
    let resolved_module = filesystem::module_to_file_path(source_roots, mod_path, true);
    if let Some(module) = resolved_module {
        // This appears to be a project import, verify it is not excluded
        Ok(!exclusion::is_path_excluded(module.file_path)?)
    } else {
        // This is not a project import
        Ok(false)
    }
}

pub fn get_normalized_imports(
    source_roots: &[PathBuf],
    file_path: &PathBuf,
    ignore_type_checking_imports: bool,
    include_string_imports: bool,
) -> Result<NormalizedImports> {
    let file_contents = filesystem::read_file_content(file_path)?;
    let file_ast =
        parse_python_source(&file_contents).map_err(|err| ImportParseError::Parsing {
            file: file_path.to_str().unwrap().to_string(),
            source: err,
        })?;
    let is_package = file_path.ends_with("__init__.py");
    let ignore_directives = get_ignore_directives(file_contents.as_str());
    let file_mod_path: Option<String> =
        filesystem::file_to_module_path(source_roots, file_path).ok();
    let mut import_visitor = ImportVisitor::new(
        file_mod_path,
        Locator::new(&file_contents),
        is_package,
        ignore_directives,
        ignore_type_checking_imports,
    );
    let mut string_import_visitor =
        StringImportVisitor::new(source_roots, Locator::new(&file_contents));

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
            import_visitor.normalized_imports.imports.len()
                + string_import_visitor.normalized_imports.len(),
        );
        result_imports.extend(import_visitor.normalized_imports.imports);
        result_imports.extend(string_import_visitor.normalized_imports);

        // TODO: catch directive ignored imports in string_import_visitor
        Ok(NormalizedImports {
            imports: result_imports,
            directive_ignored_imports: import_visitor.normalized_imports.directive_ignored_imports,
        })
    } else {
        Ok(import_visitor.normalized_imports)
    }
}

pub struct ProjectImports {
    pub imports: Vec<NormalizedImport>,
    pub directive_ignored_imports: Vec<NormalizedImport>,
}

pub fn get_project_imports(
    source_roots: &[PathBuf],
    file_path: &PathBuf,
    ignore_type_checking_imports: bool,
    include_string_imports: bool,
) -> Result<ProjectImports> {
    let normalized_imports = get_normalized_imports(
        source_roots,
        file_path,
        ignore_type_checking_imports,
        include_string_imports,
    )?;
    Ok(ProjectImports {
        imports: normalized_imports
            .imports
            .into_iter()
            .filter_map(|normalized_import| {
                is_project_import(source_roots, &normalized_import.module_path)
                    .map_or(None, |is_project_import| {
                        is_project_import.then_some(normalized_import)
                    })
            })
            .collect(),
        directive_ignored_imports: normalized_imports
            .directive_ignored_imports
            .into_iter()
            .filter_map(|normalized_import| {
                is_project_import(source_roots, &normalized_import.module_path)
                    .map_or(None, |is_project_import| {
                        is_project_import.then_some(normalized_import)
                    })
            })
            .collect(),
    })
}
