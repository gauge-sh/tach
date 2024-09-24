use std::io;
use thiserror::Error;

use crate::filesystem::FileSystemError;
use pyo3::conversion::IntoPy;
use pyo3::PyObject;
use ruff_python_parser::ParseError;

#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("Python parsing error: {0}")]
    PythonParse(#[from] ParseError),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] FileSystemError),
    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("Missing field in TOML: {0}")]
    MissingField(String),
}

pub type Result<T> = std::result::Result<T, ParsingError>;

#[derive(Debug, Clone)]
pub struct VisibilityErrorInfo {
    pub dependent_module: String,
    pub dependency_module: String,
    pub visibility: Vec<String>,
}

impl IntoPy<PyObject> for VisibilityErrorInfo {
    fn into_py(self, py: pyo3::prelude::Python<'_>) -> PyObject {
        (
            self.dependent_module,
            self.dependency_module,
            self.visibility,
        )
            .into_py(py)
    }
}

#[derive(Error, Debug)]
pub enum ModuleTreeError {
    #[error(
        "Failed to build module tree. The following modules were defined more than once: {0:?}"
    )]
    DuplicateModules(Vec<String>),
    #[error("Module configuration error: Visibility configuration conflicts with dependency configuration.")]
    VisibilityViolation(Vec<VisibilityErrorInfo>),
    #[error("Circular dependency detected: {0:?}")]
    CircularDependency(Vec<String>),
    #[error("Parsing Error while building module tree.\n{0}")]
    ParseError(#[from] ParsingError),
    #[error("Cannot insert module with empty path.")]
    InsertNodeError,
}
