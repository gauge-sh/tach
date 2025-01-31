use std::path::Path;

use crate::{
    config::{ModuleConfig, ProjectConfig},
    modules::ModuleTree,
};

#[derive(Debug)]
pub struct FileContext<'a> {
    pub project_config: &'a ProjectConfig,
    pub relative_file_path: &'a Path,
    pub file_module_config: &'a ModuleConfig,
    pub module_tree: &'a ModuleTree,
}

impl<'a> FileContext<'a> {
    pub fn new(
        project_config: &'a ProjectConfig,
        relative_file_path: &'a Path,
        file_module_config: &'a ModuleConfig,
        module_tree: &'a ModuleTree,
    ) -> Self {
        Self {
            project_config,
            relative_file_path,
            file_module_config,
            module_tree,
        }
    }
}
