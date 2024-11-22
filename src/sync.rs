use thiserror::Error;

use crate::check_int::{check, CheckError};
use crate::core::config::{
    global_visibility, DependencyConfig, ModuleConfig, ProjectConfig, RootModuleTreatment,
};
use crate::filesystem::{self as fs, ROOT_MODULE_SENTINEL_TAG};
use crate::parsing::config::dump_project_config_to_toml;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Failed to write project configuration to file.\n{0}")]
    FileWrite(#[from] std::io::Error),
    #[error("Failed to serialize project configuration to TOML.\n{0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Failed to sync project.\n{0}")]
    CheckError(#[from] CheckError),
    #[error("Failed to sync project configuration due to root module violation.\n{0}")]
    RootModuleViolation(String),
}

fn handle_detected_dependency(
    module_path: &str,
    dependency: DependencyConfig,
    project_config: &mut ProjectConfig,
) -> Result<(), SyncError> {
    let module_is_root = module_path == ROOT_MODULE_SENTINEL_TAG;
    let dependency_is_root = dependency.path == ROOT_MODULE_SENTINEL_TAG;

    if !module_is_root && !dependency_is_root {
        project_config.add_dependency_to_module(module_path, dependency);
        return Ok(());
    }

    match project_config.root_module {
        RootModuleTreatment::Ignore => Ok(()),
        RootModuleTreatment::Allow => {
            project_config.add_dependency_to_module(module_path, dependency);
            Ok(())
        }
        RootModuleTreatment::Forbid => Err(SyncError::RootModuleViolation(format!(
            "The root module is forbidden, but it was found that '{}' depends on '{}'.",
            module_path, dependency.path
        ))),
        RootModuleTreatment::DependenciesOnly => {
            if dependency_is_root {
                return Err(SyncError::RootModuleViolation(format!("No module may depend on the root module, but it was found that '{}' depends on the root module.", module_path)));
            }
            project_config.add_dependency_to_module(module_path, dependency);
            Ok(())
        }
    }
}

/// Update project configuration with auto-detected dependency constraints.
/// If prune is set to False, it will create dependencies to resolve existing errors,
/// but will not remove any constraints.
pub fn sync_dependency_constraints(
    project_root: PathBuf,
    mut project_config: ProjectConfig,
    exclude_paths: Vec<String>,
    prune: bool,
) -> Result<ProjectConfig, SyncError> {
    let mut deprecation_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut visibility_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut new_project_config = None;

    // Drain visibility patterns from modules into visibility map, restore after syncing
    project_config.modules.iter_mut().for_each(|module| {
        visibility_map.insert(module.path.clone(), module.visibility.drain(..).collect());
        module.visibility.extend(global_visibility());
    });

    if prune {
        let mut new_modules: Vec<ModuleConfig> = Vec::new();

        let source_roots: Vec<PathBuf> = project_config.prepend_roots(&project_root);
        let (valid_modules, _) =
            fs::validate_project_modules(&source_roots, project_config.modules.clone());

        for module in valid_modules.iter() {
            // Clone modules and remove declared dependencies (unless unchecked, which should keep dependencies)
            if module.unchecked {
                new_modules.push(module.clone());
            } else {
                new_modules.push(module.with_no_dependencies());
            }
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
    let check_result = check(
        project_root,
        &new_project_config,
        true,  // dependencies
        false, // interfaces
        exclude_paths,
    )?;

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

            // The project config determines whether the sync fails, ignores, or adds this dependency
            handle_detected_dependency(source_path, dependency, &mut new_project_config)?
        }
    }

    // Restore visibility settings
    for module in new_project_config.modules.iter_mut() {
        if let Some(visibility) = visibility_map.get(&module.path) {
            module.visibility = visibility.clone();
        }
    }

    Ok(new_project_config)
}

pub fn sync_project(
    project_root: PathBuf,
    project_config: ProjectConfig,
    exclude_paths: Vec<String>,
    add: bool,
) -> Result<String, SyncError> {
    let mut project_config =
        sync_dependency_constraints(project_root, project_config, exclude_paths, !add)?;

    Ok(dump_project_config_to_toml(&mut project_config)?)
}
