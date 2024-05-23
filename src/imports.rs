use std::fmt;

use pyo3::conversion::IntoPy;
use pyo3::PyObject;

#[derive(Debug, Clone)]
pub struct ImportParseError;

impl fmt::Display for ImportParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed to parse")
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
    Ok(vec![ProjectImport {
        mod_path: "testing".to_string(),
        line_no: 1,
    }])
}
