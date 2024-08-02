use std::fmt;
use std::fs;
use std::io;
use std::io::Read;
use std::path::StripPrefixError;
use std::path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use globset::Glob;
use globset::GlobSetBuilder;
use walkdir::{DirEntry, WalkDir};

use crate::exclusion::is_path_excluded;

#[derive(Debug, Clone)]
pub struct FileSystemError {
    pub message: String,
}

impl std::error::Error for FileSystemError {}

impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl From<io::Error> for FileSystemError {
    fn from(_: io::Error) -> Self {
        FileSystemError {
            message: "Encountered unexpected I/O error.".to_string(),
        }
    }
}

impl From<StripPrefixError> for FileSystemError {
    fn from(_: StripPrefixError) -> Self {
        FileSystemError {
            message: "Path does not appear to be within project root.".to_string(),
        }
    }
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
        .ok_or(FileSystemError {
            message: format!(
                "No matching source root found for filepath: {:?}",
                file_path
            ),
        })?;

    // Get the relative path from the matching root
    let relative_path = file_path.strip_prefix(matching_root)?;

    // Convert the relative path to a module path
    let mut components: Vec<_> = relative_path
        .parent()
        .ok_or(FileSystemError {
            message: format!("Encountered invalid filepath: {:?}", relative_path),
        })?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect();

    // Get the file name
    let file_name = relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(FileSystemError {
            message: format!("Encountered invalid filepath: {:?}", relative_path),
        })?;

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

pub fn module_to_file_path<P: AsRef<Path>>(roots: &[P], mod_path: &str) -> Option<ResolvedModule> {
    let mod_as_file_path = mod_path.replace('.', MAIN_SEPARATOR_STR);
    for root in roots {
        let fs_path = root.as_ref().join(&mod_as_file_path);
        let file_path = fs_path.display().to_string();

        // Check for package with .pyi file
        let init_pyi_path = fs_path.join("__init__.pyi");
        if init_pyi_path.exists() {
            return Some(ResolvedModule {
                file_path: init_pyi_path,
                member_name: None,
            });
        }

        // Check for package with .py file
        let init_py_path = fs_path.join("__init__.py");
        if init_py_path.exists() {
            return Some(ResolvedModule {
                file_path: init_py_path,
                member_name: None,
            });
        }

        // Check for .pyi file
        let pyi_file_path = format!("{}.pyi", &file_path);
        if Path::new(&pyi_file_path).exists() {
            return Some(ResolvedModule {
                file_path: PathBuf::from(pyi_file_path),
                member_name: None,
            });
        }

        // Check for .py file
        let py_file_path = format!("{}.py", &file_path);
        if Path::new(&py_file_path).exists() {
            return Some(ResolvedModule {
                file_path: PathBuf::from(py_file_path),
                member_name: None,
            });
        }

        // If the original file path does not contain a separator (e.g. 'os', 'ast')
        // then we are done checking this root.
        if !mod_as_file_path.contains(MAIN_SEPARATOR) {
            continue;
        }

        if let Some(last_sep_index) = file_path.rfind(MAIN_SEPARATOR) {
            let member_name = file_path[last_sep_index + 1..].to_string();

            // Check for member within package with .pyi file
            let init_pyi_file_path = format!(
                "{}{}__init__.pyi",
                &file_path[..last_sep_index],
                MAIN_SEPARATOR
            );
            if Path::new(&init_pyi_file_path).exists() {
                return Some(ResolvedModule {
                    file_path: PathBuf::from(init_pyi_file_path),
                    member_name: Some(member_name.clone()),
                });
            }

            // Check for member within package with .py file
            let init_py_file_path = format!(
                "{}{}__init__.py",
                &file_path[..last_sep_index],
                MAIN_SEPARATOR
            );
            if Path::new(&init_py_file_path).exists() {
                return Some(ResolvedModule {
                    file_path: PathBuf::from(init_py_file_path),
                    member_name: Some(member_name),
                });
            }

            // Check for member within .pyi file
            let pyi_file_path = format!("{}.pyi", &file_path[..last_sep_index]);
            if Path::new(&pyi_file_path).exists() {
                return Some(ResolvedModule {
                    file_path: PathBuf::from(pyi_file_path),
                    member_name: Some(member_name.clone()),
                });
            }

            // Check for member within .py file
            let py_file_path = format!("{}.py", &file_path[..last_sep_index]);
            if Path::new(&py_file_path).exists() {
                return Some(ResolvedModule {
                    file_path: PathBuf::from(py_file_path),
                    member_name: Some(member_name.clone()),
                });
            }
        }
    }
    None
}

pub fn read_file_content<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = fs::File::open(path.as_ref()).map_err(|_| FileSystemError {
        message: format!("Could not open path: {}", path.as_ref().display()),
    })?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|_| FileSystemError {
            message: format!("Could not read path: {}", path.as_ref().display()),
        })?;
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
    is_path_excluded(entry.path().to_str().unwrap()).unwrap_or(false)
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
