use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::config::{ProjectConfig, RuleSetting};
use crate::external::parsing::ProjectInfo;
use crate::external::{parsing::normalize_package_name, parsing::parse_pyproject_toml};
use crate::filesystem::relative_to;
use crate::imports::NormalizedImport;
use crate::{filesystem, imports};

use super::checks::check_missing_ignore_directive_reason;
use super::diagnostics::ExternalCheckDiagnostics;
use super::error::ExternalCheckError;
pub type Result<T> = std::result::Result<T, ExternalCheckError>;

#[derive(Debug)]
enum ImportProcessResult {
    UndeclaredDependency(String),
    UsedDependencies(Vec<String>),
    Excluded(Vec<String>),
}

pub fn check(
    project_root: &Path,
    project_config: &ProjectConfig,
    module_mappings: &HashMap<String, Vec<String>>,
    stdlib_modules: &[String],
) -> Result<ExternalCheckDiagnostics> {
    let stdlib_modules: HashSet<String> = stdlib_modules.iter().cloned().collect();
    let excluded_external_modules: HashSet<String> =
        project_config.external.exclude.iter().cloned().collect();
    let source_roots: Vec<PathBuf> = project_config.prepend_roots(project_root);
    let mut diagnostics = ExternalCheckDiagnostics::default();
    for pyproject in filesystem::walk_pyprojects(project_root.to_str().unwrap()) {
        let project_info = parse_pyproject_toml(&pyproject)?;
        let mut all_dependencies = project_info.dependencies.clone();
        for source_root in &project_info.source_paths {
            for file_path in filesystem::walk_pyfiles(source_root.to_str().unwrap()) {
                let absolute_file_path = source_root.join(&file_path);
                let display_file_path = relative_to(&absolute_file_path, project_root)?
                    .display()
                    .to_string();

                if let Ok(project_imports) = imports::get_external_imports(
                    &source_roots,
                    &absolute_file_path,
                    project_config.ignore_type_checking_imports,
                ) {
                    for import in project_imports.active_imports() {
                        match process_import(
                            import,
                            &project_info,
                            module_mappings,
                            &excluded_external_modules,
                            &stdlib_modules,
                        ) {
                            ImportProcessResult::UndeclaredDependency(module_name) => {
                                let undeclared_dep_entry: &mut Vec<String> = diagnostics
                                    .undeclared_dependencies
                                    .entry(display_file_path.to_string())
                                    .or_default();
                                undeclared_dep_entry.push(module_name);
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
                        if project_config.rules.require_ignore_directive_reasons != RuleSetting::Off
                        {
                            if let Err(e) =
                                check_missing_ignore_directive_reason(&directive_ignored_import)
                            {
                                match &project_config.rules.require_ignore_directive_reasons {
                                    RuleSetting::Error => {
                                        diagnostics.errors.push(format!(
                                            "{}:{}: {}",
                                            display_file_path,
                                            directive_ignored_import.import.line_no,
                                            e
                                        ));
                                    }
                                    RuleSetting::Warn => {
                                        diagnostics.warnings.push(format!(
                                            "{}:{}: {}",
                                            display_file_path,
                                            directive_ignored_import.import.line_no,
                                            e
                                        ));
                                    }
                                    RuleSetting::Off => {}
                                }
                            }
                        }

                        if project_config.rules.unused_ignore_directives != RuleSetting::Off {
                            if let ImportProcessResult::UsedDependencies(_)
                            | ImportProcessResult::Excluded(_) = process_import(
                                directive_ignored_import.import,
                                &project_info,
                                module_mappings,
                                &excluded_external_modules,
                                &stdlib_modules,
                            ) {
                                match project_config.rules.unused_ignore_directives {
                                    RuleSetting::Error => {
                                        diagnostics.errors.push(format!(
                                            "{}:{}: Unused ignore directive: '{}'",
                                            display_file_path,
                                            directive_ignored_import.import.line_no,
                                            directive_ignored_import.import.top_level_module_name()
                                        ));
                                    }
                                    RuleSetting::Warn => {
                                        diagnostics.warnings.push(format!(
                                            "{}:{}: Unused ignore directive: '{}'",
                                            display_file_path,
                                            directive_ignored_import.import.line_no,
                                            directive_ignored_import.import.top_level_module_name()
                                        ));
                                    }
                                    RuleSetting::Off => {}
                                }
                            }
                        }
                    }

                    for unused_directive in project_imports.unused_ignore_directives() {
                        match project_config.rules.unused_ignore_directives {
                            RuleSetting::Error => {
                                diagnostics.errors.push(format!(
                                    "{}:{}: Unused ignore directive: '{}'",
                                    display_file_path,
                                    unused_directive.line_no,
                                    unused_directive.modules.join(",")
                                ));
                            }
                            RuleSetting::Warn => {
                                diagnostics.warnings.push(format!(
                                    "{}:{}: Unused ignore directive: '{}'",
                                    display_file_path,
                                    unused_directive.line_no,
                                    unused_directive.modules.join(",")
                                ));
                            }
                            RuleSetting::Off => {}
                        }
                    }
                }
            }
        }

        diagnostics.unused_dependencies.insert(
            relative_to(&pyproject, project_root)?
                .to_string_lossy()
                .to_string(),
            all_dependencies
                .into_iter()
                .filter(|dep| !excluded_external_modules.contains(dep)) // 'exclude' should hide unused errors unconditionally
                .collect(),
        );
    }

    Ok(diagnostics)
}

fn process_import(
    import: &NormalizedImport,
    project_info: &ProjectInfo,
    module_mappings: &HashMap<String, Vec<String>>,
    excluded_external_modules: &HashSet<String>,
    stdlib_modules: &HashSet<String>,
) -> ImportProcessResult {
    let top_level_module_name = import.top_level_module_name().to_string();
    let default_distribution_names = vec![top_level_module_name.clone()];
    let distribution_names: Vec<String> = module_mappings
        .get(&top_level_module_name)
        .unwrap_or(&default_distribution_names)
        .iter()
        .map(|dist_name| normalize_package_name(dist_name))
        .collect();

    if distribution_names
        .iter()
        .any(|dist_name| excluded_external_modules.contains(dist_name))
        || stdlib_modules.contains(&top_level_module_name)
    {
        return ImportProcessResult::Excluded(distribution_names);
    }

    let is_declared = distribution_names
        .iter()
        .any(|dist_name| project_info.dependencies.contains(dist_name));

    if !is_declared {
        ImportProcessResult::UndeclaredDependency(top_level_module_name.to_string())
    } else {
        ImportProcessResult::UsedDependencies(distribution_names)
    }
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
