use std::path::PathBuf;

use super::error::InterfaceError;
use super::matcher::{CompiledInterface, CompiledInterfaces};
use super::serializable::{InterfaceMemberStatus, SerializableChecker};
use crate::core::config::{InterfaceConfig, ModuleConfig};

pub struct InterfaceChecker {
    interfaces: CompiledInterfaces,
    serializable_checker: SerializableChecker,
}

pub enum CheckResult {
    Exposed {
        marked_serializable: bool,
        is_serializable: bool,
    },
    NotExposed,
    NoInterfaces,
    TopLevelModule,
}

impl InterfaceChecker {
    pub fn build(
        interfaces: &[InterfaceConfig],
        modules: &[ModuleConfig],
        source_roots: &[PathBuf],
    ) -> Result<Self, InterfaceError> {
        let compiled = CompiledInterfaces::build(interfaces);
        let serializable_checker = SerializableChecker::build(&compiled, modules, source_roots)?;

        Ok(Self {
            interfaces: compiled,
            serializable_checker,
        })
    }

    pub fn check_member(&self, member: &str, module_path: &str) -> CheckResult {
        if member.is_empty() {
            return CheckResult::TopLevelModule;
        }

        let matching_interfaces: Vec<&CompiledInterface> =
            self.interfaces.matching(module_path).collect();

        if matching_interfaces.is_empty() {
            return CheckResult::NoInterfaces;
        }

        let mut is_exposed = false;
        let mut marked_serializable = false;
        for interface in matching_interfaces {
            if interface.expose.iter().any(|re| re.is_match(member)) {
                is_exposed = true;
                marked_serializable |= interface.serializable;
            }
        }

        if !is_exposed {
            return CheckResult::NotExposed;
        }

        CheckResult::Exposed {
            marked_serializable,
            is_serializable: matches!(
                self.serializable_checker.is_serializable(member),
                InterfaceMemberStatus::Serializable | InterfaceMemberStatus::Unknown
            ),
        }
    }
}
