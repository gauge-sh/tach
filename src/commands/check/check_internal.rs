use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use rayon::prelude::*;

use super::error::CheckError;
use crate::{
    checks::{IgnoreDirectivePostProcessor, InterfaceChecker, InternalDependencyChecker},
    config::ProjectConfig,
    diagnostics::{
        ConfigurationDiagnostic, Diagnostic, DiagnosticDetails, DiagnosticError,
        DiagnosticPipeline, FileChecker, FileProcessor, Result as DiagnosticResult,
    },
    exclusion::PathExclusions,
    filesystem::{self as fs, ProjectFile},
    interrupt::check_interrupt,
    modules::{build_module_tree, ModuleTree},
    processors::{FileModule, InternalDependencyExtractor},
};

pub type Result<T> = std::result::Result<T, CheckError>;

struct CheckInternalPipeline<'a> {
    found_imports: &'a AtomicBool,
    dependency_extractor: InternalDependencyExtractor<'a>,
    dependency_checker: Option<InternalDependencyChecker<'a>>,
    interface_checker: Option<InterfaceChecker<'a>>,
    ignore_directive_post_processor: IgnoreDirectivePostProcessor<'a>,
}

impl<'a> CheckInternalPipeline<'a> {
    pub fn new(
        project_config: &'a ProjectConfig,
        source_roots: &'a [PathBuf],
        module_tree: &'a ModuleTree,
        exclusions: &'a PathExclusions,
        found_imports: &'a AtomicBool,
    ) -> Self {
        Self {
            found_imports,
            dependency_extractor: InternalDependencyExtractor::new(
                source_roots,
                module_tree,
                project_config,
                exclusions,
            ),
            dependency_checker: None,
            interface_checker: None,
            ignore_directive_post_processor: IgnoreDirectivePostProcessor::new(project_config),
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
}

impl<'a> FileProcessor<'a, ProjectFile<'a>> for CheckInternalPipeline<'a> {
    type ProcessedFile = FileModule<'a>;

    fn process(&'a self, file_path: ProjectFile<'a>) -> DiagnosticResult<Self::ProcessedFile> {
        let file_module = self.dependency_extractor.process(file_path)?;

        if file_module.imports().peekable().peek().is_some()
            && !self.found_imports.load(Ordering::Relaxed)
        {
            // Only attempt to write if we haven't found imports yet.
            // This avoids any potential lock contention.
            self.found_imports.store(true, Ordering::Relaxed);
        }

        Ok(file_module)
    }
}

impl<'a> FileChecker<'a> for CheckInternalPipeline<'a> {
    type ProcessedFile = FileModule<'a>;
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

        self.ignore_directive_post_processor.process_diagnostics(
            &processed_file.ignore_directives,
            &mut diagnostics,
            processed_file.relative_file_path(),
        );

        Ok(diagnostics)
    }
}

pub fn check(
    project_root: PathBuf,
    project_config: &ProjectConfig,
    dependencies: bool,
    interfaces: bool,
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

    let dependency_checker = if dependencies {
        Some(InternalDependencyChecker::new(project_config, &module_tree))
    } else {
        None
    };

    let interface_checker = if interfaces {
        let interface_checker = InterfaceChecker::new(project_config, &module_tree);
        // This is expensive
        Some(interface_checker.with_type_check_cache(&valid_modules, &source_roots)?)
    } else {
        None
    };

    let exclusions = PathExclusions::new(
        &project_root,
        &project_config.exclude,
        project_config.use_regex_matching,
    )?;
    let pipeline = CheckInternalPipeline::new(
        project_config,
        &source_roots,
        &module_tree,
        &exclusions,
        &found_imports,
    )
    .with_dependency_checker(dependency_checker)
    .with_interface_checker(interface_checker);

    let diagnostics = source_roots.par_iter().flat_map(|source_root| {
        fs::walk_pyfiles(&source_root.display().to_string(), &exclusions)
            .par_bridge()
            .flat_map(|file_path| {
                if check_interrupt().is_err() {
                    // Since files are being processed in parallel,
                    // this will essentially short-circuit all remaining files.
                    // Then, we check for an interrupt right after, and return the Err if it is set
                    return vec![];
                }

                let project_file =
                    match ProjectFile::try_new(&project_root, source_root, &file_path) {
                        Ok(project_file) => project_file,
                        Err(_) => {
                            return vec![Diagnostic::new_global_warning(
                                DiagnosticDetails::Configuration(
                                    ConfigurationDiagnostic::SkippedFileIoError {
                                        file_path: file_path.display().to_string(),
                                    },
                                ),
                            )]
                        }
                    };

                match pipeline.diagnostics(project_file) {
                    Ok(diagnostics) => diagnostics,
                    Err(DiagnosticError::Io(_)) | Err(DiagnosticError::Filesystem(_)) => {
                        vec![Diagnostic::new_global_warning(
                            DiagnosticDetails::Configuration(
                                ConfigurationDiagnostic::SkippedFileIoError {
                                    file_path: file_path.display().to_string(),
                                },
                            ),
                        )]
                    }
                    Err(DiagnosticError::ImportParse(_)) => {
                        vec![Diagnostic::new_global_warning(
                            DiagnosticDetails::Configuration(
                                ConfigurationDiagnostic::SkippedFileSyntaxError {
                                    file_path: file_path.display().to_string(),
                                },
                            ),
                        )]
                    }
                    Err(_) => vec![Diagnostic::new_global_warning(
                        DiagnosticDetails::Configuration(
                            ConfigurationDiagnostic::SkippedUnknownError {
                                file_path: file_path.display().to_string(),
                            },
                        ),
                    )],
                }
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
