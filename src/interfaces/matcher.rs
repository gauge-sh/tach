use crate::core::config::InterfaceConfig;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct CompiledInterface {
    pub from_modules: Vec<Regex>,
    pub expose: Vec<Regex>,
    pub serializable: bool,
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
                serializable: interface.serializable,
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

    pub fn matching_interfaces(&self, module_path: &str) -> Vec<&CompiledInterface> {
        self.interfaces
            .iter()
            .filter(|interface| {
                interface
                    .from_modules
                    .iter()
                    .any(|re| re.is_match(module_path))
            })
            .collect()
    }
}
