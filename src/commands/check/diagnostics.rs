use std::{fmt::Display, path::PathBuf};

use pyo3::prelude::*;
use serde::Serialize;
use thiserror::Error;

use crate::config::RuleSetting;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[pyclass(eq, eq_int, module = "tach.extension")]
pub enum Severity {
    Error,
    Warning,
}

impl TryFrom<&RuleSetting> for Severity {
    type Error = ();

    fn try_from(setting: &RuleSetting) -> Result<Self, <Self as TryFrom<&RuleSetting>>::Error> {
        match setting {
            RuleSetting::Error => Ok(Self::Error),
            RuleSetting::Warn => Ok(Self::Warning),
            RuleSetting::Off => Err(()),
        }
    }
}

#[derive(Error, Debug, Clone, Serialize, PartialEq)]
#[pyclass(module = "tach.extension")]
pub enum ConfigurationDiagnostic {
    #[error("Module containing '{file_mod_path}' not found in project.")]
    ModuleNotFound { file_mod_path: String },

    #[error("Could not find module configuration.")]
    ModuleConfigNotFound(),

    #[error("Layer '{layer}' is not defined in the project.")]
    UnknownLayer { layer: String },

    #[error("No first-party imports were found. You may need to use 'tach mod' to update your Python source roots. Docs: https://docs.gauge.sh/usage/configuration#source-roots")]
    NoFirstPartyImportsFound(),

    #[error("Unexpected error: No checks were enabled.")]
    NoChecksEnabled(),

    #[error("Skipped '{file_path}' due to a syntax error.")]
    SkippedFileSyntaxError { file_path: String },

    #[error("Skipped '{file_path}' due to an I/O error.")]
    SkippedFileIoError { file_path: String },
}

#[derive(Error, Debug, Clone, Serialize, PartialEq)]
#[pyclass(module = "tach.extension")]
pub enum CodeDiagnostic {
    #[error("Module '{definition_module}' has a defined public interface. Only imports from the public interface of this module are allowed. The import '{import_mod_path}' (in module '{usage_module}') is not public.")]
    PrivateImport {
        import_mod_path: String,
        definition_module: String,
        usage_module: String,
    },

    #[error("The import '{import_mod_path}' (from module '{definition_module}') matches an interface but does not match the expected data type ('{expected_data_type}').")]
    InvalidDataTypeExport {
        import_mod_path: String,
        definition_module: String,
        usage_module: String,
        expected_data_type: String,
    },

    #[error("Cannot import '{import_mod_path}'. Module '{usage_module}' cannot depend on '{definition_module}'.")]
    InvalidImport {
        import_mod_path: String,
        usage_module: String,
        definition_module: String,
    },

    #[error("Import '{import_mod_path}' is deprecated. Module '{usage_module}' should not depend on '{definition_module}'.")]
    DeprecatedImport {
        import_mod_path: String,
        usage_module: String,
        definition_module: String,
    },

    #[error("Cannot import '{import_mod_path}'. Layer '{usage_layer}' ('{usage_module}') is lower than layer '{definition_layer}' ('{definition_module}').")]
    LayerViolation {
        import_mod_path: String,
        usage_module: String,
        usage_layer: String,
        definition_module: String,
        definition_layer: String,
    },

    #[error("Import '{import_mod_path}' is unnecessarily ignored by a directive.")]
    UnnecessarilyIgnoredImport { import_mod_path: String },

    #[error("Ignore directive is unused.")]
    UnusedIgnoreDirective(),

    #[error("Import '{import_mod_path}' is ignored without providing a reason.")]
    MissingIgnoreDirectiveReason { import_mod_path: String },

    #[error("Import '{import_mod_path}' does not match any declared dependency.")]
    UndeclaredExternalDependency { import_mod_path: String },

    #[error("External package '{package_module_name}' is not used.")]
    UnusedExternalDependency { package_module_name: String },
}

impl CodeDiagnostic {
    pub fn import_mod_path(&self) -> Option<&str> {
        match self {
            CodeDiagnostic::PrivateImport {
                import_mod_path, ..
            }
            | CodeDiagnostic::InvalidDataTypeExport {
                import_mod_path, ..
            }
            | CodeDiagnostic::InvalidImport {
                import_mod_path, ..
            }
            | CodeDiagnostic::DeprecatedImport {
                import_mod_path, ..
            }
            | CodeDiagnostic::LayerViolation {
                import_mod_path, ..
            }
            | CodeDiagnostic::UnnecessarilyIgnoredImport {
                import_mod_path, ..
            } => Some(import_mod_path),
            CodeDiagnostic::UnusedIgnoreDirective { .. } => None,
            CodeDiagnostic::MissingIgnoreDirectiveReason { .. } => None,
            CodeDiagnostic::UndeclaredExternalDependency { .. } => None,
            CodeDiagnostic::UnusedExternalDependency { .. } => None,
        }
    }

    pub fn usage_module(&self) -> Option<&str> {
        match self {
            CodeDiagnostic::PrivateImport { usage_module, .. }
            | CodeDiagnostic::InvalidDataTypeExport { usage_module, .. }
            | CodeDiagnostic::InvalidImport { usage_module, .. }
            | CodeDiagnostic::DeprecatedImport { usage_module, .. }
            | CodeDiagnostic::LayerViolation { usage_module, .. } => Some(usage_module),
            _ => None,
        }
    }

