use pyo3::prelude::*;
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;

pub const ROOT_MODULE_SENTINEL_TAG: &str = "<root>";
pub const DEFAULT_EXCLUDE_PATHS: [&str; 4] = ["tests", "docs", ".*__pycache__", ".*egg-info"];

// for serde
fn default_true() -> bool {
    true
}
fn default_source_roots() -> Vec<PathBuf> {
    vec![PathBuf::from(".")]
}

pub fn global_visibility() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_visibility() -> Vec<String> {
    global_visibility()
}

fn is_default_visibility(value: &Vec<String>) -> bool {
    value == &default_visibility()
}

fn is_true(value: &bool) -> bool {
    *value
}
fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
#[pyclass(get_all, module = "tach.extension")]
pub struct DependencyConfig {
    pub path: String,
    pub deprecated: bool,
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
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("DependencyConfig", 2)?;
            state.serialize_field("path", &self.path)?;
            state.serialize_field("deprecated", &self.deprecated)?;
            state.end()
        }
    }
}

impl DependencyConfig {
    pub fn from_deprecated_path(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            deprecated: true,
        }
    }
    pub fn from_path(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            deprecated: false,
        }
    }
}
struct DependencyConfigVisitor;

impl<'de> Visitor<'de> for DependencyConfigVisitor {
    type Value = DependencyConfig;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or map")
    }

    fn visit_str<E>(self, value: &str) -> Result<DependencyConfig, E>
    where
        E: de::Error,
    {
        Ok(DependencyConfig {
            path: value.to_string(),
            ..Default::default()
        })
    }

    // Unfortunately don't have the derived Deserialize for this
    fn visit_map<M>(self, mut map: M) -> Result<DependencyConfig, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut path = None;
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

        Ok(DependencyConfig { path, deprecated })
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, eq, module = "tach.extension")]
pub struct ModuleConfig {
    pub path: String,
    #[serde(default)]
    #[pyo3(set)]
    pub depends_on: Vec<DependencyConfig>,
    #[serde(
        default = "default_visibility",
        skip_serializing_if = "is_default_visibility"
    )]
    pub visibility: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub utility: bool,
    // TODO: Remove this in a future version
    // This will be deserialized from old config,
    // but auto-migrated to interfaces internally.
    // This means we don't want to serialize it.
    #[serde(default, skip_serializing)]
    pub strict: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub unchecked: bool,
    // Hidden field to track grouping
    // Unfortunately marked as public due to test fixtures constructing struct literals
    #[doc(hidden)]
    pub group_id: Option<usize>,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            path: Default::default(),
            depends_on: Default::default(),
            visibility: default_visibility(),
            utility: Default::default(),
            strict: Default::default(),
            unchecked: Default::default(),
            group_id: Default::default(),
        }
    }
}

#[pymethods]
impl ModuleConfig {
    #[new]
    pub fn new(path: &str, strict: bool) -> Self {
        Self {
            path: path.to_string(),
            depends_on: vec![],
            visibility: default_visibility(),
            utility: false,
            strict,
            unchecked: false,
            group_id: None,
        }
    }

    pub fn with_no_dependencies(&self) -> Self {
        let mut new_module = self.clone();
        new_module.depends_on = vec![];
        new_module
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

#[derive(Serialize, Deserialize)]
struct BulkModule {
    paths: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    depends_on: Vec<DependencyConfig>,
    #[serde(
        default = "default_visibility",
        skip_serializing_if = "is_default_visibility"
    )]
    visibility: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    utility: bool,
    #[serde(default)]
    strict: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    unchecked: bool,
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InterfaceDataTypes {
    #[default]
    All,
    Primitive,
}

impl ToString for InterfaceDataTypes {
    fn to_string(&self) -> String {
        match self {
            Self::All => "all".to_string(),
            Self::Primitive => "primitive".to_string(),
        }
    }
}

impl InterfaceDataTypes {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl IntoPy<PyObject> for InterfaceDataTypes {
    fn into_py(self, py: Python) -> PyObject {
        self.to_string().to_object(py)
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, module = "tach.extension")]
pub struct InterfaceConfig {
    pub expose: Vec<String>,
    #[serde(
        rename = "from",
        default = "default_from_modules",
        skip_serializing_if = "is_default_from_modules"
    )]
    pub from_modules: Vec<String>,
    #[serde(default, skip_serializing_if = "InterfaceDataTypes::is_default")]
    pub data_types: InterfaceDataTypes,
}

