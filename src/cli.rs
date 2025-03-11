use std::env;
use std::path::Path;

use console::Term;

#[derive(Debug, PartialEq, Eq)]
enum TerminalEnvironment {
    Unknown,
    JetBrains,
    VSCode,
}

fn detect_environment() -> TerminalEnvironment {
    let terminal_emulator = env::var("TERMINAL_EMULATOR")
        .unwrap_or_default()
        .to_lowercase();
    let term_program = env::var("TERM_PROGRAM").unwrap_or_default().to_lowercase();

    if terminal_emulator.contains("jetbrains") {
        TerminalEnvironment::JetBrains
    } else if term_program.contains("vscode") {
        TerminalEnvironment::VSCode
    } else {
        TerminalEnvironment::Unknown
    }
}

pub fn create_clickable_link(file_path: &Path, abs_path: &Path, line: &usize) -> String {
    if !is_interactive() {
        return format!("{}[L{}]", abs_path.display(), line);
    }
    let terminal_env = detect_environment();
    let file_path_str = file_path.to_string_lossy().to_string();
    let abs_path_str = abs_path.to_string_lossy().to_string();
    let link = match terminal_env {
        TerminalEnvironment::JetBrains => {
            format!("file://{}:{}", abs_path_str, line)
        }
        TerminalEnvironment::VSCode => {
            format!("vscode://file/{}:{}", abs_path_str, line)
        }
        TerminalEnvironment::Unknown => {
            format!("file://{}", abs_path_str)
        }
    };
    let display_with_line = format!("{}[L{}]", file_path_str, line);
    format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", link, display_with_line)
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
