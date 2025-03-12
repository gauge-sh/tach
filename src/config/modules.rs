use std::hash::{Hash, Hasher};
use std::ops::Not;
use std::path::PathBuf;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use globset::GlobMatcher;
use pyo3::prelude::*;
use serde::ser::{Error, SerializeSeq, SerializeStruct};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use super::root_module::ROOT_MODULE_SENTINEL_TAG;
use crate::filesystem::module_path_is_included_in_paths;
use crate::resolvers::ModuleGlob;

#[derive(Clone, Debug, Default)]
#[pyclass(module = "tach.extension")]
pub struct DependencyConfig {
    #[pyo3(get)]
    pub path: String,
    #[pyo3(get)]
    pub deprecated: bool,
    matcher: Option<GlobMatcher>,
}

impl PartialEq for DependencyConfig {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.deprecated == other.deprecated
    }
}

impl Eq for DependencyConfig {}

impl Hash for DependencyConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.deprecated.hash(state);
    }
}

impl Serialize for DependencyConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Should actually express that all fields are default except for path
        if !self.deprecated {
            serializer.serialize_str(&self.path)
        } else {
            let mut state = serializer.serialize_struct("DependencyConfig", 2)?;
            state.serialize_field("path", &self.path)?;
            state.serialize_field("deprecated", &self.deprecated)?;
            state.end()
        }
    }
}

impl DependencyConfig {
    fn get_matcher_for_path(path: &str) -> Option<GlobMatcher> {
        ModuleGlob::parse(path).map(|glob| glob.into_matcher().ok())?
    }

    pub fn new(path: &str, deprecated: bool) -> Self {
        Self {
            path: path.into(),
            deprecated,
            matcher: Self::get_matcher_for_path(path),
        }
    }

    pub fn from_deprecated_path(path: &str) -> Self {
        Self::new(path, true)
    }
    pub fn from_path(path: &str) -> Self {
        Self::new(path, false)
    }

    pub fn matches(&self, path: &str) -> bool {
        if let Some(matcher) = &self.matcher {
            // Glob matcher expects unix-style paths
            matcher.is_match(path.replace(".", "/"))
        } else {
            self.path == path
        }
    }
}
struct DependencyConfigVisitor;

impl<'de> de::Visitor<'de> for DependencyConfigVisitor {
    type Value = DependencyConfig;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or map")
    }

    fn visit_str<E>(self, value: &str) -> Result<DependencyConfig, E>
    where
        E: de::Error,
    {
        Ok(DependencyConfig::new(value, false))
    }

    // Unfortunately don't have the derived Deserialize for this
    fn visit_map<M>(self, mut map: M) -> Result<DependencyConfig, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let mut path: Option<String> = None;
        let mut deprecated = false;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "path" => {
                    path = {
                        if path.is_some() {
                            return Err(de::Error::duplicate_field("path"));
                        }
                        Some(map.next_value()?)
                    }
                }
                "deprecated" => {
                    if deprecated {
                        return Err(de::Error::duplicate_field("deprecated"));
                    }
                    deprecated = map.next_value()?;
                }
                _ => {
                    return Err(de::Error::unknown_field(&key, &["path", "deprecated"]));
                }
            }
        }

        let path = path.ok_or_else(|| de::Error::missing_field("path"))?;

        Ok(DependencyConfig::new(&path, deprecated))
    }
}

impl<'de> Deserialize<'de> for DependencyConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DependencyConfigVisitor)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ModuleOrigin {
    Glob(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
#[pyclass(eq, module = "tach.extension")]
pub struct ModuleConfig {
    #[pyo3(get)]
    pub path: String,
    #[serde(default)]
    #[pyo3(get, set)]
    pub depends_on: Option<Vec<DependencyConfig>>,
    #[serde(default)]
    #[pyo3(get, set)]
    pub cannot_depend_on: Option<Vec<DependencyConfig>>,
    #[serde(default)]
    #[pyo3(get, set)]
    pub depends_on_external: Option<Vec<String>>,
    #[serde(default)]
    #[pyo3(get, set)]
    pub cannot_depend_on_external: Option<Vec<String>>,
    #[serde(default)]
    #[pyo3(get)]
    pub layer: Option<String>,
    #[serde(default)]
    #[pyo3(get)]
    pub visibility: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Not::not")]
    #[pyo3(get)]
    pub utility: bool,
    // TODO: Remove this in a future version
    // This will be deserialized from old config,
    // but auto-migrated to interfaces internally.
    // This means we don't want to serialize it.
    #[serde(default, skip_serializing)]
    #[pyo3(get)]
    pub strict: bool,
    #[serde(default, skip_serializing_if = "Not::not")]
    #[pyo3(get)]
    pub unchecked: bool,
    // Hidden field to track grouping
    #[serde(skip)]
    group_id: Option<usize>,
    #[serde(skip)]
    origin: Option<ModuleOrigin>,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            // By default, a module can depend on nothing
            depends_on: Some(vec![]),
            cannot_depend_on: Default::default(),
            depends_on_external: Default::default(),
            cannot_depend_on_external: Default::default(),
            path: Default::default(),
            layer: Default::default(),
            visibility: Default::default(),
            utility: Default::default(),
            strict: Default::default(),
            unchecked: Default::default(),
            group_id: Default::default(),
            origin: Default::default(),
        }
    }
}

