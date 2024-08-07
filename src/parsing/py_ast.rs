use ruff_python_ast::Mod;
use ruff_python_parser::{parse, Mode};

use super::error;

/// Use the ruff-python-parser crate to parse a Python source file into an AST
pub fn parse_python_source(python_source: &str) -> error::Result<Mod> {
    Ok(parse(python_source, Mode::Module)?)
}