fn default_from_modules() -> Vec<String> {
    vec![".*".to_string()]
}

fn is_default_from_modules(value: &Vec<String>) -> bool {
    value == &default_from_modules()
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheBackend {
    #[default]
    Disk,
}

impl CacheBackend {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl IntoPy<PyObject> for CacheBackend {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Disk => "disk".to_object(py),
        }
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, module = "tach.extension")]
pub struct CacheConfig {
    #[serde(default, skip_serializing_if = "CacheBackend::is_default")]
    pub backend: CacheBackend,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_dependencies: Vec<String>,
}

impl CacheConfig {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, module = "tach.extension")]
pub struct ExternalDependencyConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rename: Vec<String>,
}

impl ExternalDependencyConfig {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Default, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct UnusedDependencies {
    pub path: String,
    pub dependencies: Vec<DependencyConfig>,
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RootModuleTreatment {
    #[default]
    Allow,
    Forbid,
    Ignore,
    DependenciesOnly,
}

impl RootModuleTreatment {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl IntoPy<PyObject> for RootModuleTreatment {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Allow => "allow".to_object(py),
            Self::Forbid => "forbid".to_object(py),
            Self::Ignore => "ignore".to_object(py),
            Self::DependenciesOnly => "dependenciesonly".to_object(py),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuleSetting {
    Error,
    Warn,
    Off,
}

impl RuleSetting {
    // These are just necessary for serde macros
    fn warn() -> Self {
        Self::Warn
    }

    fn is_warn(&self) -> bool {
        *self == Self::Warn
    }

    fn error() -> Self {
        Self::Error
    }

    fn is_error(&self) -> bool {
        *self == Self::Error
    }

    fn off() -> Self {
        Self::Off
    }

    fn is_off(&self) -> bool {
        *self == Self::Off
    }
}

impl IntoPy<PyObject> for RuleSetting {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Error => "error".to_object(py),
            Self::Warn => "warn".to_object(py),
            Self::Off => "off".to_object(py),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[pyclass(get_all, module = "tach.extension")]
pub struct RulesConfig {
    #[serde(
        default = "RuleSetting::warn",
        skip_serializing_if = "RuleSetting::is_warn"
    )]
    pub unused_ignore_directives: RuleSetting,
    #[serde(
        default = "RuleSetting::off",
        skip_serializing_if = "RuleSetting::is_off"
    )]
    pub require_ignore_directive_reasons: RuleSetting,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            unused_ignore_directives: RuleSetting::warn(),
            require_ignore_directive_reasons: RuleSetting::off(),
        }
    }
}

impl RulesConfig {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ModuleConfigOrBulk {
    Single(ModuleConfig),
    Bulk(BulkModule),
}

fn deserialize_modules<'de, D>(deserializer: D) -> Result<Vec<ModuleConfig>, D::Error>
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
                    visibility: bulk.visibility.clone(),
                    utility: bulk.utility,
                    strict: bulk.strict,
                    unchecked: bulk.unchecked,
                    group_id: Some(i),
                })
                .collect(),
        })
        .collect())
}

fn serialize_modules<S>(modules: &Vec<ModuleConfig>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use std::collections::HashMap;

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
                if let Some(first) = group_modules.first() {
                    let bulk = BulkModule {
                        paths: group_modules.iter().map(|m| m.path.clone()).collect(),
                        depends_on: first.depends_on.clone(),
                        visibility: first.visibility.clone(),
                        utility: first.utility,
                        strict: first.strict,
                        unchecked: first.unchecked,
                    };
                    seq.serialize_element(&bulk)?;
                }
            }
        }
    }

    seq.end()
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub use_regex_matching: bool,
    #[serde(default, skip_serializing_if = "RootModuleTreatment::is_default")]
    pub root_module: RootModuleTreatment,
    #[serde(default, skip_serializing_if = "RulesConfig::is_default")]
    pub rules: RulesConfig,
}

impl ProjectConfig {
    fn dependencies_for_module(&self, module: &str) -> Option<&Vec<DependencyConfig>> {
        self.modules
            .iter()
            .find(|mod_config| mod_config.path == module)
            .map(|mod_config| &mod_config.depends_on)
    }
    pub fn prepend_roots(&self, project_root: &PathBuf) -> Vec<PathBuf> {
        // don't prepend if root is "."
        self.source_roots
            .iter()
            .map(|root| {
                if root.display().to_string() == "." {
                    project_root.clone()
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
