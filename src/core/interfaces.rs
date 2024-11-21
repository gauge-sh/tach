use crate::core::config::InterfaceConfig;
use regex::Regex;

pub struct InterfaceChecker {
    interfaces: Vec<CompiledInterface>,
}

struct CompiledInterface {
    from_modules: Vec<Regex>,
    expose: Vec<Regex>,
}

impl InterfaceChecker {
    pub fn new(interfaces: Vec<InterfaceConfig>) -> Self {
        let compiled = interfaces
            .into_iter()
            .map(|interface| CompiledInterface {
                from_modules: interface
                    .from_modules
                    .into_iter()
                    .map(|pattern| Regex::new(&pattern).unwrap())
                    .collect(),
                expose: interface
                    .expose
                    .into_iter()
                    .map(|pattern| Regex::new(&pattern).unwrap())
                    .collect(),
            })
            .collect();

        Self {
            interfaces: compiled,
        }
    }

    pub fn check(&self, import_member: &str, import_mod_path: &str) -> bool {
        for interface in &self.interfaces {
            if interface
                .from_modules
                .iter()
                .any(|re| re.is_match(import_mod_path)) && interface.expose.iter().any(|re| re.is_match(import_member)) {
                return true;
            }
        }
        false
    }
}
