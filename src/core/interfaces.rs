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
    pub fn new(interfaces: &[InterfaceConfig]) -> Self {
        let compiled = interfaces
            .iter()
            .map(|interface| CompiledInterface {
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

    pub fn has_interface(&self, import_mod_path: &str) -> bool {
        self.interfaces.iter().any(|interface| {
            interface
                .from_modules
                .iter()
                .any(|re| re.is_match(import_mod_path))
        })
    }

    /// Check if the import member is exposed by any interface.
    pub fn check(&self, import_member: &str, import_mod_path: &str) -> bool {
        let mut found_matching_module = false;

        for interface in &self.interfaces {
            let matches_module = interface
                .from_modules
                .iter()
                .any(|re| re.is_match(import_mod_path));

            if matches_module {
                found_matching_module = true;
                if interface.expose.iter().any(|re| re.is_match(import_member)) {
                    return true;
                }
            }
        }

        !found_matching_module
    }
}
