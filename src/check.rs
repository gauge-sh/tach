use std::{collections::HashSet, rc::Rc};

use thiserror::Error;

use crate::core::module::{ModuleNode, ModuleTree};

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("Module containing '{file_mod_path}' not found in project.")]
    ModuleNotFound { file_mod_path: String },

    #[error("Module '{import_nearest_module_path}' is in strict mode. Only imports from the public interface of this module are allowed. The import '{import_mod_path}' (in module '{file_nearest_module_path}') is not included in __all__.")]
    StrictModeImport {
        import_mod_path: String,
        import_nearest_module_path: String,
        file_nearest_module_path: String,
    },

    #[error("Could not find module configuration.")]
    ModuleConfigNotFound,

    #[error("Invalid import {invalid_module} from {source_module}.")]
    InvalidImport {
        source_module: String,
        invalid_module: String,
    },

    #[error("Deprecated import {invalid_module} from {source_module}.")]
    DeprecatedImport {
        source_module: String,
        invalid_module: String,
    },
}

impl CheckError {
    pub fn is_dependency_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidImport { .. } | Self::DeprecatedImport { .. }
        )
    }

    pub fn is_deprecated(&self) -> bool {
        matches!(self, Self::DeprecatedImport { .. })
    }
}

fn is_top_level_module_import(mod_path: &str, module: &ModuleNode) -> bool {
    mod_path == module.full_path
}

fn import_matches_interface_members(mod_path: &str, module: &ModuleNode) -> bool {
    let mod_path_segments: Vec<&str> = mod_path.rsplitn(2, '.').collect();

    if mod_path_segments.len() == 1 {
        // If there's no '.' in the path, compare the whole path with the module's full path.
        mod_path_segments[0] == module.full_path
    } else {
        // If there's a '.', split into package path and member name.
        let mod_pkg_path = mod_path_segments[1];
        let mod_member_name = mod_path_segments[0];

        mod_pkg_path == module.full_path
            && module
                .interface_members
                .contains(&mod_member_name.to_string())
    }
}

fn check_import(
    module_tree: ModuleTree,
    import_mod_path: &str,
    file_mod_path: &str,
    file_nearest_module: Option<Rc<ModuleNode>>,
) -> Result<(), CheckError> {
    let import_nearest_module = match module_tree.find_nearest(import_mod_path) {
        Some(module) => module,
        // This should not be none since we intend to filter out any external imports,
        // but we should allow external imports if they have made it here.
        None => return Ok(()),
    };

    let file_nearest_module = file_nearest_module
        // Lookup file_mod_path if module not given
        .or_else(|| module_tree.find_nearest(file_mod_path))
        // If module not found, we should fail since the implication is that
        // an external module is importing directly from our project
        .ok_or(CheckError::ModuleNotFound {
            file_mod_path: file_mod_path.to_string(),
        })?;

    if import_nearest_module == file_nearest_module {
        // Imports within the same module are always allowed
        return Ok(());
    }

    if let Some(config) = &import_nearest_module.config {
        if config.strict
            && !is_top_level_module_import(import_mod_path, &file_nearest_module)
            && !import_matches_interface_members(import_mod_path, &file_nearest_module)
        {
            // In strict mode, import must be of the module itself or one of the
            // interface members (defined in __all__)
            return Err(CheckError::StrictModeImport {
                import_mod_path: import_mod_path.to_string(),
                import_nearest_module_path: import_nearest_module.full_path.to_string(),
                file_nearest_module_path: file_nearest_module.full_path.to_string(),
            });
        }
    }

    let file_config = file_nearest_module
        .config
        .as_ref()
        .ok_or(CheckError::ModuleConfigNotFound)?;
    let file_nearest_module_path = &file_config.path;
    let import_nearest_module_path = &import_nearest_module
        .config
        .as_ref()
        .ok_or(CheckError::ModuleConfigNotFound)?
        .path;

    // The import must be explicitly allowed in the file's config
    let allowed_dependencies: HashSet<_> = file_config
        .depends_on
        .iter()
        .filter(|dep| !dep.deprecated)
        .map(|dep| &dep.path)
        .collect();

    if allowed_dependencies.contains(import_nearest_module_path) {
        // he import matches at least one expected dependency
        return Ok(());
    }

    let deprecated_dependencies: HashSet<_> = file_config
        .depends_on
        .iter()
        .filter(|dep| dep.deprecated)
        .map(|dep| &dep.path)
        .collect();

    if deprecated_dependencies.contains(import_nearest_module_path) {
        // Dependency exists but is deprecated
        return Err(CheckError::DeprecatedImport {
            source_module: file_nearest_module_path.to_string(),
            invalid_module: import_nearest_module_path.to_string(),
        });
    }

    // This means the import is not declared as a dependency of the file
    Err(CheckError::InvalidImport {
        source_module: file_nearest_module_path.to_string(),
        invalid_module: import_nearest_module_path.to_string(),
    })
}
