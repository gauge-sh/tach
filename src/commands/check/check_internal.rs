use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};

use rayon::prelude::*;

use super::error::CheckError;
use crate::{
    checks::{
        ignore_directive::IgnoreDirectiveData, IgnoreDirectiveChecker, InterfaceChecker,
        InternalDependencyChecker,
    },
    config::{root_module::RootModuleTreatment, ProjectConfig},
    diagnostics::{
        ConfigurationDiagnostic, Diagnostic, DiagnosticDetails, DiagnosticError,
        DiagnosticPipeline, FileChecker, FileContext, FileProcessor, Result as DiagnosticResult,
    },
    exclusion::set_excluded_paths,
    filesystem::{self as fs, relative_to},
    interrupt::check_interrupt,
    modules::{build_module_tree, error::ModuleTreeError, ModuleNode, ModuleTree},
    processors::imports::{get_project_imports, NormalizedImports, ProjectImports},
};

pub type Result<T> = std::result::Result<T, CheckError>;

struct CheckInternalContext<'a> {
    project_root: &'a Path,
    source_root: &'a Path,
    source_roots: &'a [PathBuf],
    project_config: &'a ProjectConfig,
    module_tree: &'a ModuleTree,
    found_imports: &'a AtomicBool,
}

impl<'a> CheckInternalContext<'a> {
    pub fn new(
        project_root: &'a Path,
        source_root: &'a Path,
        source_roots: &'a [PathBuf],
        project_config: &'a ProjectConfig,
        module_tree: &'a ModuleTree,
        found_imports: &'a AtomicBool,
    ) -> Self {
        Self {
            project_root,
            source_root,
            source_roots,
            project_config,
            module_tree,
            found_imports,
        }
    }
}

impl<'a> AsRef<CheckInternalContext<'a>> for CheckInternalContext<'a> {
    fn as_ref(&self) -> &CheckInternalContext<'a> {
        self
    }
}

struct CheckInternalFileInformation {
    file_module: Arc<ModuleNode>,
    project_imports: NormalizedImports<ProjectImports>,
}

impl<'a> AsRef<CheckInternalFileInformation> for CheckInternalFileInformation {
    fn as_ref(&self) -> &CheckInternalFileInformation {
        self
    }
}

struct CheckInternalPipeline {
    dependency_checker: Option<InternalDependencyChecker>,
    interface_checker: Option<InterfaceChecker>,
    ignore_directive_checker: Option<IgnoreDirectiveChecker>,
}

impl CheckInternalPipeline {
    pub fn new() -> Self {
        Self {
            dependency_checker: None,
            interface_checker: None,
            ignore_directive_checker: None,
        }
    }

    pub fn with_dependency_checker(
        mut self,
        dependency_checker: Option<InternalDependencyChecker>,
    ) -> Self {
        self.dependency_checker = dependency_checker;
        self
    }

    pub fn with_interface_checker(mut self, interface_checker: Option<InterfaceChecker>) -> Self {
        self.interface_checker = interface_checker;
        self
    }

    pub fn with_ignore_directive_checker(
        mut self,
        ignore_directive_checker: Option<IgnoreDirectiveChecker>,
    ) -> Self {
        self.ignore_directive_checker = ignore_directive_checker;
        self
    }
}

impl<'a> FileProcessor<'a> for CheckInternalPipeline {
    type IR = CheckInternalFileInformation;
    type Context = CheckInternalContext<'a>;

