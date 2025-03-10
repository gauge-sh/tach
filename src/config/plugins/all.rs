use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use super::django::DjangoConfig;

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, module = "tach.extension")]
pub struct PluginsConfig {
    #[serde(default)]
    pub django: Option<DjangoConfig>,
}
