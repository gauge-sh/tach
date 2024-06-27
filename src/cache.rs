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

fn build_computation_cache<P: AsRef<Path>>(project_root: P) -> Result<DiskCache<String, String>> {
    Ok(DiskCache::<String, String>::new("computation-cache")
        .set_disk_directory(
            project_root
                .as_ref()
                .join(CACHE_DIR)
                .join("computation-cache"),
        )
        .build()?)
}

pub fn check_computation_cache(
    project_root: String,
    action: String,
    py_interpreter_version: String,
    file_dependencies: Vec<String>,
    env_dependencies: Vec<String>,
    backend: String,
) -> Result<Option<String>> {
    let cache = build_computation_cache(&project_root)?;

    // next step is to actually parse environment, external dependency versions
    // also need to allow Python to send configuration about which env vars to check, maybe even which deps to check/exclude
    let cache_key = CacheKey::from_iter(
        walk_pyfiles(&project_root)
            .map(|path| read_file_content(&path).unwrap())
            .chain(std::iter::once(action)),
    );
    Ok(cache.cache_get(&cache_key.hash)?)
}
