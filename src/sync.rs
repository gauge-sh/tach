use thiserror::Error;

use crate::check_int::check;
use crate::core::config::{DependencyConfig, ModuleConfig, ProjectConfig};
use crate::filesystem as fs;
use crate::parsing::config::dump_project_config_to_toml;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Failed to write project configuration to file.\n{0}")]
    FileWrite(#[from] std::io::Error),
    #[error("Failed to serialize project configuration to TOML.\n{0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

/// Update project configuration with auto-detected dependency constraints.
/// If prune is set to False, it will create dependencies to resolve existing errors,
/// but will not remove any constraints.
pub fn sync_dependency_constraints(
    project_root: PathBuf,
    project_config: ProjectConfig,
    exclude_paths: Vec<String>,
    prune: bool,
) -> ProjectConfig {
    let mut deprecation_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut new_project_config = None;

    if prune {
        let mut new_modules: Vec<ModuleConfig> = Vec::new();

        let source_roots: Vec<PathBuf> = project_config.prepend_roots(&project_root);

        for module in project_config.modules.iter() {
            // Filter out modules that are not found in the source roots
            match fs::module_to_pyfile_or_dir_path(&source_roots, &module.path) {
                Some(_) => new_modules.push(ModuleConfig::new(&module.path, module.strict)),
                None => new_modules.push(module.clone()),
            };

            // Track deprecations for each module
            for dependency in module.depends_on.iter() {
                if dependency.deprecated {
                    deprecation_map
                        .entry(module.path.clone())
                        .or_default()
                        .push(dependency.path.clone());
                }
            }
        }
        new_project_config = Some(project_config.with_modules(new_modules));
    }
    let mut new_project_config = new_project_config.unwrap_or(project_config);

    // If prune is false, the existing project config is reused without changes
    let check_result = check(project_root, &new_project_config, exclude_paths)
        .expect("Failed to run the check function");

    // Iterate through the check results to add dependencies to the config
    for error in check_result.errors {
        let error_info = error.error_info;

        if error_info.is_dependency_error() {
            let source_path = error_info.source_path().unwrap();
            let dep_path = error_info.invalid_path().unwrap();

            let deprecated = deprecation_map
                .get(source_path)
                .map_or(false, |deps| deps.contains(dep_path));

            let dependency = DependencyConfig {
                path: dep_path.clone(),
                deprecated,
            };

            new_project_config.add_dependency_to_module(source_path, dependency);
        }
    }

    new_project_config
}

pub fn sync_project(
    project_root: PathBuf,
    project_config: ProjectConfig,
    exclude_paths: Vec<String>,
    add: bool,
) -> Result<String, SyncError> {
    let mut project_config =
        sync_dependency_constraints(project_root, project_config, exclude_paths, !add);

    Ok(dump_project_config_to_toml(&mut project_config)?)
}
