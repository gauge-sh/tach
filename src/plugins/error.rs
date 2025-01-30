use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin setup failed: {0}")]
    SetupFailed(String),
    #[error("Plugin check failed: {0}")]
    CheckFailed(String),
}
