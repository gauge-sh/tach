use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use super::interfaces::InterfaceConfig;
use super::modules::{deserialize_modules, serialize_modules, ModuleConfig};

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
#[pyclass(get_all, module = "tach.extension")]
pub struct DomainConfig {
    #[serde(default)]
    pub root: Option<ModuleConfig>,
    #[serde(
        default,
        deserialize_with = "deserialize_modules",
        serialize_with = "serialize_modules"
    )]
    pub modules: Vec<ModuleConfig>,
    #[serde(default)]
    pub interfaces: Vec<InterfaceConfig>,
}
