use super::matcher::CompiledInterfaces;
use crate::core::config::ModuleConfig;
use std::collections::HashMap;
use std::path::PathBuf;

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
    ) -> Self {
        // TODO: Implement this
        Self {
            interface_member_status: HashMap::new(),
        }
    }

    pub fn is_serializable(&self, member: &str) -> InterfaceMemberStatus {
        self.interface_member_status
            .get(member)
            .cloned()
            .unwrap_or(InterfaceMemberStatus::Unknown)
    }
}
