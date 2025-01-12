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
    check_layers(layers, file_module_config, import_module_config)?;

    if import_module_config.utility {
        return Ok(());
    }
    let file_nearest_module_path = &file_module_config.path;
    let import_nearest_module_path = &import_module_config.path;

    match file_module_config
        .depends_on
        .iter()
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
        Ok(()) => Err(ImportCheckError::UnusedIgnoreDirective {
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

pub(super) fn check_layers(
    layers: &[String],
    source_module_config: &ModuleConfig,
    target_module_config: &ModuleConfig,
) -> Result<(), ImportCheckError> {
    match (&source_module_config.layer, &target_module_config.layer) {
        (Some(source_layer), Some(target_layer)) => {
            let source_index = layers.iter().position(|layer| layer == source_layer);
            let target_index = layers.iter().position(|layer| layer == target_layer);

            match (source_index, target_index) {
                (Some(source_index), Some(target_index)) => {
                    if source_index <= target_index {
                        Ok(())
                    } else {
                        Err(ImportCheckError::LayerViolation {
                            import_mod_path: target_module_config.path.clone(),
                            source_module: source_module_config.path.clone(),
                            source_layer: source_layer.clone(),
                            invalid_module: target_module_config.path.clone(),
                            invalid_layer: target_layer.clone(),
                        })
                    }
                }
                // If either index is not found, the layer is unknown -- ignore for now
                _ => Ok(()),
            }
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{InterfaceConfig, ModuleConfig};
    use crate::modules::ModuleTree;
    use crate::tests::check_internal::fixtures::{
        interface_config, layers, module_config, module_tree,
    };
    use std::path::PathBuf;

    use rstest::rstest;

    #[rstest]
    #[case("domain_one", "domain_one", true)]
    #[case("domain_one", "domain_one.core", true)]
    #[case("domain_one", "domain_three", true)]
    #[case("domain_two", "domain_one", true)]
    #[case("domain_two", "domain_one.public_fn", true)]
    #[case("domain_two.subdomain", "domain_one", true)]
    #[case("domain_two", "domain_one.private_fn", false)]
    #[case("domain_three", "domain_one", false)]
    #[case("domain_two", "domain_one.core", false)]
    #[case("domain_two.subdomain", "domain_one.core", false)]
    #[case("domain_two", "domain_three", false)]
    #[case("domain_two", "domain_two.subdomain", false)]
    fn test_check_import(
        module_tree: ModuleTree,
        module_config: Vec<ModuleConfig>,
        interface_config: Vec<InterfaceConfig>,
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
            &[],
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
            &[],
            RootModuleTreatment::Allow,
            &interface_checker,
            true,
        );
        assert!(check_error.is_err());
        assert!(check_error.unwrap_err().is_deprecated());
    }

    #[rstest]
    #[case("top", "top", true)]
    #[case("top", "middle", true)]
    #[case("top", "bottom", true)]
    #[case("middle", "bottom", true)]
    #[case("bottom", "top", false)]
    #[case("middle", "top", false)]
    #[case("bottom", "middle", false)]
    fn test_check_layers_hierarchy(
        layers: Vec<String>,
        #[case] source_layer: &str,
        #[case] target_layer: &str,
        #[case] expected_result: bool,
    ) {
        let source_config = ModuleConfig::new_with_layer("source", source_layer);
        let target_config = ModuleConfig::new_with_layer("target", target_layer);

        let result = check_layers(&layers, &source_config, &target_config);
        assert_eq!(result.is_ok(), expected_result);
    }

    #[rstest]
    fn test_check_layers_missing_layers() {
        let layers: Vec<String> = vec![];
        let source_config = ModuleConfig::new_with_layer("source", "any");
        let target_config = ModuleConfig::new_with_layer("target", "any");

        assert!(check_layers(&layers, &source_config, &target_config).is_ok());
    }

    #[rstest]
    fn test_check_layers_no_layer_specified() {
        let layers = vec!["top".to_string(), "bottom".to_string()];
        let source_config = ModuleConfig::default();
        let target_config = ModuleConfig::default();

        // When modules don't specify layers, they should be allowed
        assert!(check_layers(&layers, &source_config, &target_config).is_ok());
    }

    #[rstest]
    fn test_layer_violation_in_check_import(module_tree: ModuleTree, layers: Vec<String>) {
        let file_module = module_tree.find_nearest("domain_three").unwrap(); // bottom layer

        let result = check_import(
            "domain_one", // trying to import from top layer
            &module_tree,
            file_module,
            &layers,
            RootModuleTreatment::Allow,
            &None,
            true,
        );

        assert!(matches!(
            result,
            Err(ImportCheckError::LayerViolation {
                source_layer,
                invalid_layer,
                ..
            }) if source_layer == "bottom" && invalid_layer == "top"
        ));
    }
}
