use std::{io, path::PathBuf};
use thiserror::Error;

use crate::{core::config, filesystem::FileSystemError};

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] FileSystemError),
}

pub fn run_server(
    project_root: PathBuf,
    project_config: config::ProjectConfig,
) -> Result<(), ServerError> {
    println!("LSP server is not implemented yet");
    Ok(())
}
