use std::io;

use thiserror::Error;

use crate::external;
use crate::filesystem as fs;
use crate::interfaces;
use crate::modules;
use crate::processors::import;
use crate::python;

#[derive(Error, Debug)]
pub enum DiagnosticError {
    #[error("Module tree error: {0}")]
    ModuleTree(#[from] modules::error::ModuleTreeError),
    #[error("Interface error: {0}")]
    Interface(#[from] interfaces::error::InterfaceError),
    #[error("Parsing error: {0}")]
    ExternalParse(#[from] external::ParsingError),
    #[error("Python parsing error: {0}")]
    PythonParse(#[from] python::error::ParsingError),
    #[error("Import parsing error: {0}")]
    ImportParse(#[from] import::ImportParseError),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] fs::FileSystemError),
    #[error("Failed to find package for file: {0}")]
    PackageNotFound(String),
}
