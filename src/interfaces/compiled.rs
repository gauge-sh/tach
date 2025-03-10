use crate::config::{InterfaceConfig, InterfaceDataTypes};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct CompiledInterface {
    pub from_modules: Vec<Regex>,
    pub visibility: Option<Vec<String>>,
    pub expose: Vec<Regex>,
    pub data_types: InterfaceDataTypes,
    pub exclusive: bool,
}

impl CompiledInterface {
    pub fn matches_module(&self, module_path: &str) -> bool {
        self.from_modules
            .iter()
            .any(|regex| regex.is_match(module_path))
    }

    pub fn matches_member(&self, member_name: &str) -> bool {
        self.expose.iter().any(|regex| regex.is_match(member_name))
    }

    pub fn is_visible_to(&self, module_path: &str) -> bool {
        self.visibility.as_ref().map_or(true, |visibility| {
            visibility.iter().any(|v| v == module_path)
        })
    }

    pub fn is_exposed_to(&self, member: &str, module_path: &str) -> bool {
        self.matches_member(member) && self.is_visible_to(module_path)
    }

    pub fn should_type_check(&self) -> bool {
        self.data_types != InterfaceDataTypes::All
    }
}

#[derive(Debug, Clone)]
pub struct CompiledInterfaces {
    interfaces: Vec<CompiledInterface>,
}

impl<'a> CompiledInterfaces {
    pub fn build(interfaces: impl IntoIterator<Item = &'a InterfaceConfig>) -> Self {
        let compiled = interfaces
            .into_iter()
            .map(|interface| CompiledInterface {
                data_types: interface.data_types,
                from_modules: interface
                    .from_modules
                    .iter()
                    .map(|pattern| Regex::new(&format!("^{}$", pattern)).unwrap())
                    .collect(),
                expose: interface
                    .expose
                    .iter()
                    .map(|pattern| Regex::new(&format!("^{}$", pattern)).unwrap())
                    .collect(),
                visibility: interface.visibility.clone(),
                exclusive: interface.exclusive,
            })
            .collect();

        Self {
            interfaces: compiled,
        }
    }

    pub fn interfaces_for_module(
        &'a self,
        module_path: &'a str,
    ) -> impl Iterator<Item = &'a CompiledInterface> {
        self.interfaces
            .iter()
            .filter(|interface| interface.matches_module(module_path))
    }

    pub fn should_type_check(&'a self, module_path: &'a str) -> bool {
        self.interfaces_for_module(module_path)
            .any(|interface| interface.should_type_check())
    }

    pub fn get_visible_interfaces(
        &'a self,
        definition_module: &'a str,
        usage_module: &'a str,
    ) -> Vec<&'a CompiledInterface> {
        let mut interfaces = Vec::new();
        for compiled_interface in self
            .interfaces_for_module(definition_module)
            .filter(|interface| interface.is_visible_to(usage_module))
        {
            if compiled_interface.exclusive {
                // If we encounter an exclusive interface, we return it immediately
                return vec![compiled_interface];
            }
            interfaces.push(compiled_interface);
        }
        interfaces
    }

    pub fn get_interfaces_to_type_check(
        &'a self,
        module_path: &'a str,
    ) -> Vec<&'a CompiledInterface> {
        self.interfaces_for_module(module_path)
            .filter(|interface| interface.should_type_check())
            .collect()
    }

    pub fn get_data_types(&self, module_path: &str, member_name: &str) -> InterfaceDataTypes {
        // NOTE: this takes the first matching interface,
        //   however, if multiple interfaces match, we need to establish a precedence order
        self.interfaces_for_module(module_path)
            .find(|interface| interface.matches_member(member_name))
            .map(|interface| interface.data_types)
            .unwrap_or(InterfaceDataTypes::All)
    }
}
