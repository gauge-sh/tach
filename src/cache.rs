use cached::stores::DiskCacheBuildError;
use cached::{DiskCache, DiskCacheError, IOCached};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::{env, fs};
use thiserror::Error;
use toml::Value;

use crate::filesystem::{self, walk_pyfiles};

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Disk cache error: {0}")]
    DiskCache(#[from] DiskCacheError),
    #[error("Disk cache build error: {0}")]
    DiskCacheBuild(#[from] DiskCacheBuildError),
}

pub type Result<T> = std::result::Result<T, CacheError>;

#[derive(Debug)]
struct CacheKey {
    hash: String,
}

impl FromIterator<u8> for CacheKey {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        let mut hasher = DefaultHasher::new();
        for item in iter {
            item.hash(&mut hasher);
        }
        let hash = format!("{:016X}", hasher.finish());
        CacheKey { hash }
    }
}

static CACHE_DIR: &str = ".tach";

pub type ComputationCacheValue = (Vec<(u8, String)>, u8);

fn build_computation_cache<P: AsRef<Path>>(
    project_root: P,
) -> Result<DiskCache<String, ComputationCacheValue>> {
    Ok(
        DiskCache::<String, ComputationCacheValue>::new("computation-cache")
            .set_disk_directory(
                project_root
                    .as_ref()
                    .join(CACHE_DIR)
                    .join("computation-cache"),
            )
            .build()?,
    )
}

fn parse_project_dependencies<P: AsRef<Path>>(project_root: P) -> impl Iterator<Item = String> {
    let project_root = project_root.as_ref();
    let mut dependencies = Vec::new();

    // Check for requirements.txt
    let requirements_path = project_root.join("requirements.txt");
    if requirements_path.is_file() {
        if let Ok(content) = filesystem::read_file_content(&requirements_path) {
            for line in content.lines() {
                if !line.trim().is_empty() && !line.trim().starts_with('#') {
                    dependencies.push(line.trim().to_string());
                }
            }
            return dependencies.into_iter();
        }
    }

    // Check for pyproject.toml
    let pyproject_path = project_root.join("pyproject.toml");
    if pyproject_path.is_file() {
        let content = filesystem::read_file_content(&pyproject_path).unwrap_or_default();
        let toml_value = content.parse::<Value>().unwrap_or(Value::Integer(0));
        if let Some(dependencies_array) = toml_value
            .get("project")
            .and_then(|v| v.get("dependencies"))
            .and_then(|v| v.as_array())
        {
            for dep_str in dependencies_array.iter().filter_map(|dep| dep.as_str()) {
                dependencies.push(dep_str.to_string());
            }
        }
        // Handle optional dependencies if necessary
        if let Some(optional_dependencies) = toml_value
            .get("project")
            .and_then(|v| v.get("optional-dependencies"))
            .and_then(|v| v.as_table())
        {
            for deps_array in optional_dependencies
                .values()
                .filter_map(|deps| deps.as_array())
            {
                for dep_str in deps_array.iter().filter_map(|dep| dep.as_str()) {
                    dependencies.push(dep_str.to_string());
                }
            }
        }

        return dependencies.into_iter();
    }

    // Didn't find any dependencies
    println!("Did not auto-detect dependencies. Is there a 'requirements.txt' or 'pyproject.toml' in your project root?");

    vec![].into_iter()
}

fn read_file_dependencies(
    project_root: &str,
    file_dependencies: Vec<String>,
) -> impl Iterator<Item = u8> {
    filesystem::walk_globbed_files(project_root, file_dependencies)
        .flat_map(|path| fs::read(path).unwrap())
}

fn read_env_dependencies(env_dependencies: Vec<String>) -> impl Iterator<Item = String> {
    env_dependencies.into_iter().map(|var| {
        let value = env::var(&var).unwrap_or_else(|_| "".to_string());
        format!("{}={}", var, value)
    })
}

pub fn create_computation_cache_key(
    project_root: &PathBuf,
    source_roots: &[PathBuf],
    action: String,
    py_interpreter_version: String,
    file_dependencies: Vec<String>,
    env_dependencies: Vec<String>,
    _backend: String,
) -> String {
    let source_pyfiles = source_roots.iter().flat_map(|root| {
        walk_pyfiles(root.to_str().unwrap())
            .flat_map(move |path| fs::read(root.join(path)).unwrap())
    });
    let env_dependencies = read_env_dependencies(env_dependencies).flat_map(|d| d.into_bytes());
    let project_dependencies =
        parse_project_dependencies(&project_root).flat_map(|d| d.into_bytes());
    let file_dependencies =
        read_file_dependencies(project_root.to_str().unwrap(), file_dependencies);
    CacheKey::from_iter(
        source_pyfiles
            .chain(env_dependencies)
            .chain(project_dependencies)
            .chain(file_dependencies)
            .chain(action.into_bytes())
            .chain(py_interpreter_version.into_bytes()),
    )
    .hash
}

pub fn check_computation_cache(
    project_root: String,
    cache_key: String,
) -> Result<Option<ComputationCacheValue>> {
    let cache = build_computation_cache(project_root)?;

    Ok(cache.cache_get(&cache_key)?)
}

pub fn update_computation_cache(
    project_root: String,
    cache_key: String,
    value: ComputationCacheValue,
) -> Result<Option<ComputationCacheValue>> {
    let cache = build_computation_cache(project_root)?;

    Ok(cache.cache_set(cache_key, value)?)
}
