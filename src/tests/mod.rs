#[cfg(test)]
pub mod fixtures {
    use std::{collections::HashMap, sync::Arc};

    use crate::core::{
        config::{DependencyConfig, ModuleConfig},
        module::{ModuleNode, ModuleTree},
    };
    use rstest::fixture;

    #[fixture]
    pub fn example_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/tests/example")
    }

    #[fixture]
    pub fn tests_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/tests")
    }

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

    #[fixture]
    pub fn module_tree_check_int() -> ModuleTree {
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
                                    DependencyConfig::from_undeprecated_path("domain_three"),
                                ],
                                strict: true,
                            }),
                            interface_members: vec!["public_fn".to_string()],
                            children: HashMap::from([(
                                "subdomain".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "domain_one.subdomain".to_string(),
                                    config: Some(ModuleConfig::new("domain_one.subdomain", true)),
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
                                depends_on: vec![DependencyConfig::from_undeprecated_path(
                                    "domain_one",
                                )],
                                strict: false,
                            }),
                            interface_members: vec![],
                            children: HashMap::from([(
                                "subdomain".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "domain_two.subdomain".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "domain_two".to_string(),
                                        depends_on: vec![DependencyConfig::from_undeprecated_path(
                                            "domain_one",
                                        )],
                                        strict: false,
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
