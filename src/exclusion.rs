use once_cell::sync::Lazy;
use regex::{Error, Regex};
use std::sync::Mutex;

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

pub fn set_excluded_paths(exclude_paths: Vec<String>) -> Result<()> {
    match PATH_EXCLUSIONS_SINGLETON.lock() {
        Ok(mut exclusions) => {
            let _ = exclusions.insert(PathExclusions::try_from(exclude_paths)?);
            Ok(())
        }
        Err(_) => {
            return Err(PathExclusionError {
                message: "A concurrency error occurred when setting excluded paths.".to_string(),
            })
        }
    }
}

impl PathExclusions {
    fn is_path_excluded(&self, path: &str) -> bool {
        for re in &self.regexes {
            if re.is_match(path) {
                return true;
            }
        }
        return false;
    }
}

impl TryFrom<Vec<String>> for PathExclusions {
    type Error = PathExclusionError;
    fn try_from(value: Vec<String>) -> std::result::Result<Self, Self::Error> {
        let mut regexes: Vec<Regex> = vec![];
        for pattern in value.iter() {
            regexes.push(Regex::new(pattern.as_str())?);
        }
        Ok(Self { regexes })
    }
}

pub fn is_path_excluded(path: &str) -> Result<bool> {
    match PATH_EXCLUSIONS_SINGLETON.lock() {
        Ok(exclusions) => {
            if exclusions
                .as_ref()
                .is_some_and(|path_exclusions| path_exclusions.is_path_excluded(path))
            {
                return Ok(true);
            } else {
                return Ok(false);
            }
        }
        Err(_) => Err(PathExclusionError {
            message: "A concurrency error occurred when setting excluded paths.".to_string(),
        }),
    }
}
