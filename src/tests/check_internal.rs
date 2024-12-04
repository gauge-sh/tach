#[cfg(test)]
pub mod fixtures {
    use std::{collections::HashMap, sync::Arc};

    use crate::core::{
        config::{DependencyConfig, InterfaceConfig, ModuleConfig},
        module::{ModuleNode, ModuleTree},
    };
    use rstest::fixture;

    #[fixture]
    pub fn interface_config() -> Vec<InterfaceConfig> {
        vec![InterfaceConfig {
            from_modules: vec!["domain_one".to_string()],
            expose: vec!["public_fn".to_string()],
        }]
    }

    #[fixture]
    pub fn module_tree() -> ModuleTree {
        ModuleTree {
            root: Arc::new(ModuleNode {
                is_end_of_path: false,
                full_path: String::new(),
                config: None,
                interface_members: vec![],
                children: HashMap::from([
                    (
                        "domain_one".to_string(),
                        Arc::new(ModuleNode {
                            is_end_of_path: true,
                            full_path: "domain_one".to_string(),
                            config: Some(ModuleConfig {
                                path: "domain_one".to_string(),
                                depends_on: vec![
                                    DependencyConfig::from_deprecated_path("domain_one.subdomain"),
                                    DependencyConfig::from_path("domain_three"),
                                ],
                                strict: false,
                                ..Default::default()
                            }),
                            interface_members: vec![],
                            children: HashMap::from([(
                                "subdomain".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "domain_one.subdomain".to_string(),
                                    config: Some(ModuleConfig::new("domain_one.subdomain", false)),
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
                            config: Some(ModuleConfig {
                                path: "domain_two".to_string(),
                                depends_on: vec![DependencyConfig::from_path("domain_one")],
                                strict: false,
                                ..Default::default()
                            }),
                            interface_members: vec![],
                            children: HashMap::from([(
                                "subdomain".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "domain_two.subdomain".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "domain_two".to_string(),
                                        depends_on: vec![DependencyConfig::from_path("domain_one")],
                                        strict: false,
                                        ..Default::default()
                                    }),
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
                            config: Some(ModuleConfig::new("domain_three", false)),
                            interface_members: vec![],
                            children: HashMap::new(),
                        }),
                    ),
                ]),
            }),
        }
    }
}
