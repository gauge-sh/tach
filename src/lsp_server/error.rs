use crossbeam_channel::SendError;
use lsp_server::{Message, ProtocolError};
use std::io;
use thiserror::Error;

use crate::filesystem::FileSystemError;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] FileSystemError),
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("Channel error: {0}")]
    Channel(#[from] SendError<Message>),
}
