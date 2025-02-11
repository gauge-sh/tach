use std::collections::{HashMap, HashSet};

use crate::dependencies::import::{with_distribution_names, ExternalImportWithDistributionNames};
use crate::diagnostics::{CodeDiagnostic, Diagnostic, DiagnosticDetails};
use crate::diagnostics::{FileChecker, Result as DiagnosticResult};
use crate::external::parsing::ProjectInfo;
use crate::processors::file_module::FileModule;

pub struct ExternalDependencyChecker<'a> {
    project_info: &'a ProjectInfo,
    module_mappings: &'a HashMap<String, Vec<String>>,
    stdlib_modules: &'a HashSet<String>,
    excluded_external_modules: &'a HashSet<String>,
}

impl<'a> ExternalDependencyChecker<'a> {
    pub fn new(
        project_info: &'a ProjectInfo,
        module_mappings: &'a HashMap<String, Vec<String>>,
        stdlib_modules: &'a HashSet<String>,
        excluded_external_modules: &'a HashSet<String>,
    ) -> Self {
        Self {
            project_info,
            module_mappings,
            stdlib_modules,
            excluded_external_modules,
        }
    }

    fn check_import(
        &'a self,
        import: ExternalImportWithDistributionNames<'a>,
        processed_file: &FileModule<'a>,
    ) -> Option<Diagnostic> {
        if import
            .distribution_names
            .iter()
            .any(|dist_name| self.excluded_external_modules.contains(dist_name))
            || self
                .stdlib_modules
                .contains(&import.import.top_level_module_name().to_string())
        {
            return None;
        }

        let is_declared = import
            .distribution_names
            .iter()
            .any(|dist_name| self.project_info.dependencies.contains(dist_name));

        if !is_declared {
            Some(Diagnostic::new_located_error(
                processed_file.relative_file_path().to_path_buf(),
                processed_file.line_number(import.import.alias_offset),
                Some(processed_file.line_number(import.import.import_offset)),
                DiagnosticDetails::Code(CodeDiagnostic::UndeclaredExternalDependency {
                    dependency: import.import.top_level_module_name().to_string(),
                }),
            ))
        } else {
            None
        }
    }
}

impl<'a> FileChecker<'a> for ExternalDependencyChecker<'a> {
    type ProcessedFile = FileModule<'a>;
    type Output = Vec<Diagnostic>;

    fn check(&'a self, processed_file: &Self::ProcessedFile) -> DiagnosticResult<Self::Output> {
        let mut diagnostics = Vec::new();
        for import in with_distribution_names(processed_file.imports(), self.module_mappings) {
            if let Some(diagnostic) = self.check_import(import, processed_file) {
                diagnostics.push(diagnostic);
            }
        }

        Ok(diagnostics)
    }
}
