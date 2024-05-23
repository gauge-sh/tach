use rustpython_parser::{ast, Parse};

pub fn parse_python_source(python_source: &str) -> ast::Suite {
    return ast::Suite::parse(python_source, "irrelevant");
}
