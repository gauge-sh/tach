use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};

use pyo3::exceptions::PyValueError;
use pyo3::{pyclass, pymethods, PyResult};
use rayon::prelude::*;
use serde::Serialize;
use thiserror::Error;

use crate::{
    config::{root_module::RootModuleTreatment, ModuleConfig, ProjectConfig, RuleSetting},
    exclusion::{self, set_excluded_paths},
    filesystem as fs,
    imports::{get_project_imports, ImportParseError},
    interfaces::{
        check::CheckResult as InterfaceCheckResult, data_types::TypeCheckResult,
        error::InterfaceError, InterfaceChecker,
    },
    interrupt::check_interrupt,
    modules::{self, build_module_tree, ModuleNode, ModuleTree},
};

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("The path {0} is not a valid directory.")]
    InvalidDirectory(String),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] fs::FileSystemError),
    #[error("Module tree error: {0}")]
    ModuleTree(#[from] modules::error::ModuleTreeError),
    #[error("Exclusion error: {0}")]
    Exclusion(#[from] exclusion::PathExclusionError),
    #[error("Interface error: {0}")]
    Interface(#[from] InterfaceError),
    #[error("Operation cancelled by user")]
    Interrupt,
}

#[derive(Debug, Clone, Serialize)]
#[pyclass(get_all, module = "tach.extension")]
pub struct BoundaryError {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub import_mod_path: String,
    pub error_info: ImportCheckError,
}

impl BoundaryError {
    pub fn is_deprecated(&self) -> bool {
        self.error_info.is_deprecated()
    }
}

#[derive(Debug, Default, Serialize)]
#[pyclass(get_all, module = "tach.extension")]
pub struct CheckDiagnostics {
    pub errors: Vec<BoundaryError>,
    pub deprecated_warnings: Vec<BoundaryError>,
    pub warnings: Vec<String>,
}

#[pymethods]
impl CheckDiagnostics {
    #[pyo3(signature = (pretty_print = false))]
    fn serialize_json(&self, pretty_print: bool) -> PyResult<String> {
        if pretty_print {
            serde_json::to_string_pretty(&self)
                .map_err(|_| PyValueError::new_err("Failed to serialize check results."))
        } else {
            serde_json::to_string(&self)
                .map_err(|_| PyValueError::new_err("Failed to serialize check results."))
        }
    }
}

impl ParallelExtend<CheckDiagnostics> for CheckDiagnostics {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = CheckDiagnostics>,
    {
        // Reduce all diagnostics into a single one in parallel
        let combined =
            par_iter
                .into_par_iter()
                .reduce(CheckDiagnostics::default, |mut acc, item| {
                    if check_interrupt().is_err() {
                        return acc;
                    }
                    acc.errors.extend(item.errors);
                    acc.deprecated_warnings.extend(item.deprecated_warnings);
                    acc.warnings.extend(item.warnings);
                    acc
                });

        if check_interrupt().is_err() {
            return;
        }
        // Extend self with the combined results
        self.errors.extend(combined.errors);
        self.deprecated_warnings
            .extend(combined.deprecated_warnings);
        self.warnings.extend(combined.warnings);
    }
}

#[derive(Error, Debug, Clone, Serialize)]
#[pyclass(module = "tach.extension")]
pub enum ImportCheckError {
    #[error("Module containing '{file_mod_path}' not found in project.")]
    ModuleNotFound { file_mod_path: String },

    #[error("Module '{import_nearest_module_path}' has a defined public interface. Only imports from the public interface of this module are allowed. The import '{import_mod_path}' (in module '{file_nearest_module_path}') is not public.")]
    PrivateImport {
        import_mod_path: String,
        import_nearest_module_path: String,
        file_nearest_module_path: String,
    },

    #[error("The import '{import_mod_path}' (from module '{import_nearest_module_path}') matches an interface but does not match the expected data type ('{expected_data_type}').")]
    InvalidDataTypeExport {
        import_mod_path: String,
        import_nearest_module_path: String,
        expected_data_type: String,
    },

    #[error("Could not find module configuration.")]
    ModuleConfigNotFound(),

    #[error("Cannot import '{import_mod_path}'. Module '{source_module}' cannot depend on '{invalid_module}'.")]
    InvalidImport {
        import_mod_path: String,
        source_module: String,
        invalid_module: String,
    },

    #[error("Import '{import_mod_path}' is deprecated. Module '{source_module}' should not depend on '{invalid_module}'.")]
    DeprecatedImport {
        import_mod_path: String,
        source_module: String,
        invalid_module: String,
    },

    #[error("Cannot import '{import_mod_path}'. Layer '{source_layer}' ('{source_module}') is lower than layer '{invalid_layer}' ('{invalid_module}').")]
    LayerViolation {
        import_mod_path: String,
        source_module: String,
        source_layer: String,
        invalid_module: String,
        invalid_layer: String,
    },

    #[error("Import '{import_mod_path}' is unnecessarily ignored by a directive.")]
    UnusedIgnoreDirective { import_mod_path: String },

    #[error("Import '{import_mod_path}' is ignored without providing a reason.")]
    MissingIgnoreDirectiveReason { import_mod_path: String },

    #[error("No checks enabled. At least one of dependencies or interfaces must be enabled.")]
    NoChecksEnabled(),
}

#[pymethods]
impl ImportCheckError {
    pub fn is_dependency_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidImport { .. }
                | Self::DeprecatedImport { .. }
                | Self::LayerViolation { .. }
        )
    }

    pub fn is_interface_error(&self) -> bool {
        matches!(
            self,
            Self::PrivateImport { .. } | Self::InvalidDataTypeExport { .. }
        )
    }

    pub fn source_path(&self) -> Option<&String> {
        match self {
            Self::InvalidImport { source_module, .. } => Some(source_module),
            Self::DeprecatedImport { source_module, .. } => Some(source_module),
            Self::LayerViolation { source_module, .. } => Some(source_module),
            _ => None,
        }
    }

    pub fn invalid_path(&self) -> Option<&String> {
        match self {
            Self::InvalidImport { invalid_module, .. } => Some(invalid_module),
            Self::DeprecatedImport { invalid_module, .. } => Some(invalid_module),
            Self::LayerViolation { invalid_module, .. } => Some(invalid_module),
            _ => None,
        }
    }

    pub fn is_deprecated(&self) -> bool {
        matches!(self, Self::DeprecatedImport { .. })
    }

    pub fn to_pystring(&self) -> String {
        self.to_string()
    }
}

