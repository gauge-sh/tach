use std::path::PathBuf;

use pyo3::exceptions::PyValueError;
use pyo3::{pyclass, pymethods, PyResult};
use rayon::prelude::*;
use serde::Serialize;
use thiserror::Error;

use crate::interrupt::check_interrupt;

#[derive(Debug, Clone, Serialize)]
#[pyclass(get_all, module = "tach.extension")]
pub struct BoundaryError {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub import_mod_path: String,
    pub error_info: ImportCheckError,
}

impl BoundaryError {
    pub fn is_deprecated(&self) -> bool {
        self.error_info.is_deprecated()
    }
}

#[derive(Debug, Default, Serialize)]
#[pyclass(get_all, module = "tach.extension")]
pub struct CheckDiagnostics {
    pub errors: Vec<BoundaryError>,
    pub deprecated_warnings: Vec<BoundaryError>,
    pub warnings: Vec<String>,
}

#[pymethods]
impl CheckDiagnostics {
    #[pyo3(signature = (pretty_print = false))]
    fn serialize_json(&self, pretty_print: bool) -> PyResult<String> {
        if pretty_print {
            serde_json::to_string_pretty(&self)
                .map_err(|_| PyValueError::new_err("Failed to serialize check results."))
        } else {
            serde_json::to_string(&self)
                .map_err(|_| PyValueError::new_err("Failed to serialize check results."))
        }
    }
}

impl ParallelExtend<CheckDiagnostics> for CheckDiagnostics {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = CheckDiagnostics>,
    {
        // Reduce all diagnostics into a single one in parallel
        let combined =
            par_iter
                .into_par_iter()
                .reduce(CheckDiagnostics::default, |mut acc, item| {
                    if check_interrupt().is_err() {
                        return acc;
                    }
                    acc.errors.extend(item.errors);
                    acc.deprecated_warnings.extend(item.deprecated_warnings);
                    acc.warnings.extend(item.warnings);
                    acc
                });

        if check_interrupt().is_err() {
            return;
        }
        // Extend self with the combined results
        self.errors.extend(combined.errors);
        self.deprecated_warnings
            .extend(combined.deprecated_warnings);
        self.warnings.extend(combined.warnings);
    }
}

#[derive(Error, Debug, Clone, Serialize)]
#[pyclass(module = "tach.extension")]
pub enum ImportCheckError {
    #[error("Module containing '{file_mod_path}' not found in project.")]
    ModuleNotFound { file_mod_path: String },

    #[error("Module '{import_nearest_module_path}' has a defined public interface. Only imports from the public interface of this module are allowed. The import '{import_mod_path}' (in module '{file_nearest_module_path}') is not public.")]
    PrivateImport {
        import_mod_path: String,
        import_nearest_module_path: String,
        file_nearest_module_path: String,
    },

    #[error("The import '{import_mod_path}' (from module '{import_nearest_module_path}') matches an interface but does not match the expected data type ('{expected_data_type}').")]
    InvalidDataTypeExport {
        import_mod_path: String,
        import_nearest_module_path: String,
        expected_data_type: String,
    },

    #[error("Could not find module configuration.")]
    ModuleConfigNotFound(),

    #[error("Cannot import '{import_mod_path}'. Module '{source_module}' cannot depend on '{invalid_module}'.")]
    InvalidImport {
        import_mod_path: String,
        source_module: String,
        invalid_module: String,
    },

    #[error("Import '{import_mod_path}' is deprecated. Module '{source_module}' should not depend on '{invalid_module}'.")]
    DeprecatedImport {
        import_mod_path: String,
        source_module: String,
        invalid_module: String,
    },

    #[error("Cannot import '{import_mod_path}'. Layer '{source_layer}' ('{source_module}') is lower than layer '{invalid_layer}' ('{invalid_module}').")]
    LayerViolation {
        import_mod_path: String,
        source_module: String,
        source_layer: String,
        invalid_module: String,
        invalid_layer: String,
    },

    #[error("Layer '{layer}' is not defined in the project.")]
    UnknownLayer { layer: String },

    #[error("Import '{import_mod_path}' is unnecessarily ignored by a directive.")]
    UnnecessarilyIgnoredImport { import_mod_path: String },

    #[error("Ignore directive is unused.")]
    UnusedIgnoreDirective(),

    #[error("Import '{import_mod_path}' is ignored without providing a reason.")]
    MissingIgnoreDirectiveReason { import_mod_path: String },

    #[error("No checks enabled. At least one of dependencies or interfaces must be enabled.")]
    NoChecksEnabled(),
}

#[pymethods]
impl ImportCheckError {
    pub fn is_dependency_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidImport { .. }
                | Self::DeprecatedImport { .. }
                | Self::LayerViolation { .. }
        )
    }

    pub fn is_interface_error(&self) -> bool {
        matches!(
            self,
            Self::PrivateImport { .. } | Self::InvalidDataTypeExport { .. }
        )
    }

    pub fn source_path(&self) -> Option<&String> {
        match self {
            Self::InvalidImport { source_module, .. } => Some(source_module),
            Self::DeprecatedImport { source_module, .. } => Some(source_module),
            Self::LayerViolation { source_module, .. } => Some(source_module),
            _ => None,
        }
    }

    pub fn invalid_path(&self) -> Option<&String> {
        match self {
            Self::InvalidImport { invalid_module, .. } => Some(invalid_module),
            Self::DeprecatedImport { invalid_module, .. } => Some(invalid_module),
            Self::LayerViolation { invalid_module, .. } => Some(invalid_module),
            _ => None,
        }
    }

    pub fn is_deprecated(&self) -> bool {
        matches!(self, Self::DeprecatedImport { .. })
    }

    pub fn to_pystring(&self) -> String {
        self.to_string()
    }
}
