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
        Ok(PatternMatcher::Regex(regex::Regex::new(pattern)?))
    }

    pub fn from_glob(pattern: &str) -> Result<Self, PathExclusionError> {
        Ok(PatternMatcher::Glob(glob::Pattern::new(pattern)?))
    }
}

impl From<glob::PatternError> for PathExclusionError {
    fn from(_value: glob::PatternError) -> Self {
        Self {
            message: "Failed to build glob patterns for excluded paths".to_string(),
        }
    }
}

impl From<regex::Error> for PathExclusionError {
    fn from(_value: regex::Error) -> Self {
        Self {
            message: "Failed to build regex patterns for excluded paths".to_string(),
        }
    }
}
