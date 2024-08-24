use pyo3::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use super::error;
use crate::filesystem;

// for serde
fn default_true() -> bool {
    true
}

#[derive(Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct DependencyConfig {
    pub path: String,
    #[serde(default)]
    pub deprecated: bool,
}

#[derive(Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct ModuleConfig {
    pub path: String,
    #[serde(default)]
    pub depends_on: Vec<DependencyConfig>,
    #[serde(default)]
    pub strict: bool,
}

#[derive(Default, Deserialize, Clone)]
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

#[derive(Default, Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct CacheConfig {
    #[serde(default)]
    pub backend: CacheBackend,
    #[serde(default)]
    pub file_dependencies: Vec<String>,
    #[serde(default)]
    pub env_dependencies: Vec<String>,
}

#[derive(Default, Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct ExternalDependencyConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_source_roots() -> Vec<PathBuf> {
    vec![PathBuf::from(".")]
}

#[derive(Deserialize)]
#[pyclass(get_all, module = "tach.extension")]
pub struct ProjectConfig {
    #[serde(default)]
    pub modules: Vec<ModuleConfig>,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub external: ExternalDependencyConfig,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default = "default_source_roots")]
    pub source_roots: Vec<PathBuf>,
    #[serde(default)]
    pub exact: bool,
    #[serde(default)]
    pub disable_logging: bool,
    #[serde(default = "default_true")]
    pub ignore_type_checking_imports: bool,
    #[serde(default)]
    pub forbid_circular_dependencies: bool,
    #[serde(default = "default_true")]
    pub use_regex_matching: bool,
}

pub fn parse_project_config<P: AsRef<Path>>(filepath: P) -> error::Result<ProjectConfig> {
    let content = filesystem::read_file_content(filepath)?;
    let config: ProjectConfig = toml::from_str(&content)?;
    Ok(config)
}
