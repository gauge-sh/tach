use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use pyo3::{pyclass, pymethods};
use thiserror::Error;

use crate::{
    core::{
        config::{ProjectConfig, RootModuleTreatment, RuleSetting},
        interfaces::InterfaceChecker,
        module::{ModuleNode, ModuleTree},
    },
    exclusion::{self, is_path_excluded, set_excluded_paths},
    filesystem as fs,
    imports::{get_project_imports, ImportParseError},
    parsing::{self, module::build_module_tree},
};

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("The path {0} is not a valid directory.")]
    InvalidDirectory(String),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] fs::FileSystemError),
    #[error("Module tree error: {0}")]
    ModuleTree(#[from] parsing::error::ModuleTreeError),
    #[error("Exclusion error: {0}")]
    Exclusion(#[from] exclusion::PathExclusionError),
}

#[derive(Debug, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct BoundaryError {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub import_mod_path: String,
    pub error_info: ImportCheckError,
}

#[derive(Debug)]
#[pyclass(get_all, module = "tach.extension")]
pub struct CheckDiagnostics {
    pub errors: Vec<BoundaryError>,
    pub deprecated_warnings: Vec<BoundaryError>,
    pub warnings: Vec<String>,
}

#[derive(Error, Debug, Clone)]
#[pyclass(module = "tach.extension")]
pub enum ImportCheckError {
    #[error("Module containing '{file_mod_path}' not found in project.")]
    ModuleNotFound { file_mod_path: String },

