use std::collections::HashSet;
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};

use pyo3::{pyclass, pymethods};
use thiserror::Error;

use crate::core::config::{ModuleConfig, ProjectConfig};
use crate::core::module::ModuleTree;
use crate::filesystem::{self as fs};
use crate::imports::get_project_imports;
use crate::parsing::module::build_module_tree;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("Filesystem error occurred.\n{0}")]
    Filesystem(#[from] fs::FileSystemError),
    #[error("Could not find module containing path: {0}")]
    ModuleNotFound(String),
}

pub type Result<T> = std::result::Result<T, TestError>;

#[pyclass(module = "tach.extension")]
pub struct TachPytestPluginHandler {
    source_roots: Vec<PathBuf>,
    module_tree: ModuleTree,
    affected_modules: HashSet<String>,
    #[pyo3(get)]
    all_affected_modules: HashSet<PathBuf>,
    #[pyo3(get)]
    removed_test_paths: HashSet<PathBuf>,
    #[pyo3(get, set)]
    num_removed_items: i32,
    #[pyo3(get, set)]
    tests_ran_to_completion: bool,
}

#[pymethods]
impl TachPytestPluginHandler {
    #[new]
    fn new(
        project_root: PathBuf,
        project_config: &ProjectConfig,
        changed_files: Vec<PathBuf>,
        all_affected_modules: HashSet<PathBuf>,
    ) -> Self {
        let source_roots = project_config.prepend_roots(&project_root);
        let (valid_modules, invalid_modules) =
            fs::validate_project_modules(&source_roots, project_config.modules.clone());
        for invalid_module in invalid_modules {
            eprintln!(
                "Module '{}' not found. It will be ignored.",
                invalid_module.path
            );
        }

        // TODO: Remove unwraps
        let module_tree = build_module_tree(
            &source_roots,
            valid_modules,
            project_config.forbid_circular_dependencies,
        )
        .unwrap();

        let affected_modules =
            get_affected_modules(&project_root, project_config, changed_files, &module_tree)
                .unwrap();

        Self {
            source_roots,
            module_tree,
            affected_modules,
            all_affected_modules,
            removed_test_paths: HashSet::new(),
            num_removed_items: 0,
            tests_ran_to_completion: false,
        }
    }

    pub fn should_remove_items(&self, file_path: PathBuf) -> bool {
        // TODO: Remove unwrap
        let project_imports = get_project_imports(&self.source_roots, &file_path, true).unwrap();
        let mut should_remove = true;

        for import in project_imports {
            if let Some(nearest_module) = self.module_tree.find_nearest(&import.module_path) {
                if self.affected_modules.contains(&nearest_module.full_path) {
                    // If the module is affected, break early and don't remove the item
                    should_remove = false;
                    break;
                }
            }
        }
        should_remove
    }
}

fn build_module_consumer_map(modules: &Vec<ModuleConfig>) -> HashMap<&String, Vec<String>> {
    let mut consumer_map: HashMap<&String, Vec<String>> = HashMap::new();
    for module in modules {
        for dependency in &module.depends_on {
            consumer_map
                .entry(&dependency.path)
                .or_insert_with(Vec::new)
                .push(module.mod_path());
        }
    }
    consumer_map
}

fn get_changed_module_paths(
    project_root: &PathBuf,
    project_config: &ProjectConfig,
    changed_files: Vec<PathBuf>,
) -> Result<Vec<String>> {
    let source_roots: Vec<PathBuf> = project_config.prepend_roots(&project_root);

    let changed_module_paths = changed_files
        .into_iter()
        .filter(|file| {
            file.extension().unwrap_or_default() == "py"
                && source_roots.iter().any(|root| file.starts_with(root))
        })
        .map(|file| fs::file_to_module_path(&source_roots, &file))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(changed_module_paths)
}

fn find_affected_modules(
    root_module_path: &String,
    module_consumers: &HashMap<&String, Vec<String>>,
    mut known_affected_modules: HashSet<String>,
) -> HashSet<String> {
    if let Some(consumers) = module_consumers.get(root_module_path) {
        for consumer in consumers {
            if !known_affected_modules.contains(consumer) {
                known_affected_modules.insert(consumer.clone());
                known_affected_modules.extend(find_affected_modules(
                    consumer,
                    module_consumers,
                    known_affected_modules.clone(),
                ));
            }
        }
    }
    known_affected_modules
}

pub fn get_affected_modules(
    project_root: &PathBuf,
    project_config: &ProjectConfig,
    changed_files: Vec<PathBuf>,
    module_tree: &ModuleTree,
) -> Result<HashSet<String>> {
    let changed_module_paths =
        get_changed_module_paths(project_root, project_config, changed_files)?;

    let mut affected_modules = HashSet::new();
    for changed_mod_path in changed_module_paths {
        let nearest_module = module_tree
            .find_nearest(&changed_mod_path)
            .ok_or(TestError::ModuleNotFound(changed_mod_path))?;
        affected_modules.insert(nearest_module.full_path.clone());
    }

    let module_consumers = build_module_consumer_map(&project_config.modules);
    for module in affected_modules.clone() {
        affected_modules = find_affected_modules(&module, &module_consumers, affected_modules);
    }

    Ok(affected_modules.into_iter().collect())
}
