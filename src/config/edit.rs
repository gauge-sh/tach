use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigEdit {
    CreateModule { path: String },
    DeleteModule { path: String },
    MarkModuleAsUtility { path: String },
    UnmarkModuleAsUtility { path: String },
    AddDependency { path: String, dependency: String },
    RemoveDependency { path: String, dependency: String },
    AddSourceRoot { filepath: PathBuf },
    RemoveSourceRoot { filepath: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum EditError {
    #[error("Edit not applicable")]
    NotApplicable,
    #[error("Module not found")]
    ModuleNotFound,
    #[error("Module already exists")]
    ModuleAlreadyExists,
    #[error("Failed to parse config")]
    ParsingFailed,
    #[error("Failed to write to disk")]
    DiskWriteFailed,
    #[error("Config file does not exist")]
    ConfigDoesNotExist,
    #[error("Edit not implemented: {0}")]
    NotImplemented(String),
}

pub trait ConfigEditor {
    fn enqueue_edit(&mut self, edit: &ConfigEdit) -> Result<(), EditError>;
    fn apply_edits(&mut self) -> Result<(), EditError>;
}
