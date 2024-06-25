use std::env;
// use std::path::PathBuf;
use std::fmt;
use crate::imports::{Dependency};


#[derive(Debug, PartialEq, Eq)]
enum TerminalEnvironment {
    Unknown,
    JetBrains,
    VSCode,
}


fn detect_environment() -> TerminalEnvironment {
    let terminal_emulator = env::var("TERMINAL_EMULATOR").unwrap_or_default().to_lowercase();
    let term_program = env::var("TERM_PROGRAM").unwrap_or_default().to_lowercase();

    if terminal_emulator.contains("jetbrains") {
        TerminalEnvironment::JetBrains
    } else if term_program.contains("vscode") {
        TerminalEnvironment::VSCode
    } else {
        TerminalEnvironment::Unknown
    }
}

pub fn create_clickable_link(dependency: &Dependency) -> String {
    let terminal_env = detect_environment();
    let file_path = dependency.file_path.to_string_lossy().to_string();
    let abs_path = dependency.absolute_path.to_string_lossy().to_string();
    let line = dependency.import.line_no;
    let link = match terminal_env {
        TerminalEnvironment::JetBrains => {
            format!("file://{}:{}", abs_path, line)
        },
        TerminalEnvironment::VSCode => {
            format!("vscode://file/{}:{}", abs_path, line)
        },
        TerminalEnvironment::Unknown => {
            format!("file://{}", abs_path)
        }
    };
    let display_with_line = format!("{}[L{}]", file_path, line);
    return format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", link, display_with_line);
}
