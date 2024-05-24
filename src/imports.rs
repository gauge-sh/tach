use std::fmt::{self, Debug};
use std::path::{Path, PathBuf};

use pyo3::conversion::IntoPy;
use pyo3::PyObject;

use rustpython_ast::text_size::TextRange;
use rustpython_ast::Expr::Name;
use rustpython_ast::Visitor;

use crate::{filesystem, parsing};

#[derive(Debug, Clone)]
pub struct ImportParseError {
    pub message: String,
}

impl fmt::Display for ImportParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

pub type Result<T> = std::result::Result<T, ImportParseError>;

#[derive(Debug)]
pub struct ProjectImport {
    pub mod_path: String,
    pub line_no: usize,
}

pub type ProjectImports = Vec<ProjectImport>;

impl IntoPy<PyObject> for ProjectImport {
    fn into_py(self, py: pyo3::prelude::Python<'_>) -> PyObject {
        (self.mod_path, self.line_no).into_py(py)
    }
}

trait IntoProjectImports {
    fn into_project_imports<P: AsRef<Path>>(
        self,
        project_root: P,
        file_mod_path: &str,
    ) -> ProjectImports;
}

impl IntoProjectImports for rustpython_ast::StmtImport {
    fn into_project_imports<P: AsRef<Path>>(
        self,
        project_root: P,
        _file_mod_path: &str,
    ) -> ProjectImports {
        self.names
            .iter()
            .filter_map(|alias| {
                match filesystem::is_project_import(project_root.as_ref(), alias.name.as_str()) {
                    Ok(true) => Some(ProjectImport {
                        mod_path: alias.name.to_string(),
                        line_no: alias.range.start().into(),
                    }),
                    Ok(false) => None,
                    Err(_) => None,
                }
            })
            .collect()
    }
}

impl IntoProjectImports for rustpython_ast::StmtImportFrom {
    fn into_project_imports<P: AsRef<Path>>(
        self,
        project_root: P,
        file_mod_path: &str,
    ) -> ProjectImports {
        let mut imports = ProjectImports::new();

        let import_depth = self.level.unwrap_or(rustpython_ast::Int::new(0)).to_usize();
        // For relative imports (level > 0), adjust the base module path
        let base_mod_path = if let Some(ref module) = self.module {
            if import_depth > 0 {
                // Assuming `is_package` is some boolean flag we have to determine
                let is_package = false; // Replace this with actual logic if needed
                let num_paths_to_strip = if is_package {
                    import_depth - 1
                } else {
                    import_depth
                };
                let base_path_parts: Vec<&str> = file_mod_path.split(".").collect();
                let base_path_parts = if num_paths_to_strip > 0 {
                    base_path_parts[..base_path_parts.len() - num_paths_to_strip].to_vec()
                } else {
                    base_path_parts
                };

                // base_mod_path is the current file's mod path
                // minus the paths_to_strip (due to level of import)
                // plus the module we are importing from
                format!("{}.{}", base_path_parts.join("."), module)
            } else {
                module.to_string()
            }
        } else {
            // We are importing from the current package ('.')
            String::new()
        };

        // Ignored modules stub
        let ignored_modules: Option<Vec<String>> = None; // Replace with actual logic if needed

        if let Some(ref ignored) = ignored_modules {
            if ignored.is_empty() {
                return imports; // Blanket ignore of following import
            }
        }

        for name in self.names {
            let local_mod_path = format!(
                "{}{}.{}",
                ".".repeat(import_depth),
                self.module.as_deref().unwrap_or(""),
                name.asname.as_deref().unwrap_or(name.name.as_ref())
            );
            if let Some(ref ignored) = ignored_modules {
                if ignored.contains(&local_mod_path) {
                    continue; // This import is ignored by a directive
                }
            }

            let global_mod_path = match self.module {
                Some(_) => format!("{}.{}", base_mod_path, name.name.as_str()),
                None => name.name.to_string(),
            };

            match filesystem::is_project_import(project_root.as_ref(), &global_mod_path) {
                Ok(true) => imports.push(ProjectImport {
                    mod_path: global_mod_path,
                    line_no: self.range.start().into(),
                }),
                Ok(false) => (),
                Err(_) => (),
            }
        }

        // Return all project imports found
        imports
    }
}

pub struct ImportVisitor {
    project_root: String,
    file_mod_path: String,
    ignore_type_checking_imports: bool,
    pub project_imports: ProjectImports,
}

impl ImportVisitor {
    pub fn new(
        project_root: String,
        file_mod_path: String,
        ignore_type_checking_imports: bool,
    ) -> Self {
        ImportVisitor {
            project_root,
            file_mod_path,
            ignore_type_checking_imports,
            project_imports: vec![],
        }
    }
}

impl Visitor for ImportVisitor {
    fn visit_stmt_if(&mut self, node: rustpython_ast::StmtIf<TextRange>) {
        let id = match node.test.as_ref() {
            Name(ref name) => Some(name.id.as_str()),
            _ => None,
        };
        if id.unwrap_or_default() == "TYPE_CHECKING" && self.ignore_type_checking_imports {
            return;
        }

        // assume other conditional imports represent real dependencies
        self.generic_visit_stmt_if(node);
    }

    fn visit_stmt_import(&mut self, node: rustpython_ast::StmtImport<TextRange>) {
        self.project_imports
            .extend(node.into_project_imports(&self.project_root, &self.file_mod_path))
    }

    fn visit_stmt_import_from(&mut self, node: rustpython_ast::StmtImportFrom<TextRange>) {
        self.project_imports
            .extend(node.into_project_imports(&self.project_root, &self.file_mod_path))
    }
}

pub fn get_project_imports(
    project_root: String,
    file_path: String,
    ignore_type_checking_imports: bool,
) -> Result<ProjectImports> {
    let canonical_path: PathBuf = filesystem::canonical(project_root.as_ref(), file_path.as_ref())
        .map_err(|err| ImportParseError {
            message: format!("Failed to parse project imports. Failure: {}", err.message),
        })?;
    let file_contents =
        filesystem::read_file_content(canonical_path).map_err(|err| ImportParseError {
            message: format!("Failed to parse project imports. Failure: {}", err.message),
        })?;
    let file_ast =
        parsing::parse_python_source(&file_contents).map_err(|err| ImportParseError {
            message: format!("Failed to parse project imports. Failure: {:?}", err),
        })?;
    let mut import_visitor = ImportVisitor::new(
        project_root,
        filesystem::file_to_module_path(file_path.as_str()),
        ignore_type_checking_imports,
    );
    file_ast
        .into_iter()
        .for_each(|stmnt| import_visitor.visit_stmt(stmnt));
    Ok(import_visitor.project_imports)
}
