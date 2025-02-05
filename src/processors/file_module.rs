use std::{path::Path, sync::Arc};

use crate::filesystem::ProjectFile;
use crate::{config::ModuleConfig, modules::ModuleNode};

use super::dependency::Dependency;
use super::ignore_directive::IgnoreDirectives;
use super::import::NormalizedImport;
use super::reference::SourceCodeReference;
#[derive(Debug)]
pub struct FileModule<'a> {
    pub file: ProjectFile<'a>,
    pub module: Arc<ModuleNode>,
    pub ignore_directives: IgnoreDirectives,
    pub dependencies: Vec<Dependency<'a>>,
}

impl<'a> FileModule<'a> {
    pub fn new(file: ProjectFile<'a>, module: Arc<ModuleNode>) -> Self {
        Self {
            file,
            module,
            ignore_directives: IgnoreDirectives::empty(),
            dependencies: vec![],
        }
    }

    pub fn module_config(&self) -> &ModuleConfig {
        self.module.config.as_ref().unwrap()
    }

    pub fn relative_file_path(&self) -> &Path {
        &self.file.relative_file_path
    }

    pub fn extend_dependencies(&mut self, dependencies: impl IntoIterator<Item = Dependency<'a>>) {
        self.dependencies.extend(dependencies);
    }

    pub fn imports(&self) -> impl Iterator<Item = &NormalizedImport> {
        self.dependencies.iter().filter_map(|dependency| {
            if let Dependency::Import(import) = dependency {
                Some(import)
            } else {
                None
            }
        })
    }

    pub fn references(&self) -> impl Iterator<Item = &SourceCodeReference> {
        self.dependencies.iter().filter_map(|dependency| {
            if let Dependency::Reference(reference) = dependency {
                Some(reference)
            } else {
                None
            }
        })
    }
}

impl<'a> AsRef<FileModule<'a>> for FileModule<'a> {
    fn as_ref(&self) -> &FileModule<'a> {
        self
    }
}
