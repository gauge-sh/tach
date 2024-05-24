use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

#[derive(Debug, Clone)]
pub struct FileSystemError {
    pub message: String,
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

pub type Result<T> = std::result::Result<T, FileSystemError>;

pub fn canonical(root: &str, path: &str) -> Result<PathBuf> {
    let root = Path::new(root);
    let file_path = root.join(path);
    file_path.canonicalize().map_err(|_| FileSystemError {
        message: format!("Failed to canonicalize path: {}", path),
    })
}

pub fn file_to_module_path(file_path: &str) -> String {
    let file_path = file_path.trim_start_matches("./");

    if file_path == "." {
        return String::new();
    }

    let module_path = file_path.replace(MAIN_SEPARATOR, ".");

    let mut module_path = if module_path.ends_with(".py") {
        module_path.trim_end_matches(".py").to_string()
    } else {
        module_path
    };

    if module_path.ends_with(".__init__") {
        module_path.truncate(module_path.len() - 9);
    }

    if module_path == "__init__" {
        return String::new();
    }

    module_path
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

fn is_standard_lib_or_builtin_import(_module_base: &str) -> bool {
    false
}

pub fn is_project_import<P: AsRef<Path>>(root: P, mod_path: &str) -> Result<bool> {
    let root_base = root
        .as_ref()
        .canonicalize()
        .map_err(|_| FileSystemError {
            message: format!("Could not find project root: {}", root.as_ref().display()),
        })?
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let module_base = mod_path.split('.').next().unwrap();

    if is_standard_lib_or_builtin_import(module_base) {
        return Ok(false);
    }

    if root_base == module_base {
        return Ok(true);
    }

    let module_path = root.as_ref().join(module_base);
    if module_path.is_dir() || module_path.with_extension("py").is_file() {
        return Ok(true);
    }

    Ok(false)
}
