use globset::{Error, GlobBuilder, GlobMatcher};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::sync::Arc;

use crate::exclusion::PathExclusions;
use crate::filesystem;

pub fn has_glob_syntax(pattern: &str) -> bool {
    pattern.chars().enumerate().any(|(i, c)| {
        match c {
            '*' | '?' | '[' | ']' | '{' | '}' => {
                // Check if the character is escaped
                i == 0 || pattern.as_bytes()[i - 1] != b'\\'
            }
            _ => false,
        }
    })
}

pub fn build_matcher(pattern: &str) -> Result<GlobMatcher, Error> {
    let mut glob_builder = GlobBuilder::new(pattern);
    let matcher = glob_builder
        .literal_separator(true)
        .empty_alternates(true)
        .build()?
        .compile_matcher();
    Ok(matcher)
}

pub fn find_matching_directories<P: AsRef<Path>>(
    root_path: P,
    pattern: &str,
    path_exclusions: Arc<PathExclusions>,
    respect_gitignore: bool,
) -> Result<Vec<PathBuf>, Error> {
    let matcher = build_matcher(&format!(
        "{}{}{}",
        root_path.as_ref().display(),
        MAIN_SEPARATOR,
        pattern
    ))?;

    let matching_dirs = filesystem::walk_dirs(
        root_path.as_ref().to_str().unwrap(),
        path_exclusions.clone(),
        respect_gitignore,
    )
    .filter(|entry| matcher.is_match(entry.as_os_str().to_str().unwrap()))
    .collect();

    Ok(matching_dirs)
}
