use crate::checks::{ExternalDependencyChecker, IgnoreDirectivePostProcessor};
use crate::commands::helpers::import::get_external_imports;
use crate::config::ProjectConfig;
use crate::diagnostics::{
    CodeDiagnostic, ConfigurationDiagnostic, Diagnostic, DiagnosticDetails, DiagnosticError,
    DiagnosticPipeline, FileChecker, FileProcessor, Result as DiagnosticResult,
};
use crate::external::parsing::{parse_pyproject_toml, ProjectInfo};
use crate::filesystem::{walk_pyfiles, walk_pyprojects, ProjectFile};
use crate::interrupt::check_interrupt;
use crate::modules::ModuleNode;
use crate::processors::file_module::FileModule;
use crate::processors::import::with_distribution_names;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashSet;
use rayon::prelude::*;

use super::error::CheckError;

pub type Result<T> = std::result::Result<T, CheckError>;

struct CheckExternalPipeline<'a> {
    source_roots: &'a [PathBuf],
    project_config: &'a ProjectConfig,
    module_mappings: &'a HashMap<String, Vec<String>>,
    excluded_external_modules: &'a HashSet<String>,
    seen_dependencies: DashSet<String>,
    external_dependency_checker: ExternalDependencyChecker<'a>,
    ignore_directive_post_processor: IgnoreDirectivePostProcessor<'a>,
}

impl<'a> CheckExternalPipeline<'a> {
    pub fn new(
        source_roots: &'a [PathBuf],
        project_config: &'a ProjectConfig,
        project_info: &'a ProjectInfo,
        module_mappings: &'a HashMap<String, Vec<String>>,
        stdlib_modules: &'a HashSet<String>,
        excluded_external_modules: &'a HashSet<String>,
    ) -> Self {
        Self {
            source_roots,
            project_config,
            module_mappings,
            excluded_external_modules,
            seen_dependencies: DashSet::new(),
            external_dependency_checker: ExternalDependencyChecker::new(
                project_info,
                module_mappings,
                stdlib_modules,
                excluded_external_modules,
            ),
            ignore_directive_post_processor: IgnoreDirectivePostProcessor::new(project_config),
        }
    }
}

impl<'a> FileProcessor<'a, ProjectFile<'a>> for CheckExternalPipeline<'a> {
    type ProcessedFile = FileModule<'a>;

    fn process(&'a self, file_path: ProjectFile<'a>) -> DiagnosticResult<Self::ProcessedFile> {
        // NOTE: check-external does not currently make use of the module tree,
        // but it is very likely to do so in the future.
        let file_module = Arc::new(ModuleNode::empty());

        let external_imports = get_external_imports(
            self.source_roots,
            file_path.as_ref(),
            self.project_config.ignore_type_checking_imports,
        )?;

        // Track all external dependencies seen in imports
        with_distribution_names(external_imports.iter(), self.module_mappings)
            .into_iter()
            .for_each(|import| {
                import
                    .distribution_names
                    .iter()
                    .for_each(|distribution_name| {
                        self.seen_dependencies.insert(distribution_name.clone());
                    });
            });

        Ok(FileModule::new(file_path, file_module))
    }
}

impl<'a> FileChecker<'a> for CheckExternalPipeline<'a> {
    type ProcessedFile = FileModule<'a>;
    type Output = Vec<Diagnostic>;

    fn check(&'a self, processed_file: &Self::ProcessedFile) -> DiagnosticResult<Self::Output> {
        let mut diagnostics = Vec::new();
        diagnostics.extend(self.external_dependency_checker.check(processed_file)?);

        self.ignore_directive_post_processor.process_diagnostics(
            &processed_file.ignore_directives,
            &mut diagnostics,
            processed_file.relative_file_path(),
        );

        Ok(diagnostics)
    }
}

