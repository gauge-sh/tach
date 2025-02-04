use crate::{
    config::{root_module::RootModuleTreatment, DependencyConfig, ModuleConfig, ProjectConfig},
    diagnostics::{
        CodeDiagnostic, ConfigurationDiagnostic, Diagnostic, DiagnosticDetails, FileChecker,
        Result as DiagnosticResult,
    },
    modules::ModuleTree,
    processors::{file_module::FileModuleInternal, import::NormalizedImport},
};
use std::path::Path;

#[derive(Debug)]
enum LayerCheckResult {
    Ok,
    SameLayer,
    LayerNotSpecified,
    LayerViolation(Diagnostic),
    UnknownLayer(Diagnostic),
}

pub struct InternalDependencyChecker<'a> {
    project_config: &'a ProjectConfig,
    module_tree: &'a ModuleTree,
}

impl<'a> InternalDependencyChecker<'a> {
    pub fn new(project_config: &'a ProjectConfig, module_tree: &'a ModuleTree) -> Self {
        Self {
            project_config,
            module_tree,
        }
    }

    fn check_layers(
        &self,
        import: &NormalizedImport,
        layers: &[String],
        source_module_config: &ModuleConfig,
        target_module_config: &ModuleConfig,
        relative_file_path: &Path,
    ) -> LayerCheckResult {
        match (&source_module_config.layer, &target_module_config.layer) {
            (Some(source_layer), Some(target_layer)) => {
                let source_index = layers.iter().position(|layer| layer == source_layer);
                let target_index = layers.iter().position(|layer| layer == target_layer);

                match (source_index, target_index) {
                    (Some(source_index), Some(target_index)) => {
                        if source_index == target_index {
                            LayerCheckResult::SameLayer
                        } else if source_index < target_index {
                            LayerCheckResult::Ok
                        } else {
                            LayerCheckResult::LayerViolation(Diagnostic::new_located_error(
                                relative_file_path.to_path_buf(),
                                import.line_no,
                                DiagnosticDetails::Code(CodeDiagnostic::LayerViolation {
                                    import_mod_path: import.module_path.to_string(),
                                    usage_module: source_module_config.path.clone(),
                                    usage_layer: source_layer.clone(),
                                    definition_module: target_module_config.path.clone(),
                                    definition_layer: target_layer.clone(),
                                }),
                            ))
                        }
                    }
                    // If either index is not found, the layer is unknown
                    (Some(_), None) => LayerCheckResult::UnknownLayer(
                        Diagnostic::new_global_error(DiagnosticDetails::Configuration(
                            ConfigurationDiagnostic::UnknownLayer {
                                layer: target_layer.clone(),
                            },
                        )),
                    ),
                    (None, Some(_)) => LayerCheckResult::UnknownLayer(
                        Diagnostic::new_global_error(DiagnosticDetails::Configuration(
                            ConfigurationDiagnostic::UnknownLayer {
                                layer: source_layer.clone(),
                            },
                        )),
                    ),
                    _ => LayerCheckResult::UnknownLayer(Diagnostic::new_global_error(
                        DiagnosticDetails::Configuration(ConfigurationDiagnostic::UnknownLayer {
                            layer: source_layer.clone(),
                        }),
                    )),
                }
            }
            _ => LayerCheckResult::LayerNotSpecified, // At least one module does not have a layer
        }
    }

    fn check_dependencies(
        &self,
        relative_file_path: &Path,
        import: &NormalizedImport,
        file_module_config: &ModuleConfig,
        import_module_config: &ModuleConfig,
        layers: &[String],
    ) -> DiagnosticResult<Vec<Diagnostic>> {
        if import_module_config == file_module_config {
            return Ok(vec![]);
        }

        // Layer check should take precedence over other dependency checks
        match self.check_layers(
            import,
            layers,
            file_module_config,
            import_module_config,
            relative_file_path,
        ) {
            LayerCheckResult::Ok => return Ok(vec![]), // Higher layers can unconditionally import lower layers
            LayerCheckResult::LayerViolation(e) | LayerCheckResult::UnknownLayer(e) => {
                return Ok(vec![e]);
            }
            LayerCheckResult::SameLayer | LayerCheckResult::LayerNotSpecified => (), // We need to do further processing to determine if the dependency is allowed
        };

        if file_module_config.depends_on.is_none() {
            return Ok(vec![]);
        }

        if import_module_config.utility {
            return Ok(vec![]);
        }

        let file_nearest_module_path = &file_module_config.path;
        let import_nearest_module_path = &import_module_config.path;

        match file_module_config
            .dependencies_iter()
            .find(|dep| &dep.path == import_nearest_module_path)
        {
            Some(DependencyConfig {
                deprecated: true, ..
            }) => Ok(vec![Diagnostic::new_located_warning(
                relative_file_path.to_path_buf(),
                import.line_no,
                DiagnosticDetails::Code(CodeDiagnostic::DeprecatedImport {
                    import_mod_path: import.module_path.to_string(),
                    usage_module: file_nearest_module_path.to_string(),
                    definition_module: import_nearest_module_path.to_string(),
                }),
            )]),
            Some(_) => Ok(vec![]),
            None => Ok(vec![Diagnostic::new_located_error(
                relative_file_path.to_path_buf(),
                import.line_no,
                DiagnosticDetails::Code(CodeDiagnostic::InvalidImport {
                    import_mod_path: import.module_path.to_string(),
                    usage_module: file_nearest_module_path.to_string(),
                    definition_module: import_nearest_module_path.to_string(),
                }),
            )]),
        }
    }

    fn check_import(
        &self,
        import: &NormalizedImport,
        internal_file: &FileModuleInternal,
    ) -> DiagnosticResult<Vec<Diagnostic>> {
        if let Some(import_module_config) = self
            .module_tree
            .find_nearest(&import.module_path)
            .as_ref()
            .and_then(|module| module.config.as_ref())
        {
            if import_module_config.is_root()
                && self.project_config.root_module == RootModuleTreatment::Ignore
            {
                return Ok(vec![]);
            }

            self.check_dependencies(
                internal_file.relative_file_path(),
                import,
                internal_file.module_config(),
                import_module_config,
                &self.project_config.layers,
            )
        } else {
            Ok(vec![Diagnostic::new_global_error(
                DiagnosticDetails::Configuration(ConfigurationDiagnostic::ModuleConfigNotFound {
                    module_path: import.module_path.to_string(),
                }),
            )])
        }
    }
}

impl<'a> FileChecker<'a> for InternalDependencyChecker<'a> {
    type ProcessedFile = FileModuleInternal<'a>;
    type Output = Vec<Diagnostic>;

    fn check(&'a self, processed_file: &Self::ProcessedFile) -> DiagnosticResult<Self::Output> {
        let mut diagnostics = Vec::new();
        for import in processed_file.imports.all_imports() {
            diagnostics.extend(self.check_import(import, processed_file)?);
        }

        Ok(diagnostics)
    }
}
