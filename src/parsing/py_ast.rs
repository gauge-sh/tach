use std::{ops::Deref, path::PathBuf};

use ruff_python_ast::{
    statement_visitor::{walk_stmt, StatementVisitor},
    Expr, Mod, Stmt,
};
use ruff_python_parser::{parse, Mode};

use crate::filesystem::module_to_file_path;

use super::error::Result;

/// Use the ruff-python-parser crate to parse a Python source file into an AST
pub fn parse_python_source(python_source: &str) -> Result<Mod> {
    Ok(parse(python_source, Mode::Module)?)
}

struct InterfaceVisitor {
    members: Vec<String>,
}

impl StatementVisitor<'_> for InterfaceVisitor {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        if let Stmt::Assign(node) = stmt {
            if node
                .targets
                .iter()
                .any(|target| matches!(target, Expr::Name(name) if name.id == "__all__"))
            {
                if let Expr::List(list) = node.value.deref() {
                    for element in &list.elts {
                        if let Expr::StringLiteral(s) = element {
                            self.members.push(s.value.to_string());
                        }
                    }
                } else {
                    walk_stmt(self, stmt);
                }
            }
        }
    }
}

pub fn parse_interface_members(source_roots: &[PathBuf], path: &str) -> Result<Vec<String>> {
    if let Some(resolved_mod) = module_to_file_path(source_roots, path, false) {
        let python_source = std::fs::read_to_string(resolved_mod.file_path)?;
        let ast = match parse_python_source(&python_source)? {
            Mod::Module(ast) => ast,
            Mod::Expression(_) => panic!("Expected ast::Mod variant"),
        };

        let mut visitor = InterfaceVisitor {
            members: Vec::new(),
        };
        visitor.visit_body(&ast.body);

        Ok(visitor.members)
    } else {
        Ok(Vec::new())
    }
}
