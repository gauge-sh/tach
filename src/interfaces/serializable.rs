use super::error::InterfaceError;
use super::matcher::{CompiledInterface, CompiledInterfaceIterExt, CompiledInterfaces};
use crate::core::config::ModuleConfig;
use crate::filesystem::module_to_file_path;
use crate::python::parsing::parse_python_source;
use std::collections::HashMap;
use std::path::PathBuf;

use ruff_python_ast::{statement_visitor::StatementVisitor, Expr, Mod, Stmt};

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
    all_interfaces: Vec<&'a CompiledInterface>,
    // current matching interfaces to check against (based on the outer module path)
    current_interfaces: Vec<&'a CompiledInterface>,
    // module prefix of the current AST being visited (based on the outer module path)
    current_module_prefix: Option<&'a str>,
    // all interface members found in the current module
    current_interface_members: Vec<InterfaceMember>,
}

impl<'a> ModuleInterfaceVisitor<'a> {
    fn new(interfaces: Vec<&'a CompiledInterface>) -> Self {
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
            .clone()
            .into_iter()
            .matching(module_path)
            .collect();
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

pub fn parse_typed_interface_members(
    source_roots: &[PathBuf],
    module_paths: &[&str],
    interfaces: &[&CompiledInterface],
) -> Result<HashMap<String, InterfaceMemberStatus>, InterfaceError> {
    let mut member_status = HashMap::new();

    // for each module, parse the source files and use the visitor to extract the typed interface members
    let mut visitor = ModuleInterfaceVisitor::new(interfaces.to_vec());
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
            member_status.insert(member.name.clone(), InterfaceMemberStatus::Serializable);
        }
    }

    Ok(member_status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::InterfaceConfig;
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

    #[rstest]
    fn test_basic_serializable_members(
        basic_python_module: (&str, &str),
    ) -> Result<(), InterfaceError> {
        let temp_dir = TempDir::new().unwrap();
        let source_roots = setup_test_files(&temp_dir, &[basic_python_module]);

        // Create a test interface that marks the module as serializable
        let interfaces = CompiledInterfaces::build(&[InterfaceConfig {
            from_modules: vec!["my_module".to_string()],
            expose: vec![".*".to_string()], // Match everything
            serializable: true,
        }]);

        let modules = vec![ModuleConfig::new("my_module", false)];

        let checker = SerializableChecker::build(&interfaces, &modules, &source_roots)?;

        // Check that members are marked as serializable
        assert!(matches!(
            checker.is_serializable("x"),
            InterfaceMemberStatus::Serializable
        ));
        assert!(matches!(
            checker.is_serializable("y"),
            InterfaceMemberStatus::Serializable
        ));
        assert!(matches!(
            checker.is_serializable("func"),
            InterfaceMemberStatus::Serializable
        ));
        assert!(matches!(
            checker.is_serializable("MyClass"),
            InterfaceMemberStatus::Serializable
        ));
        assert!(matches!(
            checker.is_serializable("nonexistent"),
            InterfaceMemberStatus::Unknown
        ));

        Ok(())
    }

    #[rstest]
    fn test_non_serializable_interface(
        basic_python_module: (&str, &str),
    ) -> Result<(), InterfaceError> {
        let temp_dir = TempDir::new().unwrap();
        let source_roots = setup_test_files(&temp_dir, &[basic_python_module]);

        // Create a test interface that is NOT marked as serializable
        let interfaces = CompiledInterfaces::build(&[InterfaceConfig {
            from_modules: vec!["my_module".to_string()],
            expose: vec![".*".to_string()],
            serializable: false,
        }]);

        let modules = vec![ModuleConfig::new("my_module", false)];

        let checker = SerializableChecker::build(&interfaces, &modules, &source_roots)?;

        // Check that members are marked as Unknown (not serializable)
        assert!(matches!(
            checker.is_serializable("x"),
            InterfaceMemberStatus::Unknown
        ));

        Ok(())
    }

    #[rstest]
    fn test_multiple_modules() -> Result<(), InterfaceError> {
        let temp_dir = TempDir::new().unwrap();

        let files = vec![
            (
                "module1.py",
                r#"
x: int = 1
                "#,
            ),
            (
                "module2.py",
                r#"
y: str = "hello"
                "#,
            ),
        ];
        let source_roots = setup_test_files(&temp_dir, &files);

        // Create interfaces with different serialization settings
        let interfaces = CompiledInterfaces::build(&[
            InterfaceConfig {
                from_modules: vec!["module1".to_string()],
                expose: vec![".*".to_string()],
                serializable: true,
            },
            InterfaceConfig {
                from_modules: vec!["module2".to_string()],
                expose: vec![".*".to_string()],
                serializable: false,
            },
        ]);

        let modules = vec![
            ModuleConfig::new("module1", false),
            ModuleConfig::new("module2", false),
        ];

        let checker = SerializableChecker::build(&interfaces, &modules, &source_roots)?;

        // Check that members are marked correctly
        assert!(matches!(
            checker.is_serializable("x"),
            InterfaceMemberStatus::Serializable
        ));
        assert!(matches!(
            checker.is_serializable("y"),
            InterfaceMemberStatus::Unknown
        ));

        Ok(())
    }
}