impl ModuleConfig {
    pub fn new(
        path: &str,
        depends_on: Option<Vec<DependencyConfig>>,
        cannot_depend_on: Option<Vec<DependencyConfig>>,
        depends_on_external: Option<Vec<String>>,
        cannot_depend_on_external: Option<Vec<String>>,
        layer: Option<String>,
        visibility: Option<Vec<String>>,
        utility: bool,
        unchecked: bool,
    ) -> Self {
        Self {
            path: path.to_string(),
            depends_on,
            cannot_depend_on,
            depends_on_external,
            cannot_depend_on_external,
            layer,
            visibility,
            utility,
            strict: false,
            unchecked,
            group_id: None,
            origin: None,
        }
    }

    pub fn from_path(path: &str) -> Self {
        Self {
            path: path.to_string(),
            ..Default::default()
        }
    }

    pub fn from_path_and_dependencies(
        path: &str,
        depends_on: Option<Vec<DependencyConfig>>,
    ) -> Self {
        Self {
            path: path.to_string(),
            depends_on,
            ..Default::default()
        }
    }

    pub fn with_copied_origin(self, other: &Self) -> Self {
        Self {
            origin: other.origin.clone(),
            ..self
        }
    }

    pub fn with_glob_origin(self, glob: &str) -> Self {
        Self {
            origin: Some(ModuleOrigin::Glob(glob.to_string())),
            ..self
        }
    }

    pub fn overwriteable_by(&self, other: &Self) -> bool {
        // a module is overwriteable by another if
        // - they have the same path
        // - this module's origin is a glob and is not the same as the other module's origin
        self.path == other.path
            && self
                .origin
                .as_ref()
                .is_some_and(|origin| Some(origin) != other.origin.as_ref())
    }

    pub fn new_with_layer(path: &str, layer: &str) -> Self {
        // shorthand for test fixtures
        Self {
            path: path.to_string(),
            depends_on: Some(vec![]),
            cannot_depend_on: None,
            depends_on_external: None,
            cannot_depend_on_external: None,
            layer: Some(layer.to_string()),
            visibility: None,
            utility: false,
            strict: false,
            unchecked: false,
            group_id: None,
            origin: None,
        }
    }

    pub fn dependencies_iter(&self) -> impl Iterator<Item = &DependencyConfig> {
        self.depends_on
            .as_ref()
            .into_iter()
            .flat_map(|deps| deps.iter())
    }

    pub fn forbidden_dependencies_iter(&self) -> impl Iterator<Item = &DependencyConfig> {
        self.cannot_depend_on
            .as_ref()
            .into_iter()
            .flat_map(|deps| deps.iter())
    }

    pub fn with_dependencies_removed(&self) -> Self {
        Self {
            depends_on: Some(vec![]),
            ..self.clone()
        }
    }

    pub fn with_filtered_dependencies(
        &self,
        absolute_source_roots: &[PathBuf],
        included_paths: &[PathBuf],
    ) -> Self {
        match &self.depends_on {
            Some(depends_on) => Self {
                depends_on: Some(
                    depends_on
                        .iter()
                        .filter(|dep| {
                            included_paths.is_empty()
                                || module_path_is_included_in_paths(
                                    absolute_source_roots,
                                    &dep.path,
                                    included_paths,
                                )
                        })
                        .cloned()
                        .collect(),
                ),
                ..self.clone()
            },
            None => self.clone(),
        }
    }

    pub fn clone_with_path(&self, path: &str) -> Self {
        let mut new_config = self.clone();
        if path == self.path {
            return new_config;
        }

        new_config.path = path.to_string();
        new_config
    }

    pub fn new_root_config() -> Self {
        Self::from_path(ROOT_MODULE_SENTINEL_TAG)
    }

    pub fn is_root(&self) -> bool {
        self.path == ROOT_MODULE_SENTINEL_TAG
    }

    pub fn is_unchecked(&self) -> bool {
        self.unchecked
    }
}

#[pymethods]
impl ModuleConfig {
    pub fn mod_path(&self) -> String {
        if self.path == ROOT_MODULE_SENTINEL_TAG {
            return ".".to_string();
        }
        self.path.clone()
    }
}

pub fn serialize_modules_json(modules: &Vec<ModuleConfig>) -> String {
    #[derive(Serialize)]
    struct ModulesWrapper<'a> {
        modules: &'a Vec<ModuleConfig>,
    }
    serde_json::to_string(&ModulesWrapper { modules }).unwrap()
}

