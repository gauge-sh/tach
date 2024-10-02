use std::fs;
use std::io;
use std::io::Read;
use std::path::StripPrefixError;
use std::path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use globset::Glob;
use globset::GlobSetBuilder;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::core::config::ModuleConfig;
use crate::exclusion::is_path_excluded;

pub const ROOT_MODULE_SENTINEL_TAG: &str = "<root>";
pub const DEFAULT_EXCLUDE_PATHS: [&str; 4] = ["tests", "docs", ".*__pycache__", ".*egg-info"];

#[derive(Error, Debug)]
pub enum FileSystemError {
    #[error("Encountered unexpected I/O error.\n{0}")]
    Io(#[from] io::Error),
    #[error("Path does not appear to be within project root.\n{0}")]
    StripPrefix(#[from] StripPrefixError),
    #[error("{0}")]
    Other(String),
}
pub type Result<T> = std::result::Result<T, FileSystemError>;

pub fn relative_to<P: AsRef<Path>, R: AsRef<Path>>(path: P, root: R) -> Result<PathBuf> {
    let diff_path = path.as_ref().strip_prefix(root)?;
    Ok(diff_path.to_owned())
}

pub fn file_to_module_path(source_roots: &[PathBuf], file_path: &PathBuf) -> Result<String> {
    // Find the matching source root
    let matching_root = source_roots
        .iter()
        .find(|&root| file_path.starts_with(root))
        .ok_or(FileSystemError::Other(format!(
            "No matching source root found for filepath: {:?}",
            file_path
        )))?;

    // Get the relative path from the matching root
    let relative_path = file_path.strip_prefix(matching_root)?;

    // If the relative path is empty, return an error
    // indicating that the path cannot be a source root itself
    if relative_path.as_os_str().is_empty() {
        return Err(FileSystemError::Other(
            "Filepath cannot be a source root.".to_string(),
        ));
    }

    // Convert the relative path to a module path
    let mut components: Vec<_> = relative_path
        .parent()
        .ok_or(FileSystemError::Other(format!(
            "Encountered invalid filepath: {:?}",
            relative_path
        )))?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect();

    // Get the file name
    let file_name = relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(FileSystemError::Other(format!(
            "Encountered invalid filepath: {:?}",
            relative_path
        )))?;

    // If the file is not __init__.py, add its name (without extension) to the components
    if file_name != "__init__.py" {
        if let Some(stem) = Path::new(file_name).file_stem().and_then(|s| s.to_str()) {
            components.push(stem);
        }
    }

    // Join the components with dots
    let module_path = components.join(".");

    // If the module_path is empty, return ".", otherwise return the module_path
    Ok(if module_path.is_empty() {
        ".".to_string()
    } else {
        module_path
    })
}

#[derive(Debug)]
pub struct ResolvedModule {
    pub file_path: PathBuf,
    pub member_name: Option<String>,
}

pub fn module_to_file_path<P: AsRef<Path>>(
    roots: &[P],
    mod_path: &str,
    check_members: bool,
) -> Option<ResolvedModule> {
    let mod_as_file_path = mod_path.replace('.', MAIN_SEPARATOR_STR);
    for root in roots {
        let fs_path = root.as_ref().join(&mod_as_file_path);

        // Check for [package with .pyi, .py] file or [.pyi, .py] file itself
        for path in &[
            fs_path.join("__init__.pyi"),
            fs_path.join("__init__.py"),
            fs_path.with_extension("pyi"),
            fs_path.with_extension("py"),
        ] {
            if path.exists() {
                return Some(ResolvedModule {
                    file_path: path.to_path_buf(),
                    member_name: None,
                });
            }
        }
        // If the original file path does not contain a separator (e.g. 'os', 'ast')
        // then we are done checking this root.
        if !mod_as_file_path.contains(MAIN_SEPARATOR) || !check_members {
            continue;
        }

        if let Some(last_sep_index) = mod_as_file_path.rfind(MAIN_SEPARATOR) {
            let member_name = &mod_as_file_path[last_sep_index + 1..];
            let base_fs_path = root.as_ref().join(&mod_as_file_path[..last_sep_index]);

            for path in &[
                base_fs_path.join("__init__.pyi"),
                base_fs_path.join("__init__.py"),
                base_fs_path.with_extension("pyi"),
                base_fs_path.with_extension("py"),
            ] {
                if path.exists() {
                    return Some(ResolvedModule {
                        file_path: path.to_path_buf(),
                        member_name: Some(member_name.to_string()),
                    });
                }
            }
        }
    }
    None
}

pub fn module_to_pyfile_or_dir_path<P: AsRef<Path>>(
    roots: &[P],
    mod_path: &str,
) -> Option<PathBuf> {
    if mod_path.is_empty() {
        return None;
    }
    let base_path = mod_path.replace('.', MAIN_SEPARATOR_STR);

    // Iterate through each source root
    for source_root in roots {
        let source_root = source_root.as_ref();

        // Build paths
        let dir_path = source_root.join(&base_path);
        let pyinterface_path = source_root.join(format!("{}.pyi", base_path));
        let pyfile_path = source_root.join(format!("{}.py", base_path));

        if dir_path.is_dir() {
            return Some(dir_path);
        } else if pyinterface_path.exists() {
            return Some(pyinterface_path);
        } else if pyfile_path.exists() {
            return Some(pyfile_path);
        }
    }
    None
}

pub fn read_file_content<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = fs::File::open(path.as_ref())?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn direntry_is_excluded(entry: &DirEntry) -> bool {
    is_path_excluded(entry.path()).unwrap_or(false)
}

fn is_pyfile_or_dir(entry: &DirEntry) -> bool {
    if entry.path().is_dir() {
        return true;
    }
    match entry.path().extension() {
        Some(ext) => ext == "py",
        None => false,
    }
}

pub fn walk_pyfiles(root: &str) -> impl Iterator<Item = PathBuf> {
    let prefix_root = root.to_string();
    WalkDir::new(root)
        .into_iter()
        .filter_entry(move |e| !is_hidden(e) && !direntry_is_excluded(e) && is_pyfile_or_dir(e))
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file()) // filter_entry would skip dirs if they were excluded earlier
        .map(move |entry| {
            entry
                .path()
                .strip_prefix(prefix_root.as_str())
                .unwrap()
                .to_path_buf()
        })
}

pub fn walk_pyprojects(root: &str) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(move |e| !is_hidden(e) && !direntry_is_excluded(e))
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name() == "pyproject.toml")
        .map(|entry| entry.into_path())
}

