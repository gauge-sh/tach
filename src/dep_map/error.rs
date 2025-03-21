use std::io;
use thiserror::Error;

use crate::{
    filesystem, processors::import::ImportParseError, python::error::ParsingError,
    resolvers::SourceRootResolverError,
};

#[derive(Error, Debug)]
pub enum DependentMapError {
    #[error("Concurrency error occurred.")]
    Concurrency,
    #[error("I/O error occurred.\n{0}")]
    Io(#[from] io::Error),
    #[error("Failed to parse project imports.\n{0}")]
    Filesystem(#[from] filesystem::FileSystemError),
    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),
    #[error("File not found in dependent map: '{0}'")]
    FileNotFound(String),
    #[error("Parsing error: {0}")]
    Parsing(#[from] ParsingError),
    #[error("Import parsing error: {0}")]
    ImportParsing(#[from] ImportParseError),
    #[error("Source root resolution error: {0}")]
    SourceRootResolution(#[from] SourceRootResolverError),
}

pub type Result<T> = std::result::Result<T, DependentMapError>;
