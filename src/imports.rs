use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::iter;
use std::marker::PhantomData;
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

use crate::python::{error::ParsingError, parsing::parse_python_source};
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
    #[error("Failed to check if path is excluded.\n{0}")]
    Exclusion(#[from] exclusion::PathExclusionError),
}

pub type Result<T> = std::result::Result<T, ImportParseError>;

/// An import with a normalized module path and located line number
#[derive(Debug, Clone)]
pub struct NormalizedImport {
    pub module_path: String,        // Global module path
    pub alias_path: Option<String>, // (for relative imports) alias path
    pub import_line_no: usize,      // Line number of the import statement
    pub line_no: usize,             // Line number of the alias
    pub is_absolute: bool,          // Whether the import is absolute
}

impl NormalizedImport {
    pub fn top_level_module_name(&self) -> &str {
        self.module_path
            .split('.')
            .next()
            .expect("Normalized import module path is empty")
    }
}

#[derive(Debug)]
pub struct DirectiveIgnoredImport<'a> {
    pub import: &'a NormalizedImport,
    pub reason: String,
}

pub struct AllImports;
pub struct ProjectImports;
pub struct ExternalImports;

#[derive(Debug, Default)]
pub struct NormalizedImports<State = AllImports> {
    pub imports: Vec<NormalizedImport>,
    pub ignore_directives: IgnoreDirectives,
    _state: PhantomData<State>,
}

impl<State> NormalizedImports<State> {
    pub fn new(imports: Vec<NormalizedImport>, ignore_directives: IgnoreDirectives) -> Self {
        Self {
            imports,
            ignore_directives,
            _state: PhantomData,
        }
    }

    pub fn active_imports(&self) -> impl Iterator<Item = &NormalizedImport> {
        self.imports
            .iter()
            .filter(|import| !self.ignore_directives.is_ignored(import))
    }

    pub fn into_active_imports(self) -> NormalizedImports {
        NormalizedImports {
            imports: self
                .imports
                .into_iter()
                .filter(|import| !self.ignore_directives.is_ignored(import))
                .collect(),
            ignore_directives: self.ignore_directives,
            _state: PhantomData,
        }
    }

    pub fn directive_ignored_imports(&self) -> impl Iterator<Item = DirectiveIgnoredImport> {
        self.imports
            .iter()
            .filter(|&import| self.ignore_directives.is_ignored(import))
            .map(|import| DirectiveIgnoredImport {
                import,
                reason: self
                    .ignore_directives
                    .get(&import.import_line_no)
                    .unwrap()
                    .reason
                    .clone(),
            })
    }

    pub fn unused_ignore_directives(&self) -> impl Iterator<Item = &IgnoreDirective> {
        let mut directive_lines: HashSet<usize> =
            HashSet::from_iter(self.ignore_directives.lines().cloned());
        self.imports.iter().for_each(|import| {
            directive_lines.remove(&import.import_line_no);
        });
        directive_lines
            .into_iter()
            .map(|line| self.ignore_directives.get(&line).unwrap())
            .chain(self.ignore_directives.redundant_directives())
    }
}

impl NormalizedImports<AllImports> {
    pub fn into_project_imports(
        self,
        source_roots: &[PathBuf],
    ) -> NormalizedImports<ProjectImports> {
        NormalizedImports {
            imports: self
                .imports
                .into_iter()
                .filter_map(|normalized_import| {
                    is_project_import(source_roots, &normalized_import.module_path)
                        .map_or(None, |is_project_import| {
                            is_project_import.then_some(normalized_import)
                        })
                })
                .collect(),
            ignore_directives: self.ignore_directives,
            _state: PhantomData,
        }
    }

    pub fn into_external_imports(
        self,
        source_roots: &[PathBuf],
    ) -> NormalizedImports<ExternalImports> {
        NormalizedImports {
            imports: self
                .imports
                .into_iter()
                .filter_map(|normalized_import| {
                    is_project_import(source_roots, &normalized_import.module_path)
                        .map_or(None, |is_project_import| {
                            (!is_project_import).then_some(normalized_import)
                        })
                })
                .collect(),
            ignore_directives: self.ignore_directives,
            _state: PhantomData,
        }
    }
}

