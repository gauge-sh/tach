use once_cell::sync::Lazy;
use regex::{Error, Regex};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct PathExclusionError {
    pub message: String,
}

pub type Result<T> = std::result::Result<T, PathExclusionError>;

impl From<Error> for PathExclusionError {
    fn from(_value: Error) -> Self {
        Self {
            message: "Failed to build regex patterns for excluded paths".to_string(),
        }
    }
}

#[derive(Default)]
pub struct PathExclusions {
    regexes: Vec<Regex>,
}

static PATH_EXCLUSIONS_SINGLETON: Lazy<Mutex<Option<PathExclusions>>> =
    Lazy::new(|| Mutex::new(None));

pub fn set_excluded_paths(project_root: &Path, exclude_paths: &[PathBuf]) -> Result<()> {
    let mut exclusions = PATH_EXCLUSIONS_SINGLETON
        .lock()
        .map_err(|_| PathExclusionError {
            message: "A concurrency error occurred when setting excluded paths.".to_string(),
        })?;
    let absolute_excluded_paths: Vec<PathBuf> = exclude_paths
        .iter()
        .map(|path| project_root.join(path))
        .collect();
    *exclusions = Some(PathExclusions::try_from(absolute_excluded_paths)?);
    Ok(())
}

impl PathExclusions {
    fn is_path_excluded(&self, path: &str) -> bool {
        for re in &self.regexes {
            if re.is_match(path) {
                return true;
            }
        }
        false
    }
}

impl TryFrom<Vec<PathBuf>> for PathExclusions {
    type Error = PathExclusionError;
    fn try_from(value: Vec<PathBuf>) -> std::result::Result<Self, Self::Error> {
        let mut regexes: Vec<Regex> = vec![];
        for pattern in value.iter() {
            regexes.push(Regex::new(pattern.to_str().unwrap())?);
        }
        Ok(Self { regexes })
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
