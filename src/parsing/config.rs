use std::{cmp::Ordering, path::Path};

use crate::{
    core::config::ProjectConfig,
    filesystem::{self, ROOT_MODULE_SENTINEL_TAG},
};

use super::error;

pub fn dump_project_config_to_toml(config: &mut ProjectConfig) -> Result<String, toml::ser::Error> {
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

pub fn parse_project_config<P: AsRef<Path>>(filepath: P) -> error::Result<ProjectConfig> {
    let content = filesystem::read_file_content(filepath)?;
    let config: ProjectConfig = toml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        core::config::{CacheConfig, DependencyConfig, ExternalDependencyConfig, ModuleConfig},
        tests::fixtures::example_dir,
    };
    use filesystem::DEFAULT_EXCLUDE_PATHS;
    use rstest::rstest;
    #[rstest]
    fn test_parse_valid_project_config(example_dir: PathBuf) {
        // TODO: remove tach.toml when joining
        let result = parse_project_config(example_dir.join("valid/tach.toml"));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            ProjectConfig {
                modules: vec![
                    ModuleConfig {
                        path: "domain_one".to_string(),
                        depends_on: vec![DependencyConfig::from_deprecated_path("domain_two")],
                        strict: false,
                    },
                    ModuleConfig {
                        path: "domain_three".to_string(),
                        depends_on: vec![],
                        strict: false,
                    },
                    ModuleConfig {
                        path: "domain_two".to_string(),
                        depends_on: vec![DependencyConfig::from_undeprecated_path("domain_three")],
                        strict: false,
                    },
                    ModuleConfig {
                        path: ROOT_MODULE_SENTINEL_TAG.to_string(),
                        depends_on: vec![DependencyConfig::from_undeprecated_path("domain_one")],
                        strict: false,
                    }
                ],
                cache: CacheConfig::default(),
                exclude: DEFAULT_EXCLUDE_PATHS
                    .into_iter()
                    .chain(["domain_four"].into_iter())
                    .map(String::from)
                    .collect(),
                source_roots: vec![PathBuf::from(".")],
                exact: true,
                disable_logging: false,
                ignore_type_checking_imports: true,
                forbid_circular_dependencies: true,
                use_regex_matching: true,
                external: ExternalDependencyConfig::default(),
            }
        );
    }
}