    pub fn definition_module(&self) -> Option<&str> {
        match self {
            CodeDiagnostic::PrivateImport {
                definition_module, ..
            }
            | CodeDiagnostic::InvalidDataTypeExport {
                definition_module, ..
            }
            | CodeDiagnostic::InvalidImport {
                definition_module, ..
            }
            | CodeDiagnostic::DeprecatedImport {
                definition_module, ..
            }
            | CodeDiagnostic::LayerViolation {
                definition_module, ..
            } => Some(definition_module),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[pyclass(module = "tach.extension")]
pub enum DiagnosticDetails {
    Code(CodeDiagnostic),
    Configuration(ConfigurationDiagnostic),
}

impl Display for DiagnosticDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticDetails::Code(code) => write!(f, "{}", code),
            DiagnosticDetails::Configuration(config) => write!(f, "{}", config),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[pyclass(module = "tach.extension")]
pub enum Diagnostic {
    Global {
        severity: Severity,
        details: DiagnosticDetails,
    },
    Located {
        file_path: PathBuf,
        line_number: usize,
        severity: Severity,
        details: DiagnosticDetails,
    },
}

impl Diagnostic {
    pub fn new_global(severity: Severity, details: DiagnosticDetails) -> Self {
        Self::Global { severity, details }
    }

    pub fn new_located(
        severity: Severity,
        details: DiagnosticDetails,
        file_path: PathBuf,
        line_number: usize,
    ) -> Self {
        Self::Located {
            severity,
            details,
            file_path,
            line_number,
        }
    }

    pub fn into_located(self, file_path: PathBuf, line_number: usize) -> Self {
        match self {
            Self::Global { severity, details } => {
                Self::new_located(severity, details, file_path, line_number)
            }
            Self::Located { .. } => self,
        }
    }

    pub fn new_located_error(
        file_path: PathBuf,
        line_number: usize,
        details: DiagnosticDetails,
    ) -> Self {
        Self::Located {
            file_path,
            line_number,
            severity: Severity::Error,
            details,
        }
    }

    pub fn new_located_warning(
        file_path: PathBuf,
        line_number: usize,
        details: DiagnosticDetails,
    ) -> Self {
        Self::Located {
            file_path,
            line_number,
            severity: Severity::Warning,
            details,
        }
    }

    pub fn new_global_error(details: DiagnosticDetails) -> Self {
        Self::Global {
            severity: Severity::Error,
            details,
        }
    }

    pub fn new_global_warning(details: DiagnosticDetails) -> Self {
        Self::Global {
            severity: Severity::Warning,
            details,
        }
    }

    pub fn details(&self) -> &DiagnosticDetails {
        match self {
            Self::Global { details, .. } => details,
            Self::Located { details, .. } => details,
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            Self::Global { severity, .. } => severity.clone(),
            Self::Located { severity, .. } => severity.clone(),
        }
    }

    pub fn file_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Global { .. } => None,
            Self::Located { file_path, .. } => Some(file_path),
        }
    }

    pub fn line_number(&self) -> Option<usize> {
        match self {
            Self::Global { .. } => None,
            Self::Located { line_number, .. } => Some(*line_number),
        }
    }

    pub fn import_mod_path(&self) -> Option<&str> {
        match self.details() {
            DiagnosticDetails::Code(details) => details.import_mod_path(),
            _ => None,
        }
    }

    pub fn usage_module(&self) -> Option<&str> {
        match self.details() {
            DiagnosticDetails::Code(details) => details.usage_module(),
            _ => None,
        }
    }

    pub fn definition_module(&self) -> Option<&str> {
        match self.details() {
            DiagnosticDetails::Code(details) => details.definition_module(),
            _ => None,
        }
    }
}

#[pymethods]
impl Diagnostic {
    pub fn is_code(&self) -> bool {
        matches!(self.details(), DiagnosticDetails::Code { .. })
    }

    pub fn is_configuration(&self) -> bool {
        matches!(self.details(), DiagnosticDetails::Configuration { .. })
    }

    pub fn is_dependency_error(&self) -> bool {
        matches!(
            self.details(),
            DiagnosticDetails::Code(CodeDiagnostic::InvalidImport { .. })
                | DiagnosticDetails::Code(CodeDiagnostic::DeprecatedImport { .. })
                | DiagnosticDetails::Code(CodeDiagnostic::LayerViolation { .. })
        )
    }

    pub fn is_interface_error(&self) -> bool {
        matches!(
            self.details(),
            DiagnosticDetails::Code(CodeDiagnostic::PrivateImport { .. })
                | DiagnosticDetails::Code(CodeDiagnostic::InvalidDataTypeExport { .. })
        )
    }

    pub fn is_deprecated(&self) -> bool {
        matches!(
            self.details(),
            DiagnosticDetails::Code(CodeDiagnostic::DeprecatedImport { .. })
        )
    }

    pub fn is_error(&self) -> bool {
        matches!(self.severity(), Severity::Error)
    }

    pub fn is_warning(&self) -> bool {
        matches!(self.severity(), Severity::Warning)
    }

    #[pyo3(name = "to_string")]
    pub fn to_pystring(&self) -> String {
        match self {
            Self::Global { details, .. } => details.to_string(),
            Self::Located { details, .. } => details.to_string(),
        }
    }

    pub fn pyfile_path(&self) -> Option<String> {
        self.file_path()
            .map(|path| path.to_string_lossy().to_string())
    }

    pub fn pyline_number(&self) -> Option<usize> {
        self.line_number()
    }
}

#[pyfunction(signature = (diagnostics, pretty_print = false))]
pub fn serialize_diagnostics_json(diagnostics: Vec<Diagnostic>, pretty_print: bool) -> String {
    if pretty_print {
        serde_json::to_string_pretty(&diagnostics).unwrap()
    } else {
        serde_json::to_string(&diagnostics).unwrap()
    }
}
