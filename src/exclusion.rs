use once_cell::sync::Lazy;
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::pattern::PatternMatcher;
pub struct PathExclusionError {
    pub message: String,
}

pub type Result<T> = std::result::Result<T, PathExclusionError>;

pub struct PathExclusions {
    patterns: Vec<PatternMatcher>,
}

static PATH_EXCLUSIONS_SINGLETON: Lazy<Mutex<Option<PathExclusions>>> =
    Lazy::new(|| Mutex::new(None));

pub fn set_excluded_paths(
    project_root: &Path,
    exclude_paths: &[PathBuf],
    use_regex_matching: bool,
) -> Result<()> {
    let mut exclusions = PATH_EXCLUSIONS_SINGLETON
        .lock()
        .map_err(|_| PathExclusionError {
            message: "A concurrency error occurred when setting excluded paths.".to_string(),
        })?;
    let absolute_excluded_paths: Vec<PathBuf> = exclude_paths
        .iter()
        .map(|path| project_root.join(path))
        .collect();
    *exclusions = Some(PathExclusions::try_from_with_mode(
        absolute_excluded_paths,
        use_regex_matching,
    )?);
    Ok(())
}

impl PathExclusions {
    fn is_path_excluded(&self, path: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(path))
    }

    fn try_from_with_mode(
        from: Vec<PathBuf>,
        use_regex_matching: bool,
    ) -> std::result::Result<Self, PathExclusionError> {
        let mut patterns: Vec<PatternMatcher> = vec![];
        for pattern in from.iter() {
            let pattern_str = pattern.to_str().unwrap();
            patterns.push(if use_regex_matching {
                PatternMatcher::from_regex(pattern_str)?
            } else {
                PatternMatcher::from_glob(pattern_str)?
            });
        }
        Ok(Self { patterns })
    }
}

pub fn is_path_excluded(path: &str) -> Result<bool> {
    PATH_EXCLUSIONS_SINGLETON
        .lock()
        .map(|exclusions| {
            exclusions
                .as_ref()
                .is_some_and(|path_exclusions| path_exclusions.is_path_excluded(path))
        })
        .map_err(|_| PathExclusionError {
            message: "A concurrency error occurred when setting excluded paths.".to_string(),
        })
}
