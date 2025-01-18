use std::iter;
use std::path::{Path, PathBuf};

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::filesystem::file_to_module_path;

use super::interfaces::InterfaceConfig;
use super::modules::{
    default_visibility, deserialize_modules, is_default_visibility, serialize_modules,
    DependencyConfig, ModuleConfig,
};
use super::utils::*;
use crate::parsing::error::ParsingError;

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

impl DomainConfig {
    pub fn with_location(self, location: ConfigLocation) -> LocatedDomainConfig {
        let resolved_modules = self
            .modules
            .iter()
            .map(|module| Some(module.resolve(&location)))
            .chain(iter::once(
                self.root.as_ref().map(|root| root.resolve(&location)),
            ))
            .flatten()
            .collect();
        let resolved_interfaces = self
            .interfaces
            .iter()
            .map(|interface| interface.resolve(&location))
            .collect();
        LocatedDomainConfig {
            config: self,
            location,
            resolved_modules,
            resolved_interfaces,
        }
    }
}

pub const DOMAIN_ROOT_SENTINEL: &str = "<domain_root>";

trait Resolvable<T> {
    fn resolve(&self, location: &ConfigLocation) -> T;
}

impl Resolvable<DependencyConfig> for DependencyConfig {
    fn resolve(&self, location: &ConfigLocation) -> DependencyConfig {
        if self.path.starts_with("//") {
            // Absolute path does not need to be prefixed with the module path
            DependencyConfig {
                path: self.path[2..].to_string(),
                deprecated: self.deprecated,
            }
        } else {
            match self.path.as_str() {
                // Special case for the domain root sentinel, use the module path
                DOMAIN_ROOT_SENTINEL => DependencyConfig {
                    path: location.mod_path.clone(),
                    deprecated: self.deprecated,
                },
                // Relative path needs to be prefixed with the module path
                _ => DependencyConfig {
                    path: format!("{}.{}", location.mod_path, self.path),
                    deprecated: self.deprecated,
                },
            }
        }
    }
}

impl Resolvable<Vec<DependencyConfig>> for Vec<DependencyConfig> {
    fn resolve(&self, location: &ConfigLocation) -> Vec<DependencyConfig> {
        self.iter().map(|dep| dep.resolve(location)).collect()
    }
}

impl Resolvable<ModuleConfig> for DomainRootConfig {
    fn resolve(&self, location: &ConfigLocation) -> ModuleConfig {
        ModuleConfig {
            // Root modules represent the domain itself
            path: location.mod_path.clone(),
            depends_on: self.depends_on.clone().map(|deps| deps.resolve(location)),
            layer: self.layer.clone(),
            visibility: self.visibility.clone(),
            utility: self.utility,
            strict: false,
            unchecked: self.unchecked,
            group_id: None,
        }
    }
}

impl Resolvable<ModuleConfig> for ModuleConfig {
    fn resolve(&self, location: &ConfigLocation) -> ModuleConfig {
        ModuleConfig {
            path: format!("{}.{}", location.mod_path, self.path),
            depends_on: self.depends_on.clone().map(|deps| deps.resolve(location)),
            layer: self.layer.clone(),
            visibility: self.visibility.clone(),
            utility: self.utility,
            strict: false,
            unchecked: self.unchecked,
            group_id: None,
        }
    }
}

impl Resolvable<InterfaceConfig> for InterfaceConfig {
    fn resolve(&self, location: &ConfigLocation) -> InterfaceConfig {
        InterfaceConfig {
            expose: self.expose.clone(),
            from_modules: self
                .from_modules
                .iter()
                .map(|mod_path| match mod_path.as_str() {
                    DOMAIN_ROOT_SENTINEL => location.mod_path.clone(),
                    _ => format!("{}.{}", location.mod_path, mod_path),
                })
                .collect(),
            data_types: self.data_types.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[pyclass(module = "tach.extension")]
pub struct LocatedDomainConfig {
    pub config: DomainConfig,
    pub location: ConfigLocation,
    resolved_modules: Vec<ModuleConfig>,
    resolved_interfaces: Vec<InterfaceConfig>,
}

impl LocatedDomainConfig {
    pub fn modules(&self) -> impl Iterator<Item = &ModuleConfig> {
        self.resolved_modules.iter()
    }

    pub fn interfaces(&self) -> impl Iterator<Item = &InterfaceConfig> {
        self.resolved_interfaces.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[pyclass(module = "tach.extension")]
pub struct ConfigLocation {
    pub path: PathBuf,
    pub mod_path: String,
}

impl ConfigLocation {
    pub fn new(source_roots: &[PathBuf], path: &Path) -> Result<Self, ParsingError> {
        let parent_dir = path.parent().unwrap();
        let mod_path = file_to_module_path(source_roots, parent_dir)
            .map_err(|e| ParsingError::ModulePath(e.to_string()))?;
        Ok(Self {
            path: path.to_path_buf(),
            mod_path,
        })
    }
}
