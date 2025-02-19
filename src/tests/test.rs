#[cfg(test)]
pub mod fixtures {
    use std::{collections::HashMap, sync::Arc};

    use crate::config::{DependencyConfig, ModuleConfig};
    use crate::modules::{ModuleNode, ModuleTree};
    use rstest::fixture;

    #[fixture]
    pub fn modules() -> Vec<ModuleConfig> {
        vec![
            ModuleConfig::from_path("tach"),
            ModuleConfig::from_path_and_dependencies(
                "tach.__main__",
                Some(vec![DependencyConfig::from_path("tach.start")]),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.cache",
                Some(
                    ["tach", "tach.filesystem"]
                        .map(DependencyConfig::from_path)
                        .into(),
                ),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.check",
                Some(
                    ["tach.errors", "tach.filesystem", "tach.parsing"]
                        .map(DependencyConfig::from_path)
                        .into(),
                ),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.cli",
                Some(
                    [
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
                ),
            ),
            ModuleConfig::from_path("tach.colors"),
            ModuleConfig::from_path("tach.constants"),
            ModuleConfig::from_path_and_dependencies(
                "tach.core",
                Some(vec![DependencyConfig::from_path("tach.constants")]),
            ),
            ModuleConfig::from_path("tach.errors"),
            ModuleConfig::from_path_and_dependencies(
                "tach.filesystem",
                Some(
                    [
                        "tach.colors",
                        "tach.constants",
                        "tach.core",
                        "tach.errors",
                        "tach.hooks",
                    ]
                    .map(DependencyConfig::from_path)
                    .into(),
                ),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.filesystem.git_ops",
                Some(vec![DependencyConfig::from_path("tach.errors")]),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.hooks",
                Some(vec![DependencyConfig::from_path("tach.constants")]),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.interactive",
                Some(
                    ["tach.errors", "tach.filesystem"]
                        .map(DependencyConfig::from_path)
                        .into(),
                ),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.logging",
                Some(
                    ["tach", "tach.cache", "tach.parsing"]
                        .map(DependencyConfig::from_path)
                        .into(),
                ),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.mod",
                Some(
                    [
                        "tach.colors",
                        "tach.constants",
                        "tach.errors",
                        "tach.filesystem",
                        "tach.interactive",
                        "tach.parsing",
                    ]
                    .map(DependencyConfig::from_path)
                    .into(),
                ),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.parsing",
                Some(
                    ["tach.constants", "tach.core", "tach.filesystem"]
                        .map(DependencyConfig::from_path)
                        .into(),
                ),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.report",
                Some(vec![DependencyConfig::from_path("tach.errors")]),
            ),
            ModuleConfig::from_path("tach.show"),
            ModuleConfig::from_path_and_dependencies(
                "tach.start",
                Some(vec![DependencyConfig::from_path("tach.cli")]),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.sync",
                Some(
                    [
                        "tach.check",
                        "tach.errors",
                        "tach.filesystem",
                        "tach.parsing",
                    ]
                    .map(DependencyConfig::from_path)
                    .into(),
                ),
            ),
            ModuleConfig::from_path_and_dependencies(
                "tach.test",
                Some(
                    [
                        "tach.errors",
                        "tach.filesystem",
                        "tach.filesystem.git_ops",
                        "tach.parsing",
                    ]
                    .map(DependencyConfig::from_path)
                    .into(),
                ),
            ),
        ]
    }

    #[fixture]
    pub fn module_tree() -> ModuleTree {
        ModuleTree {
            root: Arc::new(ModuleNode {
                is_end_of_path: true,
                full_path: ".".to_string(),
                config: Some(ModuleConfig::new_root_config()),
                children: HashMap::from([(
                    "tach".to_string(),
                    Arc::new(ModuleNode {
                        is_end_of_path: true,
                        full_path: "tach".to_string(),
                        config: Some(ModuleConfig::from_path("tach")),
                        children: HashMap::from([
                            (
                                "__main__".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.__main__".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.__main__",
                                        Some(vec![DependencyConfig::from_path("tach.start")]),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "cache".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.cache".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.cache",
                                        Some(
                                            ["tach", "tach.filesystem"]
                                                .map(DependencyConfig::from_path)
                                                .into(),
                                        ),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "check".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.check".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.check",
                                        Some(
                                            ["tach.errors", "tach.filesystem", "tach.parsing"]
                                                .map(DependencyConfig::from_path)
                                                .into(),
                                        ),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "cli".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.cli".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.cli",
                                        Some(
                                            [
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
                                        ),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "colors".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.colors".to_string(),
                                    config: Some(ModuleConfig::from_path("tach.colors")),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "constants".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.constants".to_string(),
                                    config: Some(ModuleConfig::from_path("tach.constants")),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "core".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.core".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.core",
                                        Some(vec![DependencyConfig::from_path("tach.constants")]),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "errors".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.errors".to_string(),
                                    config: Some(ModuleConfig::from_path("tach.errors")),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "filesystem".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.filesystem".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.filesystem",
                                        Some(
                                            [
                                                "tach.colors",
                                                "tach.constants",
                                                "tach.core",
                                                "tach.errors",
                                                "tach.hooks",
                                            ]
                                            .map(DependencyConfig::from_path)
                                            .into(),
                                        ),
                                    )),
                                    children: HashMap::from([(
                                        "git_ops".to_string(),
                                        Arc::new(ModuleNode {
                                            is_end_of_path: true,
                                            full_path: "tach.filesystem.git_ops".to_string(),
                                            config: Some(ModuleConfig::from_path_and_dependencies(
                                                "tach.filesystem.git_ops",
                                                Some(vec![DependencyConfig::from_path(
                                                    "tach.errors",
                                                )]),
                                            )),
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
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.hooks",
                                        Some(vec![DependencyConfig::from_path("tach.constants")]),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "interactive".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.interactive".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.interactive",
                                        Some(
                                            ["tach.errors", "tach.filesystem"]
                                                .map(DependencyConfig::from_path)
                                                .into(),
                                        ),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "logging".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.logging".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.logging",
                                        Some(
                                            ["tach", "tach.cache", "tach.parsing"]
                                                .map(DependencyConfig::from_path)
                                                .into(),
                                        ),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "mod".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.mod".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.mod",
                                        Some(
                                            [
                                                "tach.colors",
                                                "tach.constants",
                                                "tach.errors",
                                                "tach.filesystem",
                                                "tach.interactive",
                                                "tach.parsing",
                                            ]
                                            .map(DependencyConfig::from_path)
                                            .into(),
                                        ),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "parsing".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.parsing".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.parsing",
                                        Some(
                                            ["tach.constants", "tach.core", "tach.filesystem"]
                                                .map(DependencyConfig::from_path)
                                                .into(),
                                        ),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "report".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.report".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.report",
                                        Some(vec![DependencyConfig::from_path("tach.errors")]),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "show".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.show".to_string(),
                                    config: Some(ModuleConfig::from_path("tach.show")),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "start".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.start".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.start",
                                        Some(vec![DependencyConfig::from_path("tach.cli")]),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "sync".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.sync".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.sync",
                                        Some(
                                            [
                                                "tach.check",
                                                "tach.errors",
                                                "tach.filesystem",
                                                "tach.parsing",
                                            ]
                                            .map(DependencyConfig::from_path)
                                            .into(),
                                        ),
                                    )),
                                    children: HashMap::new(),
                                }),
                            ),
                            (
                                "test".to_string(),
                                Arc::new(ModuleNode {
                                    is_end_of_path: true,
                                    full_path: "tach.test".to_string(),
                                    config: Some(ModuleConfig::from_path_and_dependencies(
                                        "tach.test",
                                        Some(
                                            [
                                                "tach.errors",
                                                "tach.filesystem",
                                                "tach.filesystem.git_ops",
                                                "tach.parsing",
                                            ]
                                            .map(DependencyConfig::from_path)
                                            .into(),
                                        ),
                                    )),
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
