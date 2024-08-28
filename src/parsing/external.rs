use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

use super::error;

pub struct ProjectInfo {
    pub dependencies: HashSet<String>,
    pub source_paths: Vec<PathBuf>,
}

pub fn parse_pyproject_toml(pyproject_path: &Path) -> error::Result<ProjectInfo> {
    let content = fs::read_to_string(pyproject_path)?;
    let toml_value: Value = toml::from_str(&content)?;
    let dependencies = extract_dependencies(&toml_value);
    let source_paths = extract_source_paths(&toml_value, pyproject_path.parent().unwrap());
    Ok(ProjectInfo {
        dependencies,
        source_paths,
    })
}

pub fn extract_dependencies(toml_value: &Value) -> HashSet<String> {
    let mut dependencies = HashSet::new();

    // Extract dependencies from standard pyproject.toml format
    let has_project_deps = toml_value
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .map_or(false, |deps| {
            extract_deps_from_value(&mut dependencies, deps);
            true
        });

    let has_poetry_deps = toml_value
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .is_some();

    // Print warning if both formats are detected
    if has_project_deps && has_poetry_deps {
        eprintln!("Warning: Both project dependencies and Poetry dependencies detected. Using project dependencies.");
    } else if has_poetry_deps {
        // Extract Poetry dependencies only if project dependencies are not present
        if let Some(deps) = toml_value
            .get("tool")
            .and_then(|tool| tool.get("poetry"))
            .and_then(|poetry| poetry.get("dependencies"))
        {
            extract_deps_from_value(&mut dependencies, deps)
        }
    }

    dependencies
}

fn extract_deps_from_value(dependencies: &mut HashSet<String>, deps: &Value) {
    match deps {
        Value::Array(deps_array) => {
            for dep_str in deps_array.iter().filter_map(|dep| dep.as_str()) {
                dependencies.insert(normalize_package_name(&extract_package_name(dep_str)));
            }
        }
        Value::Table(deps_table) => {
            for dep_name in deps_table.keys() {
                dependencies.insert(normalize_package_name(&extract_package_name(dep_name)));
            }
        }
        _ => {}
    }
}

fn extract_package_name(dep_str: &str) -> String {
    // Split on common separators and take the first part
    dep_str
        .split(&[' ', '=', '<', '>', '~', ';', '['][..])
        .next()
        .unwrap_or(dep_str)
        .to_string()
}

/// This normalizes a Python distribution name according to PyPI standards
pub fn normalize_package_name(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("_")
}

pub fn extract_source_paths(toml_value: &Value, project_root: &Path) -> Vec<PathBuf> {
    let mut source_paths = Vec::new();

    // Check for setuptools configuration
    if let Some(packages) = toml_value
        .get("tool")
        .and_then(|t| t.get("setuptools"))
        .and_then(|setuptools| setuptools.get("packages"))
        .and_then(|p| p.as_array())
    {
        for package_name in packages.iter().filter_map(|package| package.as_str()) {
            source_paths.push(project_root.join(package_name));
        }
    }

    // Check for poetry configuration
    if let Some(packages) = toml_value
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("packages"))
        .and_then(|p| p.as_array())
    {
        for package in packages {
            if let Some(include) = package.get("include").and_then(|i| i.as_str()) {
                let from = package.get("from").and_then(|f| f.as_str()).unwrap_or("");
                source_paths.push(project_root.join(from).join(include));
            }
        }
    }

    // Check for maturin configuration
    if let Some(python_source) = toml_value
        .get("tool")
        .and_then(|t| t.get("maturin"))
        .and_then(|m| m.get("python-source"))
        .and_then(|ps| ps.as_str())
    {
        source_paths.push(project_root.join(python_source));
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
