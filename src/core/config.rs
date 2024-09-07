use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::filesystem;
use crate::filesystem::ROOT_MODULE_SENTINEL_TAG;
use crate::parsing;

// for serde
fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[pyclass(get_all, module = "tach.extension")]
pub struct DependencyConfig {
    pub path: String,
    #[serde(default)]
    pub deprecated: bool,
}

impl DependencyConfig {
    pub fn from_deprecated_path(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            deprecated: true,
        }
    }
    pub fn from_undeprecated_path(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            deprecated: false,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, eq, module = "tach.extension")]
pub struct ModuleConfig {
    pub path: String,
    #[serde(default)]
    pub depends_on: Vec<DependencyConfig>,
    #[serde(default)]
    pub strict: bool,
}

#[pymethods]
impl ModuleConfig {
    #[new]
    pub fn new(path: &str, strict: bool) -> Self {
        Self {
            path: path.to_string(),
            depends_on: vec![],
            strict,
        }
    }
    #[staticmethod]
    pub fn new_root_config() -> Self {
        Self::new(ROOT_MODULE_SENTINEL_TAG, false)
    }
    pub fn mod_path(&self) -> String {
        if self.path == ROOT_MODULE_SENTINEL_TAG {
            return ".".to_string();
        }
        self.path.clone()
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct CacheConfig {
    #[serde(default)]
    pub backend: CacheBackend,
    #[serde(default)]
    pub file_dependencies: Vec<String>,
    #[serde(default)]
    pub env_dependencies: Vec<String>,
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct ExternalDependencyConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_source_roots() -> Vec<PathBuf> {
    vec![PathBuf::from(".")]
}

#[derive(Default, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct UnusedDependencies {
    pub path: String,
    pub dependencies: Vec<DependencyConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            modules: Default::default(),
            cache: Default::default(),
            external: Default::default(),
            exclude: Default::default(),
            source_roots: default_source_roots(),
            exact: Default::default(),
            disable_logging: Default::default(),
            ignore_type_checking_imports: default_true(),
            forbid_circular_dependencies: Default::default(),
            use_regex_matching: default_true(),
        }
    }
}

impl ProjectConfig {
    pub fn with_modules(&self, modules: Vec<ModuleConfig>) -> Self {
        Self {
            modules,
            cache: self.cache.clone(),
            external: self.external.clone(),
            exclude: self.exclude.clone(),
            source_roots: self.source_roots.clone(),
            exact: self.exact,
            disable_logging: self.disable_logging,
            ignore_type_checking_imports: self.ignore_type_checking_imports,
            forbid_circular_dependencies: self.forbid_circular_dependencies,
            use_regex_matching: self.forbid_circular_dependencies,
        }
    }

    fn dependencies_for_module(&self, module: &str) -> Option<&Vec<DependencyConfig>> {
        self.modules
            .iter()
            .find(|mod_config| mod_config.path == module)
            .map(|mod_config| &mod_config.depends_on)
    }
}

#[pymethods]
impl ProjectConfig {
    #[new]
    fn new() -> Self {
        ProjectConfig::default()
    }

    fn __str__(&self) -> String {
        format!("{:#?}", self)
    }
    pub fn model_dump_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    pub fn module_paths(&self) -> Vec<String> {
        self.modules
            .iter()
            .map(|module| module.path.clone())
            .collect()
    }

    pub fn set_modules(&mut self, module_paths: Vec<String>) {
        let new_module_paths: HashSet<String> = module_paths.into_iter().collect();
        let mut new_modules: Vec<ModuleConfig> = Vec::new();

        let mut original_modules_by_path: HashMap<String, ModuleConfig> = self
            .modules
            .drain(..)
            .map(|module| (module.path.clone(), module))
            .collect();

        for new_module_path in &new_module_paths {
            if let Some(mut original_module) = original_modules_by_path.remove(new_module_path) {
                original_module
                    .depends_on
                    .retain(|dep| new_module_paths.contains(&dep.path));
                new_modules.push(original_module);
            } else {
                new_modules.push(ModuleConfig {
                    path: new_module_path.to_string(),
                    ..Default::default()
                });
            }
        }

        self.modules = new_modules;
    }

    pub fn add_dependency_to_module(&mut self, module: &str, dependency: DependencyConfig) {
        if let Some(module_config) = self
            .modules
            .iter_mut()
            .find(|mod_config| mod_config.path == module)
        {
            if !module_config
                .depends_on
                .iter()
                .any(|dep| dep.path == dependency.path)
            {
                module_config.depends_on.push(dependency);
            }
        } else {
            self.modules.push(ModuleConfig {
                path: module.to_string(),
                depends_on: vec![dependency],
                ..Default::default()
            });
        }
    }

    pub fn compare_dependencies(&self, other_config: &ProjectConfig) -> Vec<UnusedDependencies> {
        let mut all_unused_dependencies = Vec::new();
        let own_module_paths: HashSet<&String> =
            self.modules.iter().map(|module| &module.path).collect();

        for module_config in &other_config.modules {
            if !own_module_paths.contains(&module_config.path) {
                all_unused_dependencies.push(UnusedDependencies {
                    path: module_config.path.clone(),
                    dependencies: module_config.depends_on.clone(),
                });
                continue;
            }

            let own_module_dependency_paths: HashSet<&String> = self
                .dependencies_for_module(&module_config.path)
                .map(|deps| deps.iter().map(|dep| &dep.path).collect())
                .unwrap_or_default();

            let current_dependency_paths: HashSet<&String> = module_config
                .depends_on
                .iter()
                .map(|dep| &dep.path)
                .collect();

            let extra_dependency_paths: HashSet<&&String> = current_dependency_paths
                .difference(&own_module_dependency_paths)
                .collect();

            if !extra_dependency_paths.is_empty() {
                let extra_dependencies: Vec<DependencyConfig> = module_config
                    .depends_on
                    .iter()
                    .filter(|dep| extra_dependency_paths.contains(&&dep.path))
                    .cloned()
                    .collect();

                all_unused_dependencies.push(UnusedDependencies {
                    path: module_config.path.clone(),
                    dependencies: extra_dependencies,
                });
            }
        }

        all_unused_dependencies
    }
}

pub fn dump_project_config_to_toml(config: &mut ProjectConfig) -> Result<String, toml::ser::Error> {
    config.modules.sort_by(|a, b| {
        if a.path == ROOT_MODULE_SENTINEL_TAG {
            Ordering::Less
        } else if b.path == ROOT_MODULE_SENTINEL_TAG {
            Ordering::Greater
        } else {
            a.path.cmp(&b.path)
        }
    });

    for module in &mut config.modules {
        module.depends_on.sort_by(|a, b| a.path.cmp(&b.path));
    }

    config.exclude.sort();
    config.source_roots.sort();

    toml::to_string(&config)
}

pub fn parse_project_config<P: AsRef<Path>>(filepath: P) -> parsing::error::Result<ProjectConfig> {
    let content = filesystem::read_file_content(filepath)?;
    let config: ProjectConfig = toml::from_str(&content)?;
    Ok(config)
}
