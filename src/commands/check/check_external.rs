use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::config::ProjectConfig;
use crate::external::parsing::parse_pyproject_toml;
use crate::{filesystem, imports};

use super::checks::{
    check_import_external, check_missing_ignore_directive_reason,
    check_unused_ignore_directive_external, ImportProcessResult,
};
use super::diagnostics::{CodeDiagnostic, Diagnostic, DiagnosticDetails};
use super::error::ExternalCheckError;
pub type Result<T> = std::result::Result<T, ExternalCheckError>;

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
    let mut diagnostics = vec![];
    for pyproject in filesystem::walk_pyprojects(project_root.to_str().unwrap()) {
        let project_info = parse_pyproject_toml(&pyproject)?;
        let mut all_dependencies = project_info.dependencies.clone();
        for source_root in &project_info.source_paths {
            for file_path in filesystem::walk_pyfiles(source_root.to_str().unwrap()) {
                let absolute_file_path = source_root.join(&file_path);

                if let Ok(project_imports) = imports::get_external_imports(
                    &source_roots,
                    &absolute_file_path,
                    project_config.ignore_type_checking_imports,
                ) {
                    for import in project_imports.active_imports() {
                        match check_import_external(
                            import,
                            &project_info,
                            module_mappings,
                            &excluded_external_modules,
                            &stdlib_modules,
                        ) {
                            ImportProcessResult::UndeclaredDependency(module_name) => {
                                diagnostics.push(Diagnostic::new_located_error(
                                    absolute_file_path.clone(),
                                    import.import_line_no,
                                    DiagnosticDetails::Code(
                                        CodeDiagnostic::UndeclaredExternalDependency {
                                            import_mod_path: module_name,
                                        },
                                    ),
                                ));
                            }
                            ImportProcessResult::UsedDependencies(deps)
                            | ImportProcessResult::Excluded(deps) => {
                                for dep in deps {
                                    all_dependencies.remove(&dep);
                                }
                            }
                        }
                    }

                    for directive_ignored_import in project_imports.directive_ignored_imports() {
                        match check_missing_ignore_directive_reason(
                            &directive_ignored_import,
                            project_config,
                        ) {
                            Ok(()) => {}
                            Err(diagnostic) => {
                                diagnostics.push(diagnostic.into_located(
                                    file_path.clone(),
                                    directive_ignored_import.import.line_no,
                                ));
                            }
                        }

                        match check_unused_ignore_directive_external(
                            &directive_ignored_import,
                            &project_info,
                            module_mappings,
                            &excluded_external_modules,
                            &stdlib_modules,
                            project_config,
                        ) {
                            Ok(()) => {}
                            Err(diagnostic) => {
                                diagnostics.push(diagnostic.into_located(
                                    file_path.clone(),
                                    directive_ignored_import.import.line_no,
                                ));
                            }
                        }
                    }

                    for unused_directive in project_imports.unused_ignore_directives() {
                        if let Ok(severity) =
                            (&project_config.rules.unused_ignore_directives).try_into()
                        {
                            diagnostics.push(Diagnostic::new_located(
                                severity,
                                DiagnosticDetails::Code(CodeDiagnostic::UnusedIgnoreDirective()),
                                file_path.clone(),
                                unused_directive.line_no,
                            ));
                        }
                    }
                }
            }
        }

        diagnostics.extend(
            all_dependencies
                .into_iter()
                .filter(|dep| !excluded_external_modules.contains(dep)) // 'exclude' should hide unused errors unconditionally
                .map(|dep| {
                    Diagnostic::new_global_error(DiagnosticDetails::Code(
                        CodeDiagnostic::UnusedExternalDependency {
                            package_module_name: dep,
                        },
                    ))
                })
                .collect::<Vec<_>>(),
        );
    }

    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use crate::config::ProjectConfig;
    use crate::tests::fixtures::example_dir;

    use super::*;
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
        let result = check(&project_root, &project_config, &module_mapping, &[]);
        assert!(result.as_ref().unwrap().undeclared_dependencies.is_empty());
        let unused_dependency_root = "src/pack-a/pyproject.toml";
        assert!(result
            .unwrap()
            .unused_dependencies
            .contains_key(unused_dependency_root));
    }

    #[rstest]
    fn check_external_dependencies_invalid_multi_package_example(
        example_dir: PathBuf,
        project_config: ProjectConfig,
    ) {
        let project_root = example_dir.join("multi_package");
        let result = check(&project_root, &project_config, &HashMap::new(), &[]);
        let expected_failure_path = "src/pack-a/src/myorg/pack_a/__init__.py";
        let r = result.unwrap();
        assert_eq!(
            r.undeclared_dependencies.keys().collect::<Vec<_>>(),
            vec![expected_failure_path]
        );
        assert_eq!(
            r.undeclared_dependencies[expected_failure_path],
            vec!["git"]
        );
    }
}
