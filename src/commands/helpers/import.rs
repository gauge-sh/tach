use std::path::{Path, PathBuf};

use crate::filesystem;
use crate::processors::ignore_directive::get_ignore_directives;
use crate::processors::import::{get_normalized_imports, NormalizedImport, Result};

pub fn get_project_imports<P: AsRef<Path>>(
    source_roots: &[PathBuf],
    file_path: P,
    ignore_type_checking_imports: bool,
    include_string_imports: bool,
) -> Result<Vec<NormalizedImport>> {
    let file_contents = filesystem::read_file_content(file_path.as_ref())?;
    let normalized_imports = get_normalized_imports(
        source_roots,
        file_path.as_ref(),
        &file_contents,
        ignore_type_checking_imports,
        include_string_imports,
    )?;
    let ignore_directives = get_ignore_directives(&file_contents);
    Ok(normalized_imports
        .into_iter()
        .filter(|import| {
            !ignore_directives.is_ignored(import)
                && filesystem::is_project_import(source_roots, &import.module_path)
        })
        .collect())
}

pub fn get_external_imports<P: AsRef<Path>>(
    source_roots: &[PathBuf],
    file_path: P,
    ignore_type_checking_imports: bool,
) -> Result<Vec<NormalizedImport>> {
    let file_contents = filesystem::read_file_content(file_path.as_ref())?;
    let normalized_imports = get_normalized_imports(
        source_roots,
        file_path.as_ref(),
        &file_contents,
        ignore_type_checking_imports,
        false,
    )?;
    let ignore_directives = get_ignore_directives(&file_contents);
    Ok(normalized_imports
        .into_iter()
        .filter(|import| {
            !ignore_directives.is_ignored(import)
                && !filesystem::is_project_import(source_roots, &import.module_path)
        })
        .collect())
}
