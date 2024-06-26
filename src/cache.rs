use cached::{self, DiskCacheError, IOCached};
use once_cell::sync::Lazy;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Mutex;

pub struct CacheError;

impl From<DiskCacheError> for CacheError {
    fn from(_: DiskCacheError) -> Self {
        CacheError
    }
}

pub type Result<T> = std::result::Result<T, CacheError>;

struct CacheKey {
    hash: String,
}

impl FromIterator<String> for CacheKey {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut hasher = DefaultHasher::new();
        for item in iter {
            item.hash(&mut hasher);
        }
        let hash = hasher.finish().to_string();
        CacheKey { hash }
    }
}

static CACHE_DIR: &'static str = ".tach";

static CACHE_SINGLETON: Lazy<Mutex<Option<cached::DiskCache<String, String>>>> = Lazy::new(|| {
    match cached::DiskCache::<String, String>::new("computation-cache")
        .set_disk_directory(Path::new(CACHE_DIR).join("computation-cache"))
        .build()
    {
        Ok(dc) => Mutex::new(Some(dc)),
        Err(err) => {
            println!(
                "WARNING: Failed to build computation cache. Error: {:?}",
                err
            );
            Mutex::new(None)
        }
    }
});

pub fn check_computation_cache(
    action: String,
    extra: Option<Vec<String>>,
) -> Result<Option<String>> {
    let cache_key = match extra {
        Some(extras) => CacheKey::from_iter(extras.into_iter().chain(std::iter::once(action))),
        None => CacheKey::from_iter(std::iter::once(action)),
    };
    match CACHE_SINGLETON.lock() {
        Ok(maybe_diskcache) => match *maybe_diskcache {
            Some(ref diskcache) => Ok(diskcache.cache_get(&cache_key.hash)?),
            None => Ok(None),
        },
        _ => Err(CacheError),
    }
}
