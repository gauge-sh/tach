use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::{InitializeParams, Uri};
use std::path::{PathBuf, MAIN_SEPARATOR_STR};
use std::thread::JoinHandle;

use lsp_server::{Connection, Message, Notification as NotificationMessage, RequestId};

use crate::commands::check::{check_external, check_internal};
use crate::config;
use crate::diagnostics::{Diagnostic, Severity};
use crate::interrupt::{check_interrupt, get_interrupt_channel};

use super::error::ServerError;

use crossbeam_channel::select;

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

impl From<Severity> for lsp_types::DiagnosticSeverity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
            Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        }
    }
}

impl From<Diagnostic> for Option<lsp_types::Diagnostic> {
    fn from(diag: Diagnostic) -> Self {
        match diag {
            Diagnostic::Global { .. } => None,
            Diagnostic::Located { line_number, .. } => Some(lsp_types::Diagnostic {
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: (line_number - 1) as u32,
                        character: 0,
                    },
                    end: lsp_types::Position {
                        line: (line_number - 1) as u32,
                        character: 99999,
                    },
                },
                severity: Some(diag.severity().into()),
                source: Some("tach".to_string()),
                message: diag.details().to_string(),
                ..Default::default()
            }),
        }
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
        eprintln!(
            "Starting LSP server @ project root: {}",
            self.project_root.display()
        );

        let (connection, io_threads) = Connection::stdio();
        eprintln!("StdIO connection started");

        let (id, params) = connection
            .initialize_start_while(|| check_interrupt().is_ok())
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

        self.main_loop(connection, params)?;
        io_threads.join()?;

        eprintln!("LSP server shutting down");
        Ok(())
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
                },
            )),
            ..Default::default()
        }
    }

    fn filter_diagnostics_results<'a, I: IntoIterator<Item = Diagnostic> + 'a>(
        &'a self,
        results: I,
        uri_pathbuf: &'a PathBuf,
    ) -> impl Iterator<Item = lsp_types::Diagnostic> + 'a {
        results.into_iter().filter_map(|e| {
            if let Some(file_path) = e.file_path() {
                if *uri_pathbuf == self.project_root.join(file_path) {
                    return e.into();
                }
            }
            None
        })
    }

    fn lint_for_diagnostics(
        &self,
        uri: Uri,
    ) -> Result<lsp_types::PublishDiagnosticsParams, ServerError> {
        let uri_pathbuf = uri_to_path(&uri);
        eprintln!("Linting for diagnostics: {uri_pathbuf:?}");
        eprintln!("Project root: {}", self.project_root.display());

        let check_result = check_internal(&self.project_root, &self.project_config, true, true)?;
        let check_external_result = check_external(&self.project_root, &self.project_config)?;

        let diagnostics = self
            .filter_diagnostics_results(
                check_result.into_iter().chain(check_external_result),
                &uri_pathbuf,
            )
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
    ) -> Result<(), ServerError> {
        let _params: InitializeParams = serde_json::from_value(params).unwrap();
        eprintln!("Starting request handler loop");
        let interrupt_channel = get_interrupt_channel();

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
                recv(interrupt_channel) -> _ => {
                    eprintln!("Shutdown signal received, exiting main loop");
                    break;
                }
            }
        }
        Ok(())
    }
}
