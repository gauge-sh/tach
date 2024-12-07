use std::path::PathBuf;

use super::compiled::CompiledInterfaces;
use super::data_types::{TypeCheckCache, TypeCheckResult};
use super::error::InterfaceError;
use crate::core::config::{InterfaceConfig, ModuleConfig};

pub struct InterfaceChecker {
    interfaces: CompiledInterfaces,
    type_check_cache: Option<TypeCheckCache>,
}

pub enum CheckResult {
    Exposed { type_check_result: TypeCheckResult },
    NotExposed,
    NoInterfaces,
    TopLevelModule,
}

impl InterfaceChecker {
    pub fn new(interfaces: &[InterfaceConfig]) -> Self {
        let compiled = CompiledInterfaces::build(interfaces);

        Self {
            interfaces: compiled,
            type_check_cache: None,
        }
    }

    pub fn with_type_check_cache(
        mut self,
        modules: &[ModuleConfig],
        source_roots: &[PathBuf],
    ) -> Result<Self, InterfaceError> {
        let type_check_cache = TypeCheckCache::build(&self.interfaces, modules, source_roots)?;
        self.type_check_cache = Some(type_check_cache);
        Ok(self)
    }

    pub fn check_member(&self, member: &str, module_path: &str) -> CheckResult {
        if member.is_empty() {
            return CheckResult::TopLevelModule;
        }

        let matching_interfaces = self.interfaces.get_interfaces(module_path);

        if matching_interfaces.is_empty() {
            return CheckResult::NoInterfaces;
        }

        let mut is_exposed = false;
        for interface in matching_interfaces {
            if interface.expose.iter().any(|re| re.is_match(member)) {
                is_exposed = true;
            }
        }

        if !is_exposed {
            return CheckResult::NotExposed;
        }

        CheckResult::Exposed {
            type_check_result: self
                .type_check_cache
                .as_ref()
                .map(|cache| cache.get_result(member))
                .unwrap_or(TypeCheckResult::Unknown),
        }
    }
}
