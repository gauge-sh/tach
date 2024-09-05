use std::io;
use thiserror::Error;

use crate::filesystem::FileSystemError;
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
    #[error("TOML serializing error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Missing field in TOML: {0}")]
    MissingField(String),
}

pub type Result<T> = std::result::Result<T, ParsingError>;

#[derive(Error, Debug)]
pub enum ModuleTreeError {
    #[error(
        "Failed to build module tree. The following modules were defined more than once: {0:?}"
    )]
    DuplicateModules(Vec<String>),
    #[error("Circular dependency detected: {0:?}")]
    CircularDependency(Vec<String>),
    #[error("Parsing Error while building module tree.\n{0}")]
    ParseError(#[from] ParsingError),
}
