use once_cell::sync::Lazy;
use glob::{Pattern, PatternError};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct PathExclusionError {
    pub message: String,
}

pub type Result<T> = std::result::Result<T, PathExclusionError>;

impl From<PatternError> for PathExclusionError {
    fn from(_value: PatternError) -> Self {
        Self {
            message: "Failed to build glob patterns for excluded paths".to_string(),
        }
    }
}

#[derive(Default)]
pub struct PathExclusions {
    patterns: Vec<Pattern>,
}

static PATH_EXCLUSIONS_SINGLETON: Lazy<Mutex<Option<PathExclusions>>> =
    Lazy::new(|| Mutex::new(None));

pub fn set_excluded_paths(project_root: &Path, exclude_paths: &[PathBuf]) -> Result<()> {
    match PATH_EXCLUSIONS_SINGLETON.lock() {
        Ok(mut exclusions) => {
            let absolute_excluded_paths: Vec<PathBuf> = exclude_paths
                .iter()
                .map(|path| project_root.join(path))
                .collect();
            let _ = exclusions.insert(PathExclusions::try_from(absolute_excluded_paths)?);
            Ok(())
        }
        Err(_) => Err(PathExclusionError {
            message: "A concurrency error occurred when setting excluded paths.".to_string(),
        }),
    }
}

impl PathExclusions {
    fn is_path_excluded(&self, path: &str) -> bool {
        for pattern in &self.patterns {
            if pattern.matches(path) {
                return true;
            }
        }
        false
    }
}

impl TryFrom<Vec<PathBuf>> for PathExclusions {
    type Error = PathExclusionError;
    fn try_from(value: Vec<PathBuf>) -> std::result::Result<Self, Self::Error> {
        let mut patterns: Vec<Pattern> = vec![];
        for pattern in value.iter() {
            patterns.push(Pattern::new(pattern.to_str().unwrap())?);
        }
        Ok(Self { patterns })
    }
}

pub fn is_path_excluded(path: &str) -> Result<bool> {
    match PATH_EXCLUSIONS_SINGLETON.lock() {
        Ok(exclusions) => {
            if exclusions
                .as_ref()
                .is_some_and(|path_exclusions| path_exclusions.is_path_excluded(path))
            {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(_) => Err(PathExclusionError {
            message: "A concurrency error occurred when setting excluded paths.".to_string(),
        }),
    }
}
