use std::collections::{HashMap, HashSet};

use crate::config::ProjectConfig;
use crate::diagnostics::{CodeDiagnostic, Diagnostic, DiagnosticDetails};
use crate::diagnostics::{FileChecker, Result as DiagnosticResult};
use crate::external::parsing::ProjectInfo;
use crate::processors::file_module::FileModuleExternal;
use crate::processors::imports::ExternalImportWithDistributionNames;

pub struct ExternalDependencyChecker<'a> {
    project_config: &'a ProjectConfig,
    project_info: &'a ProjectInfo,
    module_mappings: &'a HashMap<String, Vec<String>>,
    stdlib_modules: &'a HashSet<String>,
    excluded_external_modules: &'a HashSet<String>,
}

impl<'a> ExternalDependencyChecker<'a> {
    pub fn new(
        project_config: &'a ProjectConfig,
        project_info: &'a ProjectInfo,
        module_mappings: &'a HashMap<String, Vec<String>>,
        stdlib_modules: &'a HashSet<String>,
        excluded_external_modules: &'a HashSet<String>,
    ) -> Self {
        Self {
            project_config,
            project_info,
            module_mappings,
            stdlib_modules,
            excluded_external_modules,
        }
    }

    fn check_import(
        &'a self,
        import: ExternalImportWithDistributionNames<'a>,
        processed_file: &FileModuleExternal<'a>,
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
                import.import.import_line_no,
                DiagnosticDetails::Code(CodeDiagnostic::UndeclaredExternalDependency {
                    import_mod_path: import.import.top_level_module_name().to_string(),
                }),
            ))
        } else {
            None
        }
    }
}

impl<'a> FileChecker<'a> for ExternalDependencyChecker<'a> {
    type ProcessedFile = FileModuleExternal<'a>;
    type Output = Vec<Diagnostic>;

    fn check(&'a self, processed_file: &Self::ProcessedFile) -> DiagnosticResult<Self::Output> {
        let mut diagnostics = Vec::new();
        for import in processed_file
            .imports
            .all_imports_with_distribution_names(self.module_mappings)
        {
            if let Some(diagnostic) = self.check_import(import, processed_file) {
                diagnostics.push(diagnostic);
            }
        }

        Ok(diagnostics)
    }
}
