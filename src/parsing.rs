use rustpython_parser::{ast, Parse, ParseError};

pub fn parse_python_source(python_source: &str) -> Result<ast::Suite, ParseError> {
    ast::Suite::parse(python_source, "irrelevant")
}
