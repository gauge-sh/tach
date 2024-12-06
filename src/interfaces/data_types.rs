use super::error::InterfaceError;
use super::matcher::{CompiledInterface, CompiledInterfaces};
use crate::core::config::{InterfaceDataTypes, ModuleConfig};
use crate::filesystem::module_to_file_path;
use crate::python::parsing::parse_python_source;
use std::collections::HashMap;
use std::path::PathBuf;

use ruff_python_ast::{statement_visitor::StatementVisitor, Expr, Mod, Stmt};

#[derive(Debug, Clone)]
pub enum TypeCheckResult {
    MatchedInterface { expected: InterfaceDataTypes },
    DidNotMatchInterface { expected: InterfaceDataTypes },
    Unknown, // not in any interface marked as serializable, or could not determine
}

#[derive(Debug, Clone)]
pub struct TypeCheckCache {
    type_check_cache: HashMap<String, TypeCheckResult>,
}

impl TypeCheckCache {
    pub fn build(
        compiled_interfaces: &CompiledInterfaces,
        modules: &[ModuleConfig],
        source_roots: &[PathBuf],
    ) -> Result<Self, InterfaceError> {
        let module_paths: Vec<&str> = modules
            .iter()
            .filter_map(|module| {
                if compiled_interfaces.should_type_check(module.path.as_str()) {
                    Some(module.path.as_str())
                } else {
                    None
                }
            })
            .collect();

        Ok(Self {
            type_check_cache: type_check_all_interface_members(
                source_roots,
                &module_paths,
                compiled_interfaces,
            )?,
        })
    }

    pub fn get_result(&self, member: &str) -> TypeCheckResult {
        self.type_check_cache
            .get(member)
            .cloned()
            .unwrap_or(TypeCheckResult::Unknown)
    }
}

#[derive(Debug)]
struct FunctionParameter {
    name: String,
    annotation: Option<String>,
}

#[derive(Debug)]
enum InterfaceMemberNode {
    Variable {
        annotation: Option<String>,
    },
    Function {
        parameters: Vec<FunctionParameter>,
        return_type: Option<String>,
    },
    Class,
}

#[derive(Debug)]
struct InterfaceMember {
    name: String,
    node: InterfaceMemberNode,
}

struct ModuleInterfaceVisitor<'a> {
    // all interfaces to check against
    all_interfaces: &'a CompiledInterfaces,
    // current matching interfaces to check against (based on the outer module path)
    current_interfaces: Vec<&'a CompiledInterface>,
    // module prefix of the current AST being visited (based on the outer module path)
    current_module_prefix: Option<&'a str>,
    // all interface members found in the current module
    current_interface_members: Vec<InterfaceMember>,
}

impl<'a> ModuleInterfaceVisitor<'a> {
    fn new(interfaces: &'a CompiledInterfaces) -> Self {
        Self {
            all_interfaces: interfaces,
            current_interfaces: vec![],
            current_module_prefix: None,
            current_interface_members: vec![],
        }
    }

    fn get_interface_members(
        &mut self,
        module_path: &'a str,
        module_prefix: &'a str, // note: should include the dot separators, including a trailing dot
        body: &[Stmt],
    ) -> Vec<InterfaceMember> {
        self.current_interfaces = self
            .all_interfaces
            .get_interfaces_to_type_check(module_path);
        self.current_module_prefix = Some(module_prefix);
        self.current_interface_members.clear();
        self.visit_body(body);
        let current_interfaces: Vec<&CompiledInterface> =
            self.current_interfaces.drain(..).collect();
        let module_prefix = self.current_module_prefix.take().unwrap();
        self.current_interface_members
            .drain(..)
            .filter(|member| {
                current_interfaces.iter().any(|interface| {
                    interface
                        .expose
                        .iter()
                        .any(|re| re.is_match(&format!("{}{}", module_prefix, member.name)))
                })
            })
            .collect()
    }
}

