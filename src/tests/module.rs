#[cfg(test)]
pub mod fixtures {
    use std::{collections::HashMap, sync::Arc};

    use crate::core::{
        config::ModuleConfig,
        module::{ModuleNode, ModuleTree},
    };
    use rstest::fixture;
    #[fixture]
    pub fn module_tree() -> ModuleTree {
        ModuleTree {
            root: Arc::new(ModuleNode {
                is_end_of_path: true,
                full_path: ".".to_string(),
                config: Some(ModuleConfig::new_root_config()),
                interface_members: vec![],
                children: HashMap::from([
                    (
                        "domain_one".to_string(),
                        Arc::new(ModuleNode {
                            is_end_of_path: true,
                            full_path: "domain_one".to_string(),
                            config: Some(ModuleConfig::new("test", false)),
                            interface_members: vec!["public_fn".to_string()],
                            children: HashMap::from([(
                                "subdomain".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "domain_one.subdomain".to_string(),
                                    config: Some(ModuleConfig::new("test", false)),
                                    interface_members: vec![],
                                    children: HashMap::new(),
                                }),
                            )]),
                        }),
                    ),
                    (
                        "domain_two".to_string(),
                        Arc::new(ModuleNode {
                            is_end_of_path: true,
                            full_path: "domain_two".to_string(),
                            config: Some(ModuleConfig::new("test", false)),
                            interface_members: vec![],
                            children: HashMap::from([(
                                "subdomain".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "domain_two.subdomain".to_string(),
                                    config: Some(ModuleConfig::new("test", false)),
                                    interface_members: vec![],
                                    children: HashMap::new(),
                                }),
                            )]),
                        }),
                    ),
                    (
                        "domain_three".to_string(),
                        Arc::new(ModuleNode {
                            is_end_of_path: true,
                            full_path: "domain_three".to_string(),
                            config: Some(ModuleConfig::new("test", false)),
                            interface_members: vec![],
                            children: HashMap::new(),
                        }),
                    ),
                ]),
            }),
        }
    }
}
