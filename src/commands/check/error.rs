use thiserror::Error;

use crate::diagnostics::DiagnosticError;
use crate::exclusion;
use crate::filesystem as fs;
use crate::interfaces::error::InterfaceError;
use crate::modules;

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("The path {0} is not a valid directory.")]
    InvalidDirectory(String),
    #[error("No checks enabled.")]
    NoChecksEnabled(),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] fs::FileSystemError),
    #[error("Module tree error: {0}")]
    ModuleTree(#[from] modules::error::ModuleTreeError),
    #[error("Exclusion error: {0}")]
    Exclusion(#[from] exclusion::PathExclusionError),
    #[error("Interface error: {0}")]
    Interface(#[from] InterfaceError),
    #[error("Operation cancelled by user")]
    Interrupt,
    #[error("Diagnostic error: {0}")]
    Diagnostic(#[from] DiagnosticError),
}
