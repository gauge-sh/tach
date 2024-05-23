use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

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
