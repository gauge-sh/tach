use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use super::interfaces::InterfaceConfig;
use super::modules::{
    default_visibility, deserialize_modules, is_default_visibility, serialize_modules,
    DependencyConfig, ModuleConfig,
};
use super::utils::*;

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, module = "tach.extension")]
pub struct DomainRootConfig {
    #[serde(default)]
    #[pyo3(set)]
    pub depends_on: Option<Vec<DependencyConfig>>,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(
        default = "default_visibility",
        skip_serializing_if = "is_default_visibility"
    )]
    pub visibility: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub utility: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub unchecked: bool,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
#[pyclass(get_all, module = "tach.extension")]
pub struct DomainConfig {
    #[serde(default)]
    pub root: Option<DomainRootConfig>,
    #[serde(
        default,
        deserialize_with = "deserialize_modules",
        serialize_with = "serialize_modules"
    )]
    pub modules: Vec<ModuleConfig>,
    #[serde(default)]
    pub interfaces: Vec<InterfaceConfig>,
}
