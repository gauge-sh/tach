use globset::{Error, GlobBuilder, GlobMatcher};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use walkdir::WalkDir;

use crate::config::ignore::GitignoreCache;
use crate::exclusion::PathExclusions;
use crate::filesystem::{direntry_is_excluded, is_hidden};

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
    path_exclusions: &PathExclusions,
    gitignore_cache: &GitignoreCache,
) -> Result<Vec<PathBuf>, Error> {
    let matcher = build_matcher(&format!(
        "{}{}{}",
        root_path.as_ref().display(),
        MAIN_SEPARATOR,
        pattern
    ))?;

    let matching_dirs = WalkDir::new(root_path)
        .into_iter()
        .filter_entry(|e| {
            !is_hidden(e) && !direntry_is_excluded(e, path_exclusions, gitignore_cache)
        })
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_dir())
        .filter(|entry| {
            entry
                .path()
                .as_os_str()
                .to_str()
                .is_some_and(|path| matcher.is_match(path))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    Ok(matching_dirs)
}
