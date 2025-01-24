use super::checks::{
    check_import_internal, check_missing_ignore_directive_reason,
    check_unused_ignore_directive_internal,
};
use super::diagnostics::{CodeDiagnostic, ConfigurationDiagnostic, Diagnostic, DiagnosticDetails};
use super::error::CheckError;

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};

use rayon::prelude::*;

use crate::modules::error::ModuleTreeError;
use crate::{
    config::{root_module::RootModuleTreatment, ProjectConfig},
    exclusion::set_excluded_paths,
    filesystem as fs,
    imports::{get_project_imports, ImportParseError},
    interfaces::InterfaceChecker,
    interrupt::check_interrupt,
    modules::{build_module_tree, ModuleTree},
};

pub type Result<T> = std::result::Result<T, CheckError>;

fn process_file(
    file_path: PathBuf,
    source_root: &Path,
    source_roots: &[PathBuf],
    module_tree: &ModuleTree,
    project_config: &ProjectConfig,
    interface_checker: &Option<InterfaceChecker>,
    check_dependencies: bool,
    found_imports: &AtomicBool,
) -> Result<Vec<Diagnostic>> {
    let abs_file_path = &source_root.join(&file_path);
    let mod_path = fs::file_to_module_path(source_roots, abs_file_path)?;
    let nearest_module = module_tree
        .find_nearest(&mod_path)
        .ok_or(CheckError::ModuleTree(ModuleTreeError::ModuleNotFound(
            mod_path.to_string(),
        )))?;

    if nearest_module.is_unchecked() {
        return Ok(vec![]);
    }

    if nearest_module.is_root() && project_config.root_module == RootModuleTreatment::Ignore {
        return Ok(vec![]);
    }

    let mut diagnostics = vec![];
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
            return Ok(vec![Diagnostic::new_global_warning(
                DiagnosticDetails::Configuration(ConfigurationDiagnostic::SkippedFileSyntaxError {
                    file_path: file_path.display().to_string(),
                }),
            )]);
        }
        Err(ImportParseError::Filesystem(_)) => {
            return Ok(vec![Diagnostic::new_global_warning(
                DiagnosticDetails::Configuration(ConfigurationDiagnostic::SkippedFileIoError {
                    file_path: file_path.display().to_string(),
                }),
            )]);
        }
    };

    project_imports.active_imports().for_each(|import| {
        if let Err(diagnostic) = check_import_internal(
            &import.module_path,
            module_tree,
            Arc::clone(&nearest_module),
            &project_config.layers,
            project_config.root_module.clone(),
            interface_checker,
            check_dependencies,
        ) {
            match &diagnostic {
                Diagnostic::Global {
                    details: DiagnosticDetails::Code(_),
                    ..
                } => {
                    diagnostics.push(diagnostic.into_located(file_path.clone(), import.line_no));
                }
                Diagnostic::Global {
                    details: DiagnosticDetails::Configuration(_),
                    ..
                } => {
                    diagnostics.push(diagnostic);
                }
                _ => {}
            }
        };
    });

    project_imports
        .directive_ignored_imports()
        .for_each(|directive_ignored_import| {
            match check_unused_ignore_directive_internal(
                &directive_ignored_import,
                module_tree,
                Arc::clone(&nearest_module),
                project_config,
                interface_checker,
                check_dependencies,
            ) {
                Ok(()) => {}
                Err(diagnostic) => {
                    diagnostics.push(
                        diagnostic.into_located(
                            file_path.clone(),
                            directive_ignored_import.import.line_no,
                        ),
                    );
                }
            }
            match check_missing_ignore_directive_reason(&directive_ignored_import, project_config) {
                Ok(()) => {}
                Err(diagnostic) => {
                    diagnostics.push(
                        diagnostic.into_located(
                            file_path.clone(),
                            directive_ignored_import.import.line_no,
                        ),
                    );
                }
            }
        });

    project_imports
        .unused_ignore_directives()
        .for_each(|ignore_directive| {
            if let Ok(severity) = (&project_config.rules.unused_ignore_directives).try_into() {
                diagnostics.push(Diagnostic::new_located(
                    severity,
                    DiagnosticDetails::Code(CodeDiagnostic::UnusedIgnoreDirective()),
                    file_path.clone(),
                    ignore_directive.line_no,
                ));
            }
        });

    Ok(diagnostics)
}

pub fn check(
    project_root: PathBuf,
    project_config: &ProjectConfig,
    dependencies: bool,
    interfaces: bool,
    exclude_paths: Vec<String>,
) -> Result<Vec<Diagnostic>> {
    if !dependencies && !interfaces {
        return Err(CheckError::NoChecksEnabled());
    }

    if !project_root.is_dir() {
        return Err(CheckError::InvalidDirectory(
            project_root.display().to_string(),
        ));
    }

    let mut warnings = Vec::new();
    let found_imports = AtomicBool::new(false);
    let exclude_paths = exclude_paths.iter().map(PathBuf::from).collect::<Vec<_>>();
    let source_roots: Vec<PathBuf> = project_config.prepend_roots(&project_root);
    let (valid_modules, invalid_modules) = fs::validate_project_modules(
        &source_roots,
        project_config.all_modules().cloned().collect(),
    );

    for module in &invalid_modules {
        warnings.push(Diagnostic::new_global_warning(
            DiagnosticDetails::Configuration(ConfigurationDiagnostic::ModuleNotFound {
                file_mod_path: module.path.to_string(),
            }),
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
        let interface_checker =
            InterfaceChecker::new(&project_config.all_interfaces().cloned().collect::<Vec<_>>());
        // This is expensive
        Some(interface_checker.with_type_check_cache(&valid_modules, &source_roots)?)
    } else {
        None
    };

    let diagnostics = source_roots.par_iter().flat_map(|source_root| {
        fs::walk_pyfiles(&source_root.display().to_string())
            .par_bridge()
            .flat_map(|file_path| {
                if check_interrupt().is_err() {
                    // Since files are being processed in parallel,
                    // this will essentially short-circuit all remaining files.
                    // Then, we check for an interrupt right after, and return the Err if it is set
                    return vec![];
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
                .unwrap_or_default()
            })
    });

    if check_interrupt().is_err() {
        return Err(CheckError::Interrupt);
    }

    if !found_imports.load(Ordering::Relaxed) {
        warnings.push(Diagnostic::new_global_warning(
            DiagnosticDetails::Configuration(ConfigurationDiagnostic::NoFirstPartyImportsFound()),
        ));
    }

    Ok(diagnostics.chain(warnings).collect())
}
