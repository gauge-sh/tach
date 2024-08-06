use pyo3::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use super::error;
use crate::filesystem;

#[derive(Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct DependencyConfig {
    pub path: String,
    pub deprecated: bool,
}

#[derive(Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct ModuleConfig {
    pub path: String,
    pub depends_on: Vec<DependencyConfig>,
    pub strict: bool,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CacheBackend {
    Disk,
}

impl IntoPy<PyObject> for CacheBackend {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            CacheBackend::Disk => "disk".to_object(py),
        }
    }
}

#[derive(Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct CacheConfig {
    pub backend: CacheBackend,
    pub file_dependencies: Vec<String>,
    pub env_dependencies: Vec<String>,
}

#[derive(Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct ExternalDependencyConfig {
    pub exclude: Vec<String>,
}

#[derive(Deserialize)]
#[pyclass(get_all, module = "tach.extension")]
pub struct ProjectConfig {
    pub modules: Vec<ModuleConfig>,
    pub cache: CacheConfig,
    pub external: ExternalDependencyConfig,
    pub exclude: Vec<String>,
    pub source_roots: Vec<PathBuf>,
    pub exact: bool,
    pub disable_logging: bool,
    pub ignore_type_checking_imports: bool,
    pub forbid_circular_dependencies: bool,
}

pub fn parse_project_config<P: AsRef<Path>>(filepath: P) -> error::Result<ProjectConfig> {
    let content = filesystem::read_file_content(filepath)?;
    let config: ProjectConfig = toml::from_str(&content)?;
    Ok(config)
}
