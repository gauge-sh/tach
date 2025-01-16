use std::sync::Arc;

use super::diagnostics::ImportCheckError;
use crate::{
    config::{root_module::RootModuleTreatment, DependencyConfig, ModuleConfig, ProjectConfig},
    imports::DirectiveIgnoredImport,
    interfaces::{
        check::CheckResult as InterfaceCheckResult, data_types::TypeCheckResult, InterfaceChecker,
    },
    modules::{ModuleNode, ModuleTree},
};

fn check_dependencies(
    import_mod_path: &str,
    file_module_config: &ModuleConfig,
    import_module_config: &ModuleConfig,
    layers: &[String],
) -> Result<(), ImportCheckError> {
    // Layer check should take precedence over other dependency checks
    match check_layers(layers, file_module_config, import_module_config) {
        LayerCheckResult::Ok => return Ok(()), // Higher layers can unconditionally import lower layers
        LayerCheckResult::SameLayer | LayerCheckResult::LayerNotSpecified => (), // We need to do further processing to determine if the dependency is allowed
        LayerCheckResult::LayerViolation(e) | LayerCheckResult::UnknownLayer(e) => return Err(e),
    };

    if file_module_config.depends_on.is_none() {
        return Ok(());
    }

    if import_module_config.utility {
        return Ok(());
    }

    let file_nearest_module_path = &file_module_config.path;
    let import_nearest_module_path = &import_module_config.path;

    match file_module_config
        .dependencies_iter()
        .find(|dep| &dep.path == import_nearest_module_path)
    {
        Some(DependencyConfig {
            deprecated: true, ..
        }) => Err(ImportCheckError::DeprecatedImport {
            import_mod_path: import_mod_path.to_string(),
            source_module: file_nearest_module_path.to_string(),
            invalid_module: import_nearest_module_path.to_string(),
        }),
        Some(_) => Ok(()),
        None => Err(ImportCheckError::InvalidImport {
            import_mod_path: import_mod_path.to_string(),
            source_module: file_nearest_module_path.to_string(),
            invalid_module: import_nearest_module_path.to_string(),
        }),
    }
}

fn check_interfaces(
    import_mod_path: &str,
    import_nearest_module: &ModuleNode,
    file_nearest_module: &ModuleNode,
    interface_checker: &InterfaceChecker,
) -> Result<(), ImportCheckError> {
    let import_member = import_mod_path
        .strip_prefix(&import_nearest_module.full_path)
        .and_then(|s| s.strip_prefix('.'))
        .unwrap_or("");
    let check_result =
        interface_checker.check_member(import_member, &import_nearest_module.full_path);
    match check_result {
        InterfaceCheckResult::NotExposed => Err(ImportCheckError::PrivateImport {
            import_mod_path: import_mod_path.to_string(),
            import_nearest_module_path: import_nearest_module.full_path.to_string(),
            file_nearest_module_path: file_nearest_module.full_path.to_string(),
        }),
        InterfaceCheckResult::Exposed {
            type_check_result: TypeCheckResult::DidNotMatchInterface { expected },
        } => Err(ImportCheckError::InvalidDataTypeExport {
            import_mod_path: import_mod_path.to_string(),
            import_nearest_module_path: import_nearest_module.full_path.to_string(),
            expected_data_type: expected.to_string(),
        }),
        _ => Ok(()),
    }
}

pub(super) fn check_import(
    import_mod_path: &str,
    module_tree: &ModuleTree,
    file_nearest_module: Arc<ModuleNode>,
    layers: &[String],
    root_module_treatment: RootModuleTreatment,
    interface_checker: &Option<InterfaceChecker>,
    should_check_dependencies: bool,
) -> Result<(), ImportCheckError> {
    if !should_check_dependencies && interface_checker.is_none() {
        return Err(ImportCheckError::NoChecksEnabled());
    }

    let import_nearest_module = match module_tree.find_nearest(import_mod_path) {
        Some(module) => module,
        // This should not be none since we intend to filter out any external imports,
        // but we should allow external imports if they have made it here.
        None => return Ok(()),
    };

    if import_nearest_module.is_root() && root_module_treatment == RootModuleTreatment::Ignore {
        return Ok(());
    }

    if import_nearest_module == file_nearest_module {
        // Imports within the same module are always allowed
        return Ok(());
    }

    let file_module_config = file_nearest_module
        .config
        .as_ref()
        .ok_or(ImportCheckError::ModuleConfigNotFound())?;
    let import_module_config = import_nearest_module
        .config
        .as_ref()
        .ok_or(ImportCheckError::ModuleConfigNotFound())?;

    if let Some(interface_checker) = interface_checker {
        check_interfaces(
            import_mod_path,
            &import_nearest_module,
            &file_nearest_module,
            interface_checker,
        )?
    }

    if should_check_dependencies {
        check_dependencies(
            import_mod_path,
            file_module_config,
            import_module_config,
            layers,
        )?
    }

    Ok(())
}

