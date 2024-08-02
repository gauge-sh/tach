use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

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

pub type ExternalCheckDiagnostics = HashMap<String, Vec<String>>;

pub fn check_external_dependencies(
    project_root: &Path,
    source_roots: &[PathBuf],
    ignore_type_checking_imports: bool,
) -> Result<ExternalCheckDiagnostics> {
    let mut diagnostics: ExternalCheckDiagnostics = HashMap::new();
    for pyproject in filesystem::walk_pyprojects(project_root.to_str().unwrap()) {
        let project_info = parsing::parse_pyproject_toml(&pyproject)?;
        for source_root in &project_info.source_paths {
            let source_files = filesystem::walk_pyfiles(source_root.to_str().unwrap());
            for file_path in source_files {
                if let Ok(imports) = imports::get_normalized_imports(
                    source_roots,
                    &source_root.join(&file_path),
                    ignore_type_checking_imports,
                ) {
                    for import in imports {
                        if !imports::is_project_import(
                            project_root,
                            source_roots,
                            &import.module_path,
                        )? && !project_info.dependencies.contains(import.package_name())
                        {
                            let diagnostic = diagnostics
                                .entry(file_path.to_string_lossy().to_string())
                                .or_default();
                            diagnostic.push(import.package_name().to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(diagnostics)
}
