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
}
