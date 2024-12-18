use lsp_types::OneOf;
use lsp_types::{
    request::GotoDefinition, GotoDefinitionResponse, InitializeParams, ServerCapabilities,
};
use std::path::PathBuf;

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};

use crate::core::config;

use super::error::ServerError;

pub struct LSPServer {
    project_root: PathBuf,
    project_config: config::ProjectConfig,
}

impl LSPServer {
    pub fn new(project_root: PathBuf, project_config: config::ProjectConfig) -> Self {
        Self {
            project_root,
            project_config,
        }
    }

    pub fn run(&self) -> Result<(), ServerError> {
        // Note that  we must have our logging only write out to stderr.
        eprintln!("starting generic LSP server");

        // Create the transport. Includes the stdio (stdin and stdout) versions but this could
        // also be implemented to use sockets or HTTP.
        let (connection, io_threads) = Connection::stdio();
        eprintln!("connection made?");

        // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
        let server_capabilities = serde_json::to_value(&ServerCapabilities {
            definition_provider: Some(OneOf::Left(true)),
            ..Default::default()
        })
        .unwrap();
        eprintln!("server capabilities: {server_capabilities:?}");
        let initialization_params = match connection.initialize(server_capabilities) {
            Ok(it) => it,
            Err(e) => {
                if e.channel_is_disconnected() {
                    io_threads.join()?;
                }
                return Err(e.into());
            }
        };
        eprintln!("initialization params: {initialization_params:?}");
        self.main_loop(connection, initialization_params)?;
        io_threads.join()?;

        // Shut down gracefully.
        eprintln!("shutting down server");
        Ok(())
    }

    fn main_loop(
        &self,
        connection: Connection,
        params: serde_json::Value,
    ) -> Result<(), ServerError> {
        let _params: InitializeParams = serde_json::from_value(params).unwrap();
        eprintln!("starting example main loop");
        for msg in &connection.receiver {
            eprintln!("got msg: {msg:?}");
            match msg {
                Message::Request(req) => {
                    if connection.handle_shutdown(&req)? {
                        return Ok(());
                    }
                    eprintln!("got request: {req:?}");
                    match cast::<GotoDefinition>(req) {
                        Ok((id, params)) => {
                            eprintln!("got gotoDefinition request #{id}: {params:?}");
                            let result = Some(GotoDefinitionResponse::Array(Vec::new()));
                            let result = serde_json::to_value(&result).unwrap();
                            let resp = Response {
                                id,
                                result: Some(result),
                                error: None,
                            };
                            connection.sender.send(Message::Response(resp))?;
                            continue;
                        }
                        Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                        Err(ExtractError::MethodMismatch(req)) => req,
                    };
                    // ...
                }
                Message::Response(resp) => {
                    eprintln!("got response: {resp:?}");
                }
                Message::Notification(not) => {
                    eprintln!("got notification: {not:?}");
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
