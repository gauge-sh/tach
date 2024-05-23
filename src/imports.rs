use std::fmt::{self, Debug, Pointer};
use std::path::PathBuf;

use pyo3::conversion::IntoPy;
use pyo3::PyObject;

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
        (self.mod_path.clone(), self.line_no).into_py(py)
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
    Ok(file_ast
        .iter()
        .map(|_stmnt| ProjectImport {
            mod_path: "test".to_string(),
            line_no: 1,
        })
        .collect())
}
