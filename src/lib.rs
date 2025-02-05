pub mod cache;
pub mod checks;
pub mod cli;
pub mod colors;
pub mod commands;
pub mod config;
pub mod diagnostics;
pub mod exclusion;
pub mod external;
pub mod filesystem;
pub mod interfaces;
pub mod interrupt;
pub mod lsp;
pub mod modularity;
pub mod modules;
pub mod parsing;
pub mod pattern;
pub mod processors;
pub mod python;
pub mod tests;

use commands::{check, report, server, sync, test};
use diagnostics::serialize_diagnostics_json;
use modularity::into_usage_errors;
use std::collections::HashMap;
use std::path::PathBuf;

use pyo3::exceptions::{PyKeyboardInterrupt, PyOSError, PySyntaxError, PyValueError};
use pyo3::prelude::*;

mod errors {
    pyo3::import_exception!(tach.errors, TachCircularDependencyError);
    pyo3::import_exception!(tach.errors, TachVisibilityError);
    pyo3::import_exception!(tach.errors, TachSetupError);
}

impl From<processors::import::ImportParseError> for PyErr {
    fn from(err: processors::import::ImportParseError) -> Self {
        match err {
            processors::import::ImportParseError::Parsing { file: _, source: _ } => {
                PySyntaxError::new_err(err.to_string())
            }
            _ => PyOSError::new_err(err.to_string()),
        }
    }
}

impl From<exclusion::PathExclusionError> for PyErr {
    fn from(err: exclusion::PathExclusionError) -> Self {
        match err {
            exclusion::PathExclusionError::ConcurrencyError => PyOSError::new_err(err.to_string()),
            _ => PyValueError::new_err(err.to_string()),
        }
    }
}

