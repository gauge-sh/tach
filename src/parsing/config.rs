use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use crate::{
    colors::BColors,
    core::config::{InterfaceConfig, InterfaceDataTypes, ProjectConfig, ROOT_MODULE_SENTINEL_TAG},
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
        module.depends_on.sort_by(|a, b| a.path.cmp(&b.path));
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

pub fn parse_project_config<P: AsRef<Path>>(filepath: P) -> Result<(ProjectConfig, bool)> {
    let content = read_file_content(filepath.as_ref())?;
    let mut config: ProjectConfig = toml::from_str(&content)?;
    let did_migrate = migrate_strict_mode_to_interfaces(filepath.as_ref(), &mut config);
    Ok((config, did_migrate))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        core::config::{
            DependencyConfig, ModuleConfig, DEFAULT_EXCLUDE_PATHS, ROOT_MODULE_SENTINEL_TAG,
        },
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
                        depends_on: vec![DependencyConfig::from_deprecated_path("domain_two")],
                        strict: false,
                        ..Default::default()
                    },
                    ModuleConfig {
                        path: "domain_three".to_string(),
                        depends_on: vec![],
                        strict: false,
                        ..Default::default()
                    },
                    ModuleConfig {
                        path: "domain_two".to_string(),
                        depends_on: vec![DependencyConfig::from_path("domain_three")],
                        strict: false,
                        ..Default::default()
                    },
                    ModuleConfig {
                        path: ROOT_MODULE_SENTINEL_TAG.to_string(),
                        depends_on: vec![DependencyConfig::from_path("domain_one")],
                        strict: false,
                        ..Default::default()
                    }
                ],
                interfaces: Default::default(),
                cache: Default::default(),
                exclude: DEFAULT_EXCLUDE_PATHS
                    .into_iter()
                    .chain(["domain_four"].into_iter())
                    .map(String::from)
                    .collect(),
                source_roots: vec![PathBuf::from(".")],
                exact: true,
                disable_logging: false,
                ignore_type_checking_imports: true,
                include_string_imports: false,
                forbid_circular_dependencies: true,
                use_regex_matching: true,
                external: Default::default(),
                root_module: Default::default(),
                rules: Default::default(),
            }
        );
    }
}