fn check_import(
    import_mod_path: &str,
    module_tree: &ModuleTree,
    file_nearest_module: Arc<ModuleNode>,
    layers: &[String],
    root_module_treatment: RootModuleTreatment,
    interface_checker: &Option<InterfaceChecker>,
    check_dependencies: bool,
) -> Result<(), ImportCheckError> {
    if !check_dependencies && interface_checker.is_none() {
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

    // -- START INTERFACE CHECKS
    if let Some(interface_checker) = interface_checker {
        // When interfaces are enabled, we check whether the import is a valid export
        let import_member = import_mod_path
            .strip_prefix(&import_nearest_module.full_path)
            .and_then(|s| s.strip_prefix('.'))
            .unwrap_or("");
        let check_result =
            interface_checker.check_member(import_member, &import_nearest_module.full_path);
        match check_result {
            InterfaceCheckResult::NotExposed => {
                return Err(ImportCheckError::PrivateImport {
                    import_mod_path: import_mod_path.to_string(),
                    import_nearest_module_path: import_nearest_module.full_path.to_string(),
                    file_nearest_module_path: file_nearest_module.full_path.to_string(),
                });
            }
            InterfaceCheckResult::Exposed {
                type_check_result: TypeCheckResult::DidNotMatchInterface { expected },
            } => {
                return Err(ImportCheckError::InvalidDataTypeExport {
                    import_mod_path: import_mod_path.to_string(),
                    import_nearest_module_path: import_nearest_module.full_path.to_string(),
                    expected_data_type: expected.to_string(),
                });
            }
            _ => {}
        }
    }
    // -- END INTERFACE CHECKS

    // -- START DEPENDENCY CHECKS
    if !check_dependencies {
        return Ok(());
    }

    if !check_layers(layers, file_module_config, import_module_config) {
        return Err(ImportCheckError::LayerViolation {
            import_mod_path: import_mod_path.to_string(),
            source_module: file_nearest_module.full_path.to_string(),
            source_layer: file_module_config.layer.clone().unwrap_or("".to_string()),
            invalid_module: import_nearest_module.full_path.to_string(),
            invalid_layer: import_module_config.layer.clone().unwrap_or("".to_string()),
        });
    }

    if import_module_config.utility {
        return Ok(());
    }

    let file_nearest_module_path = &file_module_config.path;
    let import_nearest_module_path = &import_module_config.path;

    // The import must be explicitly allowed in the file's config
    let allowed_dependencies: HashSet<_> = file_module_config
        .depends_on
        .iter()
        .filter(|dep| !dep.deprecated)
        .map(|dep| &dep.path)
        .collect();

    if allowed_dependencies.contains(import_nearest_module_path) {
        // The import matches at least one expected dependency
        return Ok(());
    }

    let deprecated_dependencies: HashSet<_> = file_module_config
        .depends_on
        .iter()
        .filter(|dep| dep.deprecated)
        .map(|dep| &dep.path)
        .collect();

    if deprecated_dependencies.contains(import_nearest_module_path) {
        // Dependency exists but is deprecated
        return Err(ImportCheckError::DeprecatedImport {
            import_mod_path: import_mod_path.to_string(),
            source_module: file_nearest_module_path.to_string(),
            invalid_module: import_nearest_module_path.to_string(),
        });
    }

    // This means the import is not declared as a dependency of the file
    Err(ImportCheckError::InvalidImport {
        import_mod_path: import_mod_path.to_string(),
        source_module: file_nearest_module_path.to_string(),
        invalid_module: import_nearest_module_path.to_string(),
    })
    // -- END DEPENDENCY CHECKS
}

fn check_layers(
    layers: &[String],
    source_module_config: &ModuleConfig,
    target_module_config: &ModuleConfig,
) -> bool {
    match (&source_module_config.layer, &target_module_config.layer) {
        (Some(source_layer), Some(target_layer)) => {
            let source_index = layers.iter().position(|layer| layer == source_layer);
            let target_index = layers.iter().position(|layer| layer == target_layer);

            match (source_index, target_index) {
                // If the 'source' layer comes before the 'target' layer,
                // this means a higher layer is importing a lower layer.
                // This direction is allowed.
                (Some(source_index), Some(target_index)) => source_index <= target_index,
                // If either index is not found, the layer is unknown -- ignore for now
                _ => true,
            }
        }
        _ => true,
    }
}

fn process_file(
    file_path: PathBuf,
    source_root: &Path,
    source_roots: &[PathBuf],
    module_tree: &ModuleTree,
    project_config: &ProjectConfig,
    interface_checker: &Option<InterfaceChecker>,
    check_dependencies: bool,
    found_imports: &AtomicBool,
) -> Option<CheckDiagnostics> {
    let abs_file_path = &source_root.join(&file_path);
    let mod_path = fs::file_to_module_path(source_roots, abs_file_path).ok()?;
    let nearest_module = module_tree.find_nearest(&mod_path)?;

    if nearest_module.is_unchecked() {
        return None;
    }

    if nearest_module.is_root() && project_config.root_module == RootModuleTreatment::Ignore {
        return None;
    }

    let mut diagnostics = CheckDiagnostics::default();
    let project_imports = match get_project_imports(
        source_roots,
        abs_file_path,
        project_config.ignore_type_checking_imports,
        project_config.include_string_imports,
    ) {
        Ok(project_imports) => {
            if !project_imports.imports.is_empty() && !found_imports.load(Ordering::Relaxed) {
                // Only attempt to write if we haven't found imports yet.
                // This avoids any potential lock contention.
                found_imports.store(true, Ordering::Relaxed);
            }
            project_imports
        }
        Err(ImportParseError::Parsing { .. }) => {
            diagnostics.warnings.push(format!(
                "Skipped '{}' due to a syntax error.",
                file_path.display()
            ));
            return Some(diagnostics);
        }
        Err(ImportParseError::Filesystem(_)) => {
            diagnostics.warnings.push(format!(
                "Skipped '{}' due to an I/O error.",
                file_path.display()
            ));
            return Some(diagnostics);
        }
        Err(ImportParseError::Exclusion(_)) => {
            diagnostics.warnings.push(format!(
                "Skipped '{}'. Failed to check if the path is excluded.",
                file_path.display(),
            ));
            return Some(diagnostics);
        }
    };

    for import in project_imports.imports {
        if let Err(error_info) = check_import(
            &import.module_path,
            module_tree,
            Arc::clone(&nearest_module),
            &project_config.layers,
            project_config.root_module.clone(),
            interface_checker,
            check_dependencies,
        ) {
            let boundary_error = BoundaryError {
                file_path: file_path.clone(),
                line_number: import.line_no,
                import_mod_path: import.module_path.to_string(),
                error_info,
            };
            if boundary_error.is_deprecated() {
                diagnostics.deprecated_warnings.push(boundary_error);
            } else {
                diagnostics.errors.push(boundary_error);
            }
        };
    }

    // Process directive-ignored imports
    for directive_ignored_import in project_imports.directive_ignored_imports {
        // Check for missing ignore directive reasons
        if project_config.rules.require_ignore_directive_reasons != RuleSetting::Off
            && directive_ignored_import.reason.is_empty()
        {
            let error = BoundaryError {
                file_path: file_path.clone(),
                line_number: directive_ignored_import.import.line_no,
                import_mod_path: directive_ignored_import.import.module_path.to_string(),
                error_info: ImportCheckError::MissingIgnoreDirectiveReason {
                    import_mod_path: directive_ignored_import.import.module_path.to_string(),
                },
            };
            if project_config.rules.require_ignore_directive_reasons == RuleSetting::Error {
                diagnostics.errors.push(error);
            } else {
                diagnostics.warnings.push(format!(
                    "Import '{}' is ignored without providing a reason",
                    directive_ignored_import.import.module_path
                ));
            }
        }

        // Check for unnecessary ignore directives
        if project_config.rules.unused_ignore_directives != RuleSetting::Off {
            let is_unnecessary = check_import(
                &directive_ignored_import.import.module_path,
                module_tree,
                Arc::clone(&nearest_module),
                &project_config.layers,
                project_config.root_module.clone(),
                interface_checker,
                check_dependencies,
            )
            .is_ok();

            if is_unnecessary {
                let message = format!(
                    "Import '{}' is unnecessarily ignored by a directive.",
                    directive_ignored_import.import.module_path
                );

                if project_config.rules.unused_ignore_directives == RuleSetting::Error {
                    diagnostics.errors.push(BoundaryError {
                        file_path: file_path.clone(),
                        line_number: directive_ignored_import.import.line_no,
                        import_mod_path: directive_ignored_import.import.module_path.to_string(),
                        error_info: ImportCheckError::UnusedIgnoreDirective {
                            import_mod_path: directive_ignored_import
                                .import
                                .module_path
                                .to_string(),
                        },
                    });
                } else {
                    diagnostics.warnings.push(message);
                }
            }
        }
    }

    Some(diagnostics)
}

pub fn check(
    project_root: PathBuf,
    project_config: &ProjectConfig,
    dependencies: bool,
    interfaces: bool,
    exclude_paths: Vec<String>,
) -> Result<CheckDiagnostics, CheckError> {
    if !dependencies && !interfaces {
        return Ok(CheckDiagnostics {
            errors: Vec::new(),
            deprecated_warnings: Vec::new(),
            warnings: vec!["WARNING: No checks enabled. At least one of dependencies or interfaces must be enabled.".to_string()],
        });
    }
    if !project_root.is_dir() {
        return Err(CheckError::InvalidDirectory(
            project_root.display().to_string(),
        ));
    }

    let mut diagnostics = CheckDiagnostics::default();
    let found_imports = AtomicBool::new(false);
    let exclude_paths = exclude_paths.iter().map(PathBuf::from).collect::<Vec<_>>();
    let source_roots: Vec<PathBuf> = project_config.prepend_roots(&project_root);
    let (valid_modules, invalid_modules) =
        fs::validate_project_modules(&source_roots, project_config.modules.clone());

    for module in &invalid_modules {
        diagnostics.warnings.push(format!(
            "Module '{}' not found. It will be ignored.",
            module.path
        ));
    }

    check_interrupt().map_err(|_| CheckError::Interrupt)?;
    let module_tree = build_module_tree(
        &source_roots,
        &valid_modules,
        project_config.forbid_circular_dependencies,
        project_config.root_module.clone(),
    )?;

    set_excluded_paths(
        Path::new(&project_root),
        &exclude_paths,
        project_config.use_regex_matching,
    )?;

    let interface_checker = if interfaces {
        let interface_checker = InterfaceChecker::new(&project_config.interfaces);
        // This is expensive
        Some(interface_checker.with_type_check_cache(&valid_modules, &source_roots)?)
    } else {
        None
    };

    for source_root in &source_roots {
        let source_root_diagnostics = fs::walk_pyfiles(&source_root.display().to_string())
            .par_bridge()
            .filter_map(|file_path| {
                if check_interrupt().is_err() {
                    // Since files are being processed in parallel,
                    // this will essentially short-circuit all remaining files.
                    // Then, we check for an interrupt right after, and return the Err if it is set
                    return None;
                }
                process_file(
                    file_path,
                    source_root,
                    &source_roots,
                    &module_tree,
                    project_config,
                    &interface_checker,
                    dependencies,
                    &found_imports,
                )
            });
        check_interrupt().map_err(|_| CheckError::Interrupt)?;
        diagnostics.par_extend(source_root_diagnostics);
        check_interrupt().map_err(|_| CheckError::Interrupt)?;
    }

    if !found_imports.load(Ordering::Relaxed) {
        diagnostics.warnings.push(
            "WARNING: No first-party imports were found. You may need to use 'tach mod' to update your Python source roots. Docs: https://docs.gauge.sh/usage/configuration#source-roots"
                .to_string(),
        );
    }

    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{InterfaceConfig, ModuleConfig};
    use crate::modules::ModuleTree;
    use crate::tests::check_internal::fixtures::{
        interface_config, layers, module_config, module_tree,
    };

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

        assert_eq!(
            check_layers(&layers, &source_config, &target_config),
            expected_result
        );
    }

    #[rstest]
    fn test_check_layers_missing_layers() {
        let layers: Vec<String> = vec![];
        // Note: would validate against this
        let source_config = ModuleConfig::new_with_layer("source", "any");
        let target_config = ModuleConfig::new_with_layer("target", "any");

        assert!(check_layers(&layers, &source_config, &target_config));
    }

    #[rstest]
    fn test_check_layers_no_layer_specified() {
        let layers = vec!["top".to_string(), "bottom".to_string()];
        let source_config = ModuleConfig::default();
        let target_config = ModuleConfig::default();

        // When modules don't specify layers, they should be allowed
        assert!(check_layers(&layers, &source_config, &target_config));
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
