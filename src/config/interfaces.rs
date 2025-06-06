use std::fmt::Display;
use std::ops::Not;

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use super::utils;

#[derive(Debug, Serialize, Default, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InterfaceDataTypes {
    #[default]
    All,
    Primitive,
}

impl Display for InterfaceDataTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Primitive => write!(f, "primitive"),
        }
    }
}

impl IntoPy<PyObject> for InterfaceDataTypes {
    fn into_py(self, py: Python) -> PyObject {
        self.to_string().to_object(py)
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
#[pyclass(get_all, module = "tach.extension")]
pub struct InterfaceConfig {
    pub expose: Vec<String>,
    #[serde(
        rename = "from",
        default = "default_from_modules",
        skip_serializing_if = "is_default_from_modules"
    )]
    pub from_modules: Vec<String>,
    #[serde(default)]
    pub visibility: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "utils::is_default")]
    pub data_types: InterfaceDataTypes,
    #[serde(default, skip_serializing_if = "Not::not")]
    pub exclusive: bool,
}

fn default_from_modules() -> Vec<String> {
    vec![".*".to_string()]
}

fn is_default_from_modules(value: &Vec<String>) -> bool {
    value == &default_from_modules()
}