    #[error("Module '{import_nearest_module_path}' has a defined public interface. Only imports from the public interface of this module are allowed. The import '{import_mod_path}' (in module '{file_nearest_module_path}') is not public.")]
    StrictModeImport {
        import_mod_path: String,
        import_nearest_module_path: String,
        file_nearest_module_path: String,
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

    #[error("Import '{import_mod_path}' is unnecessarily ignored by a directive.")]
    UnusedIgnoreDirective { import_mod_path: String },

    #[error("No checks enabled. At least one of dependencies or interfaces must be enabled.")]
    NoChecksEnabled(),
}

#[pymethods]
impl ImportCheckError {
    pub fn is_dependency_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidImport { .. } | Self::DeprecatedImport { .. }
        )
    }

    pub fn is_interface_error(&self) -> bool {
        matches!(self, Self::StrictModeImport { .. })
    }

    pub fn source_path(&self) -> Option<&String> {
        match self {
            Self::InvalidImport { source_module, .. } => Some(source_module),
            Self::DeprecatedImport { source_module, .. } => Some(source_module),
            _ => None,
        }
    }

    pub fn invalid_path(&self) -> Option<&String> {
        match self {
            Self::InvalidImport { invalid_module, .. } => Some(invalid_module),
            Self::DeprecatedImport { invalid_module, .. } => Some(invalid_module),
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

fn is_top_level_module_import(mod_path: &str, module: &ModuleNode) -> bool {
    mod_path == module.full_path
}

fn import_matches_interface_members(mod_path: &str, module: &ModuleNode) -> bool {
    let mod_path_segments: Vec<&str> = mod_path.rsplitn(2, '.').collect();

    if mod_path_segments.len() == 1 {
        // If there's no '.' in the path, compare the whole path with the module's full path.
        mod_path_segments[0] == module.full_path
    } else {
        // If there's a '.', split into package path and member name.
        let mod_pkg_path = mod_path_segments[1];
        let mod_member_name = mod_path_segments[0];

        mod_pkg_path == module.full_path
            && module
                .interface_members
                .contains(&mod_member_name.to_string())
    }
}

fn check_import(
    import_mod_path: &str,
    module_tree: &ModuleTree,
    file_nearest_module: Arc<ModuleNode>,
    root_module_treatment: RootModuleTreatment,
    interface_checker: &InterfaceChecker,
    check_dependencies: bool,
    check_interfaces: bool,
) -> Result<(), ImportCheckError> {
    if !check_dependencies && !check_interfaces {
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

    if check_interfaces
        && interface_checker.has_interface(&import_nearest_module.full_path)
        && import_mod_path != &import_nearest_module.full_path
    {
        let import_member = import_mod_path
            .strip_prefix(&import_nearest_module.full_path)
            .and_then(|s| s.strip_prefix('.'))
            .unwrap_or(import_mod_path);

        if !interface_checker.check(import_member, &import_nearest_module.full_path) {
            return Err(ImportCheckError::StrictModeImport {
                import_mod_path: import_mod_path.to_string(),
                import_nearest_module_path: import_nearest_module.full_path.to_string(),
                file_nearest_module_path: file_nearest_module.full_path.to_string(),
            });
        }
    }

    if !check_dependencies {
        return Ok(());
    }

    if let Some(true) = import_nearest_module
        .config
        .as_ref()
        .map(|config| config.utility)
    {
        return Ok(());
    }

    let file_config = file_nearest_module
        .config
        .as_ref()
        .ok_or(ImportCheckError::ModuleConfigNotFound())?;
    let file_nearest_module_path = &file_config.path;
    let import_nearest_module_path = &import_nearest_module
        .config
        .as_ref()
        .ok_or(ImportCheckError::ModuleConfigNotFound())?
        .path;

    // The import must be explicitly allowed in the file's config
    let allowed_dependencies: HashSet<_> = file_config
        .depends_on
        .iter()
        .filter(|dep| !dep.deprecated)
        .map(|dep| &dep.path)
        .collect();

    if allowed_dependencies.contains(import_nearest_module_path) {
        // The import matches at least one expected dependency
        return Ok(());
    }

    let deprecated_dependencies: HashSet<_> = file_config
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

    let exclude_paths = exclude_paths.iter().map(PathBuf::from).collect::<Vec<_>>();
    if !project_root.is_dir() {
        return Err(CheckError::InvalidDirectory(
            project_root.display().to_string(),
        ));
    }
    let source_roots: Vec<PathBuf> = project_config.prepend_roots(&project_root);
    let (valid_modules, invalid_modules) =
        fs::validate_project_modules(&source_roots, project_config.modules.clone());

    let mut found_at_least_one_project_import = false;
    let mut boundary_errors = Vec::new();
    let mut boundary_warnings = Vec::new();
    let mut warnings = Vec::new();

    for module in &invalid_modules {
        warnings.push(format!(
            "Module '{}' not found. It will be ignored.",
            module.path
        ));
    }

    let module_tree = build_module_tree(
        &source_roots,
        valid_modules,
        project_config.forbid_circular_dependencies,
        project_config.root_module.clone(),
    )?;

    set_excluded_paths(
        Path::new(&project_root),
        &exclude_paths,
        project_config.use_regex_matching,
    )?;

    let interface_checker = InterfaceChecker::new(&project_config.interfaces);

    for source_root in &source_roots {
        for file_path in fs::walk_pyfiles(&source_root.display().to_string()) {
            let abs_file_path = &source_root.join(&file_path);
            if is_path_excluded(abs_file_path)? {
                continue;
            }
            let mod_path = fs::file_to_module_path(&source_roots, abs_file_path)?;
            let Some(nearest_module) = module_tree.find_nearest(&mod_path) else {
                continue;
            };

            if nearest_module.is_unchecked() {
                // If the module is 'unchecked', we skip checking its imports
                continue;
            }

            if nearest_module.is_root() && project_config.root_module == RootModuleTreatment::Ignore
            {
                continue;
            }

            let project_imports = match get_project_imports(
                &source_roots,
                abs_file_path,
                project_config.ignore_type_checking_imports,
                project_config.include_string_imports,
            ) {
                Ok(v) => v,
                Err(ImportParseError::Parsing { .. }) => {
                    warnings.push(format!(
                        "Skipping '{}' due to a syntax error.",
                        file_path.display()
                    ));
                    continue;
                }
                Err(ImportParseError::Filesystem(_)) => {
                    warnings.push(format!(
                        "Skipping '{}' due to an I/O error.",
                        file_path.display()
                    ));
                    continue;
                }
                Err(ImportParseError::Exclusion(_)) => {
                    warnings.push(format!(
                        "Skipping '{}'. Failed to check if the path is excluded.",
                        file_path.display(),
                    ));
                    continue;
                }
            };

            for import in project_imports.imports {
                found_at_least_one_project_import = true;
                let Err(error_info) = check_import(
                    &import.module_path,
                    &module_tree,
                    Arc::clone(&nearest_module),
                    project_config.root_module.clone(),
                    &interface_checker,
                    dependencies,
                    interfaces,
                ) else {
                    continue;
                };
                let boundary_error = BoundaryError {
                    file_path: file_path.clone(),
                    line_number: import.line_no,
                    import_mod_path: import.module_path.to_string(),
                    error_info,
                };
                if boundary_error.error_info.is_deprecated() {
                    boundary_warnings.push(boundary_error);
                } else {
                    boundary_errors.push(boundary_error);
                }
            }

            if project_config.rules.unused_ignore_directives == RuleSetting::Off {
                continue;
            }
            for directive_ignored_import in project_imports.directive_ignored_imports {
                if check_import(
                    &directive_ignored_import.module_path,
                    &module_tree,
                    Arc::clone(&nearest_module),
                    project_config.root_module.clone(),
                    &interface_checker,
                    dependencies,
                    interfaces,
                )
                .is_ok()
                {
                    let message = format!(
                        "Import '{}' is unnecessarily ignored by a directive.",
                        directive_ignored_import.module_path
                    );
                    match project_config.rules.unused_ignore_directives {
                        RuleSetting::Error => {
                            boundary_errors.push(BoundaryError {
                                file_path: file_path.clone(),
                                line_number: directive_ignored_import.line_no,
                                import_mod_path: directive_ignored_import.module_path.to_string(),
                                error_info: ImportCheckError::UnusedIgnoreDirective {
                                    import_mod_path: directive_ignored_import
                                        .module_path
                                        .to_string(),
                                },
                            });
                        }
                        RuleSetting::Warn => {
                            warnings.push(message);
                        }
                        // Should never be reached
                        RuleSetting::Off => {}
                    }
                }
            }
        }
    }

    if !found_at_least_one_project_import {
        warnings.push(
            "WARNING: No first-party imports were found. You may need to use 'tach mod' to update your Python source roots. Docs: https://docs.gauge.sh/usage/configuration#source-roots"
                .to_string(),
        );
    }

    Ok(CheckDiagnostics {
        errors: boundary_errors,
        deprecated_warnings: boundary_warnings,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::InterfaceConfig;
    use crate::core::module::ModuleTree;
    use crate::tests::check_int::fixtures::{interface_config, module_tree};

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
        interface_config: Vec<InterfaceConfig>,
        #[case] file_mod_path: &str,
        #[case] import_mod_path: &str,
        #[case] expected_result: bool,
    ) {
        let file_module = module_tree.find_nearest(file_mod_path).unwrap();
        let interface_checker = InterfaceChecker::new(&interface_config);

        let check_error = check_import(
            import_mod_path,
            &module_tree,
            file_module.clone(),
            RootModuleTreatment::Allow,
            &interface_checker,
            true,
            true,
        );
        let result = check_error.is_ok();
        assert_eq!(result, expected_result);
    }

    #[rstest]
    fn test_check_deprecated_import(
        module_tree: ModuleTree,
        interface_config: Vec<InterfaceConfig>,
    ) {
        let file_module = module_tree.find_nearest("domain_one").unwrap();
        let interface_checker = InterfaceChecker::new(&interface_config);

        let check_error = check_import(
            "domain_one.subdomain",
            &module_tree,
            file_module.clone(),
            RootModuleTreatment::Allow,
            &interface_checker,
            true,
            true,
        );
        assert!(check_error.is_err());
        assert!(check_error.unwrap_err().is_deprecated());
    }
}
