use glob;
use regex;

use crate::exclusion::PathExclusionError;

pub enum PatternMatcher {
    Regex(regex::Regex),
    Glob(glob::Pattern),
}

impl PatternMatcher {
    pub fn matches(&self, text: &str) -> bool {
        match self {
            PatternMatcher::Regex(re) => re.is_match(text),
            PatternMatcher::Glob(pattern) => pattern.matches(text),
        }
    }

    pub fn from_regex(pattern: &str) -> Result<Self, PathExclusionError> {
        Ok(PatternMatcher::Regex(regex::Regex::new(pattern).map_err(
            |e| PathExclusionError::RegexPatternError {
                exclude: pattern.to_string(),
                source: e,
            },
        )?))
    }

    pub fn from_glob(pattern: &str) -> Result<Self, PathExclusionError> {
        Ok(PatternMatcher::Glob(glob::Pattern::new(pattern).map_err(
            |e| PathExclusionError::GlobPatternError {
                exclude: pattern.to_string(),
                source: e,
            },
        )?))
    }
}