impl<State> Extend<NormalizedImports<State>> for NormalizedImports<State> {
    fn extend<T: IntoIterator<Item = NormalizedImports<State>>>(&mut self, iter: T) {
        for normalized_imports in iter {
            self.imports.extend(normalized_imports.imports);
            self.ignore_directives
                .extend(iter::once(normalized_imports.ignore_directives));
        }
    }
}

impl IntoPy<PyObject> for NormalizedImport {
    fn into_py(self, py: pyo3::prelude::Python<'_>) -> PyObject {
        (self.module_path, self.line_no).into_py(py)
    }
}

#[derive(Debug)]
pub struct IgnoreDirective {
    pub modules: Vec<String>,
    pub reason: String,
    pub line_no: usize,
}

#[derive(Debug, Default)]
pub struct IgnoreDirectives {
    directives: HashMap<usize, IgnoreDirective>,
    redundant_directives: Vec<IgnoreDirective>,
}

impl IgnoreDirectives {
    pub fn lines(&self) -> impl Iterator<Item = &usize> {
        self.directives.keys()
    }

    pub fn len(&self) -> usize {
        self.directives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.directives.is_empty()
    }

    pub fn add_directive(&mut self, directive: IgnoreDirective, ignored_line_no: usize) {
        match self.directives.entry(ignored_line_no) {
            Entry::Occupied(_) => {
                self.redundant_directives.push(directive);
            }
            Entry::Vacant(entry) => {
                entry.insert(directive);
            }
        }
    }

    pub fn get(&self, line_no: &usize) -> Option<&IgnoreDirective> {
        self.directives.get(line_no)
    }

    pub fn is_ignored(&self, normalized_import: &NormalizedImport) -> bool {
        self.directives
            .get(&normalized_import.import_line_no)
            .map_or(false, |directive| {
                if normalized_import.is_absolute {
                    directive.modules.is_empty()
                        || directive.modules.contains(&normalized_import.module_path)
                } else {
                    directive.modules.is_empty()
                        || directive
                            .modules
                            .contains(normalized_import.alias_path.as_ref().unwrap())
                }
            })
    }

    pub fn redundant_directives(&self) -> impl Iterator<Item = &IgnoreDirective> {
        self.redundant_directives.iter()
    }
}

impl Extend<IgnoreDirectives> for IgnoreDirectives {
    fn extend<T: IntoIterator<Item = IgnoreDirectives>>(&mut self, iter: T) {
        for directives in iter {
            self.directives.extend(directives.directives);
            self.redundant_directives
                .extend(directives.redundant_directives);
        }
    }
}

static TACH_IGNORE_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| Regex::new(r"# *tach-ignore(?:\(([^)]*)\))?((?:\s+[\w.]+)*)\s*$").unwrap());

fn get_ignore_directives(file_content: &str) -> IgnoreDirectives {
    if !file_content.contains("tach-ignore") {
        return IgnoreDirectives::default();
    }

    let mut ignores = IgnoreDirectives::default();

    for (lineno, line) in file_content.lines().enumerate() {
        if !line.contains("tach-ignore") {
            continue;
        }

        let normal_lineno = lineno + 1;
        if let Some(captures) = TACH_IGNORE_REGEX.captures(line) {
            let reason = captures
                .get(1)
                .map_or("".to_string(), |m| m.as_str().to_string());
            let ignored_modules = captures.get(2).map_or("", |m| m.as_str());
            let modules: Vec<String> = if ignored_modules.is_empty() {
                Vec::new()
            } else {
                ignored_modules
                    .split_whitespace()
                    .map(|module| module.to_string())
                    .collect()
            };

            let directive = IgnoreDirective {
                modules,
                reason,
                line_no: normal_lineno,
            };

            if line.trim_start().starts_with('#') {
                ignores.add_directive(directive, normal_lineno + 1);
            } else {
                ignores.add_directive(directive, normal_lineno);
            }
        }
    }

    ignores
}

pub struct ImportVisitor<'a> {
    file_mod_path: Option<String>,
    locator: Locator<'a>,
    is_package: bool,
    ignore_type_checking_imports: bool,
    pub normalized_imports: Vec<NormalizedImport>,
}

