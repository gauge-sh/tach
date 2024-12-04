use thiserror::Error;

use crate::python::error::ParsingError;

#[derive(Debug, Clone)]
pub struct VisibilityErrorInfo {
    pub dependent_module: String,
    pub dependency_module: String,
    pub visibility: Vec<String>,
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
    #[error("Root module violation: {0:?}")]
    RootModuleViolation(String),
    #[error("Parsing Error while building module tree.\n{0}")]
    ParseError(#[from] ParsingError),
    #[error("Cannot insert module with empty path.")]
    InsertNodeError,
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
}
