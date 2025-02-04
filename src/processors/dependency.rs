use std::path::PathBuf;
use std::sync::Arc;

use crate::config::root_module::RootModuleTreatment;
use crate::config::ProjectConfig;
use crate::diagnostics::{FileProcessor, Result as DiagnosticResult};
use crate::filesystem::{self, ProjectFile};
use crate::modules::error::ModuleTreeError;
use crate::modules::{ModuleNode, ModuleTree};

use super::file_module::FileModule;
use super::import::NormalizedImport;
use super::reference::SourceCodeReference;

#[derive(Debug)]
pub enum Dependency<'a> {
    Import(NormalizedImport),
    Reference(SourceCodeReference<'a>),
}

impl From<NormalizedImport> for Dependency<'_> {
    fn from(normalized_import: NormalizedImport) -> Self {
        Dependency::Import(normalized_import)
    }
}

impl<'a> From<SourceCodeReference<'a>> for Dependency<'a> {
    fn from(source_code_reference: SourceCodeReference<'a>) -> Self {
        Dependency::Reference(source_code_reference)
    }
}

#[derive(Debug)]
pub struct InternalDependencyExtractor<'a> {
    module_tree: &'a ModuleTree,
    source_roots: &'a [PathBuf],
    project_config: &'a ProjectConfig,
}

impl<'a> InternalDependencyExtractor<'a> {
    pub fn new(
        source_roots: &'a [PathBuf],
        module_tree: &'a ModuleTree,
        project_config: &'a ProjectConfig,
    ) -> Self {
        Self {
            source_roots,
            module_tree,
            project_config,
        }
    }
}

impl<'a> FileProcessor<'a, ProjectFile<'a>> for InternalDependencyExtractor<'a> {
    type ProcessedFile = FileModule<'a>;

    fn process(&self, file_path: ProjectFile<'a>) -> DiagnosticResult<Self::ProcessedFile> {
        let mod_path = filesystem::file_to_module_path(self.source_roots, file_path.as_ref())?;
        let module = self
            .module_tree
            .find_nearest(mod_path.as_ref())
            .ok_or(ModuleTreeError::ModuleNotFound(mod_path))?;

        if module.is_unchecked() {
            return Ok(FileModule::new(file_path, module));
        }

        if module.is_root() && self.project_config.root_module == RootModuleTreatment::Ignore {
            return Ok(FileModule::new(file_path, module));
        }

        // let project_imports = get_project_imports(
        //     self.source_roots,
        //     file_path.as_ref(),
        //     self.project_config.ignore_type_checking_imports,
        //     self.project_config.include_string_imports,
        // )?;
        Ok(FileModule::new(file_path, module))
    }
}

#[derive(Debug)]
pub struct ExternalDependencyExtractor<'a> {
    source_roots: &'a [PathBuf],
    project_config: &'a ProjectConfig,
}

impl<'a> ExternalDependencyExtractor<'a> {
    pub fn new(source_roots: &'a [PathBuf], project_config: &'a ProjectConfig) -> Self {
        Self {
            source_roots,
            project_config,
        }
    }
}

impl<'a> FileProcessor<'a, ProjectFile<'a>> for ExternalDependencyExtractor<'a> {
    type ProcessedFile = FileModule<'a>;

    fn process(&self, file_path: ProjectFile<'a>) -> DiagnosticResult<Self::ProcessedFile> {
        let module = Arc::new(ModuleNode::empty());
        // let external_imports = get_external_imports(
        //     self.source_roots,
        //     file_path.as_ref(),
        //     self.project_config.ignore_type_checking_imports,
        // )?;
        Ok(FileModule::new(file_path, module))
    }
}
