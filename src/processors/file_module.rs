use std::marker::PhantomData;
use std::path::PathBuf;
use std::{path::Path, sync::Arc};

use crate::filesystem::ProjectFile;
use crate::{config::ModuleConfig, modules::ModuleNode};

use super::imports::{AllImports, ExternalImports, NormalizedImports, ProjectImports};

#[derive(Debug)]
pub struct FileModule<'a, State = AllImports> {
    pub file: ProjectFile<'a>,
    pub module: Arc<ModuleNode>,
    pub imports: NormalizedImports<State>,
    _state: PhantomData<State>,
}

impl<'a, State> FileModule<'a, State> {
    pub fn module_config(&self) -> &ModuleConfig {
        self.module.config.as_ref().unwrap()
    }

    pub fn relative_file_path(&self) -> &Path {
        &self.file.relative_file_path
    }
}

impl<'a, State> AsRef<FileModule<'a, State>> for FileModule<'a, State> {
    fn as_ref(&self) -> &FileModule<'a, State> {
        self
    }
}

impl<'a> FileModule<'a, AllImports> {
    pub fn new(
        file: ProjectFile<'a>,
        module: Arc<ModuleNode>,
        imports: NormalizedImports<AllImports>,
    ) -> Self {
        Self {
            file,
            module,
            imports,
            _state: PhantomData,
        }
    }

    pub fn into_internal(self, source_roots: &[PathBuf]) -> FileModule<'a, ProjectImports> {
        FileModule {
            file: self.file,
            module: self.module,
            imports: self.imports.into_project_imports(source_roots),
            _state: PhantomData,
        }
    }

    pub fn into_external(self, source_roots: &[PathBuf]) -> FileModule<'a, ExternalImports> {
        FileModule {
            file: self.file,
            module: self.module,
            imports: self.imports.into_external_imports(source_roots),
            _state: PhantomData,
        }
    }
}

pub type FileModuleInternal<'a> = FileModule<'a, ProjectImports>;

impl<'a> FileModuleInternal<'a> {
    pub fn new(
        file: ProjectFile<'a>,
        module: Arc<ModuleNode>,
        imports: NormalizedImports<ProjectImports>,
    ) -> Self {
        Self {
            file,
            module,
            imports,
            _state: PhantomData,
        }
    }
}

pub type FileModuleExternal<'a> = FileModule<'a, ExternalImports>;

impl<'a> FileModuleExternal<'a> {
    pub fn new(
        file: ProjectFile<'a>,
        module: Arc<ModuleNode>,
        imports: NormalizedImports<ExternalImports>,
    ) -> Self {
        Self {
            file,
            module,
            imports,
            _state: PhantomData,
        }
    }
}
