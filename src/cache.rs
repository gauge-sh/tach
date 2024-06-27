use cached::stores::DiskCacheBuildError;
use cached::{DiskCache, DiskCacheError, IOCached};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use crate::filesystem::{read_file_content, walk_pyfiles};

pub struct CacheError;

impl From<DiskCacheError> for CacheError {
    fn from(_: DiskCacheError) -> Self {
        CacheError
    }
}

impl From<DiskCacheBuildError> for CacheError {
    fn from(_: DiskCacheBuildError) -> Self {
        CacheError
    }
}

pub type Result<T> = std::result::Result<T, CacheError>;

#[derive(Debug)]
struct CacheKey {
    hash: String,
}

impl FromIterator<String> for CacheKey {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut hasher = DefaultHasher::new();
        for item in iter {
            item.hash(&mut hasher);
        }
        let hash = format!("{:016X}", hasher.finish());
        CacheKey { hash }
    }
}

static CACHE_DIR: &'static str = ".tach";

pub type ComputationCacheValue = (String, String, u8);

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

pub fn create_computation_cache_key(
    project_root: String,
    action: String,
    py_interpreter_version: String,
    file_dependencies: Vec<String>,
    env_dependencies: Vec<String>,
    backend: String,
) -> String {
    // next step is to actually parse environment, external dependency versions
    CacheKey::from_iter(
        walk_pyfiles(&project_root)
            .map(|path| read_file_content(&path).unwrap())
            .chain(std::iter::once(action)),
    )
    .hash
}

pub fn check_computation_cache(
    project_root: String,
    cache_key: String,
) -> Result<Option<ComputationCacheValue>> {
    let cache = build_computation_cache(&project_root)?;

    Ok(cache.cache_get(&cache_key)?)
}

pub fn update_computation_cache(
    project_root: String,
    cache_key: String,
    value: ComputationCacheValue,
) -> Result<Option<ComputationCacheValue>> {
    let cache = build_computation_cache(&project_root)?;

    Ok(cache.cache_set(cache_key, value)?)
}
