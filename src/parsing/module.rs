use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;

use crate::core::config::ModuleConfig;
use crate::core::module::ModuleTree;
use petgraph::algo::kosaraju_scc;
use petgraph::graphmap::DiGraphMap;

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
    source_roots: Vec<PathBuf>,
    modules: Vec<ModuleConfig>,
    forbid_circular_dependencies: bool,
) -> Result<ModuleTree, Box<dyn Error>> {
    // Check for duplicate modules
    let duplicate_modules = find_duplicate_modules(&modules);
    if !duplicate_modules.is_empty() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Failed to build module tree. The following modules were defined more than once: {:?}",
                duplicate_modules
            ),
        )));
    }

    // Check for circular dependencies if forbidden
    if forbid_circular_dependencies {
        let module_paths = find_modules_with_cycles(&modules);
        if !module_paths.is_empty() {
            // return Err(Box::new(TachCircularDependencyError::new(module_paths)));
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Circular dependency error: {:?}", module_paths),
            )));
        }
    }

    // Construct the ModuleTree
    let mut tree = ModuleTree::new();
    for module in modules {
        let interface_members = parse_interface_members(&source_roots, &module.path)?;
        let mod_path = module.mod_path();
        tree.insert(module, mod_path, interface_members);
    }

    Ok(tree)
}
