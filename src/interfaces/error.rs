use std::io;
use thiserror::Error;

use crate::python::error::ParsingError;

#[derive(Error, Debug)]
pub enum InterfaceError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Parsing error: {0}")]
    Parsing(#[from] ParsingError),
}