impl From<report::ReportCreationError> for PyErr {
    fn from(err: report::ReportCreationError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<cache::CacheError> for PyErr {
    fn from(err: cache::CacheError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<check::CheckError> for PyErr {
    fn from(err: check::CheckError) -> Self {
        match err {
            check::CheckError::Interrupt => PyKeyboardInterrupt::new_err(err.to_string()),
            check::CheckError::ModuleTree(modules::error::ModuleTreeError::CircularDependency(
                c,
            )) => errors::TachCircularDependencyError::new_err(c),
            check::CheckError::ModuleTree(
                modules::error::ModuleTreeError::VisibilityViolation(v),
            ) => errors::TachVisibilityError::new_err(v),
            _ => PyValueError::new_err(err.to_string()),
        }
    }
}

impl From<python::error::ParsingError> for PyErr {
    fn from(err: python::error::ParsingError) -> Self {
        match err {
            python::error::ParsingError::PythonParse(err) => {
                PySyntaxError::new_err(err.to_string())
            }
            python::error::ParsingError::Io(err) => PyOSError::new_err(err.to_string()),
            python::error::ParsingError::Filesystem(err) => PyOSError::new_err(err.to_string()),
        }
    }
}

impl From<parsing::error::ParsingError> for PyErr {
    fn from(err: parsing::error::ParsingError) -> Self {
        match err {
            parsing::error::ParsingError::Io(err) => PyOSError::new_err(err.to_string()),
            parsing::error::ParsingError::Filesystem(err) => PyOSError::new_err(err.to_string()),
            parsing::error::ParsingError::TomlParse(err) => PyValueError::new_err(err.to_string()),
            parsing::error::ParsingError::MissingField(err) => PyValueError::new_err(err),
            parsing::error::ParsingError::ModulePath(err) => PyValueError::new_err(err),
        }
    }
}
impl From<sync::SyncError> for PyErr {
    fn from(err: sync::SyncError) -> Self {
        match err {
            sync::SyncError::FileWrite(err) => PyOSError::new_err(err.to_string()),
            sync::SyncError::TomlSerialize(err) => PyOSError::new_err(err.to_string()),
            sync::SyncError::CheckError(err) => err.into(),
            sync::SyncError::RootModuleViolation(err) => PyValueError::new_err(err.to_string()),
            sync::SyncError::EditError(err) => PyValueError::new_err(err.to_string()),
        }
    }
}

impl From<lsp::error::ServerError> for PyErr {
    fn from(err: lsp::error::ServerError) -> Self {
        match err {
            lsp::error::ServerError::Initialize => errors::TachSetupError::new_err(err.to_string()),
            _ => PyOSError::new_err(err.to_string()),
        }
    }
}

impl From<config::edit::EditError> for PyErr {
    fn from(err: config::edit::EditError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<config::error::ConfigError> for PyErr {
    fn from(err: config::error::ConfigError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl IntoPy<PyObject> for modules::error::VisibilityErrorInfo {
    fn into_py(self, py: pyo3::prelude::Python<'_>) -> PyObject {
        (
            self.dependent_module,
            self.dependency_module,
            self.visibility,
        )
            .into_py(py)
    }
}

/// Parse project config
#[pyfunction]
#[pyo3(signature = (filepath))]
fn parse_project_config(
    filepath: PathBuf,
) -> parsing::config::Result<(config::ProjectConfig, bool)> {
    parsing::config::parse_project_config(filepath)
}

#[pyfunction]
#[pyo3(signature = (config))]
fn dump_project_config_to_toml(
    config: &mut config::ProjectConfig,
) -> Result<String, sync::SyncError> {
    // TODO: Error handling hack
    parsing::config::dump_project_config_to_toml(config).map_err(sync::SyncError::TomlSerialize)
}

/// Get first-party imports from file_path
#[pyfunction]
#[pyo3(signature = (source_roots, file_path, ignore_type_checking_imports=false, include_string_imports=false))]
fn get_project_imports(
    source_roots: Vec<String>,
    file_path: String,
    ignore_type_checking_imports: bool,
    include_string_imports: bool,
) -> processors::import::Result<Vec<processors::import::NormalizedImport>> {
    let source_roots: Vec<PathBuf> = source_roots.iter().map(PathBuf::from).collect();
    let file_path = PathBuf::from(file_path);
    commands::helpers::import::get_project_imports(
        &source_roots,
        &file_path,
        ignore_type_checking_imports,
        include_string_imports,
    )
}

/// Get third-party imports from file_path
#[pyfunction]
#[pyo3(signature = (source_roots, file_path, ignore_type_checking_imports=false))]
fn get_external_imports(
    source_roots: Vec<String>,
    file_path: String,
    ignore_type_checking_imports: bool,
) -> processors::import::Result<Vec<processors::import::NormalizedImport>> {
    let source_roots: Vec<PathBuf> = source_roots.iter().map(PathBuf::from).collect();
    let file_path = PathBuf::from(file_path);
    commands::helpers::import::get_external_imports(
        &source_roots,
        &file_path,
        ignore_type_checking_imports,
    )
}

/// Set excluded paths globally.
/// This is called separately in order to set up a singleton instance holding regex/glob patterns,
/// since they would be expensive to build for every call.
#[pyfunction]
#[pyo3(signature = (project_root, exclude_paths, use_regex_matching))]
fn set_excluded_paths(
    project_root: String,
    exclude_paths: Vec<String>,
    use_regex_matching: bool,
) -> exclusion::Result<()> {
    let project_root = PathBuf::from(project_root);
    let exclude_paths: Vec<PathBuf> = exclude_paths.iter().map(PathBuf::from).collect();
    exclusion::set_excluded_paths(&project_root, &exclude_paths, use_regex_matching)
}

/// Validate external dependency imports against pyproject.toml dependencies
#[pyfunction]
#[pyo3(signature = (project_root, project_config, module_mappings, stdlib_modules))]
fn check_external_dependencies(
    project_root: String,
    project_config: config::ProjectConfig,
    module_mappings: HashMap<String, Vec<String>>,
    stdlib_modules: Vec<String>,
) -> check::check_external::Result<Vec<diagnostics::Diagnostic>> {
    let project_root = PathBuf::from(project_root);
    check::check_external::check(
        &project_root,
        &project_config,
        &module_mappings,
        &stdlib_modules,
    )
}

/// Create a report of dependencies and usages of a given path
#[pyfunction]
#[pyo3(signature = (project_root, project_config, path, include_dependency_modules, include_usage_modules, skip_dependencies, skip_usages, raw))]
fn create_dependency_report(
    project_root: String,
    project_config: &config::ProjectConfig,
    path: String,
    include_dependency_modules: Option<Vec<String>>,
    include_usage_modules: Option<Vec<String>>,
    skip_dependencies: bool,
    skip_usages: bool,
    raw: bool,
) -> report::Result<String> {
    let project_root = PathBuf::from(project_root);
    let file_path = PathBuf::from(path);
    report::create_dependency_report(
        &project_root,
        project_config,
        &file_path,
        include_dependency_modules,
        include_usage_modules,
        skip_dependencies,
        skip_usages,
        raw,
    )
}

#[pyfunction]
#[pyo3(signature = (project_root, source_roots, action, py_interpreter_version, file_dependencies, env_dependencies, backend))]
fn create_computation_cache_key(
    project_root: String,
    source_roots: Vec<String>,
    action: String,
    py_interpreter_version: String,
    file_dependencies: Vec<String>,
    env_dependencies: Vec<String>,
    backend: String,
) -> String {
    let project_root = PathBuf::from(project_root);
    let source_roots: Vec<PathBuf> = source_roots.iter().map(PathBuf::from).collect();
    cache::create_computation_cache_key(
        &project_root,
        &source_roots,
        action,
        py_interpreter_version,
        file_dependencies,
        env_dependencies,
        backend,
    )
}

#[pyfunction]
#[pyo3(signature = (project_root, cache_key))]
fn check_computation_cache(
    project_root: String,
    cache_key: String,
) -> cache::Result<Option<cache::ComputationCacheValue>> {
    cache::check_computation_cache(project_root, cache_key)
}

#[pyfunction]
#[pyo3(signature = (project_root, cache_key, value))]
fn update_computation_cache(
    project_root: String,
    cache_key: String,
    value: cache::ComputationCacheValue,
) -> cache::Result<Option<cache::ComputationCacheValue>> {
    cache::update_computation_cache(project_root, cache_key, value)
}

#[pyfunction]
#[pyo3(name = "check", signature = (project_root, project_config, dependencies, interfaces, exclude_paths))]
fn check_internal(
    project_root: PathBuf,
    project_config: &config::ProjectConfig,
    dependencies: bool,
    interfaces: bool,
    exclude_paths: Vec<String>,
) -> check::check_internal::Result<Vec<diagnostics::Diagnostic>> {
    check::check_internal(
        project_root,
        project_config,
        dependencies,
        interfaces,
        exclude_paths,
    )
}

#[pyfunction]
pub fn format_diagnostics(
    project_root: PathBuf,
    diagnostics: Vec<diagnostics::Diagnostic>,
) -> String {
    check::format::DiagnosticFormatter::new(project_root).format_diagnostics(&diagnostics)
}

#[pyfunction]
#[pyo3(signature = (project_root, project_config, exclude_paths))]
fn detect_unused_dependencies(
    project_root: PathBuf,
    project_config: &mut config::ProjectConfig,
    exclude_paths: Vec<String>,
) -> Result<Vec<sync::UnusedDependencies>, sync::SyncError> {
    sync::detect_unused_dependencies(project_root, project_config, exclude_paths)
}

#[pyfunction]
#[pyo3(signature = (project_root, project_config, exclude_paths, add))]
pub fn sync_project(
    project_root: PathBuf,
    project_config: config::ProjectConfig,
    exclude_paths: Vec<String>,
    add: bool,
) -> Result<(), sync::SyncError> {
    sync::sync_project(project_root, project_config, exclude_paths, add)
}

#[pyfunction]
#[pyo3(signature = (project_root, project_config))]
fn run_server(
    project_root: PathBuf,
    project_config: config::ProjectConfig,
) -> Result<(), lsp::error::ServerError> {
    server::run_server(project_root, project_config)
}

#[pyfunction]
#[pyo3(signature = (modules))]
fn serialize_modules_json(modules: Vec<config::ModuleConfig>) -> String {
    config::serialize_modules_json(&modules)
}

#[pymodule]
fn extension(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    interrupt::setup_interrupt_handler();
    m.add_class::<config::ProjectConfig>()?;
    m.add_class::<config::ModuleConfig>()?;
    m.add_class::<config::InterfaceConfig>()?;
    m.add_class::<config::RulesConfig>()?;
    m.add_class::<config::DependencyConfig>()?;
    m.add_class::<diagnostics::Diagnostic>()?;
    m.add_class::<test::TachPytestPluginHandler>()?;
    m.add_class::<modularity::UsageError>()?;
    m.add_function(wrap_pyfunction_bound!(parse_project_config, m)?)?;
    m.add_function(wrap_pyfunction_bound!(get_project_imports, m)?)?;
    m.add_function(wrap_pyfunction_bound!(get_external_imports, m)?)?;
    m.add_function(wrap_pyfunction_bound!(set_excluded_paths, m)?)?;
    m.add_function(wrap_pyfunction_bound!(check_external_dependencies, m)?)?;
    m.add_function(wrap_pyfunction_bound!(create_dependency_report, m)?)?;
    m.add_function(wrap_pyfunction_bound!(create_computation_cache_key, m)?)?;
    m.add_function(wrap_pyfunction_bound!(check_computation_cache, m)?)?;
    m.add_function(wrap_pyfunction_bound!(update_computation_cache, m)?)?;
    m.add_function(wrap_pyfunction_bound!(dump_project_config_to_toml, m)?)?;
    m.add_function(wrap_pyfunction_bound!(check_internal, m)?)?;
    m.add_function(wrap_pyfunction_bound!(format_diagnostics, m)?)?;
    m.add_function(wrap_pyfunction_bound!(detect_unused_dependencies, m)?)?;
    m.add_function(wrap_pyfunction_bound!(sync_project, m)?)?;
    m.add_function(wrap_pyfunction_bound!(run_server, m)?)?;
    m.add_function(wrap_pyfunction_bound!(serialize_modules_json, m)?)?;
    m.add_function(wrap_pyfunction_bound!(serialize_diagnostics_json, m)?)?;
    m.add_function(wrap_pyfunction_bound!(into_usage_errors, m)?)?;
    Ok(())
}
