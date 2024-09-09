use pyo3::conversion::IntoPy;
use pyo3::PyObject;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::filesystem::relative_to;
use crate::parsing::external::{normalize_package_name, parse_pyproject_toml};
use crate::{filesystem, imports, parsing};

#[derive(Error, Debug)]
pub enum ExternalCheckError {
    #[error("Parsing error: {0}")]
    Parse(#[from] parsing::error::ParsingError),
    #[error("Import parsing error: {0}")]
    ImportParse(#[from] imports::ImportParseError),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] filesystem::FileSystemError),
}

pub type Result<T> = std::result::Result<T, ExternalCheckError>;

#[derive(Default)]
pub struct ExternalCheckDiagnostics {
    // Undeclared dependencies by source filepath
    undeclared_dependencies: HashMap<String, Vec<String>>,
    // Unused dependencies by project configuration filepath
    unused_dependencies: HashMap<String, Vec<String>>,
}

impl IntoPy<PyObject> for ExternalCheckDiagnostics {
    fn into_py(self, py: pyo3::prelude::Python<'_>) -> PyObject {
        (self.undeclared_dependencies, self.unused_dependencies).into_py(py)
    }
}

pub fn check_external_dependencies(
    project_root: &Path,
    source_roots: &[PathBuf],
    module_mappings: &HashMap<String, Vec<String>>,
    ignore_type_checking_imports: bool,
) -> Result<ExternalCheckDiagnostics> {
    let mut diagnostics = ExternalCheckDiagnostics::default();
    for pyproject in filesystem::walk_pyprojects(project_root.to_str().unwrap()) {
        let project_info = parse_pyproject_toml(&pyproject)?;
        let mut all_dependencies = project_info.dependencies.clone();
        for source_root in &project_info.source_paths {
            let source_files = filesystem::walk_pyfiles(source_root.to_str().unwrap());
            for file_path in source_files {
                let absolute_file_path = source_root.join(&file_path);
                let display_file_path = relative_to(&absolute_file_path, project_root)?
                    .to_string_lossy()
                    .to_string();
                if let Ok(imports) = imports::get_normalized_imports(
                    source_roots,
                    &absolute_file_path,
                    ignore_type_checking_imports,
                ) {
                    for import in imports {
                        let top_level_module_name = import.top_level_module_name();
                        let default_distribution_names = vec![top_level_module_name.to_string()];
                        let distribution_names: Vec<String> = module_mappings
                            .get(top_level_module_name)
                            .unwrap_or(&default_distribution_names)
                            .iter()
                            .map(|dist_name| normalize_package_name(dist_name))
                            .collect();
                        for dist_name in distribution_names.iter() {
                            all_dependencies.remove(dist_name);
                        }
                        if !imports::is_project_import(source_roots, &import.module_path)?
                            && distribution_names
                                .iter()
                                .all(|dist_name| !project_info.dependencies.contains(dist_name))
                        {
                            // Found a possible undeclared dependency in this file
                            let undeclared_dep_entry: &mut Vec<String> = diagnostics
                                .undeclared_dependencies
                                .entry(display_file_path.clone())
                                .or_default();
                            undeclared_dep_entry.push(top_level_module_name.to_string());
                        }
                    }
                }
            }
        }
        diagnostics.unused_dependencies.insert(
            relative_to(&pyproject, project_root)?
                .to_string_lossy()
                .to_string(),
            all_dependencies.into_iter().collect(),
        );
    }

    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use crate::core::config::ProjectConfig;
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
        let result = check_external_dependencies(
            &project_root,
            &project_config.source_roots,
            &module_mapping,
            project_config.ignore_type_checking_imports,
        );
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
        let result = check_external_dependencies(
            &project_root,
            &project_config.source_roots,
            &HashMap::new(),
            project_config.ignore_type_checking_imports,
        );
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
