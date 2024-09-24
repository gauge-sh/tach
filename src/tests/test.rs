#[cfg(test)]
pub mod fixtures {
    use std::{collections::HashMap, sync::Arc};

    use crate::core::{
        config::{DependencyConfig, ModuleConfig},
        module::{ModuleNode, ModuleTree},
    };
    use rstest::fixture;

    #[fixture]
    pub fn modules() -> Vec<ModuleConfig> {
        vec![
            ModuleConfig::new("tach", true),
            ModuleConfig {
                path: "tach.__main__".to_string(),
                depends_on: vec![DependencyConfig::from_path("tach.start")],
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.cache".to_string(),
                depends_on: ["tach", "tach.filesystem"]
                    .map(DependencyConfig::from_path)
                    .into(),
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.check".to_string(),
                depends_on: ["tach.errors", "tach.filesystem", "tach.parsing"]
                    .map(DependencyConfig::from_path)
                    .into(),
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.cli".to_string(),
                depends_on: [
                    "tach",
                    "tach.cache",
                    "tach.check",
                    "tach.colors",
                    "tach.constants",
                    "tach.core",
                    "tach.errors",
                    "tach.filesystem",
                    "tach.logging",
                    "tach.mod",
                    "tach.parsing",
                    "tach.report",
                    "tach.show",
                    "tach.sync",
                    "tach.test",
                ]
                .map(DependencyConfig::from_path)
                .into(),
                strict: true,
                ..Default::default()
            },
            ModuleConfig::new("tach.colors", true),
            ModuleConfig::new("tach.constants", true),
            ModuleConfig {
                path: "tach.core".to_string(),
                depends_on: vec![DependencyConfig::from_path("tach.constants")],
                strict: true,
                ..Default::default()
            },
            ModuleConfig::new("tach.errors", true),
            ModuleConfig {
                path: "tach.filesystem".to_string(),
                depends_on: [
                    "tach.colors",
                    "tach.constants",
                    "tach.core",
                    "tach.errors",
                    "tach.hooks",
                ]
                .map(DependencyConfig::from_path)
                .into(),
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.filesystem.git_ops".to_string(),
                depends_on: vec![DependencyConfig::from_path("tach.errors")],
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.hooks".to_string(),
                depends_on: vec![DependencyConfig::from_path("tach.constants")],
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.interactive".to_string(),
                depends_on: ["tach.errors", "tach.filesystem"]
                    .map(DependencyConfig::from_path)
                    .into(),
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.logging".to_string(),
                depends_on: ["tach", "tach.cache", "tach.parsing"]
                    .map(DependencyConfig::from_path)
                    .into(),
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.mod".to_string(),
                depends_on: [
                    "tach.colors",
                    "tach.constants",
                    "tach.errors",
                    "tach.filesystem",
                    "tach.interactive",
                    "tach.parsing",
                ]
                .map(DependencyConfig::from_path)
                .into(),
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.parsing".to_string(),
                depends_on: ["tach.constants", "tach.core", "tach.filesystem"]
                    .map(DependencyConfig::from_path)
                    .into(),
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.report".to_string(),
                depends_on: vec![DependencyConfig::from_path("tach.errors")],
                strict: true,
                ..Default::default()
            },
            ModuleConfig::new("tach.show", true),
            ModuleConfig {
                path: "tach.start".to_string(),
                depends_on: vec![DependencyConfig::from_path("tach.cli")],
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.sync".to_string(),
                depends_on: [
                    "tach.check",
                    "tach.errors",
                    "tach.filesystem",
                    "tach.parsing",
                ]
                .map(DependencyConfig::from_path)
                .into(),
                strict: true,
                ..Default::default()
            },
            ModuleConfig {
                path: "tach.test".to_string(),
                depends_on: [
                    "tach.errors",
                    "tach.filesystem",
                    "tach.filesystem.git_ops",
                    "tach.parsing",
                ]
                .map(DependencyConfig::from_path)
                .into(),
                strict: false,
                ..Default::default()
            },
        ]
    }