impl<'a> ImportVisitor<'a> {
    pub fn new(
        file_mod_path: Option<String>,
        locator: Locator<'a>,
        is_package: bool,
        ignore_type_checking_imports: bool,
    ) -> Self {
        ImportVisitor {
            file_mod_path,
            locator,
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
        let line_no = self
            .locator
            .compute_line_index(import_statement.range.start())
            .get();

        for alias in &import_statement.names {
            let import = NormalizedImport {
                module_path: alias.name.to_string(),
                alias_path: None,
                line_no: self.locator.compute_line_index(alias.range.start()).get(),
                import_line_no: line_no,
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

        let line_no = self
            .locator
            .compute_line_index(import_statement.range.start())
            .get();

        for name in &import_statement.names {
            let global_mod_path = format!("{}.{}", base_mod_path, name.name.as_str());
            let import = NormalizedImport {
                module_path: global_mod_path,
                alias_path: Some(name.asname.as_ref().unwrap_or(&name.name).to_string()),
                line_no: self.locator.compute_line_index(name.range.start()).get(),
                import_line_no: line_no,
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
            let line_no = self
                .locator
                .compute_line_index(string_literal.range.start())
                .get();
            self.normalized_imports.push(NormalizedImport {
                module_path: string_literal.value.to_string(),
                alias_path: None,
                line_no,
                import_line_no: line_no,
                is_absolute: true,
            });
        }
    }
}

/// Source Roots here are assumed to be absolute paths
pub fn is_project_import<P: AsRef<Path>>(source_roots: &[P], mod_path: &str) -> Result<bool> {
    let resolved_module = filesystem::module_to_file_path(source_roots, mod_path, true);
    if let Some(module) = resolved_module {
        // This appears to be a project import, verify it is not excluded
        Ok(!exclusion::is_path_excluded(module.file_path))
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
            import_visitor.normalized_imports.len()
                + string_import_visitor.normalized_imports.len(),
        );
        result_imports.extend(import_visitor.normalized_imports);
        result_imports.extend(string_import_visitor.normalized_imports);

        Ok(NormalizedImports::new(result_imports, ignore_directives))
    } else {
        Ok(NormalizedImports::new(
            import_visitor.normalized_imports,
            ignore_directives,
        ))
    }
}

pub fn get_project_imports(
    source_roots: &[PathBuf],
    file_path: &PathBuf,
    ignore_type_checking_imports: bool,
    include_string_imports: bool,
) -> Result<NormalizedImports<ProjectImports>> {
    let normalized_imports = get_normalized_imports(
        source_roots,
        file_path,
        ignore_type_checking_imports,
        include_string_imports,
    )?;
    Ok(normalized_imports.into_project_imports(source_roots))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
    "# tach-ignore\nfrom foo import bar",
    2,  // The import is on line 2
    vec![]  // Empty vec means blanket ignore
)]
    #[case(
    "# tach-ignore(test reason)\nfrom foo import bar",
    2,
    vec![]
)]
    #[case(
    "# tach-ignore foo bar\nfrom foo import bar",
    2,
    vec!["foo".to_string(), "bar".to_string()]
)]
    #[case(
    "from foo import bar  # tach-ignore",
    1,
    vec![]
)]
    #[case(
    "from foo import bar  # tach-ignore(skip this)\nother code",
    1,
    vec![]
)]
    #[case(
    "from foo import bar  # tach-ignore foo baz",
    1,
    vec!["foo".to_string(), "baz".to_string()]
)]
    fn test_get_ignore_directives(
        #[case] content: &str,
        #[case] expected_line: usize,
        #[case] expected_modules: Vec<String>,
    ) {
        let directives = get_ignore_directives(content);
        assert_eq!(directives.len(), 1);

        let directive = directives
            .get(&expected_line)
            .expect("Should have directive");
        assert_eq!(directive.modules, expected_modules);
    }

    #[test]
    fn test_no_directives() {
        let content = "from foo import bar\nother code";
        let directives = get_ignore_directives(content);
        assert!(directives.is_empty());
    }
}
