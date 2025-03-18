use thiserror::Error;

use crate::filesystem::FileSystemError;
use crate::resolvers::SourceRootResolverError;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file does not exist")]
    ConfigDoesNotExist,
    #[error("Failed to build file walker.\n{0}")]
    FileWalker(#[from] FileSystemError),
    #[error("Failed to resolve source roots.\n{0}")]
    SourceRootResolution(#[from] SourceRootResolverError),
}
