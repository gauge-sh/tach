use std::collections::{HashMap, HashSet};

use crate::config::ProjectConfig;
use crate::dependencies::import::{with_distribution_names, ExternalImportWithDistributionNames};
use crate::diagnostics::{CodeDiagnostic, Diagnostic, DiagnosticDetails};
use crate::diagnostics::{FileChecker, Result as DiagnosticResult};
use crate::processors::file_module::FileModule;
use crate::resolvers::PackageResolver;

pub struct ExternalDependencyChecker<'a> {
    package_resolver: &'a PackageResolver<'a>,
    module_mappings: &'a HashMap<String, Vec<String>>,
    stdlib_modules: &'a HashSet<String>,
    excluded_external_modules: &'a HashSet<String>,
    project_config: &'a ProjectConfig,
}

impl<'a> ExternalDependencyChecker<'a> {
    pub fn new(
        project_config: &'a ProjectConfig,
        module_mappings: &'a HashMap<String, Vec<String>>,
        stdlib_modules: &'a HashSet<String>,
        excluded_external_modules: &'a HashSet<String>,
        package_resolver: &'a PackageResolver<'a>,
    ) -> Self {
        Self {
            package_resolver,
            module_mappings,
            stdlib_modules,
            excluded_external_modules,
            project_config,
        }
    }

    fn check_module_external_dependencies(
        &'a self,
        processed_file: &FileModule<'a>,
        import: &ExternalImportWithDistributionNames<'a>,
    ) -> Option<Diagnostic> {
        let module_config = processed_file.module_config();
        if module_config
            .cannot_depend_on_external
            .as_ref()
            .is_some_and(|external_dependencies| {
                external_dependencies
                    .iter()
                    .any(|dependency| import.top_level_module_name() == dependency)
            })
        {
            let diagnostic =
                DiagnosticDetails::Code(CodeDiagnostic::ModuleForbiddenExternalDependency {
                    dependency: import.top_level_module_name().to_string(),
                    usage_module: module_config.path.clone(),
                });

            if !import.is_global_scope() {
                if let Ok(severity) = (&self.project_config.rules.local_imports).try_into() {
                    return Some(Diagnostic::new_located(
                        severity,
                        diagnostic,
                        processed_file.relative_file_path().to_path_buf(),
                        processed_file.line_number(import.alias_offset()),
                        Some(processed_file.line_number(import.import_offset())),
                    ));
                }
                return None;
            }

            Some(Diagnostic::new_located_error(
                processed_file.relative_file_path().to_path_buf(),
                processed_file.line_number(import.alias_offset()),
                Some(processed_file.line_number(import.import_offset())),
                diagnostic,
            ))
        } else if module_config
            .depends_on_external
            .as_ref()
            .is_some_and(|external_dependencies| {
                !external_dependencies
                    .iter()
                    .any(|dependency| import.top_level_module_name() == dependency)
            })
        {
            let diagnostic =
                DiagnosticDetails::Code(CodeDiagnostic::ModuleUndeclaredExternalDependency {
                    dependency: import.top_level_module_name().to_string(),
                    usage_module: module_config.path.clone(),
                });

            if !import.is_global_scope() {
                if let Ok(severity) = (&self.project_config.rules.local_imports).try_into() {
                    return Some(Diagnostic::new_located(
                        severity,
                        diagnostic,
                        processed_file.relative_file_path().to_path_buf(),
                        processed_file.line_number(import.alias_offset()),
                        Some(processed_file.line_number(import.import_offset())),
                    ));
                }
                return None;
            }

            Some(Diagnostic::new_located_error(
                processed_file.relative_file_path().to_path_buf(),
                processed_file.line_number(import.alias_offset()),
                Some(processed_file.line_number(import.import_offset())),
                diagnostic,
            ))
        } else {
            None
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
                .contains(&import.top_level_module_name().to_string())
        {
            return None;
        }

        let is_declared = import
            .distribution_names
            .iter()
            .any(|dist_name| processed_file.declared_dependencies().contains(dist_name));

        if !is_declared {
            let diagnostic =
                DiagnosticDetails::Code(CodeDiagnostic::UndeclaredExternalDependency {
                    dependency: import.top_level_module_name().to_string(),
                    package_name: processed_file
                        .package
                        .name
                        .as_ref()
                        .map_or(processed_file.package.root.display().to_string(), |name| {
                            name.to_string()
                        }),
                });

            if !import.is_global_scope() {
                if let Ok(severity) = (&self.project_config.rules.local_imports).try_into() {
                    return Some(Diagnostic::new_located(
                        severity,
                        diagnostic,
                        processed_file.relative_file_path().to_path_buf(),
                        processed_file.line_number(import.alias_offset()),
                        Some(processed_file.line_number(import.import_offset())),
                    ));
                }
                return None;
            }

            Some(Diagnostic::new_located_error(
                processed_file.relative_file_path().to_path_buf(),
                processed_file.line_number(import.alias_offset()),
                Some(processed_file.line_number(import.import_offset())),
                diagnostic,
            ))
        } else {
            self.check_module_external_dependencies(processed_file, &import)
        }
    }
}

impl<'a> FileChecker<'a> for ExternalDependencyChecker<'a> {
    type ProcessedFile = FileModule<'a>;
    type Output = Vec<Diagnostic>;

    fn check(&'a self, processed_file: &Self::ProcessedFile) -> DiagnosticResult<Self::Output> {
        let mut diagnostics = Vec::new();
        for import in with_distribution_names(
            processed_file.imports(),
            self.package_resolver,
            self.module_mappings,
        ) {
            if let Some(diagnostic) = self.check_import(import, processed_file) {
                diagnostics.push(diagnostic);
            }
        }

        Ok(diagnostics)
    }
}
