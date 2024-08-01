use std::io;
use std::path::{Path, PathBuf};

use pyo3::conversion::IntoPy;
use pyo3::PyObject;
use thiserror::Error;

use crate::{filesystem, imports, parsing};

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("Parsing error: {0}")]
    Parse(#[from] parsing::ParsingError),
    #[error("Import parsing error: {0}")]
    ImportParse(#[from] imports::ImportParseError),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, CheckError>;

pub struct ExternalCheckDiagnostics {
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl IntoPy<PyObject> for ExternalCheckDiagnostics {
    fn into_py(self, py: pyo3::prelude::Python<'_>) -> PyObject {
        (self.errors, self.warnings).into_py(py)
    }
}

pub fn check_external_dependencies(
    project_root: &Path,
    source_roots: &[PathBuf],
    ignore_type_checking_imports: bool,
) -> Result<ExternalCheckDiagnostics> {
    let mut errors: Vec<String> = Vec::new();
    let warnings: Vec<String> = Vec::new();
    for pyproject in filesystem::walk_pyprojects(project_root.to_str().unwrap()) {
        println!("Checking {}", pyproject.display());
        // todo: error handling
        let project_info = parsing::parse_pyproject_toml(&pyproject)?;
        println!("Dependencies: {:?}", project_info.dependencies);
        println!("Source paths: {:?}", project_info.source_paths);
        for source_root in &project_info.source_paths {
            let source_files = filesystem::walk_pyfiles(source_root.to_str().unwrap());
            for file_path in source_files {
                if let Ok(imports) = imports::get_normalized_imports(
                    source_roots,
                    &source_root.join(&file_path),
                    ignore_type_checking_imports,
                ) {
                    println!("Imports: {:?}", imports);
                    for import in imports {
                        if !imports::is_project_import(
                            project_root,
                            source_roots,
                            &import.module_path,
                        )? && !project_info.dependencies.contains(import.package_name())
                        {
                            errors.push(format!(
                                "External dependency '{}' found in {}",
                                import.module_path.as_str(),
                                file_path.display()
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(ExternalCheckDiagnostics { errors, warnings })
}
