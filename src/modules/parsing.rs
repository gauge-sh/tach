use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::core::config::{
    global_visibility, ModuleConfig, RootModuleTreatment, ROOT_MODULE_SENTINEL_TAG,
};
use petgraph::algo::kosaraju_scc;
use petgraph::graphmap::DiGraphMap;

use super::error::{ModuleTreeError, VisibilityErrorInfo};
use super::tree::ModuleTree;

pub fn find_duplicate_modules(modules: &[ModuleConfig]) -> Vec<&String> {
    let mut duplicate_module_paths = Vec::new();
    let mut seen = HashSet::new();

    for module in modules {
        if seen.contains(&module.path) {
            duplicate_module_paths.push(&module.path);
        } else {
            seen.insert(&module.path);
        }
    }

    duplicate_module_paths
}

fn visibility_matches_module_path(visibility: &str, module_path: &str) -> bool {
    // If visibility pattern is exactly '*', any module path matches
    if visibility == "*" {
        return true;
    }

    let visibility_components: Vec<&str> = visibility.split('.').collect();
    let module_components: Vec<&str> = module_path.split('.').collect();

    // If the number of components doesn't match, return false
    if visibility_components.len() != module_components.len() {
        return false;
    }

    // Compare each component
    visibility_components
        .iter()
        .zip(module_components.iter())
        .all(|(vis_comp, mod_comp)| *vis_comp == "*" || *vis_comp == *mod_comp)
}

pub fn find_visibility_violations(modules: &[ModuleConfig]) -> Vec<VisibilityErrorInfo> {
    let mut visibility_by_path: HashMap<String, Vec<String>> = HashMap::new();
    let mut globally_visible_paths: HashSet<String> = HashSet::new();

    let global_vis = global_visibility();
    modules.iter().for_each(|module| {
        if module.visibility == global_vis {
            globally_visible_paths.insert(module.mod_path().clone());
        } else {
            visibility_by_path.insert(module.mod_path().clone(), module.visibility.clone());
        }
    });

    let mut results: Vec<VisibilityErrorInfo> = Vec::new();
    for module in modules.iter() {
        for dependency_config in module.depends_on.iter() {
            if let Some(visibility) = visibility_by_path.get(&dependency_config.path) {
                // check if visibility of this dependency doesn't match the current module
                if !visibility.iter().any(|visibility_pattern| {
                    visibility_matches_module_path(visibility_pattern, &module.mod_path())
                }) {
                    results.push(VisibilityErrorInfo {
                        dependent_module: module.mod_path().clone(),
                        dependency_module: dependency_config.path.clone(),
                        visibility: visibility.clone(),
                    })
                }
            }
        }
    }

    results
}

pub fn find_modules_with_cycles(modules: &[ModuleConfig]) -> Vec<&String> {
    let mut graph = DiGraphMap::new();

    // Add nodes
    for module in modules {
        graph.add_node(&module.path);
    }

    // Add dependency edges
    for module in modules {
        for dependency in &module.depends_on {
            graph.add_edge(&module.path, &dependency.path, None::<()>);
        }
    }

    // Find strongly connected components (SCCs)
    let sccs = kosaraju_scc(&graph);

    // Filter SCCs to find cycles
    let mut modules_with_cycles = Vec::new();
    for scc in sccs {
        if scc.len() > 1 {
            modules_with_cycles.extend(scc);
        }
    }

    modules_with_cycles
}

fn validate_root_module_treatment(
    root_module_treatment: RootModuleTreatment,
    modules: &[ModuleConfig],
) -> Result<(), ModuleTreeError> {
    match root_module_treatment {
        RootModuleTreatment::Allow | RootModuleTreatment::Ignore => Ok(()),
        RootModuleTreatment::Forbid => {
            let root_module_violations: Vec<String> = modules
                .iter()
                .filter_map(|module| {
                    if module.path == ROOT_MODULE_SENTINEL_TAG
                        || module
                            .depends_on
                            .iter()
                            .any(|dep| dep.path == ROOT_MODULE_SENTINEL_TAG)
                    {
                        return Some(module.path.clone());
                    }
                    None
                })
                .collect();

            if root_module_violations.is_empty() {
                Ok(())
            } else {
                Err(ModuleTreeError::RootModuleViolation(format!(
                    "The root module ('{}') is forbidden, but was found in module configuration for modules: {}.",
                    ROOT_MODULE_SENTINEL_TAG,
                    root_module_violations.into_iter().map(|module| format!("'{}'", module)).collect::<Vec<_>>().join(", ")
                )))
            }
        }
        RootModuleTreatment::DependenciesOnly => {
            let root_module_violations: Vec<String> = modules
                .iter()
                .filter(|module| {
                    module
                        .depends_on
                        .iter()
                        .any(|dep| dep.path == ROOT_MODULE_SENTINEL_TAG)
                })
                .map(|module| module.path.clone())
                .collect();

            if root_module_violations.is_empty() {
                Ok(())
            } else {
                Err(ModuleTreeError::RootModuleViolation(format!(
                    "The root module ('{}') is set to allow dependencies only, but was found as a dependency in: {}.",
                    ROOT_MODULE_SENTINEL_TAG,
                    root_module_violations.into_iter().map(|module| format!("'{}'", module)).collect::<Vec<_>>().join(", ")
                )))
            }
        }
    }
}

pub fn build_module_tree(
    source_roots: &[PathBuf],
    modules: &[ModuleConfig],
    forbid_circular_dependencies: bool,
    root_module_treatment: RootModuleTreatment,
) -> Result<ModuleTree, ModuleTreeError> {
    // Check for duplicate modules
    let duplicate_modules = find_duplicate_modules(modules);
    if !duplicate_modules.is_empty() {
        return Err(ModuleTreeError::DuplicateModules(
            duplicate_modules.iter().map(|s| s.to_string()).collect(),
        ));
    }

    // Check for visibility errors (dependency declared on invisible module)
    let visibility_error_info = find_visibility_violations(modules);
    if !visibility_error_info.is_empty() {
        return Err(ModuleTreeError::VisibilityViolation(visibility_error_info));
    }

    // Check for root module treatment errors
    validate_root_module_treatment(root_module_treatment, modules)?;

    // Check for circular dependencies if forbidden
    if forbid_circular_dependencies {
        let module_paths = find_modules_with_cycles(modules);
        if !module_paths.is_empty() {
            return Err(ModuleTreeError::CircularDependency(
                module_paths.iter().map(|s| s.to_string()).collect(),
            ));
        }
    }

    // Construct the ModuleTree
    let mut tree = ModuleTree::new();
    for module in modules {
        let mod_path = module.mod_path();
        tree.insert(module.clone(), mod_path)?;
    }

    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parsing::config::parse_project_config, tests::fixtures::example_dir};
    use rstest::rstest;
    #[rstest]
    fn test_valid_circular_dependencies(example_dir: PathBuf) {
        let project_config = parse_project_config(example_dir.join("valid/tach.toml"));
        assert!(project_config.is_ok());
        let (project_config, _) = project_config.unwrap();
        let modules = project_config.modules;
        let modules_with_cycles = find_modules_with_cycles(&modules);
        assert!(modules_with_cycles.is_empty());
    }

    #[rstest]
    fn test_cycles_circular_dependencies(example_dir: PathBuf) {
        let project_config = parse_project_config(example_dir.join("cycles/tach.toml"));
        assert!(project_config.is_ok());
        let (project_config, _) = project_config.unwrap();
        let modules = project_config.modules;
        let module_paths = find_modules_with_cycles(&modules);
        assert_eq!(module_paths, ["domain_one", "domain_two", "domain_three"]);
    }
}