use std::collections::HashSet;
use std::path::PathBuf;

use crate::core::config::ModuleConfig;
use crate::core::module::ModuleTree;
use petgraph::algo::kosaraju_scc;
use petgraph::graphmap::DiGraphMap;

use super::error::ModuleTreeError;
use super::py_ast::parse_interface_members;

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

pub fn build_module_tree(
    source_roots: &[PathBuf],
    modules: Vec<ModuleConfig>,
    forbid_circular_dependencies: bool,
) -> Result<ModuleTree, ModuleTreeError> {
    // Check for duplicate modules
    let duplicate_modules = find_duplicate_modules(&modules);
    if !duplicate_modules.is_empty() {
        return Err(ModuleTreeError::DuplicateModules(
            duplicate_modules.iter().map(|s| s.to_string()).collect(),
        ));
    }

    // Check for circular dependencies if forbidden
    if forbid_circular_dependencies {
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
        let interface_members = parse_interface_members(source_roots, &module.path)?;
        let mod_path = module.mod_path();
        tree.insert(module, mod_path, interface_members)?;
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
        let modules = project_config.unwrap().modules;
        let modules_with_cycles = find_modules_with_cycles(&modules);
        assert!(modules_with_cycles.is_empty());
    }

    #[rstest]
    fn test_cycles_circular_dependencies(example_dir: PathBuf) {
        let project_config = parse_project_config(example_dir.join("cycles/tach.toml"));
        assert!(project_config.is_ok());
        let modules = project_config.unwrap().modules;
        let module_paths = find_modules_with_cycles(&modules);
        assert_eq!(module_paths, ["domain_one", "domain_two", "domain_three"]);
    }
}