pub(super) fn check_unused_ignore_directive(
    directive_ignored_import: &DirectiveIgnoredImport,
    module_tree: &ModuleTree,
    nearest_module: Arc<ModuleNode>,
    project_config: &ProjectConfig,
    interface_checker: &Option<InterfaceChecker>,
    check_dependencies: bool,
) -> Result<(), ImportCheckError> {
    match check_import(
        &directive_ignored_import.import.module_path,
        module_tree,
        Arc::clone(&nearest_module),
        &project_config.layers,
        project_config.root_module.clone(),
        interface_checker,
        check_dependencies,
    ) {
        Ok(()) => Err(ImportCheckError::UnnecessarilyIgnoredImport {
            import_mod_path: directive_ignored_import.import.module_path.to_string(),
        }),
        Err(_) => Ok(()),
    }
}

pub(super) fn check_missing_ignore_directive_reason(
    directive_ignored_import: &DirectiveIgnoredImport,
) -> Result<(), ImportCheckError> {
    if directive_ignored_import.reason.is_empty() {
        Err(ImportCheckError::MissingIgnoreDirectiveReason {
            import_mod_path: directive_ignored_import.import.module_path.to_string(),
        })
    } else {
        Ok(())
    }
}

#[derive(Debug)]
enum LayerCheckResult {
    Ok,
    SameLayer,
    LayerNotSpecified,
    LayerViolation(ImportCheckError),
    UnknownLayer(ImportCheckError),
}

fn check_layers(
    layers: &[String],
    source_module_config: &ModuleConfig,
    target_module_config: &ModuleConfig,
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
                        LayerCheckResult::LayerViolation(ImportCheckError::LayerViolation {
                            import_mod_path: target_module_config.path.clone(),
                            source_module: source_module_config.path.clone(),
                            source_layer: source_layer.clone(),
                            invalid_module: target_module_config.path.clone(),
                            invalid_layer: target_layer.clone(),
                        })
                    }
                }
                // If either index is not found, the layer is unknown
                (Some(_), None) => LayerCheckResult::UnknownLayer(ImportCheckError::UnknownLayer {
                    layer: target_layer.clone(),
                }),
                (None, Some(_)) => LayerCheckResult::UnknownLayer(ImportCheckError::UnknownLayer {
                    layer: source_layer.clone(),
                }),
                _ => LayerCheckResult::UnknownLayer(ImportCheckError::UnknownLayer {
                    layer: source_layer.clone(),
                }),
            }
        }
        _ => LayerCheckResult::LayerNotSpecified, // At least one module does not have a layer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::check_internal::diagnostics::ImportCheckError;
    use crate::config::{InterfaceConfig, ModuleConfig};
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
    fn test_check_import(
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

        let check_error = check_import(
            import_mod_path,
            &module_tree,
            file_module.clone(),
            &layers,
            RootModuleTreatment::Allow,
            &interface_checker,
            true,
        );
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

        let check_error = check_import(
            "domain_one.subdomain",
            &module_tree,
            file_module.clone(),
            &layers,
            RootModuleTreatment::Allow,
            &interface_checker,
            true,
        );
        assert!(check_error.is_err());
        assert!(check_error.unwrap_err().is_deprecated());
    }

    #[rstest]
    #[case("top", "top", LayerCheckResult::SameLayer)]
    #[case("top", "middle", LayerCheckResult::Ok)]
    #[case("top", "bottom", LayerCheckResult::Ok)]
    #[case("middle", "bottom", LayerCheckResult::Ok)]
    #[case("bottom", "top", LayerCheckResult::LayerViolation(ImportCheckError::LayerViolation {
        source_layer: "bottom".to_string(),
        invalid_layer: "top".to_string(),
        import_mod_path: "".to_string(),
        source_module: "".to_string(),
        invalid_module: "".to_string(),
    }))]
    #[case("middle", "top", LayerCheckResult::LayerViolation(ImportCheckError::LayerViolation {
        source_layer: "middle".to_string(),
        invalid_layer: "top".to_string(),
        import_mod_path: "".to_string(),
        source_module: "".to_string(),
        invalid_module: "".to_string(),
    }))]
    #[case("bottom", "middle", LayerCheckResult::LayerViolation(ImportCheckError::LayerViolation {
        source_layer: "bottom".to_string(),
        invalid_layer: "middle".to_string(),
        import_mod_path: "".to_string(),
        source_module: "".to_string(),
        invalid_module: "".to_string(),
    }))]
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

        let check_error = check_import(
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
