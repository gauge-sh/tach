use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

pub const ROOT_MODULE_SENTINEL_TAG: &str = "<root>";

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RootModuleTreatment {
    Allow,
    Forbid,
    #[default]
    Ignore,
    DependenciesOnly,
}

impl RootModuleTreatment {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl IntoPy<PyObject> for RootModuleTreatment {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Allow => "allow".to_object(py),
            Self::Forbid => "forbid".to_object(py),
            Self::Ignore => "ignore".to_object(py),
            Self::DependenciesOnly => "dependenciesonly".to_object(py),
        }
    }
}
