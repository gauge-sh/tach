use super::error::InterfaceError;
use super::matcher::{CompiledInterface, CompiledInterfaceIterExt, CompiledInterfaces};
use crate::core::config::ModuleConfig;
use crate::filesystem::module_to_file_path;
use crate::python::parsing::parse_python_source;
use std::collections::HashMap;
use std::path::PathBuf;

use ruff_python_ast::{
    statement_visitor::{walk_stmt, StatementVisitor},
    Expr, Mod, Stmt,
};

#[derive(Debug, Clone)]
pub enum InterfaceMemberStatus {
    Serializable,
    NotSerializable,
    Unknown, // not in any interface marked as serializable, or could not determine
}

#[derive(Debug, Clone)]
pub struct SerializableChecker {
    interface_member_status: HashMap<String, InterfaceMemberStatus>,
}

impl SerializableChecker {
    pub fn build(
        compiled_interfaces: &CompiledInterfaces,
        modules: &[ModuleConfig],
        source_roots: &[PathBuf],
    ) -> Result<Self, InterfaceError> {
        let serializable_interfaces: Vec<&CompiledInterface> =
            compiled_interfaces.serializable().collect();
        let module_paths: Vec<&str> = modules
            .iter()
            .filter_map(|module| {
                if serializable_interfaces
                    .clone()
                    .into_iter()
                    .matching(module.path.as_str())
                    .next()
                    .is_some()
                {
                    Some(module.path.as_str())
                } else {
                    None
                }
            })
            .collect();

        Ok(Self {
            interface_member_status: parse_typed_interface_members(
                source_roots,
                &module_paths,
                &serializable_interfaces,
            )?,
        })
    }

    pub fn is_serializable(&self, member: &str) -> InterfaceMemberStatus {
        self.interface_member_status
            .get(member)
            .cloned()
            .unwrap_or(InterfaceMemberStatus::Unknown)
    }
}

struct ModuleInterfaceVisitor<'a> {
    interfaces: Vec<&'a CompiledInterface>,
    member_status: &'a mut HashMap<String, InterfaceMemberStatus>,
}

impl StatementVisitor<'_> for ModuleInterfaceVisitor<'_> {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        // extract the top level assignments, functions and classes
        // with type annotations
        // then check if the types are serializable (use unknown for classes for now)
        // then update the interface_member_status map
        ()
    }
}

pub fn parse_typed_interface_members(
    source_roots: &[PathBuf],
    module_paths: &[&str],
    interfaces: &[&CompiledInterface],
) -> Result<HashMap<String, InterfaceMemberStatus>, InterfaceError> {
    let mut member_status = HashMap::new();

    // for each module, parse the source files
    let mut visitor = ModuleInterfaceVisitor {
        interfaces: interfaces.to_vec(),
        member_status: &mut member_status,
    };

    for module_path in module_paths {
        // dont want to fully traverse the module, but should probably use the interface patterns to find the appropriate files?
        let resolved_mod = module_to_file_path(source_roots, module_path, false).unwrap();
        let python_source = std::fs::read_to_string(resolved_mod.file_path).unwrap();
        let ast = match parse_python_source(&python_source) {
            Ok(Mod::Module(ast)) => ast,
            _ => panic!("Expected ast::Mod variant"),
        };
        visitor.visit_body(&ast.body);
    }

    Ok(member_status)
}
