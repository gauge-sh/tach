use std::path::{Path, PathBuf};

use pyo3::prelude::*;
use ruff_linter::Locator;

use crate::config::ProjectConfig;
use crate::dependencies::import::LocatedImport;
use crate::exclusion::PathExclusions;
use crate::filesystem;
use crate::processors::ignore_directive::get_ignore_directives;
use crate::processors::import::{get_normalized_imports, Result};

#[pyclass(get_all)]
pub struct PythonImport {
    pub module_path: String,
    pub line_number: usize,
}

impl IntoPy<PyObject> for LocatedImport {
    fn into_py(self, py: Python<'_>) -> PyObject {
        PythonImport {
            module_path: self.import.module_path,
            line_number: self.alias_line_number,
        }
        .into_py(py)
    }
}

pub fn get_located_project_imports<P: AsRef<Path>>(
    project_root: &PathBuf,
    source_roots: &[PathBuf],
    file_path: P,
    project_config: &ProjectConfig,
) -> Result<Vec<LocatedImport>> {
    let file_contents = filesystem::read_file_content(file_path.as_ref())?;
    let line_index = Locator::new(&file_contents).to_index().clone();
    let normalized_imports = get_normalized_imports(
        source_roots,
        file_path.as_ref(),
        &file_contents,
        project_config.ignore_type_checking_imports,
        project_config.include_string_imports,
    )?;
    let ignore_directives = get_ignore_directives(&file_contents);

    let exclusions = PathExclusions::new(
        project_root,
        &project_config.exclude,
        project_config.use_regex_matching,
    )?;

    Ok(normalized_imports
        .into_iter()
        .map(|import| {
            LocatedImport::new(
                line_index.line_index(import.import_offset).get(),
                line_index.line_index(import.alias_offset).get(),
                import,
            )
        })
        .filter(|import| {
            !ignore_directives.is_ignored(import)
                && filesystem::is_project_import(source_roots, import.module_path(), &exclusions)
        })
        .collect())
}

pub fn get_located_external_imports<P: AsRef<Path>>(
    project_root: &PathBuf,
    source_roots: &[PathBuf],
    file_path: P,
    project_config: &ProjectConfig,
) -> Result<Vec<LocatedImport>> {
    let file_contents = filesystem::read_file_content(file_path.as_ref())?;
    let line_index = Locator::new(&file_contents).to_index().clone();
    let normalized_imports = get_normalized_imports(
        source_roots,
        file_path.as_ref(),
        &file_contents,
        project_config.ignore_type_checking_imports,
        false,
    )?;
    let ignore_directives = get_ignore_directives(&file_contents);
    let exclusions = PathExclusions::new(
        project_root,
        &project_config.exclude,
        project_config.use_regex_matching,
    )?;
    Ok(normalized_imports
        .into_iter()
        .map(|import| {
            LocatedImport::new(
                line_index.line_index(import.import_offset).get(),
                line_index.line_index(import.alias_offset).get(),
                import,
            )
        })
        .filter(|import| {
            !ignore_directives.is_ignored(import)
                && !filesystem::is_project_import(source_roots, import.module_path(), &exclusions)
        })
        .collect())
}
