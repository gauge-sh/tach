use crate::core::config::{InterfaceConfig, InterfaceDataTypes};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct CompiledInterface {
    pub from_modules: Vec<Regex>,
    pub expose: Vec<Regex>,
    pub data_types: InterfaceDataTypes,
}

impl CompiledInterface {
    pub fn matches(&self, module_path: &str) -> bool {
        self.from_modules
            .iter()
            .any(|regex| regex.is_match(module_path))
    }

    pub fn should_type_check(&self, module_path: &str) -> bool {
        self.data_types != InterfaceDataTypes::All && self.matches(module_path)
    }
}

#[derive(Debug, Clone)]
pub struct CompiledInterfaces {
    interfaces: Vec<CompiledInterface>,
}

impl CompiledInterfaces {
    pub fn build(interfaces: &[InterfaceConfig]) -> Self {
        let compiled = interfaces
            .iter()
            .map(|interface| CompiledInterface {
                data_types: interface.data_types.clone(),
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
            })
            .collect();

        Self {
            interfaces: compiled,
        }
    }

    pub fn should_type_check(&self, module_path: &str) -> bool {
        self.interfaces
            .iter()
            .any(|interface| interface.should_type_check(module_path))
    }

    pub fn get_interfaces(&self, module_path: &str) -> Vec<&CompiledInterface> {
        self.interfaces
            .iter()
            .filter(|interface| interface.matches(module_path))
            .collect()
    }

    pub fn get_interfaces_to_type_check(&self, module_path: &str) -> Vec<&CompiledInterface> {
        self.interfaces
            .iter()
            .filter(|interface| interface.should_type_check(module_path))
            .collect()
    }
}
