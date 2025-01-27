use std::io;

use thiserror::Error;

use crate::exclusion;
use crate::external;
use crate::filesystem as fs;
use crate::imports;
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
}

#[derive(Error, Debug)]
pub enum ExternalCheckError {
    #[error("Parsing error: {0}")]
    Parse(#[from] external::ParsingError),
    #[error("Import parsing error: {0}")]
    ImportParse(#[from] imports::ImportParseError),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] fs::FileSystemError),
}
