use ruff_linter::Locator;
use ruff_source_file::LineIndex;
use ruff_text_size::TextSize;
use std::collections::HashSet;
use std::{path::Path, sync::Arc};

use crate::dependencies::{Dependency, NormalizedImport, SourceCodeReference};
use crate::filesystem::ProjectFile;
use crate::processors::ignore_directive::{get_ignore_directives, IgnoreDirectives};
use crate::resolvers::Package;
use crate::{config::ModuleConfig, modules::ModuleNode};

#[derive(Debug)]
pub struct FileModule<'a> {
    pub file: ProjectFile<'a>,
    pub module: Arc<ModuleNode>,
    pub package: &'a Package,
    pub ignore_directives: IgnoreDirectives,
    pub dependencies: Vec<Dependency>,
    line_index: LineIndex,
}

impl<'a> FileModule<'a> {
    pub fn new(file: ProjectFile<'a>, module: Arc<ModuleNode>, package: &'a Package) -> Self {
        Self {
            ignore_directives: get_ignore_directives(&file.contents),
            line_index: Locator::new(&file.contents).to_index().clone(),
            file,
            module,
            package,
            dependencies: vec![],
        }
    }

    pub fn file_path(&self) -> &Path {
        &self.file.file_path
    }

    pub fn contents(&self) -> &str {
        &self.file.contents
    }

    pub fn line_number(&self, offset: TextSize) -> usize {
        self.line_index.line_index(offset).get()
    }

    pub fn module_config(&self) -> &ModuleConfig {
        self.module.config.as_ref().unwrap()
    }

    pub fn relative_file_path(&self) -> &Path {
        &self.file.relative_file_path
    }

    pub fn extend_dependencies(&mut self, dependencies: impl IntoIterator<Item = Dependency>) {
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

    pub fn declared_dependencies(&self) -> &HashSet<String> {
        &self.package.dependencies
    }
}

impl<'a> AsRef<FileModule<'a>> for FileModule<'a> {
    fn as_ref(&self) -> &FileModule<'a> {
        self
    }
}
