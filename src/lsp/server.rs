use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::{InitializeParams, Uri};
use std::path::{PathBuf, MAIN_SEPARATOR_STR};
use std::thread::{self, JoinHandle};

use lsp_server::{Connection, Message, Notification as NotificationMessage, RequestId};

use crate::check_internal::check;
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

fn uri_to_path(uri: &Uri) -> PathBuf {
    // This assumes that the URI has an absolute file path
    let segments: Vec<_> = uri
        .path()
        .segments()
        .map(|estr| estr.decode().into_string_lossy().into_owned())
        .collect();

    if cfg!(windows) {
        // On Windows, join segments directly (first segment will be like "c:")
        segments.join(MAIN_SEPARATOR_STR).into()
    } else {
        // On POSIX, ensure path starts with /
        format!("/{}", segments.join(MAIN_SEPARATOR_STR)).into()
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

        let (id, params) = connection
            .initialize_start_while(|| shutdown_rx.is_empty())
            .map_err(|_| ServerError::Initialize)?;
        eprintln!("Initialization started with params: {params:?}");

        let server_capabilities = serde_json::json!({
            "capabilities": serde_json::to_value(self.server_capabilities()).unwrap(),
        });
        eprintln!("Server capabilities: {server_capabilities:?}");

        match connection.initialize_finish(id, server_capabilities) {
            Ok(()) => (),
            Err(e) => {
                if e.channel_is_disconnected() {
                    io_threads.join()?;
                }
                return Err(ServerError::Initialize);
            }
        };

        self.main_loop(connection, params, shutdown_rx)?;
        io_threads.join()?;

        eprintln!("LSP server shutting down");
        Ok(())
    }

    fn lint_for_diagnostics(
        &self,
        uri: Uri,
    ) -> Result<lsp_types::PublishDiagnosticsParams, ServerError> {
        let uri_pathbuf = uri_to_path(&uri);
        eprintln!("Linting for diagnostics: {uri_pathbuf:?}");
        eprintln!("Project root: {}", self.project_root.display());

        let check_result = check(
            self.project_root.clone(),
            &self.project_config,
            true,
            true,
            self.project_config.exclude.clone(),
        )?;
        let diagnostics = check_result
            .errors
            .into_iter()
            .filter_map(|e| {
                if self.project_config.source_roots.iter().any(|source_root| {
                    let full_path = self.project_root.join(source_root).join(&e.file_path);
                    uri_pathbuf == full_path
                }) {
                    Some(lsp_types::Diagnostic {
                        range: lsp_types::Range {
                            start: lsp_types::Position {
                                line: (e.line_number - 1) as u32,
                                character: 0,
                            },
                            end: lsp_types::Position {
                                line: (e.line_number - 1) as u32,
                                character: 99999,
                            },
                        },
                        severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                        source: Some("tach".to_string()),
                        message: e.error_info.to_string(),
                        ..Default::default()
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(lsp_types::PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        })
    }

    fn publish_diagnostics(
        &self,
        connection: &Connection,
        params: &lsp_types::PublishDiagnosticsParams,
    ) -> Result<(), ServerError> {
        connection
            .sender
            .send(Message::Notification(NotificationMessage {
                method: lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
                params: serde_json::to_value(params).unwrap(),
            }))?;
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
                            eprintln!("Received message");
                            match msg {
                                Message::Request(req) => {
                                    if connection.handle_shutdown(&req)? {
                                        return Ok(());
                                    }
                                    match req.method.as_str() {
                                        lsp_types::request::DocumentDiagnosticRequest::METHOD => {
                                            eprintln!("Received Diagnostic request");
                                            let (_, data): (RequestId, lsp_types::DocumentDiagnosticParams) = req.extract(lsp_types::request::DocumentDiagnosticRequest::METHOD).unwrap();
                                            let diagnostics = self.lint_for_diagnostics(data.text_document.uri.clone())?;
                                            self.publish_diagnostics(&connection, &diagnostics)?;
                                        }
                                        _ => {
                                            eprintln!("[Ignored] Received request: {:?}", req.method);
                                        }
                                    }
                                }
                                Message::Response(resp) => {
                                    eprintln!("[Ignored] Got response: {:?}", resp.id);
                                }
                                Message::Notification(notification) => {
                                    eprintln!("Received notification: {:?}", notification.method);
                                    match notification.method.as_str() {
                                        lsp_types::notification::DidOpenTextDocument::METHOD => {
                                            eprintln!("Received DidOpen notification");
                                            let data: lsp_types::DidOpenTextDocumentParams = notification.extract(lsp_types::notification::DidOpenTextDocument::METHOD).unwrap();
                                            let diagnostics = self.lint_for_diagnostics(data.text_document.uri.clone())?;
                                            self.publish_diagnostics(&connection, &diagnostics)?;
                                        }
                                        lsp_types::notification::DidSaveTextDocument::METHOD => {
                                            eprintln!("Received DidSave notification");
                                            let data: lsp_types::DidSaveTextDocumentParams = notification.extract(lsp_types::notification::DidSaveTextDocument::METHOD).unwrap();
                                            let diagnostics = self.lint_for_diagnostics(data.text_document.uri.clone())?;
                                            self.publish_diagnostics(&connection, &diagnostics)?;
                                        }
                                        lsp_types::notification::DidCloseTextDocument::METHOD => {
                                            eprintln!("Received DidClose notification");
                                            let data: lsp_types::DidCloseTextDocumentParams = notification.extract(lsp_types::notification::DidCloseTextDocument::METHOD).unwrap();
                                            let diagnostics = lsp_types::PublishDiagnosticsParams {
                                                uri: data.text_document.uri.clone(),
                                                diagnostics: vec![],
                                                version: None,
                                            };
                                            self.publish_diagnostics(&connection, &diagnostics)?;
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
