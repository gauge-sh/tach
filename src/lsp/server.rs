use lsp_types::notification::Notification;
use lsp_types::InitializeParams;
use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use lsp_server::{Connection, ExtractError, Message, Request, RequestId};

use crate::core::config;

use super::error::ServerError;

use crossbeam_channel::{select, unbounded};

pub struct LSPServer {
    project_root: PathBuf,
    project_config: config::ProjectConfig,
}

pub struct ServerHandle {
    shutdown_sender: crossbeam_channel::Sender<()>,
    join_handle: JoinHandle<Result<(), ServerError>>,
}

impl ServerHandle {
    pub fn shutdown(self) -> Result<(), ServerError> {
        self.shutdown_sender.send(())?;
        let _ = self
            .join_handle
            .join()
            .map_err(|_| ServerError::ThreadPanic)?;
        Ok(())
    }
}

impl LSPServer {
    pub fn new(project_root: PathBuf, project_config: config::ProjectConfig) -> Self {
        Self {
            project_root,
            project_config,
        }
    }

    pub fn run(&self) -> Result<(), ServerError> {
        let (shutdown_tx, shutdown_rx) = unbounded();

        ctrlc::set_handler(move || {
            eprintln!("Received Ctrl+C, initiating shutdown...");
            let _ = shutdown_tx.send(());
        })?;

        self.do_run(shutdown_rx)
    }

    pub fn run_in_thread(self) -> Result<ServerHandle, ServerError> {
        let (shutdown_tx, shutdown_rx) = unbounded();
        let cloned_shutdown_tx = shutdown_tx.clone();
        ctrlc::set_handler(move || {
            eprintln!("Received Ctrl+C, initiating shutdown...");
            let _ = cloned_shutdown_tx.send(());
        })?;
        let join_handle = thread::spawn(move || self.do_run(shutdown_rx));

        Ok(ServerHandle {
            shutdown_sender: shutdown_tx,
            join_handle,
        })
    }

    fn server_capabilities(&self) -> lsp_types::ServerCapabilities {
        lsp_types::ServerCapabilities {
            diagnostic_provider: Some(lsp_types::DiagnosticServerCapabilities::Options(
                lsp_types::DiagnosticOptions {
                    identifier: Some("tach".into()),
                    inter_file_dependencies: false,
                    workspace_diagnostics: false,
                    work_done_progress_options: lsp_types::WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                },
            )),
            text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Options(
                lsp_types::TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(lsp_types::TextDocumentSyncKind::INCREMENTAL),
                    save: Some(lsp_types::TextDocumentSyncSaveOptions::Supported(true)),
                    will_save: Some(false),
                    will_save_wait_until: Some(false),
                    ..Default::default()
                },
            )),
            ..Default::default()
        }
    }

    fn do_run(&self, shutdown_rx: crossbeam_channel::Receiver<()>) -> Result<(), ServerError> {
        eprintln!(
            "Starting LSP server @ project root: {}",
            self.project_root.display()
        );

        let (connection, io_threads) = Connection::stdio();
        eprintln!("StdIO connection started");

        let server_capabilities = serde_json::to_value(self.server_capabilities()).unwrap();
        eprintln!("Server capabilities: {server_capabilities:?}");

        let initialization_params = match connection.initialize(server_capabilities) {
            Ok(it) => it,
            Err(e) => {
                if e.channel_is_disconnected() {
                    io_threads.join()?;
                }
                return Err(e.into());
            }
        };
        eprintln!("Initialization params: {initialization_params:?}");

        self.main_loop(connection, initialization_params, shutdown_rx)?;
        io_threads.join()?;

        eprintln!("LSP server shutting down");
        Ok(())
    }

    fn main_loop(
        &self,
        connection: Connection,
        params: serde_json::Value,
        shutdown_rx: crossbeam_channel::Receiver<()>,
    ) -> Result<(), ServerError> {
        let _params: InitializeParams = serde_json::from_value(params).unwrap();
        eprintln!("Starting request handler loop");

        loop {
            select! {
                // Handle LSP messages
                recv(connection.receiver) -> msg => {
                    match msg {
                        Ok(msg) => {
                            eprintln!("Received message: {msg:?}");
                            match msg {
                                Message::Request(req) => {
                                    if connection.handle_shutdown(&req)? {
                                        return Ok(());
                                    }
                                    eprintln!("[Ignored] Received request: {req:?}");
                                }
                                Message::Response(resp) => {
                                    eprintln!("[Ignored] Got response: {resp:?}");
                                }
                                Message::Notification(notification) => {
                                    eprintln!("Received notification: {notification:?}");
                                    match notification.method.as_str() {
                                        lsp_types::notification::DidOpenTextDocument::METHOD => {
                                            eprintln!("Received DidOpen notification");
                                            let data: lsp_types::DidOpenTextDocumentParams = notification.extract(lsp_types::notification::DidOpenTextDocument::METHOD).unwrap();
                                            eprintln!("DidOpen notification data: {data:?}");
                                            // how to actually read the text document
                                        }
                                        lsp_types::notification::DidSaveTextDocument::METHOD => {
                                            eprintln!("Received DidSave notification");
                                            let data: lsp_types::DidSaveTextDocumentParams = notification.extract(lsp_types::notification::DidSaveTextDocument::METHOD).unwrap();
                                            eprintln!("DidSave notification data: {data:?}");
                                        }
                                        lsp_types::notification::DidCloseTextDocument::METHOD => {
                                            eprintln!("Received DidClose notification");
                                            let data: lsp_types::DidCloseTextDocumentParams = notification.extract(lsp_types::notification::DidCloseTextDocument::METHOD).unwrap();
                                            eprintln!("DidClose notification data: {data:?}");
                                        }
                                        _ => {
                                            eprintln!("Received unknown notification: {}", notification.method);
                                        }
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error receiving message: {err:?}");
                            break;
                        }
                    }
                }
                // Handle shutdown signal
                recv(shutdown_rx) -> _ => {
                    eprintln!("Shutdown signal received, exiting main loop");
                    break;
                }
            }
        }
        Ok(())
    }
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}
