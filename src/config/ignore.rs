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
    ///
    /// # Returns
    ///
    /// A new GitignoreMatcher instance configured based on the provided arguments.
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let mut local_builder = GitignoreBuilder::new(root.as_ref());
        local_builder.add(root.as_ref().join(".gitignore"));

        let local_gitignore = local_builder
            .build()
            .expect("Failed to build local gitignore patterns");
        let global_gitignore = GitignoreBuilder::new("").build_global().0;

        Self {
            local: Some(local_gitignore),
            global: Some(global_gitignore),
        }
    }

    /// Create a new GitignoreMatcher that does not check any paths.
    ///
    /// # Returns
    ///
    /// A new GitignoreMatcher instance that does not check any paths.
    pub fn disabled() -> Self {
        Self {
            local: None,
            global: None,
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

    /// Check if a path or any of its parent directories matches any gitignore pattern.
    ///
    /// Returns:
    ///   - `Match::None` if neither the path nor its parents are ignored
    ///   - `Match::Ignore("local"|"global")` if the path or a parent matches an ignore pattern
    ///   - `Match::Whitelist("local"|"global")` if the path or a parent matches a whitelist pattern
    fn matched_with_parents<P: AsRef<Path>>(&self, path: P, is_dir: bool) -> Match<&'static str> {
        if let Some(local) = &self.local {
            match local.matched_path_or_any_parents(path.as_ref(), is_dir) {
                Match::None => (),
                match_result => return match_result.map(|_| "local"),
            }
        }

        if let Some(global) = &self.global {
            match global.matched_path_or_any_parents(path.as_ref(), is_dir) {
                Match::None => (),
                match_result => return match_result.map(|_| "global"),
            }
        }

        Match::None
    }

    /// Check if a path or any of its parent directories is ignored by any gitignore pattern.
    ///
    /// Returns `true` if the path or any parent matches an ignore pattern, `false` otherwise.
    pub fn is_ignored_with_parents<P: AsRef<Path>>(&self, path: P, is_dir: bool) -> bool {
        matches!(self.matched_with_parents(path, is_dir), Match::Ignore(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use tempfile::TempDir;

    #[fixture]
    fn temp_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    fn create_gitignore(dir: &TempDir, content: &str) {
        std::fs::write(dir.path().join(".gitignore"), content).unwrap();
    }

    #[rstest]
    fn test_empty_matcher(temp_dir: TempDir) {
        let matcher = GitignoreMatcher::new(temp_dir.path());
        assert!(!matcher.is_ignored("some/path", false));
        assert!(!matcher.is_ignored("file.txt", false));
    }

    #[rstest]
    #[case("*.txt", "file.txt", true, true, false)]
    #[case("*.txt", "path/to/doc.txt", true, true, false)]
    #[case("build/", "build/output.txt", false, true, false)]
    #[case("build/", "src/build/file.txt", false, true, false)]
    #[case("!important.txt", "important.txt", false, false, false)]
    #[case("/node_modules/", "node_modules/package/file.js", false, true, false)]
    #[case("foo/", "foo/bar/baz.txt", false, true, false)]
    fn test_gitignore_patterns(
        #[case] pattern: &str,
        #[case] path: &str,
        #[case] should_ignore: bool,
        #[case] should_ignore_with_parents: bool,
        #[case] is_dir: bool,
        temp_dir: TempDir,
    ) {
        create_gitignore(&temp_dir, pattern);
        let matcher = GitignoreMatcher::new(temp_dir.path());

        assert_eq!(
            matcher.is_ignored(path, is_dir),
            should_ignore,
            "Path should {} be ignored",
            if should_ignore { "" } else { "not" }
        );

        assert_eq!(
            matcher.is_ignored_with_parents(path, is_dir),
            should_ignore_with_parents,
            "Path should {} be ignored with parents",
            if should_ignore_with_parents {
                ""
            } else {
                "not"
            }
        );

        let never_ignore_matcher = GitignoreMatcher::disabled();
        assert!(
            !never_ignore_matcher.is_ignored(path, is_dir),
            "Path should never be ignored when matcher is disabled"
        );
        assert!(
            !never_ignore_matcher.is_ignored_with_parents(path, is_dir),
            "Path should never be ignored with parents when matcher is disabled"
        );
    }

    #[rstest]
    fn test_directory_vs_file_matching(temp_dir: TempDir) {
        create_gitignore(&temp_dir, "build/\n!build/keep.txt");
        let matcher = GitignoreMatcher::new(temp_dir.path());

        // Test regular matching
        assert!(
            matcher.is_ignored("build", true),
            "Expected build to be ignored"
        );
        assert!(
            !matcher.is_ignored("build/output.txt", false),
            "Expected build/output.txt to not be ignored"
        );
        assert!(
            !matcher.is_ignored("build/keep.txt", false),
            "Expected build/keep.txt to not be ignored"
        );

        // Test matching with parents
        assert!(
            matcher.is_ignored_with_parents("build", true),
            "Expected build to be ignored (with parents)"
        );
        assert!(
            matcher.is_ignored_with_parents("build/output.txt", false),
            "Expected build/output.txt to be ignored (with parents)"
        );
        assert!(
            !matcher.is_ignored_with_parents("build/keep.txt", false),
            "Expected build/keep.txt to not be ignored (with parents)"
        );
        assert!(
            matcher.is_ignored_with_parents("build/subdir/file.txt", false),
            "Expected build/subdir/file.txt to be ignored due to parent"
        );
    }
}
