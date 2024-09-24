pub mod cache;
pub mod check_ext;
pub mod check_int;
pub mod cli;
pub mod colors;
pub mod core;
pub mod exclusion;
pub mod filesystem;
pub mod imports;
pub mod parsing;
pub mod pattern;
pub mod reports;
pub mod sync;
pub mod test;
pub mod tests;

use core::config::ProjectConfig;
use std::collections::HashMap;
use std::path::PathBuf;

use cache::ComputationCacheValue;
use pyo3::exceptions::{PyOSError, PySyntaxError, PyValueError};
use pyo3::prelude::*;
use sync::SyncError;

impl From<imports::ImportParseError> for PyErr {
    fn from(err: imports::ImportParseError) -> Self {
        match err {
            imports::ImportParseError::Parsing { file: _, source: _ } => {
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

impl From<reports::ReportCreationError> for PyErr {
    fn from(err: reports::ReportCreationError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<cache::CacheError> for PyErr {
    fn from(err: cache::CacheError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

impl From<check_ext::ExternalCheckError> for PyErr {
    fn from(err: check_ext::ExternalCheckError) -> Self {
        PyOSError::new_err(err.to_string())
    }
}

impl From<check_int::CheckError> for PyErr {
    fn from(err: check_int::CheckError) -> Self {
        if let check_int::CheckError::ModuleTree(
            parsing::error::ModuleTreeError::CircularDependency(c),
        ) = err
        {
            PyErr::new::<TachCircularDependencyError, _>(c)
        } else if let check_int::CheckError::ModuleTree(
            parsing::error::ModuleTreeError::VisibilityViolation(v),
        ) = err
        {
            PyErr::new::<TachVisibilityError, _>(v)
        } else {
            PyValueError::new_err(err.to_string())
        }
    }
}

impl From<parsing::ParsingError> for PyErr {
    fn from(err: parsing::ParsingError) -> Self {
        match err {
            parsing::ParsingError::PythonParse(err) => PySyntaxError::new_err(err.to_string()),
            parsing::ParsingError::Io(err) => PyOSError::new_err(err.to_string()),
            parsing::ParsingError::Filesystem(err) => PyOSError::new_err(err.to_string()),
            parsing::ParsingError::TomlParse(err) => PyValueError::new_err(err.to_string()),
            parsing::ParsingError::MissingField(err) => PyValueError::new_err(err),
        }
    }
}
impl From<sync::SyncError> for PyErr {
    fn from(err: sync::SyncError) -> Self {
        match err {
            SyncError::FileWrite(err) => PyOSError::new_err(err.to_string()),
            SyncError::TomlSerialize(err) => PyOSError::new_err(err.to_string()),
        }
    }
}

/// Parse project config
#[pyfunction]
#[pyo3(signature = (filepath))]
fn parse_project_config(filepath: PathBuf) -> parsing::Result<core::config::ProjectConfig> {
    parsing::config::parse_project_config(filepath)
}

#[pyfunction]
#[pyo3(signature = (config))]
fn dump_project_config_to_toml(config: &mut ProjectConfig) -> Result<String, SyncError> {
    // TODO: Error handling hack
    parsing::config::dump_project_config_to_toml(config).map_err(SyncError::TomlSerialize)
}

#[pyfunction]
#[pyo3(signature = (source_roots, file_path, ignore_type_checking_imports=false))]
fn get_normalized_imports(
    source_roots: Vec<String>,
    file_path: String,
    ignore_type_checking_imports: bool,
) -> imports::Result<imports::NormalizedImports> {
    let source_roots: Vec<PathBuf> = source_roots.iter().map(PathBuf::from).collect();
    let file_path = PathBuf::from(file_path);
    imports::get_normalized_imports(&source_roots, &file_path, ignore_type_checking_imports)
}

/// Get first-party imports from file_path
#[pyfunction]
#[pyo3(signature = (source_roots, file_path, ignore_type_checking_imports=false))]
fn get_project_imports(
    source_roots: Vec<String>,
    file_path: String,
    ignore_type_checking_imports: bool,
) -> imports::Result<imports::NormalizedImports> {
    let source_roots: Vec<PathBuf> = source_roots.iter().map(PathBuf::from).collect();
    let file_path = PathBuf::from(file_path);
    imports::get_project_imports(&source_roots, &file_path, ignore_type_checking_imports)
}

/// Get third-party imports from file_path
#[pyfunction]
#[pyo3(signature = (source_roots, file_path, ignore_type_checking_imports=false))]
fn get_external_imports(
    source_roots: Vec<String>,
    file_path: String,
    ignore_type_checking_imports: bool,
) -> imports::Result<imports::NormalizedImports> {
    let source_roots: Vec<PathBuf> = source_roots.iter().map(PathBuf::from).collect();
    let file_path = PathBuf::from(file_path);
    Ok(
        imports::get_normalized_imports(&source_roots, &file_path, ignore_type_checking_imports)?
            .into_iter()
            .filter_map(|import| {
                imports::is_project_import(&source_roots, &import.module_path).map_or(
                    None,
                    |is_project_import| {
                        if is_project_import {
                            None
                        } else {
                            Some(import)
                        }
                    },
                )
            })
            .collect(),
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
#[pyo3(signature = (project_root, source_roots, module_mappings, ignore_type_checking_imports=false))]
fn check_external_dependencies(
    project_root: String,
    source_roots: Vec<String>,
    module_mappings: HashMap<String, Vec<String>>,
    ignore_type_checking_imports: bool,
) -> check_ext::Result<check_ext::ExternalCheckDiagnostics> {
    let project_root = PathBuf::from(project_root);
    let source_roots: Vec<PathBuf> = source_roots.iter().map(PathBuf::from).collect();
    check_ext::check_external_dependencies(
        &project_root,
        &source_roots,
        &module_mappings,
        ignore_type_checking_imports,
    )
}

/// Create a report of dependencies and usages of a given path
#[pyfunction]
#[pyo3(signature = (project_root, source_roots, path, include_dependency_modules, include_usage_modules, skip_dependencies, skip_usages, ignore_type_checking_imports=false))]
fn create_dependency_report(
    project_root: String,
    source_roots: Vec<String>,
    path: String,
    include_dependency_modules: Option<Vec<String>>,
    include_usage_modules: Option<Vec<String>>,
    skip_dependencies: bool,
    skip_usages: bool,
    ignore_type_checking_imports: bool,
) -> reports::Result<String> {
    let project_root = PathBuf::from(project_root);
    let source_roots: Vec<PathBuf> = source_roots.iter().map(PathBuf::from).collect();
    let file_path = PathBuf::from(path);
    reports::create_dependency_report(
        &project_root,
        &source_roots,
        &file_path,
        include_dependency_modules,
        include_usage_modules,
        skip_dependencies,
        skip_usages,
        ignore_type_checking_imports,
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
) -> cache::Result<Option<ComputationCacheValue>> {
    cache::check_computation_cache(project_root, cache_key)
}

#[pyfunction]
#[pyo3(signature = (project_root, cache_key, value))]
fn update_computation_cache(
    project_root: String,
    cache_key: String,
    value: ComputationCacheValue,
) -> cache::Result<Option<ComputationCacheValue>> {
    cache::update_computation_cache(project_root, cache_key, value)
}

#[pyfunction]
#[pyo3(signature = (source_roots, path))]
fn parse_interface_members(
    source_roots: Vec<PathBuf>,
    path: String,
) -> parsing::Result<Vec<String>> {
    parsing::py_ast::parse_interface_members(&source_roots, &path)
}

#[pyfunction]
#[pyo3(signature = (project_root, project_config, exclude_paths))]
fn check(
    project_root: PathBuf,
    project_config: &ProjectConfig,
    exclude_paths: Vec<String>,
) -> PyResult<check_int::CheckDiagnostics> {
    check_int::check(project_root, project_config, exclude_paths).map_err(Into::into)
}

#[pyfunction]
#[pyo3(signature = (project_root, project_config, exclude_paths, prune))]
fn sync_dependency_constraints(
    project_root: PathBuf,
    project_config: ProjectConfig,
    exclude_paths: Vec<String>,
    prune: bool,
) -> ProjectConfig {
    sync::sync_dependency_constraints(project_root, project_config, exclude_paths, prune)
}

#[pyfunction]
#[pyo3(signature = (project_root, project_config, exclude_paths, add))]
pub fn sync_project(
    project_root: PathBuf,
    project_config: ProjectConfig,
    exclude_paths: Vec<String>,
    add: bool,
) -> Result<String, SyncError> {
    sync::sync_project(project_root, project_config, exclude_paths, add)
}

#[pymodule]
fn extension(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<core::config::ProjectConfig>()?;
    m.add_class::<core::config::ModuleConfig>()?;
    m.add_class::<test::TachPytestPluginHandler>()?;
    m.add_function(wrap_pyfunction_bound!(parse_project_config, m)?)?;
    m.add_function(wrap_pyfunction_bound!(get_project_imports, m)?)?;
    m.add_function(wrap_pyfunction_bound!(get_external_imports, m)?)?;
    m.add_function(wrap_pyfunction_bound!(get_normalized_imports, m)?)?;
    m.add_function(wrap_pyfunction_bound!(set_excluded_paths, m)?)?;
    m.add_function(wrap_pyfunction_bound!(check_external_dependencies, m)?)?;
    m.add_function(wrap_pyfunction_bound!(create_dependency_report, m)?)?;
    m.add_function(wrap_pyfunction_bound!(create_computation_cache_key, m)?)?;
    m.add_function(wrap_pyfunction_bound!(check_computation_cache, m)?)?;
    m.add_function(wrap_pyfunction_bound!(update_computation_cache, m)?)?;
    m.add_function(wrap_pyfunction_bound!(parse_interface_members, m)?)?;
    m.add_function(wrap_pyfunction_bound!(dump_project_config_to_toml, m)?)?;
    m.add_function(wrap_pyfunction_bound!(check, m)?)?;
    m.add_function(wrap_pyfunction_bound!(sync_dependency_constraints, m)?)?;
    m.add_function(wrap_pyfunction_bound!(sync_project, m)?)?;
    m.add(
        "TachCircularDependencyError",
        py.get_type_bound::<TachCircularDependencyError>(),
    )?;
    m.add(
        "TachVisibilityError",
        py.get_type_bound::<TachVisibilityError>(),
    )?;
    Ok(())
}

#[pyclass(extends=PyValueError)]
struct TachCircularDependencyError {
    #[pyo3(get)]
    dependencies: Vec<String>,
}

#[pymethods]
impl TachCircularDependencyError {
    #[new]
    fn new(dependencies: Vec<String>) -> Self {
        Self { dependencies }
    }
}

#[pyclass(extends=PyValueError)]
struct TachVisibilityError {
    #[pyo3(get)]
    visibility_errors: Vec<PyObject>,
}

#[pymethods]
impl TachVisibilityError {
    #[new]
    fn new(visibility_errors: Vec<PyObject>) -> Self {
        Self { visibility_errors }
    }
}
