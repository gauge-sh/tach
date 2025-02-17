use ignore::{
    gitignore::{Gitignore, GitignoreBuilder},
    Match,
};
use std::path::Path;

#[derive(Debug)]
pub struct GitignoreMatcher {
    local: Option<Gitignore>,
    global: Option<Gitignore>,
}

/// Matcher for checking if a path is ignored by gitignore patterns.
impl GitignoreMatcher {
    /// Create a new GitignoreMatcher that checks paths against gitignore patterns.
    ///
    /// # Arguments
    ///
    /// * `root` - The root directory path to search for .gitignore files.
    /// * `never_ignore` - If true, the matcher will never consider any paths ignored,
    ///                   effectively disabling gitignore functionality.
    ///
    /// # Returns
    ///
    /// A new GitignoreMatcher instance configured based on the provided arguments.
    /// If never_ignore is true, returns a matcher with no patterns that will never
    /// match any paths.
    pub fn new<P: AsRef<Path>>(root: P, never_ignore: bool) -> Self {
        if never_ignore {
            return Self {
                local: None,
                global: None,
            };
        }

        let mut local_builder = GitignoreBuilder::new(root);
        local_builder.add(".gitignore");

        let local_gitignore = local_builder
            .build()
            .expect("Failed to build local gitignore patterns");
        let global_gitignore = GitignoreBuilder::new("").build_global().0;

        Self {
            local: Some(local_gitignore),
            global: Some(global_gitignore),
        }
    }

    /// Check if a path matches any gitignore pattern (local or global).
    ///
    /// Returns:
    ///   - `Match::None` if the path isn't ignored
    ///   - `Match::Ignore("local"|"global")` if the path matches an ignore pattern
    ///   - `Match::Whitelist("local"|"global")` if the path matches a whitelist pattern
    fn matched<P: AsRef<Path>>(&self, path: P, is_dir: bool) -> Match<&'static str> {
        if let Some(local) = &self.local {
            match local.matched(path.as_ref(), is_dir) {
                Match::None => (),
                match_result => return match_result.map(|_| "local"),
            }
        }

        // Then check global patterns
        if let Some(global) = &self.global {
            match global.matched(path.as_ref(), is_dir) {
                Match::None => (),
                match_result => return match_result.map(|_| "global"),
            }
        }

        Match::None
    }

    /// Check if a path is ignored by any gitignore pattern (local or global).
    ///
    /// Returns `true` if the path matches an ignore pattern, `false` otherwise.
    pub fn is_ignored<P: AsRef<Path>>(&self, path: P, is_dir: bool) -> bool {
        matches!(self.matched(path, is_dir), Match::Ignore(_))
    }
}
