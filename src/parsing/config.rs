use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use rayon::prelude::*;

use crate::{
    colors::BColors,
    config::{
        project::PyProjectWrapper, root_module::ROOT_MODULE_SENTINEL_TAG, ConfigLocation,
        DomainConfig, InterfaceConfig, InterfaceDataTypes, LocatedDomainConfig, ProjectConfig,
    },
    filesystem::{self, read_file_content},
    python::parsing::parse_interface_members,
    resolvers::SourceRootResolver,
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
                visibility: None,
                data_types: InterfaceDataTypes::All,
                exclusive: false,
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

pub fn parse_domain_config<P: AsRef<Path>>(
    source_roots: &[PathBuf],
    filepath: P,
) -> Result<LocatedDomainConfig> {
    let content = read_file_content(filepath.as_ref())?;
    let config: DomainConfig = toml::from_str(&content)?;
    let location = ConfigLocation::new(source_roots, filepath.as_ref())?;
    Ok(config.with_location(location))
}

pub fn add_domain_configs<P: AsRef<Path>>(config: &mut ProjectConfig, root_dir: P) -> Result<()> {
    let root_dir = root_dir.as_ref().to_path_buf();
    let file_walker =
        filesystem::FSWalker::try_new(&root_dir, &config.exclude, config.respect_gitignore)?;
    let source_root_resolver = SourceRootResolver::new(&root_dir, &file_walker);
    let source_roots = source_root_resolver.resolve(&config.source_roots)?;
    let mut domain_configs = file_walker
        .walk_domain_config_files(root_dir.as_os_str().to_str().unwrap())
        .par_bridge()
        .map(|filepath| parse_domain_config(&source_roots, filepath))
        .collect::<Result<Vec<_>>>()?;
    domain_configs.drain(..).for_each(|domain| {
        config.add_domain(domain);
    });
    Ok(())
}

pub fn parse_project_config<P: AsRef<Path>>(filepath: P) -> Result<(ProjectConfig, bool)> {
    let content = read_file_content(filepath.as_ref())?;
    let mut config: ProjectConfig = toml::from_str(&content)?;
    config.set_location(filepath.as_ref().to_path_buf());
    let did_migrate = migrate_strict_mode_to_interfaces(filepath.as_ref(), &mut config)
        || migrate_deprecated_regex_exclude(&mut config);
    add_domain_configs(&mut config, filepath.as_ref().parent().unwrap())?;
    Ok((config, did_migrate))
}

pub fn parse_project_config_from_pyproject<P: AsRef<Path>>(filepath: P) -> Result<ProjectConfig> {
    let content = read_file_content(filepath.as_ref())?;
    let mut config: ProjectConfig = toml::from_str::<PyProjectWrapper>(&content)?.into();
    config.set_location(filepath.as_ref().to_path_buf());
    add_domain_configs(&mut config, filepath.as_ref().parent().unwrap())?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use super::*;
    use crate::{
        config::{
            project::DEFAULT_EXCLUDE_PATHS, root_module::ROOT_MODULE_SENTINEL_TAG, DependencyConfig,
        },
        tests::fixtures::example_dir,
    };
    use rstest::rstest;

    #[rstest]
    fn test_parse_valid_project_config(example_dir: PathBuf) {
        let result = parse_project_config(example_dir.join("valid/tach.toml"));
        assert!(result.is_ok());
        let (config, _) = result.unwrap();

        let module_paths: Vec<_> = config.module_paths();
        assert_eq!(
            module_paths,
            vec![
                "domain_one",
                "domain_three",
                "domain_two",
                ROOT_MODULE_SENTINEL_TAG
            ]
        );

        assert_eq!(
            config
                .dependencies_for_module("domain_one")
                .unwrap()
                .iter()
                .collect::<HashSet<_>>(),
            [DependencyConfig::from_deprecated_path("domain_two")]
                .iter()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            config
                .dependencies_for_module("domain_three")
                .unwrap()
                .iter()
                .collect::<HashSet<_>>(),
            [].iter().collect::<HashSet<_>>()
        );
        assert_eq!(
            config
                .dependencies_for_module("domain_two")
                .unwrap()
                .iter()
                .collect::<HashSet<_>>(),
            [DependencyConfig::from_path("domain_three")]
                .iter()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            config
                .dependencies_for_module(ROOT_MODULE_SENTINEL_TAG)
                .unwrap()
                .iter()
                .collect::<HashSet<_>>(),
            [DependencyConfig::from_path("domain_one")]
                .iter()
                .collect::<HashSet<_>>()
        );

        let expected_excludes: Vec<String> = DEFAULT_EXCLUDE_PATHS
            .into_iter()
            .chain(["domain_four.py"].into_iter())
            .map(String::from)
            .collect();
        assert_eq!(config.exclude, expected_excludes);

        assert!(config.exact);
        assert!(config.forbid_circular_dependencies);
    }

    #[rstest]
    fn test_parse_domain_config(example_dir: PathBuf) {
        let source_roots = vec![example_dir.join("distributed_config")];
        let result = parse_domain_config(
            &source_roots,
            example_dir.join("distributed_config/project/module_one/tach.domain.toml"),
        );
        assert!(result.is_ok());
        let config = result.unwrap();

        let modules: Vec<_> = config.modules().map(|m| m.path.as_str()).collect();
        assert_eq!(modules, vec!["project.module_one"]);

        assert_eq!(
            config.modules().next().unwrap().depends_on,
            Some(vec![DependencyConfig::from_path("project.module_two")])
        );
    }

    #[rstest]
    fn test_parse_nested_project_config(example_dir: PathBuf) {
        let result = parse_project_config(example_dir.join("distributed_config/tach.toml"));
        assert!(result.is_ok());
        let (config, _) = result.unwrap();

        let module_paths: HashSet<_> = config.module_paths().into_iter().collect();
        assert_eq!(
            module_paths,
            vec![
                "project.top_level",
                "project.module_one",
                "project.module_two"
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<HashSet<_>>()
        );

        assert_eq!(
            config
                .dependencies_for_module("project.top_level")
                .unwrap()
                .iter()
                .collect::<HashSet<_>>(),
            [DependencyConfig::from_path("project.module_two")]
                .iter()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            config
                .dependencies_for_module("project.module_one")
                .unwrap()
                .iter()
                .collect::<HashSet<_>>(),
            [DependencyConfig::from_path("project.module_two")]
                .iter()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            config
                .dependencies_for_module("project.module_two")
                .unwrap()
                .iter()
                .collect::<HashSet<_>>(),
            [].iter().collect::<HashSet<_>>()
        );
    }
}
