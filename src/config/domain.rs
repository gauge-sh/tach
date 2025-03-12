use std::iter;
use std::ops::Not;
use std::path::{Path, PathBuf};

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::filesystem::file_to_module_path;

use super::edit::{ConfigEdit, ConfigEditor, EditError};
use super::interfaces::InterfaceConfig;
use super::modules::{deserialize_modules, serialize_modules, DependencyConfig, ModuleConfig};
use crate::parsing::error::ParsingError;

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DomainRootConfig {
    #[serde(default)]
    pub depends_on: Option<Vec<DependencyConfig>>,
    #[serde(default)]
    pub cannot_depend_on: Option<Vec<DependencyConfig>>,
    #[serde(default)]
    pub depends_on_external: Option<Vec<String>>,
    #[serde(default)]
    pub cannot_depend_on_external: Option<Vec<String>>,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub visibility: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Not::not")]
    pub utility: bool,
    #[serde(default, skip_serializing_if = "Not::not")]
    pub unchecked: bool,
}

impl DomainRootConfig {
    pub fn with_dependencies_removed(&self) -> Self {
        Self {
            depends_on: Some(vec![]),
            ..self.clone()
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
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
    pub fn with_dependencies_removed(&self) -> Self {
        Self {
            modules: self
                .modules
                .iter()
                .map(|module| module.with_dependencies_removed())
                .collect(),
            root: self
                .root
                .as_ref()
                .map(|root| root.with_dependencies_removed()),
            ..self.clone()
        }
    }

    pub fn with_location(self, location: ConfigLocation) -> LocatedDomainConfig {
        let resolved_modules = iter::once(self.root.as_ref().map(|root| root.resolve(&location)))
            .chain(
                self.modules
                    .iter()
                    .map(|module| Some(module.resolve(&location))),
            )
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
            pending_edits: Default::default(),
        }
    }
}

pub const DOMAIN_ROOT_SENTINEL: &str = "<domain_root>";

trait Resolvable<T> {
    fn resolve(&self, location: &ConfigLocation) -> T;
}

impl<T, U, I> Resolvable<Vec<U>> for I
where
    I: IntoIterator<Item = T> + AsRef<[T]>,
    T: Resolvable<U>,
{
    fn resolve(&self, location: &ConfigLocation) -> Vec<U> {
        self.as_ref()
            .iter()
            .map(|item| item.resolve(location))
            .collect()
    }
}

impl<T> Resolvable<String> for T
where
    T: AsRef<str>,
{
    fn resolve(&self, location: &ConfigLocation) -> String {
        if self.as_ref().starts_with("//") {
            // Absolute path
            self.as_ref()[2..].to_string()
        } else if self.as_ref() == DOMAIN_ROOT_SENTINEL {
            // Domain root sentinel
            location.mod_path.clone()
        } else {
            // Relative path
            format!("{}.{}", location.mod_path, self.as_ref())
        }
    }
}

impl Resolvable<DependencyConfig> for DependencyConfig {
    fn resolve(&self, location: &ConfigLocation) -> DependencyConfig {
        DependencyConfig::new(&self.path.as_str().resolve(location), self.deprecated)
    }
}

impl Resolvable<ModuleConfig> for DomainRootConfig {
    fn resolve(&self, location: &ConfigLocation) -> ModuleConfig {
        ModuleConfig::new(
            // Root modules represent the domain itself
            &location.mod_path,
            self.depends_on.clone().map(|deps| deps.resolve(location)),
            self.cannot_depend_on
                .clone()
                .map(|deps| deps.resolve(location)),
            self.depends_on_external.clone(),
            self.cannot_depend_on_external.clone(),
            self.layer.clone(),
            self.visibility.clone().map(|vis| vis.resolve(location)),
            self.utility,
            self.unchecked,
        )
    }
}

impl Resolvable<ModuleConfig> for ModuleConfig {
    fn resolve(&self, location: &ConfigLocation) -> ModuleConfig {
        ModuleConfig::new(
            &format!("{}.{}", location.mod_path, self.path),
            self.depends_on.clone().map(|deps| deps.resolve(location)),
            self.cannot_depend_on
                .clone()
                .map(|deps| deps.resolve(location)),
            self.depends_on_external.clone(),
            self.cannot_depend_on_external.clone(),
            self.layer.clone(),
            self.visibility.clone().map(|vis| vis.resolve(location)),
            self.utility,
            self.unchecked,
        )
        .with_copied_origin(self)
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
            visibility: self.visibility.clone().map(|vis| vis.resolve(location)),
            data_types: self.data_types,
            exclusive: self.exclusive,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocatedDomainConfig {
    pub config: DomainConfig,
    pub location: ConfigLocation,
    resolved_modules: Vec<ModuleConfig>,
    resolved_interfaces: Vec<InterfaceConfig>,
    pending_edits: Vec<ConfigEdit>,
}

impl LocatedDomainConfig {
    pub fn modules(&self) -> impl Iterator<Item = &ModuleConfig> {
        self.resolved_modules.iter()
    }

    pub fn interfaces(&self) -> impl Iterator<Item = &InterfaceConfig> {
        self.resolved_interfaces.iter()
    }

    pub fn with_dependencies_removed(&self) -> Self {
        self.config
            .with_dependencies_removed()
            .with_location(self.location.clone())
    }

    pub fn normalize_module_path(&self, module_path: &str) -> String {
        if module_path == self.location.mod_path {
            DOMAIN_ROOT_SENTINEL.to_string()
        } else if module_path.starts_with(&self.location.mod_path) {
            return module_path
                .strip_prefix(&self.location.mod_path)
                .map(|p| p.trim_start_matches('.'))
                .unwrap()
                .to_string();
        } else {
            return format!("//{}", module_path);
        }
    }
}

impl ConfigEditor for LocatedDomainConfig {
    fn enqueue_edit(&mut self, edit: &ConfigEdit) -> Result<(), EditError> {
        match edit {
            ConfigEdit::CreateModule { path }
            | ConfigEdit::DeleteModule { path }
            | ConfigEdit::MarkModuleAsUtility { path }
            | ConfigEdit::UnmarkModuleAsUtility { path }
            | ConfigEdit::AddDependency { path, .. }
            | ConfigEdit::RemoveDependency { path, .. } => {
                if path.starts_with(&self.location.mod_path) {
                    // If this module path appears to belong to this domain, enqueue the edit
                    self.pending_edits.push(edit.clone());
                    Ok(())
                } else {
                    Err(EditError::NotApplicable)
                }
            }
            ConfigEdit::AddSourceRoot { .. } | ConfigEdit::RemoveSourceRoot { .. } => {
                Err(EditError::NotApplicable)
            }
        }
    }

    fn apply_edits(&mut self) -> Result<(), EditError> {
        if self.pending_edits.is_empty() {
            return Ok(());
        }

        let toml_str = std::fs::read_to_string(&self.location.path)
            .map_err(|_| EditError::ConfigDoesNotExist)?;
        let mut doc = toml_str
            .parse::<toml_edit::DocumentMut>()
            .map_err(|_| EditError::ParsingFailed)?;

        for edit in &self.pending_edits {
            match edit {
                ConfigEdit::CreateModule { path } => {
                    // Check if this is a root module
                    if path == &self.location.mod_path {
                        let mut root_table = toml_edit::Table::new();
                        root_table.insert("depends_on", toml_edit::value(toml_edit::Array::new()));
                        doc.insert("root", toml_edit::Item::Table(root_table));
                        continue;
                    }

                    let relative_path = path
                        .strip_prefix(&self.location.mod_path)
                        .map(|p| p.trim_start_matches('.'))
                        .unwrap_or(path);

                    let mut module_table = toml_edit::Table::new();
                    module_table.insert("path", toml_edit::value(relative_path));
                    module_table.insert("depends_on", toml_edit::value(toml_edit::Array::new()));

                    if let Some(root) = &self.config.root {
                        if let Some(layer) = &root.layer {
                            module_table.insert("layer", toml_edit::value(layer));
                        }
                    }

                    let modules = doc["modules"]
                        .or_insert(toml_edit::Item::ArrayOfTables(Default::default()));
                    if let toml_edit::Item::ArrayOfTables(array) = modules {
                        array.push(module_table);
                    }
                }
                ConfigEdit::DeleteModule { path } => {
                    // Check if this is a root module
                    if path == &self.location.mod_path {
                        doc.remove("root");
                        continue;
                    }

                    let relative_path = path
                        .strip_prefix(&self.location.mod_path)
                        .map(|p| p.trim_start_matches('.'))
                        .unwrap_or(path);

                    if let toml_edit::Item::ArrayOfTables(modules) = &mut doc["modules"] {
                        let mut module_index = None;
                        for (i, table) in modules.iter_mut().enumerate() {
                            if table
                                .get("path")
                                .map(|p| p.as_str() == Some(relative_path))
                                .unwrap_or(false)
                            {
                                module_index = Some(i);
                                break;
                            } else if table
                                .get("paths")
                                .map(|p| p.as_array().is_some())
                                .unwrap_or(false)
                            {
                                table["paths"]
                                    .as_array_mut()
                                    .unwrap()
                                    .retain(|p| p.as_str().unwrap() != relative_path);
                                if table["paths"].as_array().unwrap().is_empty() {
                                    module_index = Some(i);
                                }
                                break;
                            }
                        }
                        if let Some(index) = module_index {
                            modules.remove(index);
                        }
                    }
                }
                ConfigEdit::MarkModuleAsUtility { path }
                | ConfigEdit::UnmarkModuleAsUtility { path } => {
                    // Check if this is a root module
                    if path == &self.location.mod_path {
                        if let Some(toml_edit::Item::Table(root)) = doc.get_mut("root") {
                            match edit {
                                ConfigEdit::MarkModuleAsUtility { .. } => {
                                    root.insert("utility", toml_edit::value(true));
                                }
                                ConfigEdit::UnmarkModuleAsUtility { .. } => {
                                    root.remove("utility");
                                }
                                _ => unreachable!(),
                            }
                        }
                        continue;
                    }

                    let relative_path = path
                        .strip_prefix(&self.location.mod_path)
                        .map(|p| p.trim_start_matches('.'))
                        .unwrap_or(path);

                    if let toml_edit::Item::ArrayOfTables(modules) = &mut doc["modules"] {
                        for table in modules.iter_mut() {
                            if table
                                .get("path")
                                .map(|p| p.as_str() == Some(relative_path))
                                .unwrap_or(false)
                            {
                                match edit {
                                    ConfigEdit::MarkModuleAsUtility { .. } => {
                                        table.insert("utility", toml_edit::value(true));
                                    }
                                    ConfigEdit::UnmarkModuleAsUtility { .. } => {
                                        table.remove("utility");
                                    }
                                    _ => unreachable!(),
                                }
                            } else if table.get("paths").is_some_and(|p| {
                                p.as_array()
                                    .map(|p| p.iter().any(|p| p.as_str() == Some(relative_path)))
                                    .unwrap_or(false)
                            }) {
                                return Err(EditError::NotImplemented(
                                    "Cannot mark utilities for multi-path modules".to_string(),
                                ));
                            }
                        }
                    }
                }
                ConfigEdit::AddDependency { path, dependency }
                | ConfigEdit::RemoveDependency { path, dependency } => {
                    // Check if this is a root module
                    if path == &self.location.mod_path {
                        if let Some(toml_edit::Item::Table(root)) = doc.get_mut("root") {
                            match edit {
                                ConfigEdit::AddDependency { .. } => {
                                    if let Some(toml_edit::Item::Value(toml_edit::Value::Array(
                                        array,
                                    ))) = root.get_mut("depends_on")
                                    {
                                        array.push(self.normalize_module_path(dependency));
                                    } else {
                                        root.insert(
                                            "depends_on",
                                            toml_edit::value(toml_edit::Array::from_iter(
                                                iter::once(self.normalize_module_path(dependency)),
                                            )),
                                        );
                                    }
                                }
                                ConfigEdit::RemoveDependency { .. } => {
                                    if let toml_edit::Item::Value(toml_edit::Value::Array(array)) =
                                        &mut root["depends_on"]
                                    {
                                        array.retain(|dep| {
                                            dep.as_str()
                                                .map(|d| {
                                                    d != self.normalize_module_path(dependency)
                                                })
                                                .unwrap_or(true)
                                        });
                                    }
                                }
                                _ => unreachable!(),
                            }
                        }
                        continue;
                    }

                    let relative_path = path
                        .strip_prefix(&self.location.mod_path)
                        .map(|p| p.trim_start_matches('.'))
                        .unwrap_or(path);

                    if let toml_edit::Item::ArrayOfTables(modules) = &mut doc["modules"] {
                        for table in modules.iter_mut() {
                            let is_target_module = table
                                .get("path")
                                .map(|p| p.as_str() == Some(relative_path))
                                .unwrap_or(false)
                                || table
                                    .get("paths")
                                    .map(|p| {
                                        p.as_array().is_some_and(|p| {
                                            p.iter().any(|p| p.as_str() == Some(relative_path))
                                        })
                                    })
                                    .unwrap_or(false);

                            if is_target_module {
                                match edit {
                                    ConfigEdit::AddDependency { .. } => {
                                        if let Some(toml_edit::Item::Value(
                                            toml_edit::Value::Array(array),
                                        )) = table.get_mut("depends_on")
                                        {
                                            array.push(self.normalize_module_path(dependency));
                                        } else {
                                            table.insert(
                                                "depends_on",
                                                toml_edit::value(toml_edit::Array::from_iter(
                                                    iter::once(
                                                        self.normalize_module_path(dependency),
                                                    ),
                                                )),
                                            );
                                        }
                                    }
                                    ConfigEdit::RemoveDependency { .. } => {
                                        if let toml_edit::Item::Value(toml_edit::Value::Array(
                                            array,
                                        )) = &mut table["depends_on"]
                                        {
                                            array.retain(|dep| {
                                                dep.as_str()
                                                    .map(|d| {
                                                        d != self.normalize_module_path(dependency)
                                                    })
                                                    .unwrap_or(true)
                                            });
                                        }
                                    }
                                    _ => unreachable!(),
                                }
                            }
                        }
                    }
                }
                ConfigEdit::AddSourceRoot { .. } | ConfigEdit::RemoveSourceRoot { .. } => {
                    return Err(EditError::NotApplicable);
                }
            }
        }

        std::fs::write(&self.location.path, doc.to_string())
            .map_err(|_| EditError::DiskWriteFailed)?;

        self.pending_edits.clear();
        Ok(())
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
