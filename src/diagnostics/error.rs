use std::io;

use thiserror::Error;

use crate::external;
use crate::filesystem as fs;
use crate::interfaces::error::InterfaceError;
use crate::modules;
use crate::processors::imports;

#[derive(Error, Debug)]
pub enum DiagnosticError {
    #[error("Module tree error: {0}")]
    ModuleTree(#[from] modules::error::ModuleTreeError),
    #[error("Interface error: {0}")]
    Interface(#[from] InterfaceError),
    #[error("Parsing error: {0}")]
    Parse(#[from] external::ParsingError),
    #[error("Import parsing error: {0}")]
    ImportParse(#[from] imports::ImportParseError),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] fs::FileSystemError),
}
