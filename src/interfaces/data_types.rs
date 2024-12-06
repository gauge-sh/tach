use super::compiled::{CompiledInterface, CompiledInterfaces};
use super::error::InterfaceError;
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
    Unknown, // not in any interface with data type constraints, or could not determine data type
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

pub fn is_primitive_type(annotation: &str) -> bool {
    matches!(
        annotation,
        "int" | "str" | "bool" | "float" | "list" | "dict" | "tuple"
    )
}

trait DataTypeChecker {
    fn type_check_function(
        &self,
        parameters: &[FunctionParameter],
        return_type: &Option<String>,
    ) -> TypeCheckResult;
    fn type_check_variable(&self, annotation: &Option<String>) -> TypeCheckResult;
    fn type_check_class(&self) -> TypeCheckResult;
}

impl DataTypeChecker for InterfaceDataTypes {
    fn type_check_function(
        &self,
        parameters: &[FunctionParameter],
        return_type: &Option<String>,
    ) -> TypeCheckResult {
        match self {
            InterfaceDataTypes::All => TypeCheckResult::MatchedInterface {
                expected: self.clone(),
            },
            InterfaceDataTypes::Primitive => {
                if parameters
                    .iter()
                    .all(|p| is_primitive_type(p.annotation.as_deref().unwrap_or("")))
                    && return_type.as_ref().map_or(false, |t| is_primitive_type(t))
                {
                    TypeCheckResult::MatchedInterface {
                        expected: self.clone(),
                    }
                } else {
                    TypeCheckResult::DidNotMatchInterface {
                        expected: self.clone(),
                    }
                }
            }
        }
    }

    fn type_check_variable(&self, annotation: &Option<String>) -> TypeCheckResult {
        match self {
            InterfaceDataTypes::All => TypeCheckResult::MatchedInterface {
                expected: self.clone(),
            },
            InterfaceDataTypes::Primitive => {
                if is_primitive_type(annotation.as_deref().unwrap_or("")) {
                    TypeCheckResult::MatchedInterface {
                        expected: self.clone(),
                    }
                } else {
                    TypeCheckResult::DidNotMatchInterface {
                        expected: self.clone(),
                    }
                }
            }
        }
    }

    fn type_check_class(&self) -> TypeCheckResult {
        TypeCheckResult::Unknown
    }
}

pub fn type_check_interface_member(
    interface_member: &InterfaceMember,
    data_types: &InterfaceDataTypes,
) -> TypeCheckResult {
    // NOTE: will need more parameters/state to do this for most cases
    match &interface_member.node {
        InterfaceMemberNode::Variable { annotation } => data_types.type_check_variable(annotation),
        InterfaceMemberNode::Function {
            parameters,
            return_type,
        } => data_types.type_check_function(parameters, return_type),
        InterfaceMemberNode::Class => data_types.type_check_class(),
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

        member_status.extend(interface_members.iter().map(|member| {
            (
                member.name.clone(),
                type_check_interface_member(
                    member,
                    interfaces.get_data_types(module_path, &member.name),
                ),
            )
        }));
    }

    Ok(member_status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::InterfaceConfig;
    use rstest::*;
    use std::fs;
    use tempfile::TempDir;

    #[fixture]
    fn temp_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[fixture]
    fn basic_interface() -> InterfaceConfig {
        InterfaceConfig {
            expose: vec![".*".to_string()],
            from_modules: vec!["my_module".to_string()],
            data_types: InterfaceDataTypes::Primitive,
        }
    }

    fn setup_test_files(temp_dir: &TempDir, source_files: &[(&str, &str)]) -> Vec<PathBuf> {
        // Create source files in temp directory
        for (file_name, content) in source_files {
            let file_path = temp_dir.path().join(file_name);
            fs::write(file_path, content.trim()).unwrap();
        }

        // Return temp dir path as the only source root
        vec![temp_dir.path().to_path_buf()]
    }

    #[rstest]
    #[case("int", true)]
    #[case("str", true)]
    #[case("bool", true)]
    #[case("float", true)]
    #[case("list", true)]
    #[case("dict", true)]
    #[case("tuple", true)]
    #[case("CustomType", false)]
    #[case("MyClass", false)]
    fn test_is_primitive_type(#[case] type_name: &str, #[case] expected: bool) {
        assert_eq!(is_primitive_type(type_name), expected);
    }

    #[rstest]
    fn test_type_check_primitive_function() {
        let data_types = InterfaceDataTypes::Primitive;

        // Test primitive function
        let primitive_func = vec![
            FunctionParameter {
                name: "a".to_string(),
                annotation: Some("int".to_string()),
            },
            FunctionParameter {
                name: "b".to_string(),
                annotation: Some("str".to_string()),
            },
        ];
        let return_type = Some("bool".to_string());

        match data_types.type_check_function(&primitive_func, &return_type) {
            TypeCheckResult::MatchedInterface { expected } => {
                assert_eq!(expected, InterfaceDataTypes::Primitive)
            }
            _ => panic!("Expected MatchedInterface"),
        }

        // Test non-primitive function
        let non_primitive_func = vec![FunctionParameter {
            name: "a".to_string(),
            annotation: Some("CustomType".to_string()),
        }];
        let return_type = Some("bool".to_string());

        match data_types.type_check_function(&non_primitive_func, &return_type) {
            TypeCheckResult::DidNotMatchInterface { expected } => {
                assert_eq!(expected, InterfaceDataTypes::Primitive)
            }
            _ => panic!("Expected DidNotMatchInterface"),
        }
    }

    #[rstest]
    fn test_type_check_cache_build(temp_dir: TempDir, basic_interface: InterfaceConfig) {
        let source_files = [(
            "my_module.py",
            r#"
x: int = 1
y: str = "hello"
z: CustomType = custom()

def func(a: int, b: str) -> bool:
    pass

def custom_func(a: CustomType) -> CustomType:
    pass
            "#,
        )];

        let source_roots = setup_test_files(&temp_dir, &source_files);
        let interfaces = CompiledInterfaces::build(&[basic_interface]);
        let modules = vec![ModuleConfig::new("my_module", false)];

        let cache = TypeCheckCache::build(&interfaces, &modules, &source_roots).unwrap();

        // Test primitive variable
        match cache.get_result("x") {
            TypeCheckResult::MatchedInterface { expected } => {
                assert_eq!(expected, InterfaceDataTypes::Primitive)
            }
            _ => panic!("Expected MatchedInterface for x"),
        }

        // Test non-primitive variable
        match cache.get_result("z") {
            TypeCheckResult::DidNotMatchInterface { expected } => {
                assert_eq!(expected, InterfaceDataTypes::Primitive)
            }
            _ => panic!("Expected DidNotMatchInterface for z"),
        }

        // Test primitive function
        match cache.get_result("func") {
            TypeCheckResult::MatchedInterface { expected } => {
                assert_eq!(expected, InterfaceDataTypes::Primitive)
            }
            _ => panic!("Expected MatchedInterface for func"),
        }

        // Test non-primitive function
        match cache.get_result("custom_func") {
            TypeCheckResult::DidNotMatchInterface { expected } => {
                assert_eq!(expected, InterfaceDataTypes::Primitive)
            }
            _ => panic!("Expected DidNotMatchInterface for custom_func"),
        }
    }
}
