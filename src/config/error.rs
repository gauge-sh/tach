use thiserror::Error;

use crate::exclusion::PathExclusionError;
use crate::resolvers::SourceRootResolverError;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file does not exist")]
    ConfigDoesNotExist,
    #[error("Failed to handle excluded paths.\n{0}")]
    PathExclusion(#[from] PathExclusionError),
    #[error("Failed to resolve source roots.\n{0}")]
    SourceRootResolution(#[from] SourceRootResolverError),
}
