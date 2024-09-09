use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use crate::parsing::error::ModuleTreeError;

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
    pub children: HashMap<String, Arc<ModuleNode>>,
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
    path.split('.').collect()
}

/// The core data structure for tach, representing the modules in a project
/// with a tree structure for module path lookups.
///
#[derive(Debug)]
pub struct ModuleTree {
    pub root: Arc<ModuleNode>,
}

impl Default for ModuleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleTree {
    pub fn new() -> Self {
        Self {
            root: Arc::new(ModuleNode::implicit_root()),
        }
    }

    pub fn get(&self, path: &str) -> Option<Arc<ModuleNode>> {
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
            Some(node)
        } else {
            None
        }
    }

    pub fn insert(
        &mut self,
        config: ModuleConfig,
        path: String,
        interface_members: Vec<String>,
    ) -> Result<(), ModuleTreeError> {
        if path.is_empty() {
            return Err(ModuleTreeError::InsertNodeError);
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
        Ok(())
    }

    pub fn find_nearest(&self, path: &str) -> Option<Arc<ModuleNode>> {
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
            Some(nearest_parent)
        } else {
            None
        }
    }

    pub fn iter(&self) -> ModuleTreeIterator {
        ModuleTreeIterator::new(self)
    }
}

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
        while let Some(node) = self.stack.pop_front() {
            self.stack.extend(node.children.values().map(Arc::clone));
            if node.is_end_of_path {
                return Some(node);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rstest::{fixture, rstest};

    use super::*;
    use crate::tests::module::fixtures::module_tree;

    #[fixture]
    fn test_config() -> ModuleConfig {
        ModuleConfig::new("test", false)
    }

    #[rstest]
    fn test_iterate_over_empty_tree() {
        let tree = ModuleTree::new();
        let paths: Vec<String> = tree.iter().map(|node| node.full_path.clone()).collect();
        assert_eq!(paths, ["."]);
    }
    #[rstest]
    fn test_iterate_over_populated_tree(module_tree: ModuleTree) {
        let paths: HashSet<String> = module_tree
            .iter()
            .map(|node| node.full_path.clone())
            .collect();
        assert_eq!(
            paths,
            HashSet::from(
                [
                    ".",
                    "domain_one",
                    "domain_one.subdomain",
                    "domain_two",
                    "domain_two.subdomain",
                    "domain_three"
                ]
                .map(String::from)
            )
        );
    }

    #[rstest]
    fn test_get_nonexistent_path(module_tree: ModuleTree) {
        assert!(module_tree.get("fakepath").is_none());
    }

    #[rstest]
    fn test_get_empty_path() {
        let tree = ModuleTree::new();
        assert!(tree.get("").is_none());
    }

    #[rstest]
    fn test_get_actual_path(module_tree: ModuleTree) {
        assert!(module_tree.get("domain_one").is_some());
    }

    #[rstest]
    fn test_insert_empty_path(test_config: ModuleConfig) {
        let mut tree = ModuleTree::new();
        let result = tree.insert(test_config, "".to_string(), vec![]);
        assert!(matches!(
            result.unwrap_err(),
            ModuleTreeError::InsertNodeError
        ));
    }

    #[rstest]
    fn test_insert_single_level_path(test_config: ModuleConfig) {
        let mut tree = ModuleTree::new();
        let result = tree.insert(test_config, "domain".to_string(), vec![]);
        assert!(result.is_ok());
        let paths: Vec<String> = tree.iter().map(|node| node.full_path.clone()).collect();
        assert_eq!(paths, [".", "domain"]);
    }

    #[rstest]
    fn test_insert_multi_level_path(test_config: ModuleConfig) {
        let mut tree = ModuleTree::new();
        let result = tree.insert(test_config, "domain.subdomain".to_string(), vec![]);
        assert!(result.is_ok());
        let paths: Vec<String> = tree.iter().map(|node| node.full_path.clone()).collect();
        assert_eq!(paths, [".", "domain.subdomain"]);
    }

    #[rstest]
    fn test_find_nearest_at_root(module_tree: ModuleTree) {
        let module = module_tree.find_nearest("other_domain");
        assert_eq!(module, Some(module_tree.root));
    }

    #[rstest]
    fn test_find_nearest_in_single_domain(module_tree: ModuleTree) {
        let module = module_tree.find_nearest("domain_one.thing");
        assert_eq!(module.unwrap().full_path, "domain_one");
    }

    #[rstest]
    fn test_find_nearest_in_nested_domain(module_tree: ModuleTree) {
        let module = module_tree.find_nearest("domain_two.subdomain.thing");
        assert_eq!(module.unwrap().full_path, "domain_two.subdomain");
    }
}
