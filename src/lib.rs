pub mod exclusion;
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

impl From<exclusion::PathExclusionError> for PyErr {
    fn from(err: exclusion::PathExclusionError) -> Self {
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
    imports::get_project_imports(project_root, file_path, ignore_type_checking_imports)
}

/// Set excluded paths globally.
/// This is called separately in order to set up a singleton instance holding regexes,
/// since they would be expensive to build for every call.
#[pyfunction]
#[pyo3(signature = (exclude_paths))]
fn set_excluded_paths(exclude_paths: Vec<String>) -> exclusion::Result<()> {
    exclusion::set_excluded_paths(exclude_paths)
}

#[pymodule]
fn extension(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_project_imports, m)?)?;
    m.add_function(wrap_pyfunction!(set_excluded_paths, m)?)?;
    Ok(())
}
