use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use toml::Value;

use pyo3::conversion::IntoPy;
use pyo3::PyObject;

use crate::filesystem::walk_pyfiles;

#[derive(Debug)]
enum DependencyError {
    Io(io::Error),
    Toml(toml::de::Error),
    PyprojectNotFound,
    ImportExtractionFailed,
}

impl std::fmt::Display for DependencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DependencyError::Io(err) => write!(f, "IO error: {}", err),
            DependencyError::Toml(err) => write!(f, "TOML parsing error: {}", err),
            DependencyError::PyprojectNotFound => write!(f, "pyproject.toml not found"),
            DependencyError::ImportExtractionFailed => {
                write!(f, "Failed to extract imports from Python file")
            }
        }
    }
}

impl std::error::Error for DependencyError {}

impl From<io::Error> for DependencyError {
    fn from(err: io::Error) -> DependencyError {
        DependencyError::Io(err)
    }
}

impl From<toml::de::Error> for DependencyError {
    fn from(err: toml::de::Error) -> DependencyError {
        DependencyError::Toml(err)
    }
}

struct ProjectInfo {
    dependencies: HashSet<String>,
    source_paths: Vec<PathBuf>,
}

pub struct ExternalCheckResult {
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl IntoPy<PyObject> for ExternalCheckResult {
    fn into_py(self, py: pyo3::prelude::Python<'_>) -> PyObject {
        (self.errors, self.warnings).into_py(py)
    }
}

pub fn check_external_dependencies(
    project_root: &Path,
) -> Result<ExternalCheckResult, DependencyError> {
    let pyproject_path = find_pyproject_toml(project_root)?;
    let project_info = parse_pyproject_toml(&pyproject_path)?;
    // use project_info.source_paths
    let source_files = walk_pyfiles(project_root.to_str().unwrap());

    let mut warnings: Vec<String> = Vec::new();
    let mut errors = Vec::new();

    for file_path in source_files {
        // need to implement this in imports module
        let imports = extract_imports(&file_path)?;
        for import in imports {
            if !project_info.dependencies.contains(&import) {
                errors.push(format!(
                    "External dependency '{}' found in {}",
                    import,
                    file_path.display()
                ));
            }
        }
    }

    Ok(ExternalCheckResult { errors, warnings })
}

fn find_pyproject_toml(project_root: &Path) -> Result<PathBuf, DependencyError> {
    // need to figure out a way to more intelligently iterate over these
    // also should move to filesystem module
    let pyproject_path = project_root.join("pyproject.toml");
    if pyproject_path.exists() {
        Ok(pyproject_path)
    } else {
        Err(DependencyError::PyprojectNotFound)
    }
}

fn parse_pyproject_toml(pyproject_path: &Path) -> Result<ProjectInfo, DependencyError> {
    // this should move to parsing module
    let content = fs::read_to_string(pyproject_path)?;
    let toml_value: Value = toml::from_str(&content)?;

    let dependencies = extract_dependencies(&toml_value);
    let source_paths = extract_source_paths(&toml_value, pyproject_path.parent().unwrap());

    Ok(ProjectInfo {
        dependencies,
        source_paths,
    })
}

fn extract_dependencies(toml_value: &Value) -> HashSet<String> {
    // this should probably move to parsing module
    let mut dependencies = HashSet::new();

    // this should also check poetry
    if let Some(project) = toml_value.get("project") {
        if let Some(deps) = project.get("dependencies") {
            if let Some(deps_array) = deps.as_array() {
                for dep in deps_array {
                    if let Some(dep_str) = dep.as_str() {
                        dependencies.insert(dep_str.split_whitespace().next().unwrap().to_string());
                    }
                }
            }
        }
    }

    dependencies
}

fn extract_source_paths(toml_value: &Value, project_root: &Path) -> Vec<PathBuf> {
    // this should probably move to parsing module
    let mut source_paths = Vec::new();

    // Check for setuptools configuration
    if let Some(setuptools) = toml_value.get("tool").and_then(|t| t.get("setuptools")) {
        if let Some(packages) = setuptools.get("packages").and_then(|p| p.as_array()) {
            for package in packages {
                if let Some(package_name) = package.as_str() {
                    source_paths.push(project_root.join(package_name));
                }
            }
        }
    }

    // Check for poetry configuration
    if let Some(poetry) = toml_value.get("tool").and_then(|t| t.get("poetry")) {
        if let Some(packages) = poetry.get("packages").and_then(|p| p.as_array()) {
            for package in packages {
                if let Some(include) = package.get("include").and_then(|i| i.as_str()) {
                    let from = package.get("from").and_then(|f| f.as_str()).unwrap_or("");
                    source_paths.push(project_root.join(from).join(include));
                }
            }
        }
    }

    // If no specific configuration found, use conventional locations
    if source_paths.is_empty() {
        let src_dir = project_root.join("src");
        if src_dir.exists() {
            source_paths.push(src_dir);
        } else {
            source_paths.push(project_root.to_path_buf());
        }
    }

    source_paths
}
