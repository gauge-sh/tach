use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::cache::CacheConfig;
use super::external::ExternalDependencyConfig;
use super::interfaces::InterfaceConfig;
use super::modules::{deserialize_modules, serialize_modules, DependencyConfig, ModuleConfig};
use super::root_module::RootModuleTreatment;
use super::rules::RulesConfig;
use super::utils::*;

#[derive(Default, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct UnusedDependencies {
    pub path: String,
    pub dependencies: Vec<DependencyConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
#[pyclass(get_all, module = "tach.extension")]
pub struct ProjectConfig {
    #[serde(
        default,
        deserialize_with = "deserialize_modules",
        serialize_with = "serialize_modules"
    )]
    pub modules: Vec<ModuleConfig>,
    #[serde(default)]
    pub interfaces: Vec<InterfaceConfig>,
    #[serde(default, skip_serializing_if = "is_empty")]
    pub layers: Vec<String>,
    #[serde(default, skip_serializing_if = "CacheConfig::is_default")]
    pub cache: CacheConfig,
    #[serde(default, skip_serializing_if = "ExternalDependencyConfig::is_default")]
    pub external: ExternalDependencyConfig,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default = "default_source_roots")]
    #[pyo3(set)]
    pub source_roots: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub exact: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_logging: bool,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    #[pyo3(set)]
    pub ignore_type_checking_imports: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub include_string_imports: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub forbid_circular_dependencies: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_regex_matching: bool,
    #[serde(default, skip_serializing_if = "RootModuleTreatment::is_default")]
    pub root_module: RootModuleTreatment,
    #[serde(default, skip_serializing_if = "RulesConfig::is_default")]
    pub rules: RulesConfig,
}

pub fn default_source_roots() -> Vec<PathBuf> {
    vec![PathBuf::from(".")]
}

pub const DEFAULT_EXCLUDE_PATHS: [&str; 5] = [
    "**/tests",
    "**/docs",
    "**/*__pycache__",
    "**/*egg-info",
    "**/venv",
];

pub fn default_excludes() -> Vec<String> {
    DEFAULT_EXCLUDE_PATHS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            modules: Default::default(),
            interfaces: Default::default(),
            layers: Default::default(),
            cache: Default::default(),
            external: Default::default(),
            exclude: default_excludes(),
            source_roots: default_source_roots(),
            exact: Default::default(),
            disable_logging: Default::default(),
            ignore_type_checking_imports: true,
            include_string_imports: Default::default(),
            forbid_circular_dependencies: Default::default(),
            use_regex_matching: Default::default(),
            root_module: Default::default(),
            rules: Default::default(),
        }
    }
}

impl ProjectConfig {
    fn dependencies_for_module(&self, module: &str) -> Option<&Vec<DependencyConfig>> {
        self.modules
            .iter()
            .find(|mod_config| mod_config.path == module)
            .map(|mod_config| mod_config.depends_on.as_ref())?
    }
    pub fn prepend_roots(&self, project_root: &Path) -> Vec<PathBuf> {
        // don't prepend if root is "."
        self.source_roots
            .iter()
            .map(|root| {
                if root.display().to_string() == "." {
                    project_root.to_path_buf()
                } else {
                    project_root.join(root)
                }
            })
            .collect()
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

    pub fn utility_paths(&self) -> Vec<String> {
        self.modules
            .iter()
            .filter(|module| module.utility)
            .map(|module| module.path.clone())
            .collect()
    }

    pub fn with_modules(&self, modules: Vec<ModuleConfig>) -> Self {
        Self {
            modules,
            interfaces: self.interfaces.clone(),
            layers: self.layers.clone(),
            cache: self.cache.clone(),
            external: self.external.clone(),
            exclude: self.exclude.clone(),
            source_roots: self.source_roots.clone(),
            exact: self.exact,
            disable_logging: self.disable_logging,
            ignore_type_checking_imports: self.ignore_type_checking_imports,
            include_string_imports: self.include_string_imports,
            forbid_circular_dependencies: self.forbid_circular_dependencies,
            use_regex_matching: self.use_regex_matching,
            root_module: self.root_module.clone(),
            rules: self.rules.clone(),
        }
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
                if let Some(deps) = original_module.depends_on.as_mut() {
                    deps.retain(|dep| new_module_paths.contains(&dep.path))
                }
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

    pub fn mark_utilities(&mut self, utility_paths: Vec<String>) {
        for module in &mut self.modules {
            module.utility = utility_paths.contains(&module.path);
        }
    }

    pub fn add_dependency_to_module(&mut self, module: &str, dependency: DependencyConfig) {
        if let Some(module_config) = self
            .modules
            .iter_mut()
            .find(|mod_config| mod_config.path == module)
        {
            match &mut module_config.depends_on {
                Some(depends_on) => {
                    if !depends_on.iter().any(|dep| dep.path == dependency.path) {
                        depends_on.push(dependency);
                    }
                }
                None => module_config.depends_on = Some(vec![dependency]),
            }
        } else {
            self.modules.push(ModuleConfig {
                path: module.to_string(),
                depends_on: Some(vec![dependency]),
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
                    dependencies: module_config.depends_on.clone().unwrap_or_default(),
                });
                continue;
            }

            let own_module_dependency_paths: HashSet<&String> = self
                .dependencies_for_module(&module_config.path)
                .map(|deps| deps.iter().map(|dep| &dep.path).collect())
                .unwrap_or_default();

            let current_dependency_paths: HashSet<&String> = module_config
                .dependencies_iter()
                .map(|dep| &dep.path)
                .collect();

            let extra_dependency_paths: HashSet<&&String> = current_dependency_paths
                .difference(&own_module_dependency_paths)
                .collect();

            if !extra_dependency_paths.is_empty() {
                let extra_dependencies: Vec<DependencyConfig> = module_config
                    .dependencies_iter()
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
