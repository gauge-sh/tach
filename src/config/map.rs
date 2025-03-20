use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(default, deny_unknown_fields)]
#[pyclass(module = "tach.extension")]
pub struct MapConfig {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[pyo3(get)]
    pub extra_dependencies: HashMap<String, Vec<String>>,
}
