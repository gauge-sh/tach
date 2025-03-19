use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuleSetting {
    Error,
    Warn,
    Off,
}

impl RuleSetting {
    // These are just necessary for serde macros
    fn warn() -> Self {
        Self::Warn
    }

    fn is_warn(&self) -> bool {
        *self == Self::Warn
    }

    fn error() -> Self {
        Self::Error
    }

    fn is_error(&self) -> bool {
        *self == Self::Error
    }

    fn off() -> Self {
        Self::Off
    }

    pub fn is_off(&self) -> bool {
        *self == Self::Off
    }
}

impl IntoPy<PyObject> for RuleSetting {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Error => "error".to_object(py),
            Self::Warn => "warn".to_object(py),
            Self::Off => "off".to_object(py),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, module = "tach.extension")]
pub struct RulesConfig {
    #[serde(
        default = "RuleSetting::warn",
        skip_serializing_if = "RuleSetting::is_warn"
    )]
    pub unused_ignore_directives: RuleSetting,
    #[serde(
        default = "RuleSetting::off",
        skip_serializing_if = "RuleSetting::is_off"
    )]
    pub require_ignore_directive_reasons: RuleSetting,
    #[serde(
        default = "RuleSetting::error",
        skip_serializing_if = "RuleSetting::is_error"
    )]
    pub unused_external_dependencies: RuleSetting,
    #[serde(
        default = "RuleSetting::error",
        skip_serializing_if = "RuleSetting::is_error"
    )]
    pub local_imports: RuleSetting,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            unused_ignore_directives: RuleSetting::warn(),
            require_ignore_directive_reasons: RuleSetting::off(),
            unused_external_dependencies: RuleSetting::error(),
            local_imports: RuleSetting::error(),
        }
    }
}
