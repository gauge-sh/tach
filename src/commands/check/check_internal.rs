use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use rayon::prelude::*;

use super::error::CheckError;
use crate::{
    checks::{IgnoreDirectiveChecker, InterfaceChecker, InternalDependencyChecker},
    config::{root_module::RootModuleTreatment, ProjectConfig},
    diagnostics::{
        ConfigurationDiagnostic, Diagnostic, DiagnosticDetails, DiagnosticError,
        DiagnosticPipeline, FileChecker, FileProcessor, Result as DiagnosticResult,
    },
    exclusion::set_excluded_paths,
    filesystem::{self as fs},
    interrupt::check_interrupt,
    modules::{build_module_tree, error::ModuleTreeError, ModuleTree},
    processors::imports::{get_project_imports, NormalizedImports},
    processors::internal_file::{InternalFile, ProcessedInternalFile},
};

pub type Result<T> = std::result::Result<T, CheckError>;

struct CheckInternalPipeline<'a> {
    _project_root: &'a Path,
    project_config: &'a ProjectConfig,
    source_roots: &'a [PathBuf],
    module_tree: &'a ModuleTree,
    found_imports: &'a AtomicBool,
    dependency_checker: Option<InternalDependencyChecker<'a>>,
    interface_checker: Option<InterfaceChecker<'a>>,
    ignore_directive_checker: Option<IgnoreDirectiveChecker<'a>>,
}

impl<'a> CheckInternalPipeline<'a> {
    pub fn new(
        project_root: &'a Path,
        project_config: &'a ProjectConfig,
        source_roots: &'a [PathBuf],
        module_tree: &'a ModuleTree,
        found_imports: &'a AtomicBool,
    ) -> Self {
        Self {
            _project_root: project_root,
            project_config,
            source_roots,
            module_tree,
            found_imports,
            dependency_checker: None,
            interface_checker: None,
            ignore_directive_checker: None,
        }
    }

    pub fn with_dependency_checker(
        mut self,
        dependency_checker: Option<InternalDependencyChecker<'a>>,
    ) -> Self {
        self.dependency_checker = dependency_checker;
        self
    }

    pub fn with_interface_checker(
        mut self,
        interface_checker: Option<InterfaceChecker<'a>>,
    ) -> Self {
        self.interface_checker = interface_checker;
        self
    }

    pub fn with_ignore_directive_checker(
        mut self,
        ignore_directive_checker: Option<IgnoreDirectiveChecker<'a>>,
    ) -> Self {
        self.ignore_directive_checker = ignore_directive_checker;
        self
    }
}

impl<'a> FileProcessor<'a, InternalFile<'a>> for CheckInternalPipeline<'a> {
    type ProcessedFile = ProcessedInternalFile<'a>;

    fn process(&'a self, file_path: InternalFile<'a>) -> DiagnosticResult<Self::ProcessedFile> {
        let mod_path = fs::file_to_module_path(self.source_roots, file_path.as_ref())?;
        let file_module =
            self.module_tree
                .find_nearest(&mod_path)
                .ok_or(DiagnosticError::ModuleTree(
                    ModuleTreeError::ModuleNotFound(mod_path.to_string()),
                ))?;

        if file_module.is_unchecked() {
            return Ok(ProcessedInternalFile::new(
                file_path,
                file_module,
                NormalizedImports::empty(),
            ));
        }

        if file_module.is_root() && self.project_config.root_module == RootModuleTreatment::Ignore {
            return Ok(ProcessedInternalFile::new(
                file_path,
                file_module,
                NormalizedImports::empty(),
            ));
        }

        let project_imports = get_project_imports(
            self.source_roots,
            file_path.as_ref(),
            self.project_config.ignore_type_checking_imports,
            self.project_config.include_string_imports,
        )?;

        if !project_imports.imports.is_empty() && !self.found_imports.load(Ordering::Relaxed) {
            // Only attempt to write if we haven't found imports yet.
            // This avoids any potential lock contention.
            self.found_imports.store(true, Ordering::Relaxed);
        }

        Ok(ProcessedInternalFile::new(
            file_path,
            file_module,
            project_imports,
        ))
    }
}

impl<'a> FileChecker<'a> for CheckInternalPipeline<'a> {
    type ProcessedFile = ProcessedInternalFile<'a>;
    type Output = Vec<Diagnostic>;

    fn check(&'a self, processed_file: &Self::ProcessedFile) -> DiagnosticResult<Self::Output> {
        let mut diagnostics = Vec::new();
        diagnostics.extend(
            self.dependency_checker
                .as_ref()
                .map_or(Ok(vec![]), |checker| checker.check(processed_file))?,
        );

        diagnostics.extend(
            self.interface_checker
                .as_ref()
                .map_or(Ok(vec![]), |checker| checker.check(processed_file))?,
        );

        diagnostics.extend(
            self.ignore_directive_checker
                .as_ref()
                .map_or(vec![], |checker| {
                    checker.check(
                        &processed_file.project_imports.ignore_directives,
                        &diagnostics,
                        processed_file.relative_file_path(),
                    )
                }),
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
        Some(InternalDependencyChecker::new(project_config, &module_tree))
    } else {
        None
    };

    let interface_checker = if interfaces {
        let interface_checker = InterfaceChecker::new(
            project_config,
            &module_tree,
            &project_config.all_interfaces().cloned().collect::<Vec<_>>(),
        );
        // This is expensive
        Some(interface_checker.with_type_check_cache(&valid_modules, &source_roots)?)
    } else {
        None
    };

    let pipeline = CheckInternalPipeline::new(
        &project_root,
        project_config,
        &source_roots,
        &module_tree,
        &found_imports,
    )
    .with_dependency_checker(dependency_checker)
    .with_interface_checker(interface_checker)
    .with_ignore_directive_checker(Some(IgnoreDirectiveChecker::new(project_config)));

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

                let internal_file = InternalFile::new(&project_root, source_root, &file_path);
                pipeline.diagnostics(internal_file).unwrap_or_default()
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
