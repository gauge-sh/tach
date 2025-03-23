use std::path::Path;

use console::Term;

pub fn create_clickable_link(file_path: &Path, line: &usize) -> String {
    format!("{}:{}", file_path.display(), line)
}

pub fn supports_emoji() -> bool {
    let term = Term::stdout();
    term.is_term() && term.features().wants_emoji()
}

pub fn is_interactive() -> bool {
    let term = Term::stdout();
    term.is_term() && term.features().is_attended()
}

pub struct EmojiIcons;

impl EmojiIcons {
    pub const SUCCESS: &str = "✅";
    pub const WARNING: &str = "⚠️ ";
    pub const FAIL: &str = "❌";
}

pub struct SimpleIcons;

impl SimpleIcons {
    pub const SUCCESS: &str = "[OK]";
    pub const WARNING: &str = "[WARN]";
    pub const FAIL: &str = "[FAIL]";
}

pub fn success() -> &'static str {
    if supports_emoji() {
        EmojiIcons::SUCCESS
    } else {
        SimpleIcons::SUCCESS
    }
}

pub fn warning() -> &'static str {
    if supports_emoji() {
        EmojiIcons::WARNING
    } else {
        SimpleIcons::WARNING
    }
}

pub fn fail() -> &'static str {
    if supports_emoji() {
        EmojiIcons::FAIL
    } else {
        SimpleIcons::FAIL
    }
}
