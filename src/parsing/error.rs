use std::io;
use thiserror::Error;

use crate::filesystem::FileSystemError;
use crate::resolvers::SourceRootResolverError;
#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] FileSystemError),
    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("Missing field in TOML: {0}")]
    MissingField(String),
    #[error("Module path error: {0}")]
    ModulePath(String),
    #[error("Source root resolution error: {0}")]
    SourceRootResolution(#[from] SourceRootResolverError),
}