pub fn check(
    project_root: &Path,
    project_config: &ProjectConfig,
    module_mappings: &HashMap<String, Vec<String>>,
    stdlib_modules: &[String],
) -> Result<Vec<Diagnostic>> {
    let stdlib_modules: HashSet<String> = stdlib_modules.iter().cloned().collect();
    let excluded_external_modules: HashSet<String> =
        project_config.external.exclude.iter().cloned().collect();
    let source_roots: Vec<PathBuf> = project_config.prepend_roots(project_root);

    let diagnostics = walk_pyprojects(project_root.to_string_lossy().as_ref())
        .par_bridge()
        .flat_map(|pyproject| {
            let project_info = match parse_pyproject_toml(&pyproject) {
                Ok(project_info) => project_info,
                Err(_) => {
                    return vec![Diagnostic::new_global_error(
                        DiagnosticDetails::Configuration(
                            ConfigurationDiagnostic::SkippedPyProjectParsingError {
                                file_path: pyproject.to_string_lossy().to_string(),
                            },
                        ),
                    )];
                }
            };
            let pipeline = CheckExternalPipeline::new(
                &source_roots,
                project_config,
                &project_info,
                module_mappings,
                &stdlib_modules,
                &excluded_external_modules,
            );
            let mut project_diagnostics: Vec<Diagnostic> = project_info
                .source_paths
                .par_iter()
                .flat_map(|source_root| {
                    walk_pyfiles(&source_root.display().to_string())
                        .par_bridge()
                        .flat_map(|file_path| {
                            if check_interrupt().is_err() {
                                // Since files are being processed in parallel,
                                // this will essentially short-circuit all remaining files.
                                // Then, we check for an interrupt right after, and return the Err if it is set
                                return vec![];
                            }

                            match pipeline.diagnostics(ProjectFile::new(
                                project_root,
                                source_root,
                                &file_path,
                            )) {
                                Ok(diagnostics) => diagnostics,
                                Err(DiagnosticError::Io(_))
                                | Err(DiagnosticError::Filesystem(_)) => {
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
                })
                .collect();

            let all_seen_dependencies: HashSet<String> =
                pipeline.seen_dependencies.into_iter().collect();
            let unused_dependency_diagnostics = project_info
                .dependencies
                .difference(&all_seen_dependencies)
                .filter(|&dep| !pipeline.excluded_external_modules.contains(dep)) // 'exclude' should hide unused errors unconditionally
                .map(|dep| {
                    Diagnostic::new_global_error(DiagnosticDetails::Code(
                        CodeDiagnostic::UnusedExternalDependency {
                            package_module_name: dep.clone(),
                        },
                    ))
                });

            project_diagnostics.extend(unused_dependency_diagnostics);
            project_diagnostics
        });

    if check_interrupt().is_err() {
        return Err(CheckError::Interrupt);
    }

    Ok(diagnostics.collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProjectConfig;
    use crate::diagnostics::Severity;
    use crate::tests::fixtures::example_dir;
    use rstest::*;

    #[fixture]
    fn project_config() -> ProjectConfig {
        ProjectConfig {
            source_roots: [
                "src/pack-a/src",
                "src/pack-b/src",
                "src/pack-c/src",
                "src/pack-d/src",
                "src/pack-e/src",
                "src/pack-f/src",
                "src/pack-g/src",
            ]
            .iter()
            .map(PathBuf::from)
            .collect(),
            ignore_type_checking_imports: true,
            ..Default::default()
        }
    }

    #[fixture]
    fn module_mapping() -> HashMap<String, Vec<String>> {
        HashMap::from([("git".to_string(), vec!["gitpython".to_string()])])
    }

    #[rstest]
    fn check_external_dependencies_multi_package_example(
        example_dir: PathBuf,
        project_config: ProjectConfig,
        module_mapping: HashMap<String, Vec<String>>,
    ) {
        let project_root = example_dir.join("multi_package");
        let result = check(&project_root, &project_config, &module_mapping, &[]).unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0],
            Diagnostic::Global {
                severity: Severity::Error,
                details: DiagnosticDetails::Code(CodeDiagnostic::UnusedExternalDependency { .. })
            }
        ));
        assert_eq!(
            result[0].details(),
            &DiagnosticDetails::Code(CodeDiagnostic::UnusedExternalDependency {
                package_module_name: "unused".to_string()
            })
        );
    }

    #[rstest]
    fn check_external_dependencies_invalid_multi_package_example(
        example_dir: PathBuf,
        project_config: ProjectConfig,
    ) {
        let project_root = example_dir.join("multi_package");
        let result = check(&project_root, &project_config, &HashMap::new(), &[]).unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.iter().any(|d| d.details()
            == &DiagnosticDetails::Code(CodeDiagnostic::UndeclaredExternalDependency {
                import_mod_path: "git".to_string()
            })));
        assert!(result.iter().any(|d| d.details()
            == &DiagnosticDetails::Code(CodeDiagnostic::UnusedExternalDependency {
                package_module_name: "gitpython".to_string()
            })));
        assert!(result.iter().any(|d| d.details()
            == &DiagnosticDetails::Code(CodeDiagnostic::UnusedExternalDependency {
                package_module_name: "unused".to_string()
            })));
    }
}