    #[fixture]
    pub fn module_tree() -> ModuleTree {
        ModuleTree {
            root: Arc::new(ModuleNode {
                is_end_of_path: true,
                full_path: ".".to_string(),
                config: Some(ModuleConfig::new_root_config()),
                interface_members: vec![],
                children: HashMap::from([(
                    "tach".to_string(),
                    Arc::new(ModuleNode {
                        is_end_of_path: true,
                        full_path: "tach".to_string(),
                        config: Some(ModuleConfig::new("tach", true)),
                        interface_members: vec!["__version__".to_string()],
                        children: HashMap::from([
                            (
                                "__main__".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.__main__".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.__main__".to_string(),
                                        depends_on: vec![DependencyConfig::from_path("tach.start")],
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: vec![],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "cache".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.cache".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.cache".to_string(),
                                        depends_on: ["tach", "tach.filesystem"]
                                            .map(DependencyConfig::from_path)
                                            .into(),
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: vec![
                                        "get_uid".to_string(),
                                        "update_latest_version".to_string(),
                                        "get_latest_version".to_string(),
                                    ],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "check".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.check".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.check".to_string(),
                                        depends_on: [
                                            "tach.errors",
                                            "tach.filesystem",
                                            "tach.parsing",
                                        ]
                                        .map(DependencyConfig::from_path)
                                        .into(),
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: vec![
                                        "BoundaryError".to_string(),
                                        "check".to_string(),
                                    ],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "cli".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.cli".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.cli".to_string(),
                                        depends_on: [
                                            "tach",
                                            "tach.cache",
                                            "tach.check",
                                            "tach.colors",
                                            "tach.constants",
                                            "tach.core",
                                            "tach.errors",
                                            "tach.filesystem",
                                            "tach.logging",
                                            "tach.mod",
                                            "tach.parsing",
                                            "tach.report",
                                            "tach.show",
                                            "tach.sync",
                                            "tach.test",
                                        ]
                                        .map(DependencyConfig::from_path)
                                        .into(),
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: vec!["main".to_string()],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "colors".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.colors".to_string(),
                                    config: Some(ModuleConfig::new("tach.colors", true)),
                                    interface_members: vec!["BCOLORS".to_string()],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "constants".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.constants".to_string(),
                                    config: Some(ModuleConfig::new("tach.constants", true)),
                                    interface_members: [
                                        "PACKAGE_NAME",
                                        "TOOL_NAME",
                                        "CONFIG_FILE_NAME",
                                        "PACKAGE_FILE_NAME",
                                        "ROOT_MODULE_SENTINEL_TAG",
                                        "DEFAULT_EXCLUDE_PATHS",
                                    ]
                                    .map(str::to_string)
                                    .into(),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "core".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.core".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.core".to_string(),
                                        depends_on: vec![DependencyConfig::from_path(
                                            "tach.constants",
                                        )],
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: [
                                        "ProjectConfig",
                                        "ModuleConfig",
                                        "ModuleNode",
                                        "ModuleTree",
                                        "UnusedDependencies",
                                    ]
                                    .map(str::to_string)
                                    .into(),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "errors".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.errors".to_string(),
                                    config: Some(ModuleConfig::new("tach.errors", true)),
                                    interface_members: [
                                        "TachError",
                                        "TachParseError",
                                        "TachSetupError",
                                    ]
                                    .map(str::to_string)
                                    .into(),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "filesystem".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.filesystem".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.filesystem".to_string(),
                                        depends_on: [
                                            "tach.colors",
                                            "tach.constants",
                                            "tach.core",
                                            "tach.errors",
                                            "tach.hooks",
                                        ]
                                        .map(DependencyConfig::from_path)
                                        .into(),
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: [
                                        "get_cwd",
                                        "chdir",
                                        "canonical",
                                        "read_file",
                                        "write_file",
                                        "delete_file",
                                        "parse_ast",
                                        "walk",
                                        "walk_pyfiles",
                                        "file_to_module_path",
                                        "module_to_file_path_no_members",
                                        "module_to_pyfile_or_dir_path",
                                        "get_project_config_path",
                                        "find_project_config_root",
                                        "install_pre_commit",
                                        "validate_project_modules",
                                        "ProjectModuleValidationResult",
                                    ]
                                    .map(str::to_string)
                                    .into(),
                                    children: HashMap::from([(
                                        "git_ops".to_string(),
                                        Arc::new(ModuleNode {
                                            is_end_of_path: true,
                                            full_path: "tach.filesystem.git_ops".to_string(),
                                            config: Some(ModuleConfig {
                                                path: "tach.filesystem.git_ops".to_string(),
                                                depends_on: vec![DependencyConfig::from_path(
                                                    "tach.errors",
                                                )],
                                                strict: true,
                                                ..Default::default()
                                            }),
                                            interface_members: vec!["get_changed_files".to_string()],
                                            children: HashMap::new(),
                                        }),
                                    )]),
                                }),
                            ),
                            (
                                "hooks".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.hooks".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.hooks".to_string(),
                                        depends_on: vec![DependencyConfig::from_path(
                                            "tach.constants",
                                        )],
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: vec![
                                        "build_pre_commit_hook_content".to_string()
                                    ],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "interactive".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.interactive".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.interactive".to_string(),
                                        depends_on: ["tach.errors", "tach.filesystem"]
                                            .map(DependencyConfig::from_path)
                                            .into(),
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: [
                                        "get_selected_modules_interactive",
                                        "InteractiveModuleConfiguration",
                                    ]
                                    .map(str::to_string)
                                    .into(),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "logging".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.logging".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.logging".to_string(),
                                        depends_on: ["tach", "tach.cache", "tach.parsing"]
                                            .map(DependencyConfig::from_path)
                                            .into(),
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: [
                                        "logger".to_string(),
                                        "LogDataModel".to_string(),
                                    ]
                                    .into(),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "mod".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.mod".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.mod".to_string(),
                                        depends_on: [
                                            "tach.colors",
                                            "tach.constants",
                                            "tach.errors",
                                            "tach.filesystem",
                                            "tach.interactive",
                                            "tach.parsing",
                                        ]
                                        .map(DependencyConfig::from_path)
                                        .into(),
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: vec!["mod_edit_interactive".to_string()],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "parsing".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.parsing".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.parsing".to_string(),
                                        depends_on: [
                                            "tach.constants",
                                            "tach.core",
                                            "tach.filesystem",
                                        ]
                                        .map(DependencyConfig::from_path)
                                        .into(),
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: [
                                        "parse_project_config",
                                        "dump_project_config_to_yaml",
                                        "parse_interface_members",
                                        "build_module_tree",
                                    ]
                                    .map(str::to_string)
                                    .into(),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "report".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.report".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.report".to_string(),
                                        depends_on: vec![DependencyConfig::from_path(
                                            "tach.errors",
                                        )],
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: vec!["report".to_string()],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "show".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.show".to_string(),
                                    config: Some(ModuleConfig::new("tach.show", true)),
                                    interface_members: vec!["generate_show_url".to_string()],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "start".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.start".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.start".to_string(),
                                        depends_on: vec![DependencyConfig::from_path("tach.cli")],
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: vec!["start".to_string()],
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "sync".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.sync".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.sync".to_string(),
                                        depends_on: [
                                            "tach.check",
                                            "tach.errors",
                                            "tach.filesystem",
                                            "tach.parsing",
                                        ]
                                        .map(DependencyConfig::from_path)
                                        .into(),
                                        strict: true,
                                        ..Default::default()
                                    }),
                                    interface_members: [
                                        "sync_project",
                                        "prune_dependency_constraints",
                                    ]
                                    .map(str::to_string)
                                    .into(),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "test".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.test".to_string(),
                                    config: Some(ModuleConfig {
                                        path: "tach.test".to_string(),
                                        depends_on: [
                                            "tach.errors",
                                            "tach.filesystem",
                                            "tach.filesystem.git_ops",
                                            "tach.parsing",
                                        ]
                                        .map(DependencyConfig::from_path)
                                        .into(),
                                        strict: false,
                                        ..Default::default()
                                    }),
                                    interface_members: vec![],
                                    children: HashMap::new(),
                                }),
                            ),
                        ]),
                    }),
                )]),
            }),
        }
    }
}
