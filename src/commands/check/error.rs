use thiserror::Error;

use crate::diagnostics::DiagnosticError;
use crate::filesystem as fs;
use crate::interfaces::error::InterfaceError;
use crate::modules;
use crate::resolvers;

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
    #[error("Interface error: {0}")]
    Interface(#[from] InterfaceError),
    #[error("Operation cancelled by user")]
    Interrupt,
    #[error("Diagnostic error: {0}")]
    Diagnostic(#[from] DiagnosticError),
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Package resolution error: {0}")]
    PackageResolution(#[from] resolvers::PackageResolutionError),
    #[error("Source root resolution error: {0}")]
    SourceRootResolution(#[from] resolvers::SourceRootResolverError),
}