pub fn walk_globbed_files(root: &str, patterns: Vec<String>) -> impl Iterator<Item = PathBuf> {
    let mut glob_builder = GlobSetBuilder::new();

    for pattern in patterns {
        glob_builder.add(Glob::new(&pattern).unwrap());
    }

    let glob_set = glob_builder.build().unwrap();

    let walker = WalkDir::new(root).into_iter();
    let owned_root = root.to_string();
    walker
        .filter_entry(|e| !is_hidden(e))
        .map(|res| res.unwrap().into_path())
        .filter(move |path| {
            path.is_file()
                && glob_set.is_match(
                    relative_to(path, PathBuf::from(&owned_root)).unwrap_or(path.to_path_buf()),
                )
        })
}

/// Returns a tuple of (valid, invalid) modules
pub fn validate_project_modules(
    source_roots: &[PathBuf],
    modules: Vec<ModuleConfig>,
) -> (Vec<ModuleConfig>, Vec<ModuleConfig>) {
    let mut result = (Vec::new(), Vec::new());

    for module in modules {
        if module.path == ROOT_MODULE_SENTINEL_TAG
            || module_to_pyfile_or_dir_path(source_roots, &module.path).is_some()
        {
            // valid module
            result.0.push(module);
        } else {
            // invalid module
            result.1.push(module);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::fixtures::tests_dir;
    use rstest::rstest;

    #[rstest]
    #[case(&["."], "__init__.py", ".")]
    #[case(&["."], "domain_one/__init__.py", "domain_one")]
    #[case(&["."], "domain_one/interface.py", "domain_one.interface")]
    #[case(&["source/root"], "source/root/domain.py", "domain")]
    #[case(&["src1", "src2"], "src1/core/lib/cat.py", "core.lib.cat")]
    fn test_file_to_mod_path(
        tests_dir: PathBuf,
        #[case] roots: &[&str],
        #[case] file_path: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(
            file_to_module_path(
                roots
                    .iter()
                    .map(|r| tests_dir.join(r))
                    .collect::<Vec<_>>()
                    .as_slice(),
                &tests_dir.join(file_path)
            )
            .unwrap(),
            expected
        );
    }
}
