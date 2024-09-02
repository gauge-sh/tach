use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::filesystem::relative_to;
use crate::parsing::external::{normalize_package_name, parse_pyproject_toml};
use crate::{filesystem, imports, parsing};

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("Parsing error: {0}")]
    Parse(#[from] parsing::error::ParsingError),
    #[error("Import parsing error: {0}")]
    ImportParse(#[from] imports::ImportParseError),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] filesystem::FileSystemError),
}

pub type Result<T> = std::result::Result<T, CheckError>;

pub type ExternalCheckDiagnostics = HashMap<String, Vec<String>>;

pub fn check_external_dependencies(
    project_root: &Path,
    source_roots: &[PathBuf],
    module_mappings: &HashMap<String, Vec<String>>,
    ignore_type_checking_imports: bool,
) -> Result<ExternalCheckDiagnostics> {
    let mut diagnostics: ExternalCheckDiagnostics = HashMap::new();
    for pyproject in filesystem::walk_pyprojects(project_root.to_str().unwrap()) {
        let project_info = parse_pyproject_toml(&pyproject)?;
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
                        if !imports::is_project_import(source_roots, &import.module_path)?
                            && distribution_names
                                .iter()
                                .all(|dist_name| !project_info.dependencies.contains(dist_name))
                        {
                            let diagnostic =
                                diagnostics.entry(display_file_path.clone()).or_default();
                            diagnostic.push(import.top_level_module_name().to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(diagnostics)
}
