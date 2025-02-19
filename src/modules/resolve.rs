use globset::{Error as GlobError, GlobBuilder, GlobMatcher};
use rayon::prelude::*;
use std::path::PathBuf;

use crate::{config::root_module::ROOT_MODULE_SENTINEL_TAG, exclusion::PathExclusions, filesystem};

#[derive(Debug)]
enum ModuleGlobSegment {
    Literal(String),
    Wildcard,
    DoubleWildcard,
}

#[derive(Debug)]
pub struct ModuleGlob {
    segments: Vec<ModuleGlobSegment>,
}

impl ModuleGlob {
    pub fn parse(pattern: &str) -> Option<Self> {
        if !pattern.contains('*') {
            // No wildcards, not a glob
            return None;
        }

        let segments: Vec<ModuleGlobSegment> = pattern
            .split('.')
            .map(|s| match s {
                "*" => ModuleGlobSegment::Wildcard,
                "**" => ModuleGlobSegment::DoubleWildcard,
                _ => ModuleGlobSegment::Literal(s.to_string()),
            })
            .collect();

        if segments
            .iter()
            .all(|s| matches!(s, ModuleGlobSegment::Literal(_)))
        {
            // No wildcard segments, not a glob
            return None;
        }

        Some(Self { segments })
    }

    pub fn into_matcher(self) -> Result<GlobMatcher, GlobError> {
        let mut pattern = self
            .segments
            .iter()
            .map(|s| match s {
                ModuleGlobSegment::Literal(s) => globset::escape(s),
                ModuleGlobSegment::Wildcard => "*".to_string(),
                ModuleGlobSegment::DoubleWildcard => "**".to_string(),
            })
            .collect::<Vec<_>>()
            .join("/");

        if pattern.ends_with("/**") {
            // We want this to match both the module itself and any submodules,
            //   which means we need to make the trailing slash optional.
            pattern = pattern[..pattern.len() - 3].to_string();
            pattern = format!("{}{{,/**}}", &pattern);
        }

        // Add allowed file extensions to the pattern
        pattern = format!("{}{{,.py,.pyi}}", pattern);

        let mut glob_builder = GlobBuilder::new(&pattern);
        let matcher = glob_builder
            .literal_separator(true)
            .empty_alternates(true)
            .build()?
            .compile_matcher();
        Ok(matcher)
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
    exclusions: &'a PathExclusions,
}

impl<'a> ModuleResolver<'a> {
    pub fn new(source_roots: &'a [PathBuf], exclusions: &'a PathExclusions) -> Self {
        Self {
            source_roots,
            exclusions,
        }
    }

    pub fn is_module_path_glob(&self, path: &str) -> bool {
        ModuleGlob::parse(path).is_some()
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
                filesystem::walk_pymodules(root.as_os_str().to_str().unwrap(), self.exclusions)
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
