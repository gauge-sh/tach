pub mod filesystem;
pub mod imports;
pub mod parsing;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

impl From<imports::ImportParseError> for PyErr {
    fn from(err: imports::ImportParseError) -> Self {
        PyValueError::new_err(err.message)
    }
}

/// Get first-party imports from file_path relative to project_root
#[pyfunction]
#[pyo3(signature = (project_root, file_path, ignore_type_checking_imports=false))]
fn get_project_imports(
    project_root: String,
    file_path: String,
    ignore_type_checking_imports: bool,
) -> imports::Result<imports::ProjectImports> {
    return imports::get_project_imports(project_root, file_path, ignore_type_checking_imports);
}

#[pymodule]
fn extension(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_project_imports, m)?)?;
    Ok(())
}
