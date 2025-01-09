use std::path::PathBuf;

pub const DEFAULT_EXCLUDE_PATHS: [&str; 4] = ["tests", "docs", ".*__pycache__", ".*egg-info"];

// for serde
pub fn default_true() -> bool {
    true
}
pub fn default_source_roots() -> Vec<PathBuf> {
    vec![PathBuf::from(".")]
}

pub fn default_excludes() -> Vec<String> {
    DEFAULT_EXCLUDE_PATHS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn global_visibility() -> Vec<String> {
    vec!["*".to_string()]
}

pub fn default_visibility() -> Vec<String> {
    global_visibility()
}

pub fn is_default_visibility(value: &Vec<String>) -> bool {
    value == &default_visibility()
}

pub fn is_true(value: &bool) -> bool {
    *value
}

pub fn is_false(value: &bool) -> bool {
    !*value
}
