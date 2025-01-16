pub mod checks;
use checks::{check_import, check_missing_ignore_directive_reason, check_unused_ignore_directive};
pub mod diagnostics;
use diagnostics::ImportCheckError;
pub use diagnostics::{BoundaryError, CheckDiagnostics};
pub mod error;
pub use error::CheckError;

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};

use rayon::prelude::*;

use crate::{
    config::{root_module::RootModuleTreatment, ProjectConfig, RuleSetting},
    exclusion::set_excluded_paths,
    filesystem as fs,
    imports::{get_project_imports, ImportParseError},
    interfaces::InterfaceChecker,
    interrupt::check_interrupt,
    modules::{build_module_tree, ModuleTree},
};

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

    project_imports.active_imports().for_each(|import| {
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
    });

    project_imports
        .directive_ignored_imports()
        .for_each(|directive_ignored_import| {
            if project_config.rules.unused_ignore_directives != RuleSetting::Off {
                let check_result = check_unused_ignore_directive(
                    &directive_ignored_import,
                    module_tree,
                    Arc::clone(&nearest_module),
                    project_config,
                    interface_checker,
                    check_dependencies,
                );
                match (check_result, &project_config.rules.unused_ignore_directives) {
                    (Err(e), RuleSetting::Error) => {
                        diagnostics.errors.push(BoundaryError {
                            file_path: file_path.clone(),
                            line_number: directive_ignored_import.import.line_no,
                            import_mod_path: directive_ignored_import
                                .import
                                .module_path
                                .to_string(),
                            error_info: e,
                        });
                    }
                    (Err(e), RuleSetting::Warn) => {
                        diagnostics.warnings.push(e.to_string());
                    }
                    (Ok(()), _) | (_, RuleSetting::Off) => {}
                }
            }
            if project_config.rules.require_ignore_directive_reasons != RuleSetting::Off {
                let check_result = check_missing_ignore_directive_reason(&directive_ignored_import);
                match (
                    check_result,
                    &project_config.rules.require_ignore_directive_reasons,
                ) {
                    (Err(e), RuleSetting::Error) => {
                        diagnostics.errors.push(BoundaryError {
                            file_path: file_path.clone(),
                            line_number: directive_ignored_import.import.line_no,
                            import_mod_path: directive_ignored_import
                                .import
                                .module_path
                                .to_string(),
                            error_info: e,
                        });
                    }
                    (Err(e), RuleSetting::Warn) => {
                        diagnostics.warnings.push(e.to_string());
                    }
                    (Ok(()), _) | (_, RuleSetting::Off) => {}
                }
            }
        });

    project_imports
        .unused_ignore_directives()
        .for_each(
            |ignore_directive| match project_config.rules.unused_ignore_directives {
                RuleSetting::Error => {
                    diagnostics.errors.push(BoundaryError {
                        file_path: file_path.clone(),
                        line_number: ignore_directive.line_no,
                        import_mod_path: ignore_directive.modules.join(", "),
                        error_info: ImportCheckError::UnusedIgnoreDirective(),
                    });
                }
                RuleSetting::Warn => {
                    diagnostics.warnings.push(format!(
                        "Unused ignore directive: '{}' in file '{}'",
                        ignore_directive.modules.join(","),
                        file_path.display()
                    ));
                }
                RuleSetting::Off => {}
            },
        );

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