impl StatementVisitor<'_> for ModuleInterfaceVisitor<'_> {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign(node) => {
                for target in &node.targets {
                    self.current_interface_members.push(InterfaceMember {
                        name: match target {
                            Expr::Name(name) => name.id.clone(),
                            _ => panic!("Expected Expr::Name"),
                        },
                        node: InterfaceMemberNode::Variable { annotation: None },
                    });
                }
            }
            Stmt::AnnAssign(node) => {
                self.current_interface_members.push(InterfaceMember {
                    name: match node.target.as_ref() {
                        Expr::Name(name) => name.id.clone(),
                        _ => panic!("Expected Expr::Name"),
                    },
                    node: InterfaceMemberNode::Variable {
                        annotation: match node.annotation.as_ref() {
                            Expr::Name(name) => Some(name.id.clone()),
                            Expr::StringLiteral(s) => Some(s.value.to_string()),
                            _ => None,
                        },
                    },
                });
            }
            Stmt::FunctionDef(node) => {
                self.current_interface_members.push(InterfaceMember {
                    name: node.name.id.clone(),
                    node: InterfaceMemberNode::Function {
                        parameters: node
                            .parameters
                            .iter_non_variadic_params()
                            .map(|p| FunctionParameter {
                                name: p.parameter.name.to_string(),
                                annotation: match &p.parameter.annotation {
                                    Some(annotation) => match annotation.as_ref() {
                                        Expr::Name(name) => Some(name.id.clone()),
                                        Expr::StringLiteral(s) => Some(s.value.to_string()),
                                        _ => None,
                                    },
                                    None => None,
                                },
                            })
                            .collect(),
                        return_type: match node.returns.as_ref() {
                            Some(r) => match r.as_ref() {
                                Expr::Name(name) => Some(name.id.clone()),
                                Expr::StringLiteral(s) => Some(s.value.to_string()),
                                _ => None,
                            },
                            None => None,
                        },
                    },
                });
            }
            Stmt::ClassDef(node) => {
                self.current_interface_members.push(InterfaceMember {
                    name: node.name.id.clone(),
                    node: InterfaceMemberNode::Class,
                });
            }
            _ => (),
        }
    }
}

pub fn type_check_all_interface_members(
    source_roots: &[PathBuf],
    module_paths: &[&str],
    interfaces: &CompiledInterfaces,
) -> Result<HashMap<String, TypeCheckResult>, InterfaceError> {
    let mut member_status = HashMap::new();

    // for each module, parse the source files and use the visitor to extract the typed interface members
    let mut visitor = ModuleInterfaceVisitor::new(interfaces);
    for module_path in module_paths {
        let resolved_mod = module_to_file_path(source_roots, module_path, false).unwrap();
        // first get this working for only the module file itself
        let python_source = std::fs::read_to_string(resolved_mod.file_path).unwrap();
        let ast = match parse_python_source(&python_source) {
            Ok(Mod::Module(ast)) => ast,
            _ => panic!("Expected ast::Mod variant"),
        };
        let interface_members = visitor.get_interface_members(module_path, "", &ast.body);

        println!("{:?}", interface_members);
        for member in interface_members.iter() {
            member_status.insert(member.name.clone(), TypeCheckResult::Unknown);
        }
    }

    Ok(member_status)
}

#[cfg(test)]
mod tests {
    
    
    use rstest::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_files(temp_dir: &TempDir, files: &[(&str, &str)]) -> Vec<PathBuf> {
        let source_root = temp_dir.path().to_path_buf();
        for (path, content) in files {
            let full_path = source_root.join(path);
            std::fs::create_dir_all(full_path.parent().unwrap()).unwrap();
            std::fs::write(full_path, content).unwrap();
        }
        vec![source_root]
    }

    #[fixture]
    fn basic_python_module() -> (&'static str, &'static str) {
        (
            "my_module.py",
            r#"
x: int = 1
y = "hello"

def func(a: int, b: str) -> bool:
    pass

class MyClass:
    pass
            "#,
        )
    }
}
