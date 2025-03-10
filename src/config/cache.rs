use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use super::utils;

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheBackend {
    #[default]
    Disk,
}

impl IntoPy<PyObject> for CacheBackend {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Disk => "disk".to_object(py),
        }
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, module = "tach.extension")]
pub struct CacheConfig {
    #[serde(default, skip_serializing_if = "utils::is_default")]
    pub backend: CacheBackend,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_dependencies: Vec<String>,
}
