pub mod cache;
pub mod check;
pub mod cli;
pub mod colors;
pub mod exclusion;
pub mod filesystem;
pub mod imports;
pub mod parsing;
pub mod pattern;
pub mod reports;

use std::collections::HashMap;
use std::path::PathBuf;

use cache::ComputationCacheValue;
use pyo3::exceptions::{PyOSError, PySyntaxError, PyValueError};
use pyo3::prelude::*;

impl From<imports::ImportParseError> for PyErr {
    fn from(err: imports::ImportParseError) -> Self {
        match err.err_type {
            imports::ImportParseErrorType::FILESYSTEM => PyOSError::new_err(err.message),
            imports::ImportParseErrorType::PARSING => PySyntaxError::new_err(err.message),
        }
    }
}

impl From<exclusion::PathExclusionError> for PyErr {
    fn from(err: exclusion::PathExclusionError) -> Self {
        PyValueError::new_err(err.message)
    }
}

impl From<reports::ReportCreationError> for PyErr {
    fn from(err: reports::ReportCreationError) -> Self {
        PyValueError::new_err(err.message)
    }
}

impl From<cache::CacheError> for PyErr {
    fn from(_: cache::CacheError) -> Self {
        PyValueError::new_err("Failure accessing computation cache.")
    }
}

impl From<check::CheckError> for PyErr {
    fn from(err: check::CheckError) -> Self {
        match err {
            check::CheckError::Parse(err) => PyOSError::new_err(err.to_string()),
            check::CheckError::ImportParse(err) => err.into(),
            check::CheckError::Io(err) => PyOSError::new_err(err.to_string()),
            check::CheckError::Filesystem(err) => PyOSError::new_err(err.to_string()),
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

/// Parse project config
#[pyfunction]
#[pyo3(signature = (filepath))]
fn parse_project_config(filepath: String) -> parsing::Result<parsing::config::ProjectConfig> {
    parsing::config::parse_project_config(filepath)
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
) -> check::Result<check::ExternalCheckDiagnostics> {
    let project_root = PathBuf::from(project_root);
    let source_roots: Vec<PathBuf> = source_roots.iter().map(PathBuf::from).collect();
    check::check_external_dependencies(
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

#[pymodule]
fn extension(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<parsing::config::ProjectConfig>()?;
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
    Ok(())
}
