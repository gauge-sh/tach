use ruff_python_ast::Mod;
use ruff_python_parser::{parse, Mode, ParseError};

pub fn parse_python_source(python_source: &str) -> Result<Mod, ParseError> {
    parse(python_source, Mode::Module)
}
