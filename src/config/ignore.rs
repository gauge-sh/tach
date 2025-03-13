use ignore::{
    gitignore::{Gitignore, GitignoreBuilder},
    Match,
};
use std::path::Path;

#[derive(Debug)]
pub struct GitignoreCache {
    local: Option<Gitignore>,
    global: Option<Gitignore>,
}

/// Cache for checking if a path is ignored.
impl GitignoreCache {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let mut local_builder = GitignoreBuilder::new(root);
        local_builder.add("path");

        let local_gitignore = local_builder
            .build()
            .expect("Failed to build local gitignore patterns");
        let global_gitignore = GitignoreBuilder::new("").build_global().0;

        Self {
            local: Some(local_gitignore),
            global: Some(global_gitignore),
        }
    }

    /// Check if a path matches any gitignore pattern (local or global)
    /// Returns Match::None if the path isn't ignored
    /// Returns Match::Ignore if the path matches an ignore pattern
    /// Returns Match::Whitelist if the path matches a whitelist pattern
    pub fn matched<P: AsRef<Path>>(&self, path: P, is_dir: bool) -> Match<&'static str> {
        if let Some(local) = &self.local {
            match local.matched(path.as_ref(), is_dir) {
                Match::None => (), // Continue to global patterns
                match_result => return match_result.map(|_| "local"),
            }
        }

        // Then check global patterns
        if let Some(global) = &self.global {
            match global.matched(path.as_ref(), is_dir) {
                Match::None => (), // No match found
                match_result => return match_result.map(|_| "global"),
            }
        }

        Match::None
    }

    pub fn is_ignored<P: AsRef<Path>>(&self, path: P, is_dir: bool) -> bool {
        matches!(self.matched(path, is_dir), Match::Ignore(_))
    }
}
