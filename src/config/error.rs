use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file does not exist")]
    ConfigDoesNotExist,
}
