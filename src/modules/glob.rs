use globset::{Error as GlobError, GlobBuilder, GlobMatcher};
use std::path::{Path, PathBuf};

use crate::{exclusion::PathExclusions, filesystem};

#[derive(Debug)]
enum ModuleGlobSegment {
    Literal(String),
    Wildcard,
    DoubleWildcard,
}

#[derive(Debug)]
struct ModuleGlob {
    segments: Vec<ModuleGlobSegment>,
}

impl ModuleGlob {
    pub fn parse(pattern: &str) -> Option<Self> {
        if !pattern.contains('*') {
            // No wildcards, not a glob
            return None;
        }

        let segments = pattern
            .split('.')
            .map(|s| match s {
                "*" => ModuleGlobSegment::Wildcard,
                "**" => ModuleGlobSegment::DoubleWildcard,
                _ => ModuleGlobSegment::Literal(s.to_string()),
            })
            .collect();
        Some(Self { segments })
    }

    pub fn into_matcher(self) -> Result<GlobMatcher, GlobError> {
        let pattern = self
            .segments
            .iter()
            .map(|s| match s {
                ModuleGlobSegment::Literal(s) => globset::escape(s),
                ModuleGlobSegment::Wildcard => "*".to_string(),
                ModuleGlobSegment::DoubleWildcard => "**".to_string(),
            })
            .collect::<Vec<_>>()
            .join("/");
        let mut glob_builder = GlobBuilder::new(&pattern);
        Ok(glob_builder
            .literal_separator(true)
            .build()?
            .compile_matcher())
    }
}

#[derive(Debug)]
pub struct ModuleGlobResolver {
    modules: Vec<PathBuf>,
}

impl ModuleGlobResolver {
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

    pub fn resolve_module_path(&self, path: &str) -> Result<Vec<String>, GlobError> {
        let glob = match ModuleGlob::parse(path) {
            Some(glob) => glob,
            // If not a glob, return the path as is
            None => return Ok(vec![path.to_string()]),
        };

        let matcher = glob.into_matcher()?;
        let res = Ok(self
            .modules
            .iter()
            .filter(|m| matcher.is_match(m))
            .map(|m| {
                m.with_extension("")
                    .display()
                    .to_string()
                    .replace(std::path::MAIN_SEPARATOR, ".")
            })
            .collect());
        eprintln!("Resolved glob for: {} to {:?}", path, res);
        res
    }
}
