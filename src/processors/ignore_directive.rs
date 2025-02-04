use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::diagnostics::Diagnostic;

use super::import::NormalizedImport;

#[derive(Debug, Clone)]
pub struct IgnoreDirective {
    pub modules: Vec<String>,
    pub reason: String,
    pub line_no: usize,         // Where is the directive literally written
    pub ignored_line_no: usize, // Where is the directive being applied
}

impl IgnoreDirective {
    pub fn matches_diagnostic(&self, diagnostic: &Diagnostic) -> bool {
        // If the diagnostic is not on the line that the directive is being applied, it is not a match
        if Some(self.ignored_line_no) != diagnostic.line_number() {
            return false;
        }

        // If the directive is a blanket ignore, it matches any diagnostic
        if self.modules.is_empty() {
            return true;
        }

        // If applicable, check if the diagnostic has specified a matching module path
        diagnostic.import_mod_path().is_none_or(|import_mod_path| {
            self.modules
                .iter()
                .any(|module| import_mod_path.ends_with(module))
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct IgnoreDirectives {
    directives: HashMap<usize, IgnoreDirective>,
    redundant_directives: Vec<IgnoreDirective>,
}

impl IgnoreDirectives {
    pub fn empty() -> Self {
        Self {
            directives: HashMap::new(),
            redundant_directives: Vec::new(),
        }
    }

    pub fn active_directives(&self) -> impl Iterator<Item = &IgnoreDirective> {
        self.directives.values()
    }

    pub fn redundant_directives(&self) -> impl Iterator<Item = &IgnoreDirective> {
        self.redundant_directives.iter()
    }

    pub fn len(&self) -> usize {
        self.directives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.directives.is_empty()
    }

    pub fn add_directive(&mut self, directive: IgnoreDirective) {
        match self.directives.entry(directive.ignored_line_no) {
            Entry::Occupied(_) => {
                self.redundant_directives.push(directive);
            }
            Entry::Vacant(entry) => {
                entry.insert(directive);
            }
        }
    }

    pub fn get(&self, line_no: &usize) -> Option<&IgnoreDirective> {
        self.directives.get(line_no)
    }

    pub fn is_ignored(&self, normalized_import: &NormalizedImport) -> bool {
        self.directives
            .get(&normalized_import.import_line_no)
            .is_some_and(|directive| {
                if normalized_import.is_absolute {
                    directive.modules.is_empty()
                        || directive.modules.contains(&normalized_import.module_path)
                } else {
                    directive.modules.is_empty()
                        || directive
                            .modules
                            .contains(normalized_import.alias_path.as_ref().unwrap())
                }
            })
    }

    pub fn remove_matching_directives(&mut self, normalized_import: &NormalizedImport) {
        self.directives
            .retain(|line_no, _directive| *line_no != normalized_import.import_line_no);
        self.redundant_directives
            .retain(|directive| directive.line_no != normalized_import.import_line_no);
    }
}

impl Extend<IgnoreDirectives> for IgnoreDirectives {
    fn extend<T: IntoIterator<Item = IgnoreDirectives>>(&mut self, iter: T) {
        for directives in iter {
            self.directives.extend(directives.directives);
            self.redundant_directives
                .extend(directives.redundant_directives);
        }
    }
}

static TACH_IGNORE_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| Regex::new(r"# *tach-ignore(?:\(([^)]*)\))?((?:\s+[\w.]+)*)\s*$").unwrap());

pub fn get_ignore_directives(file_content: &str) -> IgnoreDirectives {
    if !file_content.contains("tach-ignore") {
        return IgnoreDirectives::default();
    }

    let mut ignores = IgnoreDirectives::default();

    for (lineno, line) in file_content.lines().enumerate() {
        if !line.contains("tach-ignore") {
            continue;
        }

        let normal_lineno = lineno + 1;
        if let Some(captures) = TACH_IGNORE_REGEX.captures(line) {
            let reason = captures
                .get(1)
                .map_or("".to_string(), |m| m.as_str().to_string());
            let ignored_modules = captures.get(2).map_or("", |m| m.as_str());
            let modules: Vec<String> = if ignored_modules.is_empty() {
                Vec::new()
            } else {
                ignored_modules
                    .split_whitespace()
                    .map(|module| module.to_string())
                    .collect()
            };

            let mut ignored_line_no = normal_lineno;
            if line.trim_start().starts_with('#') {
                ignored_line_no = normal_lineno + 1;
            }
            let directive = IgnoreDirective {
                modules,
                reason,
                line_no: normal_lineno,
                ignored_line_no,
            };

            ignores.add_directive(directive);
        }
    }

    ignores
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
    "# tach-ignore\nfrom foo import bar",
    2,  // The import is on line 2
    vec![]  // Empty vec means blanket ignore
)]
    #[case(
    "# tach-ignore(test reason)\nfrom foo import bar",
    2,
    vec![]
)]
    #[case(
    "# tach-ignore foo bar\nfrom foo import bar",
    2,
    vec!["foo".to_string(), "bar".to_string()]
)]
    #[case(
    "from foo import bar  # tach-ignore",
    1,
    vec![]
)]
    #[case(
    "from foo import bar  # tach-ignore(skip this)\nother code",
    1,
    vec![]
)]
    #[case(
    "from foo import bar  # tach-ignore foo baz",
    1,
    vec!["foo".to_string(), "baz".to_string()]
)]
    fn test_get_ignore_directives(
        #[case] content: &str,
        #[case] expected_line: usize,
        #[case] expected_modules: Vec<String>,
    ) {
        let directives = get_ignore_directives(content);
        assert_eq!(directives.len(), 1);

        let directive = directives
            .get(&expected_line)
            .expect("Should have directive");
        assert_eq!(directive.modules, expected_modules);
    }

    #[test]
    fn test_no_directives() {
        let content = "from foo import bar\nother code";
        let directives = get_ignore_directives(content);
        assert!(directives.is_empty());
    }
}
