use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{config::ModuleConfig, filesystem::relative_to, modules::ModuleNode};

use super::imports::{NormalizedImports, ProjectImports};

#[derive(Debug)]
pub struct InternalFile<'a> {
    pub project_root: &'a Path,
    pub file_path: PathBuf,
    pub relative_file_path: PathBuf,
}

impl<'a> InternalFile<'a> {
    pub fn new(project_root: &'a Path, source_root: &'a Path, file_path: &'a Path) -> Self {
        let absolute_file_path = source_root.join(file_path);
        Self {
            project_root,
            relative_file_path: relative_to(&absolute_file_path, project_root).unwrap(),
            file_path: absolute_file_path,
        }
    }
}

impl AsRef<Path> for InternalFile<'_> {
    fn as_ref(&self) -> &Path {
        &self.file_path
    }
}

pub struct ProcessedInternalFile<'a> {
    pub internal_file: InternalFile<'a>,
    pub file_module: Arc<ModuleNode>,
    pub project_imports: NormalizedImports<ProjectImports>,
}

impl<'a> ProcessedInternalFile<'a> {
    pub fn new(
        internal_file: InternalFile<'a>,
        file_module: Arc<ModuleNode>,
        project_imports: NormalizedImports<ProjectImports>,
    ) -> Self {
        Self {
            internal_file,
            file_module,
            project_imports,
        }
    }

    pub fn file_module_config(&self) -> &ModuleConfig {
        self.file_module.config.as_ref().unwrap()
    }

    pub fn relative_file_path(&self) -> &Path {
        &self.internal_file.relative_file_path
    }
}

impl<'a> AsRef<ProcessedInternalFile<'a>> for ProcessedInternalFile<'a> {
    fn as_ref(&self) -> &ProcessedInternalFile<'a> {
        self
    }
}
