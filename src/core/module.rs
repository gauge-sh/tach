use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use pyo3::pyclass;

use super::config::ModuleConfig;

/// A node in the module tree.
///
/// If 'is_end_of_path' is True, this node represents a module in the project,
/// and must have 'config' and 'full_path' set.
///
/// If 'is_end_of_path' is False, this node does not represent a real module,
/// and must have 'config' None and 'full_path' as the empty string.
///
// #[derive(Debug)]
// #[pyclass(module = "tach.extension")]
#[derive(Clone)]
pub struct ModuleNode {
    is_end_of_path: bool,
    full_path: String,
    config: Option<ModuleConfig>,
    interface_members: Vec<String>,
    children: HashMap<String, Arc<ModuleNode>>,
}

// #[pymethods]
impl ModuleNode {
    // #[staticmethod]
    pub fn empty() -> Self {
        Self {
            is_end_of_path: false,
            full_path: String::new(),
            config: None,
            interface_members: vec![],
            children: HashMap::new(),
        }
    }

    // #[staticmethod]
    pub fn implicit_root() -> Self {
        let config = ModuleConfig::new_root_config();
        Self {
            is_end_of_path: true,
            full_path: ".".to_string(),
            config: Some(config),
            interface_members: vec![],
            children: HashMap::new(),
        }
    }

    pub fn fill(
        &mut self,
        config: ModuleConfig,
        full_path: String,
        interface_members: Vec<String>,
    ) {
        self.is_end_of_path = true;
        self.config = Some(config);
        self.full_path = full_path;
        self.interface_members = interface_members;
    }
}

fn split_module_path(path: &str) -> Vec<&str> {
    if path == "." {
        return vec![];
    }
    path.split(".").collect()
}

/// The core data structure for tach, representing the modules in a project
/// with a tree structure for module path lookups.
///
// #[derive(Debug)]
// #[pyclass(module = "tach.extension")]
pub struct ModuleTree {
    root: Arc<ModuleNode>,
}

// #[pymethods]
impl Default for ModuleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleTree {
    // #[new]
    pub fn new() -> Self {
        Self {
            root: Arc::new(ModuleNode::implicit_root()),
        }
    }

    pub fn get(&self, path: &str) -> Option<ModuleNode> {
        if path.is_empty() {
            return None;
        }

        let mut node = Arc::clone(&self.root);
        for part in split_module_path(path) {
            if let Some(child) = node.children.get(part) {
                node = Arc::clone(child);
            } else {
                return None;
            }
        }

        if node.is_end_of_path {
            Some((*node).clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, config: ModuleConfig, path: String, interface_members: Vec<String>) {
        if path.is_empty() {
            panic!("Cannot insert module with empty path.");
        }

        let mut node = Arc::get_mut(&mut self.root).unwrap();
        for part in split_module_path(&path) {
            node = Arc::get_mut(
                node.children
                    .entry(part.to_owned())
                    .or_insert(Arc::new(ModuleNode::empty())),
            )
            .unwrap();
        }

        node.fill(config, path, interface_members);
    }

    pub fn find_nearest(&self, path: &str) -> Option<ModuleNode> {
        let mut node = Arc::clone(&self.root);
        let mut nearest_parent = Arc::clone(&self.root);

        for part in split_module_path(path) {
            if let Some(child) = node.children.get(part) {
                node = Arc::clone(child);
                if node.is_end_of_path {
                    nearest_parent = Arc::clone(&node);
                }
            } else {
                break;
            }
        }

        if nearest_parent.is_end_of_path {
            Some((*nearest_parent).clone())
        } else {
            None
        }
    }

    pub fn iter(&self) -> ModuleTreeIterator {
        ModuleTreeIterator::new(self)
    }
}

#[pyclass(module = "tach.extension")]
pub struct ModuleTreeIterator {
    stack: VecDeque<Arc<ModuleNode>>,
}

impl ModuleTreeIterator {
    pub fn new(tree: &ModuleTree) -> Self {
        let mut stack = VecDeque::new();
        stack.push_back(Arc::clone(&tree.root));
        Self { stack }
    }
}

impl Iterator for ModuleTreeIterator {
    type Item = Arc<ModuleNode>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.stack.pop_front() {
            for child in node.children.values() {
                self.stack.push_back(Arc::clone(child));
            }
            if node.is_end_of_path {
                return Some(node);
            }
        }
        None
    }
}