    fn process(
        &'a self,
        file_path: &Path,
        context: &'a Self::Context,
    ) -> DiagnosticResult<Self::IR> {
        let abs_file_path = &context.source_root.join(file_path);
        let mod_path = fs::file_to_module_path(context.source_roots, abs_file_path)?;
        let file_module =
            context
                .module_tree
                .find_nearest(&mod_path)
                .ok_or(DiagnosticError::ModuleTree(
                    ModuleTreeError::ModuleNotFound(mod_path.to_string()),
                ))?;

        if file_module.is_unchecked() {
            return Ok(CheckInternalFileInformation {
                file_module: Arc::clone(&file_module),
                project_imports: NormalizedImports::empty(),
            });
        }

        if file_module.is_root()
            && context.project_config.root_module == RootModuleTreatment::Ignore
        {
            return Ok(CheckInternalFileInformation {
                file_module: Arc::clone(&file_module),
                project_imports: NormalizedImports::empty(),
            });
        }

        let project_imports = get_project_imports(
            context.source_roots,
            abs_file_path,
            context.project_config.ignore_type_checking_imports,
            context.project_config.include_string_imports,
        )?;

        if !project_imports.imports.is_empty() && !context.found_imports.load(Ordering::Relaxed) {
            // Only attempt to write if we haven't found imports yet.
            // This avoids any potential lock contention.
            context.found_imports.store(true, Ordering::Relaxed);
        }

        Ok(CheckInternalFileInformation {
            file_module: Arc::clone(&file_module),
            project_imports,
        })
    }
}

impl<'a> FileChecker<'a> for CheckInternalPipeline {
    type IR = CheckInternalFileInformation;
    type Context = CheckInternalContext<'a>;
    type Output = Vec<Diagnostic>;

    fn check(
        &'a self,
        file_path: &Path,
        input: &Self::IR,
        context: &'a Self::Context,
    ) -> DiagnosticResult<Self::Output> {
        // This would delegate to the DependencyChecker, InterfaceChecker, etc.
        let relative_file_path =
            relative_to(context.source_root.join(file_path), context.project_root)?;
        let file_module_config = match input.file_module.config.as_ref() {
            Some(config) => config,
            None => {
                return Ok(vec![Diagnostic::new_global_error(
                    DiagnosticDetails::Configuration(
                        ConfigurationDiagnostic::ModuleConfigNotFound {
                            module_path: input.file_module.full_path.to_string(),
                        },
                    ),
                )]);
            }
        };
        let file_context = FileContext::new(
            context.project_config,
            &relative_file_path,
            file_module_config,
            context.module_tree,
        );
        let mut diagnostics = Vec::new();

        diagnostics.extend(
            self.dependency_checker
                .as_ref()
                .map_or(Ok(vec![]), |checker| {
                    checker.check(file_path, &input.project_imports, &file_context)
                })?,
        );

        diagnostics.extend(
            self.interface_checker
                .as_ref()
                .map_or(Ok(vec![]), |checker| {
                    checker.check(file_path, &input.project_imports, &file_context)
                })?,
        );

        let ignore_directive_data =
            IgnoreDirectiveData::new(&input.project_imports.ignore_directives, &diagnostics);
        diagnostics.extend(
            self.ignore_directive_checker
                .as_ref()
                .map_or(Ok(vec![]), |checker| {
                    checker.check(file_path, &ignore_directive_data, &file_context)
                })?,
        );

        Ok(diagnostics)
    }
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

    let dependency_checker = if dependencies {
        Some(InternalDependencyChecker::new())
    } else {
        None
    };

    let interface_checker = if interfaces {
        let interface_checker =
            InterfaceChecker::new(&project_config.all_interfaces().cloned().collect::<Vec<_>>());
        // This is expensive
        Some(interface_checker.with_type_check_cache(&valid_modules, &source_roots)?)
    } else {
        None
    };

    let pipeline = CheckInternalPipeline::new()
        .with_dependency_checker(dependency_checker)
        .with_interface_checker(interface_checker)
        .with_ignore_directive_checker(Some(IgnoreDirectiveChecker::new()));

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
                let context = CheckInternalContext::new(
                    &project_root,
                    source_root,
                    &source_roots,
                    project_config,
                    &module_tree,
                    &found_imports,
                );
                pipeline
                    .diagnostics(&file_path, &context)
                    .unwrap_or_default()
            })
    });

    if check_interrupt().is_err() {
        return Err(CheckError::Interrupt);
    }

    let mut final_diagnostics: Vec<Diagnostic> = diagnostics.collect();
    if !found_imports.load(Ordering::Relaxed) {
        final_diagnostics.push(Diagnostic::new_global_warning(
            DiagnosticDetails::Configuration(ConfigurationDiagnostic::NoFirstPartyImportsFound()),
        ));
    }

    Ok(final_diagnostics)
}
