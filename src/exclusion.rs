use once_cell::sync::Lazy;
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
use thiserror::Error;

use crate::pattern::PatternMatcher;

#[derive(Error, Debug)]
pub enum PathExclusionError {
    #[error("A concurrency error occurred when setting excluded paths.")]
    ConcurrencyError,
    #[error("Failed to build glob pattern for excluded path:\n{exclude}\n{source}")]
    GlobPatternError {
        exclude: String,
        #[source]
        source: glob::PatternError,
    },
    #[error("Failed to build regex pattern for excluded path:\n{exclude}\n{source}")]
    RegexPatternError {
        exclude: String,
        #[source]
        source: regex::Error,
    },
}

pub type Result<T> = std::result::Result<T, PathExclusionError>;

pub struct PathExclusions {
    project_root: PathBuf,
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
        .map_err(|_| PathExclusionError::ConcurrencyError)?;
    *exclusions = Some(PathExclusions::try_from_with_mode(
        project_root,
        exclude_paths.into(),
        use_regex_matching,
    )?);
    Ok(())
}

impl PathExclusions {
    // Input MUST be an absolute path within the project root
    fn is_path_excluded<P: AsRef<Path>>(&self, path: P) -> bool {
        // This is for portability across OS
        // Exclude patterns in 'tach.toml' are universally written with forward slashes,
        // so we force our relative path to have forward slashes before checking for a match.
        let path_with_forward_slashes: String = path
            .as_ref()
            .strip_prefix(&self.project_root)
            .unwrap()
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");

        self.patterns
            .iter()
            .any(|p| p.matches(&path_with_forward_slashes))
    }

    fn try_from_with_mode<P: AsRef<Path>>(
        project_root: P,
        from: Vec<PathBuf>,
        use_regex_matching: bool,
    ) -> Result<Self> {
        let mut patterns: Vec<PatternMatcher> = vec![];
        for pattern in from.iter() {
            let pattern_str = pattern.to_str().unwrap();
            patterns.push(if use_regex_matching {
                PatternMatcher::from_regex(pattern_str)?
            } else {
                PatternMatcher::from_glob(pattern_str)?
            });
        }
        Ok(Self {
            project_root: project_root.as_ref().to_path_buf(),
            patterns,
        })
    }
}

pub fn is_path_excluded<P: AsRef<Path>>(path: P) -> Result<bool> {
    PATH_EXCLUSIONS_SINGLETON
        .lock()
        .map(|exclusions| {
            exclusions
                .as_ref()
                .is_some_and(|path_exclusions| path_exclusions.is_path_excluded(path))
        })
        .map_err(|_| PathExclusionError::ConcurrencyError)
}