#[derive(Serialize, Deserialize)]
struct BulkModule {
    paths: Vec<String>,
    #[serde(default)]
    depends_on: Option<Vec<DependencyConfig>>,
    #[serde(default)]
    cannot_depend_on: Option<Vec<DependencyConfig>>,
    #[serde(default)]
    depends_on_external: Option<Vec<String>>,
    #[serde(default)]
    cannot_depend_on_external: Option<Vec<String>>,
    #[serde(default)]
    layer: Option<String>,
    #[serde(default)]
    visibility: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Not::not")]
    utility: bool,
    #[serde(default, skip_serializing_if = "Not::not")]
    unchecked: bool,
}

impl TryFrom<&[&ModuleConfig]> for BulkModule {
    type Error = String;

    fn try_from(modules: &[&ModuleConfig]) -> Result<Self, Self::Error> {
        if modules.is_empty() {
            return Err("Cannot create BulkModule from empty slice".to_string());
        }

        let first = modules[0];
        let mut bulk = BulkModule {
            paths: modules.iter().map(|m| m.path.clone()).collect(),
            depends_on: None,
            cannot_depend_on: None,
            depends_on_external: None,
            cannot_depend_on_external: None,
            layer: first.layer.clone(),
            visibility: first.visibility.clone(),
            utility: first.utility,
            unchecked: first.unchecked,
        };

        let mut unique_deps: HashSet<DependencyConfig> = HashSet::new();
        let mut unique_external_deps: HashSet<String> = HashSet::new();
        for module in modules {
            // We merge dependencies from all modules, since they may have been mutated in commands like 'sync'
            if let Some(depends_on) = &module.depends_on {
                unique_deps.extend(depends_on.clone());
            }
            if let Some(depends_on_external) = &module.depends_on_external {
                unique_external_deps.extend(depends_on_external.clone());
            }

            // Validate that other fields match the first module
            if module.cannot_depend_on != first.cannot_depend_on {
                return Err(format!(
                    "Inconsistent 'cannot_depend_on' list in bulk module group for path {}",
                    module.path
                ));
            }
            if module.cannot_depend_on_external != first.cannot_depend_on_external {
                return Err(format!(
                    "Inconsistent 'cannot_depend_on_external' list in bulk module group for path {}",
                    module.path
                ));
            }
            if module.layer != first.layer {
                return Err(format!(
                    "Inconsistent layer in bulk module group for path {}",
                    module.path
                ));
            }
            if module.visibility != first.visibility {
                return Err(format!(
                    "Inconsistent visibility in bulk module group for path {}",
                    module.path
                ));
            }
            if module.utility != first.utility {
                return Err(format!(
                    "Inconsistent utility setting in bulk module group for path {}",
                    module.path
                ));
            }
            if module.strict != first.strict {
                return Err(format!(
                    "Inconsistent strict setting in bulk module group for path {}",
                    module.path
                ));
            }
            if module.unchecked != first.unchecked {
                return Err(format!(
                    "Inconsistent unchecked setting in bulk module group for path {}",
                    module.path
                ));
            }
        }

        if !unique_deps.is_empty() {
            bulk.depends_on = Some(unique_deps.into_iter().collect());
        }
        if !unique_external_deps.is_empty() {
            bulk.depends_on_external = Some(unique_external_deps.into_iter().collect());
        }
        Ok(bulk)
    }
}

pub fn serialize_modules<S>(modules: &Vec<ModuleConfig>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut grouped: HashMap<Option<usize>, Vec<&ModuleConfig>> = HashMap::new();

    for module in modules {
        grouped.entry(module.group_id).or_default().push(module);
    }

    let mut seq = serializer.serialize_seq(Some(grouped.len()))?;

    for (group_key, group_modules) in grouped {
        match group_key {
            // Single modules (no group)
            None => {
                for module in group_modules {
                    seq.serialize_element(module)?;
                }
            }
            // Grouped modules
            Some(_) => {
                if !group_modules.is_empty() {
                    let bulk =
                        BulkModule::try_from(group_modules.as_slice()).map_err(S::Error::custom)?;
                    seq.serialize_element(&bulk)?;
                }
            }
        }
    }

    seq.end()
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ModuleConfigOrBulk {
    Single(ModuleConfig),
    Bulk(BulkModule),
}

pub fn deserialize_modules<'de, D>(deserializer: D) -> Result<Vec<ModuleConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let configs: Vec<ModuleConfigOrBulk> = Vec::deserialize(deserializer)?;

    Ok(configs
        .into_iter()
        .enumerate()
        .flat_map(|(i, config)| match config {
            ModuleConfigOrBulk::Single(module) => vec![module],
            ModuleConfigOrBulk::Bulk(bulk) => bulk
                .paths
                .into_iter()
                .map(|path| ModuleConfig {
                    path,
                    depends_on: bulk.depends_on.clone(),
                    cannot_depend_on: bulk.cannot_depend_on.clone(),
                    depends_on_external: bulk.depends_on_external.clone(),
                    cannot_depend_on_external: bulk.cannot_depend_on_external.clone(),
                    layer: bulk.layer.clone(),
                    visibility: bulk.visibility.clone(),
                    utility: bulk.utility,
                    strict: false,
                    unchecked: bulk.unchecked,
                    group_id: Some(i),
                    origin: None,
                })
                .collect(),
        })
        .collect())
}
