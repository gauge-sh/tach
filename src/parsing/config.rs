use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use crate::{
    colors::BColors,
    config::root_module::ROOT_MODULE_SENTINEL_TAG,
    config::{InterfaceConfig, InterfaceDataTypes, ProjectConfig},
    filesystem::read_file_content,
    python::parsing::parse_interface_members,
};

use super::error;

pub type Result<T> = std::result::Result<T, error::ParsingError>;

pub fn dump_project_config_to_toml(
    config: &mut ProjectConfig,
) -> std::result::Result<String, toml::ser::Error> {
    config.modules.sort_by(|a, b| {
        if a.path == ROOT_MODULE_SENTINEL_TAG {
            Ordering::Less
        } else if b.path == ROOT_MODULE_SENTINEL_TAG {
            Ordering::Greater
        } else {
            a.path.cmp(&b.path)
        }
    });

    for module in &mut config.modules {
        if let Some(depends_on) = &mut module.depends_on {
            depends_on.sort_by(|a, b| a.path.cmp(&b.path));
        }
    }

    config.exclude.sort();
    config.source_roots.sort();

    toml::to_string(&config)
}

fn migrate_strict_mode_to_interfaces(filepath: &Path, config: &mut ProjectConfig) -> bool {
    if config.modules.iter().any(|m| m.strict) {
        println!(
            "{}WARNING: Strict mode is deprecated. Migrating to interfaces.{}",
            BColors::WARNING,
            BColors::ENDC
        );
    } else {
        // No strict modules, so no need to migrate
        return false;
    }

    let mut interfaces: Vec<InterfaceConfig> = vec![];
    let abs_source_roots: Vec<PathBuf> = config
        .source_roots
        .iter()
        .map(|r| filepath.parent().unwrap().join(r))
        .collect();
    for module in &mut config.modules {
        if module.strict {
            let interface_members =
                parse_interface_members(&abs_source_roots, &module.path).unwrap_or_default();
            interfaces.push(InterfaceConfig {
                expose: interface_members,
                from_modules: vec![module.path.clone()],
                data_types: InterfaceDataTypes::All,
            });
        }
    }
    config.interfaces = interfaces;
    true
}

const DEPRECATED_REGEX_EXCLUDE_PATHS: [&str; 2] = [".*__pycache__", ".*egg-info"];
const REPLACEMENT_GLOB_EXCLUDE_PATHS: [&str; 2] = ["**/*__pycache__", "**/*egg-info"];
const EXPECTED_EXCLUDE_PATHS: [&str; 5] = [
    "tests",
    "docs",
    "**/*__pycache__",
    "**/*egg-info",
    "**/*venv",
];

fn migrate_deprecated_regex_exclude(config: &mut ProjectConfig) -> bool {
    if config.use_regex_matching {
        return false;
    }

    let mut did_migrate = false;
    config.exclude.iter_mut().for_each(|exclude_path| {
        if let Some(index) = DEPRECATED_REGEX_EXCLUDE_PATHS
            .iter()
            .position(|&p| p == exclude_path)
        {
            did_migrate = true;
            *exclude_path = REPLACEMENT_GLOB_EXCLUDE_PATHS[index].to_string();
        }
    });

    if did_migrate {
        println!(
            "{}Migrating default regex exclude paths to glob patterns.{}",
            BColors::WARNING,
            BColors::ENDC
        );

        // If config indicates that the user has added any paths that are not in the expected list,
        // print a warning and suggest that the user update their exclude paths.
        if !config
            .exclude
            .iter()
            .all(|path| EXPECTED_EXCLUDE_PATHS.contains(&path.as_str()))
        {
            println!("\n");
            println!(
                "{}---- WARNING: Your exclude paths may need to be updated. ----{}",
                BColors::WARNING,
                BColors::ENDC
            );
            println!(
                "{}Please verify that your exclude patterns are valid glob patterns (not regex).{}",
                BColors::WARNING,
                BColors::ENDC
            );
            println!(
                "{}The default configuration has changed from regex to glob matching.{}",
                BColors::WARNING,
                BColors::ENDC
            );
            println!("\n");
        }
    }

    did_migrate
}

pub fn parse_project_config<P: AsRef<Path>>(filepath: P) -> Result<(ProjectConfig, bool)> {
    let content = read_file_content(filepath.as_ref())?;
    let mut config: ProjectConfig = toml::from_str(&content)?;
    let did_migrate = migrate_strict_mode_to_interfaces(filepath.as_ref(), &mut config)
        || migrate_deprecated_regex_exclude(&mut config);
    Ok((config, did_migrate))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        config::project::DEFAULT_EXCLUDE_PATHS,
        config::root_module::ROOT_MODULE_SENTINEL_TAG,
        config::{DependencyConfig, ModuleConfig},
        tests::fixtures::example_dir,
    };
    use rstest::rstest;
    #[rstest]
    fn test_parse_valid_project_config(example_dir: PathBuf) {
        // TODO: remove tach.toml when joining
        let result = parse_project_config(example_dir.join("valid/tach.toml"));
        assert!(result.is_ok());
        let (config, _) = result.unwrap();
        assert_eq!(
            config,
            ProjectConfig {
                modules: vec![
                    ModuleConfig {
                        path: "domain_one".to_string(),
                        depends_on: Some(vec![DependencyConfig::from_deprecated_path(
                            "domain_two"
                        )]),
                        strict: false,
                        ..Default::default()
                    },
                    ModuleConfig {
                        path: "domain_three".to_string(),
                        depends_on: Some(vec![]),
                        strict: false,
                        ..Default::default()
                    },
                    ModuleConfig {
                        path: "domain_two".to_string(),
                        depends_on: Some(vec![DependencyConfig::from_path("domain_three")]),
                        strict: false,
                        ..Default::default()
                    },
                    ModuleConfig {
                        path: ROOT_MODULE_SENTINEL_TAG.to_string(),
                        depends_on: Some(vec![DependencyConfig::from_path("domain_one")]),
                        strict: false,
                        ..Default::default()
                    }
                ],
                exclude: DEFAULT_EXCLUDE_PATHS
                    .into_iter()
                    .chain(["domain_four.py"].into_iter())
                    .map(String::from)
                    .collect(),
                exact: true,
                forbid_circular_dependencies: true,
                ..Default::default()
            }
        );
    }
}
