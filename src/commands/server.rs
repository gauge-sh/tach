use std::path::PathBuf;

use crate::core::config;
use crate::lsp::{error::ServerError, server::LSPServer};

pub fn run_server(
    project_root: PathBuf,
    project_config: config::ProjectConfig,
) -> Result<(), ServerError> {
    let server = LSPServer::new(project_root, project_config);
    server.run()
}
