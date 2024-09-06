use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use super::config::ModuleConfig;

/// A node in the module tree.
///
/// If 'is_end_of_path' is True, this node represents a module in the project,
/// and must have 'config' and 'full_path' set.
///
/// If 'is_end_of_path' is False, this node does not represent a real module,
/// and must have 'config' None and 'full_path' as the empty string.
///
#[derive(PartialEq, Debug)]
pub struct ModuleNode {
    pub is_end_of_path: bool,
    pub full_path: String,
    pub config: Option<ModuleConfig>,
    pub interface_members: Vec<String>,
    pub children: HashMap<String, Rc<ModuleNode>>,
}

impl ModuleNode {
    pub fn empty() -> Self {
        Self {
            is_end_of_path: false,
            full_path: String::new(),
            config: None,
            interface_members: vec![],
            children: HashMap::new(),
        }
    }

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
#[derive(Debug)]
pub struct ModuleTree {
    pub root: Rc<ModuleNode>,
}

impl Default for ModuleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleTree {
    pub fn new() -> Self {
        Self {
            root: Rc::new(ModuleNode::implicit_root()),
        }
    }

    pub fn get(&self, path: &str) -> Option<Rc<ModuleNode>> {
        if path.is_empty() {
            return None;
        }

        let mut node = Rc::clone(&self.root);
        for part in split_module_path(path) {
            if let Some(child) = node.children.get(part) {
                node = Rc::clone(child);
            } else {
                return None;
            }
        }

        if node.is_end_of_path {
            Some(node)
        } else {
            None
        }
    }

    pub fn insert(&mut self, config: ModuleConfig, path: String, interface_members: Vec<String>) {
        if path.is_empty() {
            panic!("Cannot insert module with empty path.");
        }

        let mut node = Rc::get_mut(&mut self.root).unwrap();
        for part in split_module_path(&path) {
            node = Rc::get_mut(
                node.children
                    .entry(part.to_owned())
                    .or_insert(Rc::new(ModuleNode::empty())),
            )
            .unwrap();
        }

        node.fill(config, path, interface_members);
    }

    pub fn find_nearest(&self, path: &str) -> Option<Rc<ModuleNode>> {
        let mut node = Rc::clone(&self.root);
        let mut nearest_parent = None;

        for part in split_module_path(path) {
            if let Some(child) = node.children.get(part) {
                node = Rc::clone(child);
                if node.is_end_of_path {
                    nearest_parent = Some(Rc::clone(&node));
                }
            } else {
                break;
            }
        }

        nearest_parent
    }

    pub fn iter(&self) -> ModuleTreeIterator {
        ModuleTreeIterator::new(self)
    }
}

pub struct ModuleTreeIterator {
    stack: VecDeque<Rc<ModuleNode>>,
}

impl ModuleTreeIterator {
    pub fn new(tree: &ModuleTree) -> Self {
        let mut stack = VecDeque::new();
        stack.push_back(Rc::clone(&tree.root));
        Self { stack }
    }
}

impl Iterator for ModuleTreeIterator {
    type Item = Rc<ModuleNode>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.stack.pop_front() {
            for child in node.children.values() {
                self.stack.push_back(Rc::clone(child));
            }
            if node.is_end_of_path {
                return Some(node);
            }
        }
        None
    }
}
