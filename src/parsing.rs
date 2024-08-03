use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml::Value;

use ruff_python_ast::Mod;
use ruff_python_parser::{parse, Mode, ParseError};

#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("Python parsing error: {0}")]
    PythonParse(#[from] ParseError),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("Missing field in TOML: {0}")]
    MissingField(String),
}

pub type Result<T> = std::result::Result<T, ParsingError>;

/// Use the ruff-python-parser crate to parse a Python source file into an AST
pub fn parse_python_source(python_source: &str) -> Result<Mod> {
    Ok(parse(python_source, Mode::Module)?)
}

pub struct ProjectInfo {
    pub dependencies: HashSet<String>,
    pub source_paths: Vec<PathBuf>,
}

pub fn parse_pyproject_toml(pyproject_path: &Path) -> Result<ProjectInfo> {
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
    let mut has_project_deps = false;
    let mut has_poetry_deps = false;

    // Extract dependencies from standard pyproject.toml format
    if let Some(project) = toml_value.get("project") {
        if let Some(deps) = project.get("dependencies") {
            extract_deps_from_value(&mut dependencies, deps);
            has_project_deps = true;
        }
    }

    // Check for Poetry dependencies
    if let Some(tool) = toml_value.get("tool") {
        if let Some(poetry) = tool.get("poetry") {
            if poetry.get("dependencies").is_some() {
                has_poetry_deps = true;
            }
        }
    }

    // Print warning if both formats are detected
    if has_project_deps && has_poetry_deps {
        eprintln!("Warning: Both project dependencies and Poetry dependencies detected. Using project dependencies.");
    } else if has_poetry_deps {
        // Extract Poetry dependencies only if project dependencies are not present
        if let Some(tool) = toml_value.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                if let Some(deps) = poetry.get("dependencies") {
                    extract_deps_from_value(&mut dependencies, deps);
                }
            }
        }
    }

    dependencies
}

fn extract_deps_from_value(dependencies: &mut HashSet<String>, deps: &Value) {
    match deps {
        Value::Array(deps_array) => {
            for dep in deps_array {
                if let Some(dep_str) = dep.as_str() {
                    dependencies.insert(normalize_package_name(&extract_package_name(dep_str)));
                }
            }
        }
        Value::Table(deps_table) => {
            for dep_name in deps_table.keys() {
                dependencies.insert(normalize_package_name(dep_name));
            }
        }
        _ => {}
    }
}

fn extract_package_name(dep_str: &str) -> String {
    // Split on common separators and take the first part
    dep_str
        .split(|c| c == ' ' || c == '=' || c == '<' || c == '>' || c == '~' || c == ';')
        .next()
        .unwrap_or(dep_str)
        .to_string()
}

/// This normalizes a Python distribution name according to PyPI standards
pub fn normalize_package_name(name: &str) -> String {
    let lowercase = name.to_lowercase();
    let normalized = lowercase
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("_");
    normalized
}

pub fn extract_source_paths(toml_value: &Value, project_root: &Path) -> Vec<PathBuf> {
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

    // Check for maturin configuration
    if let Some(maturin) = toml_value.get("tool").and_then(|t| t.get("maturin")) {
        if let Some(python_source) = maturin.get("python-source").and_then(|ps| ps.as_str()) {
            source_paths.push(project_root.join(python_source));
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
