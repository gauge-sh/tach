use crossbeam_channel::SendError;
use lsp_server::{Message, ProtocolError};
use std::io;
use thiserror::Error;

use crate::check_internal::CheckError;
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
    #[error("Channel error: {0}")]
    ChannelFlag(#[from] crossbeam_channel::SendError<()>),
    #[error("Encountered error while handling shutdown")]
    Shutdown(#[from] ctrlc::Error),
    #[error("Thread panicked")]
    ThreadPanic,
    #[error("Failed to lint files: {0}")]
    Lint(#[from] CheckError),
    #[error("Failed to initialize LSP server")]
    Initialize,
}
