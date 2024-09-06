use thiserror::Error;

use crate::check_int::check;
use crate::core::config::{
    dump_project_config_to_toml, DependencyConfig, ModuleConfig, ProjectConfig,
};
use crate::filesystem as fs;
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
        let mut existing_modules: Vec<ModuleConfig> = Vec::new();

        let source_roots: Vec<PathBuf> = project_config
            .source_roots
            .iter()
            .map(|r| project_root.join(r))
            .collect();

        for module in project_config.modules.iter() {
            // Filter out modules that are not found in the source roots
            let module_path = fs::module_to_pyfile_or_dir_path(&source_roots, &module.path);

            if module_path.is_some() {
                existing_modules.push(module.clone());
            }

            // Track deprecations for each module
            for dependency in module.depends_on.iter() {
                if dependency.deprecated {
                    deprecation_map
                        .entry(module.path.clone())
                        .or_insert_with(Vec::new)
                        .push(dependency.path.clone());
                }
            }
        }

        // Create a new configuration with the updated module list
        let new_modules: Vec<ModuleConfig> = existing_modules
            .into_iter()
            .map(|mut module| {
                module.depends_on.clear(); // Clear dependencies for pruning
                module
            })
            .collect();

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
                .map_or(false, |deps| deps.contains(&dep_path));

            let dependency = DependencyConfig {
                path: dep_path.clone(),
                deprecated,
            };

            new_project_config.add_dependency_to_module(&source_path, dependency);
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
