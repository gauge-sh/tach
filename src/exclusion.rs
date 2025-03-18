use itertools::Itertools;
use std::path::{Path, PathBuf};
use thiserror::Error;

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
}

pub type Result<T> = std::result::Result<T, PathExclusionError>;

#[derive(Debug)]
pub struct PathExclusions {
    project_root: PathBuf,
    patterns: Vec<glob::Pattern>,
}

impl PathExclusions {
    pub fn empty<P: AsRef<Path>>(project_root: P) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            patterns: vec![],
        }
    }

    pub fn try_new<P: AsRef<Path>>(project_root: P, exclude_paths: &[String]) -> Result<Self> {
        let patterns = exclude_paths
            .iter()
            .map(|pattern| {
                glob::Pattern::new(pattern).map_err(|e| PathExclusionError::GlobPatternError {
                    exclude: pattern.to_string(),
                    source: e,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            project_root: project_root.as_ref().to_path_buf(),
            patterns,
        })
    }

    // Input MUST be an absolute path within the project root
    pub fn is_path_excluded<P: AsRef<Path>>(&self, path: P) -> bool {
        // This is for portability across OS
        // Exclude patterns in 'tach.toml' are universally written with forward slashes,
        // so we force our relative path to have forward slashes before checking for a match.
        let path_with_forward_slashes: String = path
            .as_ref()
            .strip_prefix(&self.project_root)
            .unwrap()
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .join("/");

        self.patterns
            .iter()
            .any(|p| p.matches(&path_with_forward_slashes))
    }
}
