use std::path::PathBuf;

use crate::{
    config::{ModuleConfig, RootModuleTreatment},
    filesystem,
    resolvers::{glob, ModuleResolver},
};

use super::{
    validation::{
        find_duplicate_modules, find_modules_with_cycles, find_visibility_violations,
        validate_root_module_treatment,
    },
    ModuleTree, ModuleTreeError,
};

pub struct ModuleTreeBuilder<'a> {
    resolver: ModuleResolver<'a>,
    forbid_circular_dependencies: bool,
    root_module_treatment: RootModuleTreatment,
}

impl<'a> ModuleTreeBuilder<'a> {
    pub fn new(
        source_roots: &'a [PathBuf],
        file_walker: &'a filesystem::FSWalker,
        forbid_circular_dependencies: bool,
        root_module_treatment: RootModuleTreatment,
    ) -> Self {
        Self {
            resolver: ModuleResolver::new(source_roots, file_walker),
            forbid_circular_dependencies,
            root_module_treatment,
        }
    }

    pub fn resolve_modules<'b, T: IntoIterator<Item = &'b ModuleConfig>>(
        &self,
        modules: T,
    ) -> (Vec<ModuleConfig>, Vec<ModuleConfig>) {
        let mut resolved_modules = Vec::new();
        let mut unresolved_modules = Vec::new();

        for module in modules {
            let mod_path = module.mod_path();
            if let Ok(resolved_paths) = self.resolver.resolve_module_path(&mod_path) {
                resolved_modules.extend(resolved_paths.into_iter().map(|path| {
                    if glob::has_glob_syntax(&mod_path) {
                        module.clone_with_path(&path).with_glob_origin(&mod_path)
                    } else {
                        module.clone_with_path(&path)
                    }
                }));
            } else {
                unresolved_modules.push(module.clone());
            }
        }

        (resolved_modules, unresolved_modules)
    }

    pub fn build<T: IntoIterator<Item = ModuleConfig>>(
        self,
        modules: T,
    ) -> Result<ModuleTree, ModuleTreeError> {
        // Collect modules
        let modules: Vec<ModuleConfig> = modules.into_iter().collect();

        // Check for duplicate modules
        let duplicate_modules = find_duplicate_modules(&modules);
        if !duplicate_modules.is_empty() {
            return Err(ModuleTreeError::DuplicateModules(
                duplicate_modules.iter().map(|s| s.to_string()).collect(),
            ));
        }

        // Check for visibility errors (dependency declared on invisible module)
        let visibility_error_info = find_visibility_violations(&modules)?;
        if !visibility_error_info.is_empty() {
            return Err(ModuleTreeError::VisibilityViolation(visibility_error_info));
        }

        // Check for root module treatment errors
        validate_root_module_treatment(self.root_module_treatment, &modules)?;

        // Check for circular dependencies if forbidden
        if self.forbid_circular_dependencies {
            let module_paths = find_modules_with_cycles(&modules);
            if !module_paths.is_empty() {
                return Err(ModuleTreeError::CircularDependency(
                    module_paths.iter().map(|s| s.to_string()).collect(),
                ));
            }
        }

        // Construct the ModuleTree
        let mut tree = ModuleTree::new();
        for module in modules {
            let path = module.mod_path();
            tree.insert(module, path)?;
        }

        Ok(tree)
    }
}
