#[cfg(test)]
pub mod fixtures {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use crate::config::{DependencyConfig, InterfaceConfig, InterfaceDataTypes, ModuleConfig};
    use crate::modules::{ModuleNode, ModuleTree};
    use rstest::fixture;

    #[fixture]
    pub fn interface_config() -> Vec<InterfaceConfig> {
        vec![InterfaceConfig {
            expose: vec!["public_fn".to_string()],
            from_modules: vec!["domain_one".to_string()],
            data_types: InterfaceDataTypes::All,
        }]
    }

    #[fixture]
    pub fn layers() -> Vec<String> {
        vec![
            "top".to_string(),
            "middle".to_string(),
            "bottom".to_string(),
        ]
    }

    #[fixture]
    pub fn module_tree() -> ModuleTree {
        ModuleTree {
            root: Arc::new(ModuleNode {
                is_end_of_path: false,
                full_path: String::new(),
                config: None,
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
                                layer: Some("top".to_string()),
                                ..Default::default()
                            }),
                            children: HashMap::from([(
                                "subdomain".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "domain_one.subdomain".to_string(),
                                    config: Some(ModuleConfig::new_with_layer(
                                        "domain_one.subdomain",
                                        "middle",
                                    )),
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
                                layer: Some("top".to_string()),
                                ..Default::default()
                            }),
                            children: HashMap::from([(
                                "subdomain".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "domain_two.subdomain".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "domain_two".to_string(),
                                        depends_on: vec![DependencyConfig::from_path("domain_one")],
                                        strict: false,
                                        layer: Some("top".to_string()),
                                        ..Default::default()
                                    }),
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
                            config: Some(ModuleConfig::new_with_layer("domain_three", "bottom")),
                            children: HashMap::new(),
                        }),
                    ),
                ]),
            }),
        }
    }

    #[fixture]
    pub fn module_config() -> Vec<ModuleConfig> {
        vec![
            ModuleConfig {
                path: "domain_one".to_string(),
                depends_on: vec![
                    DependencyConfig::from_deprecated_path("domain_one.subdomain"),
                    DependencyConfig::from_path("domain_three"),
                ],
                strict: false,
                layer: Some("top".to_string()),
                ..Default::default()
            },
            ModuleConfig::new_with_layer("domain_one.subdomain", "middle"),
            ModuleConfig {
                path: "domain_two".to_string(),
                depends_on: vec![DependencyConfig::from_path("domain_one")],
                strict: false,
                layer: Some("top".to_string()),
                ..Default::default()
            },
            ModuleConfig::new_with_layer("domain_three", "bottom"),
        ]
    }

    #[fixture]
    pub fn source_roots() -> Vec<PathBuf> {
        vec![PathBuf::from("src")]
    }
}
