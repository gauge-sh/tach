use globset::{Error as GlobError, GlobBuilder, GlobMatcher};
use std::path::{Path, PathBuf};

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
            // NOTE: Using 'pattern{,/**}' does not work due to a bug in globset
            //   so we instead use '{pattern,pattern/**}'
            pattern = format!("{{{},{}/**}}", &pattern, &pattern);
        }
        let mut glob_builder = GlobBuilder::new(&pattern);
        let matcher = glob_builder
            .literal_separator(true)
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
pub struct ModuleResolver {
    modules: Vec<PathBuf>,
}

impl ModuleResolver {
    fn collect_modules<P: AsRef<Path>>(
        source_roots: &[P],
        exclusions: &PathExclusions,
    ) -> Vec<PathBuf> {
        let mut modules = Vec::new();
        for root in source_roots {
            modules.extend(filesystem::walk_pymodules(
                root.as_ref().to_str().unwrap(),
                exclusions,
            ));
        }
        modules
    }

    pub fn new<P: AsRef<Path>>(source_roots: &[P], exclusions: &PathExclusions) -> Self {
        Self {
            modules: Self::collect_modules(source_roots, exclusions),
        }
    }

    pub fn validate_module_path_literal(&self, path: &str) -> bool {
        self.modules.iter().any(|m| {
            m.with_extension("")
                .display()
                .to_string()
                .replace(std::path::MAIN_SEPARATOR, ".")
                == path
        })
    }

    pub fn is_module_path_glob(&self, path: &str) -> bool {
        ModuleGlob::parse(path).is_some()
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
            .modules
            .iter()
            .map(|m| m.with_extension(""))
            .filter(|m| matcher.is_match(m))
            .map(|m| {
                m.display()
                    .to_string()
                    .replace(std::path::MAIN_SEPARATOR, ".")
            })
            .collect())
    }
}
