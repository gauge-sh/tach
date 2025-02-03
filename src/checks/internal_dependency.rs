use crate::{
    config::{root_module::RootModuleTreatment, DependencyConfig, ModuleConfig, ProjectConfig},
    diagnostics::{
        CodeDiagnostic, ConfigurationDiagnostic, Diagnostic, DiagnosticDetails, FileChecker,
        Result as DiagnosticResult,
    },
    modules::ModuleTree,
    processors::{file_module::FileModuleInternal, imports::NormalizedImport},
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{InterfaceConfig, ModuleConfig};
    use crate::diagnostics::Diagnostic;
    use crate::modules::ModuleTree;
    use crate::tests::check_internal::fixtures::{
        interface_config, layers, module_config, module_tree,
    };
    use std::path::PathBuf;

    use rstest::rstest;

    #[rstest]
    #[case("domain_one", "domain_two", true)] // same layer, explicit dependency
    #[case("domain_one", "domain_one.subdomain", false)] // same layer, parent->child (deprecated)
    #[case("domain_one.subdomain", "domain_one", false)] // same layer, child->parent not allowed
    #[case("domain_one", "service_one", true)] // top->middle allowed
    #[case("domain_one", "data_one", true)] // top->bottom allowed
    #[case("service_one", "service_one.internal", true)] // same layer, explicit dependency
    #[case("service_one", "data_one", true)] // middle->bottom allowed
    #[case("service_one", "domain_one", false)] // middle->top denied
    #[case("data_one", "service_one", false)] // bottom->middle denied
    #[case("data_one", "domain_one", false)] // bottom->top denied
    fn test_check_dependencies(
        module_tree: ModuleTree,
        module_config: Vec<ModuleConfig>,
        layers: Vec<String>,
        #[case] file_mod_path: &str,
        #[case] import_mod_path: &str,
        #[case] expected_result: bool,
    ) {
        let file_module = module_tree.find_nearest(file_mod_path).unwrap();

        todo!();
        let result = check_error.is_ok();
        assert_eq!(result, expected_result);
    }

    #[rstest]
    fn test_check_deprecated_import(
        module_tree: ModuleTree,
        module_config: Vec<ModuleConfig>,
        interface_config: Vec<InterfaceConfig>,
        layers: Vec<String>,
    ) {
        let file_module = module_tree.find_nearest("domain_one").unwrap();
        let interface_checker = Some(
            InterfaceChecker::new(&interface_config)
                .with_type_check_cache(&module_config, &[PathBuf::from(".")])
                .unwrap(),
        );

        let check_error = check_import_internal(
            "domain_one.subdomain",
            &module_tree,
            file_module.clone(),
            &layers,
            RootModuleTreatment::Allow,
            &interface_checker,
            true,
        );
        assert!(check_error.is_err());
        assert!(check_error
            .unwrap_err()
            .iter()
            .any(|err| err.is_deprecated()));
    }

    #[rstest]
    #[case("top", "top", LayerCheckResult::SameLayer)]
    #[case("top", "middle", LayerCheckResult::Ok)]
    #[case("top", "bottom", LayerCheckResult::Ok)]
    #[case("middle", "bottom", LayerCheckResult::Ok)]
    #[case("bottom", "top", LayerCheckResult::LayerViolation(Diagnostic::new_global_error(
        DiagnosticDetails::Code(CodeDiagnostic::LayerViolation {
            import_mod_path: "".to_string(),
            usage_module: "".to_string(),
            usage_layer: "bottom".to_string(),
            definition_module: "".to_string(),
            definition_layer: "top".to_string(),
        }),
    )))]
    #[case("middle", "top", LayerCheckResult::LayerViolation(Diagnostic::new_global_error(
        DiagnosticDetails::Code(CodeDiagnostic::LayerViolation {
            import_mod_path: "".to_string(),
            usage_module: "".to_string(),
            usage_layer: "middle".to_string(),
            definition_module: "".to_string(),
            definition_layer: "top".to_string(),
        }),
    )))]
    #[case("bottom", "middle", LayerCheckResult::LayerViolation(Diagnostic::new_global_error(
        DiagnosticDetails::Code(CodeDiagnostic::LayerViolation {
            import_mod_path: "".to_string(),
            usage_module: "".to_string(),
            usage_layer: "bottom".to_string(),
            definition_module: "".to_string(),
            definition_layer: "middle".to_string(),
        }),
    )))]
    fn test_check_layers_hierarchy(
        layers: Vec<String>,
        #[case] source_layer: &str,
        #[case] target_layer: &str,
        #[case] expected_pattern: LayerCheckResult,
    ) {
        let source_config = ModuleConfig::new_with_layer("source", source_layer);
        let target_config = ModuleConfig::new_with_layer("target", target_layer);

        let result = check_layers(&layers, &source_config, &target_config);
        match (result, expected_pattern) {
            (LayerCheckResult::Ok, LayerCheckResult::Ok) => (),
            (LayerCheckResult::SameLayer, LayerCheckResult::SameLayer) => (),
            (LayerCheckResult::LayerViolation(_), LayerCheckResult::LayerViolation(_)) => (),
            (actual, expected) => panic!("Expected {:?} but got {:?}", expected, actual),
        }
    }

    #[rstest]
    fn test_check_layers_missing_layers() {
        let layers: Vec<String> = vec![];
        let source_config = ModuleConfig::new_with_layer("source", "any");
        let target_config = ModuleConfig::new_with_layer("target", "any");

        let result = check_layers(&layers, &source_config, &target_config);
        assert!(matches!(result, LayerCheckResult::UnknownLayer(_)));
    }

    #[rstest]
    fn test_check_layers_no_layer_specified() {
        let layers = vec!["top".to_string(), "bottom".to_string()];
        let source_config = ModuleConfig::default();
        let target_config = ModuleConfig::default();

        let result = check_layers(&layers, &source_config, &target_config);
        assert!(matches!(result, LayerCheckResult::LayerNotSpecified));
    }

    #[rstest]
    #[case("unrestricted", "domain_one", false)] // middle->top denied due to layer check
    #[case("unrestricted", "service_one", true)] // same layer allowed
    #[case("unrestricted", "data_one", true)] // middle->bottom allowed
    fn test_check_import_unrestricted_dependencies(
        module_tree: ModuleTree,
        module_config: Vec<ModuleConfig>,
        interface_config: Vec<InterfaceConfig>,
        layers: Vec<String>,
        #[case] file_mod_path: &str,
        #[case] import_mod_path: &str,
        #[case] expected_result: bool,
    ) {
        let file_module = module_tree.find_nearest(file_mod_path).unwrap();
        let interface_checker = Some(
            InterfaceChecker::new(&interface_config)
                .with_type_check_cache(&module_config, &[PathBuf::from(".")])
                .unwrap(),
        );

        let check_error = check_import_internal(
            import_mod_path,
            &module_tree,
            file_module.clone(),
            &layers,
            RootModuleTreatment::Allow,
            &interface_checker,
            true,
        );
        let result = check_error.is_ok();
        assert_eq!(
            result, expected_result,
            "Expected import from '{}' to '{}' to be {}, but got {}",
            file_mod_path, import_mod_path, expected_result, result
        );
    }
}
