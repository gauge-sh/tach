use globset::{Error as GlobError, GlobMatcher};
use itertools::Itertools;
use rayon::prelude::*;
use std::path::PathBuf;

use super::glob;
use crate::config::root_module::ROOT_MODULE_SENTINEL_TAG;
use crate::filesystem;

#[derive(Debug)]
pub struct ModuleGlob {
    segments: Vec<String>,
}

impl ModuleGlob {
    pub fn parse(pattern: &str) -> Option<Self> {
        if !glob::has_glob_syntax(pattern) {
            return None;
        }

        Some(Self {
            segments: pattern.split('.').map(|s| s.to_string()).collect(),
        })
    }

    pub fn into_matcher(self) -> Result<GlobMatcher, GlobError> {
        let mut pattern = self.segments.iter().join("/");

        if pattern.ends_with("/**") {
            // We want this to match both the module itself and any submodules,
            //   which means we need to make the trailing slash optional.
            pattern = pattern[..pattern.len() - 3].to_string();
            pattern = format!("{}{{,/**}}", &pattern);
        }

        // Add allowed file extensions to the pattern
        pattern = format!("{}{{,.py,.pyi}}", pattern);

        glob::build_matcher(&pattern)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleResolverError {
    #[error("Error handling glob: {0}")]
    Glob(#[from] GlobError),
    #[error("Module path '{path}' is not valid")]
    InvalidModulePath { path: String },
}

#[derive(Debug)]
pub struct ModuleResolver<'a> {
    source_roots: &'a [PathBuf],
    file_walker: &'a filesystem::FSWalker,
}

impl<'a> ModuleResolver<'a> {
    pub fn new(source_roots: &'a [PathBuf], file_walker: &'a filesystem::FSWalker) -> Self {
        Self {
            source_roots,
            file_walker,
        }
    }

    fn validate_module_path_literal(&self, path: &str) -> bool {
        filesystem::module_to_pyfile_or_dir_path(self.source_roots, path).is_some()
    }

    pub fn resolve_module_path(&self, path: &str) -> Result<Vec<String>, ModuleResolverError> {
        if path == "." {
            return Ok(vec![ROOT_MODULE_SENTINEL_TAG.to_string()]);
        }

        let glob = match ModuleGlob::parse(path) {
            Some(glob) => glob,
            // If not a glob, validate the path as a literal
            None => {
                if self.validate_module_path_literal(path) {
                    return Ok(vec![path.to_string()]);
                } else {
                    return Err(ModuleResolverError::InvalidModulePath {
                        path: path.to_string(),
                    });
                }
            }
        };

        let matcher = glob.into_matcher()?;
        Ok(self
            .source_roots
            .par_iter()
            .flat_map(|root| {
                self.file_walker
                    .walk_pymodules(root.as_os_str().to_str().unwrap())
                    .par_bridge()
                    .filter(|m| matcher.is_match(m))
                    .map(|m| {
                        m.with_extension("")
                            .display()
                            .to_string()
                            .replace(std::path::MAIN_SEPARATOR, ".")
                    })
            })
            .collect())
    }
}
