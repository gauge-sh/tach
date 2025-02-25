use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::exclusion::PathExclusions;
use crate::external::error::ParsingError;
use crate::external::parsing;
use crate::filesystem::{module_to_file_path, FileSystemError};

#[derive(Error, Debug)]
pub enum PackageResolutionError {
    #[error("File system error during package resolution: {0}")]
    FileSystem(#[from] FileSystemError),
    #[error("Error parsing package root dependencies: {0}")]
    Parsing(#[from] ParsingError),
    #[error("Source root '{0}' does not appear to be within project root.")]
    InvalidSourceRoot(String),
    #[error("Packages defined with setup.py ('{0}') are not supported. ")]
    SetupPyNotSupported(String),
    #[error("Package root not found for path: '{0}'")]
    PackageRootNotFound(String),
}

type Result<T> = std::result::Result<T, PackageResolutionError>;

fn is_pyproject_toml_package_root<P: AsRef<Path>>(directory: P) -> bool {
    directory.as_ref().join("pyproject.toml").exists()
}

fn is_setup_py_package_root<P: AsRef<Path>>(directory: P) -> bool {
    directory.as_ref().join("setup.py").exists()
}

fn is_requirements_txt_package_root<P: AsRef<Path>>(directory: P) -> bool {
    directory.as_ref().join("requirements.txt").exists()
}

fn get_package_root<P: AsRef<Path>>(directory: P) -> Option<PackageRoot> {
    if is_pyproject_toml_package_root(directory.as_ref()) {
        return Some(PackageRoot::Pyproject(directory.as_ref().to_path_buf()));
    }

    if is_setup_py_package_root(directory.as_ref()) {
        return Some(PackageRoot::SetupPy(directory.as_ref().to_path_buf()));
    }

    if is_requirements_txt_package_root(directory.as_ref()) {
        return Some(PackageRoot::RequirementsTxt(
            directory.as_ref().to_path_buf(),
        ));
    }

    None
}

// TODO: Let users configure other paths to look for [Custom(PathBuf)]
#[derive(Debug)]
enum PackageRoot {
    Pyproject(PathBuf),
    SetupPy(PathBuf),
    RequirementsTxt(PathBuf),
    Empty(PathBuf),
}

impl PackageRoot {
    fn root(&self) -> &PathBuf {
        match self {
            PackageRoot::Pyproject(path) => path,
            PackageRoot::SetupPy(path) => path,
            PackageRoot::RequirementsTxt(path) => path,
            PackageRoot::Empty(path) => path,
        }
    }
}

fn find_package_root<P1: AsRef<Path>, P2: AsRef<Path>>(
    project_root: P1,
    path: P2,
) -> Result<PackageRoot> {
    if !path.as_ref().starts_with(project_root.as_ref()) {
        return Err(PackageResolutionError::InvalidSourceRoot(
            path.as_ref().display().to_string(),
        ));
    }

    let mut current_dir = path.as_ref().to_path_buf();
    let project_root = project_root.as_ref();

    while current_dir != project_root {
        if let Some(package_root) = get_package_root(&current_dir) {
            return Ok(package_root);
        }

        current_dir = match current_dir.parent() {
            Some(parent) => parent.to_path_buf(),
            None => break,
        };
    }

    if let Some(package_root) = get_package_root(&current_dir) {
        return Ok(package_root);
    }

    Ok(PackageRoot::Empty(current_dir))
}

#[derive(Debug)]
pub struct Package {
    pub root: PathBuf,
    pub source_roots: Vec<PathBuf>,
    pub dependencies: HashSet<String>,
}

impl TryFrom<PackageRoot> for Package {
    type Error = PackageResolutionError;

    fn try_from(value: PackageRoot) -> std::result::Result<Self, Self::Error> {
        match value {
            PackageRoot::Pyproject(path) => {
                let project_info = parsing::parse_pyproject_toml(&path.join("pyproject.toml"))?;

                Ok(Self {
                    root: path,
                    source_roots: vec![],
                    dependencies: project_info.dependencies,
                })
            }
            PackageRoot::SetupPy(path) => Err(PackageResolutionError::SetupPyNotSupported(
                path.display().to_string(),
            )),
            PackageRoot::RequirementsTxt(path) => {
                let dependencies = parsing::parse_requirements_txt(&path.join("requirements.txt"))?;

                Ok(Self {
                    root: path,
                    source_roots: vec![],
                    dependencies,
                })
            }
            PackageRoot::Empty(path) => Ok(Self::empty(path)),
        }
    }
}

impl Package {
    pub fn empty<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            source_roots: vec![],
            dependencies: HashSet::new(),
        }
    }

    fn set_source_roots(&mut self, source_roots: Vec<PathBuf>) {
        self.source_roots = source_roots;
    }
}

#[derive(Debug)]
pub enum PackageResolution<'a> {
    Found {
        source_root: PathBuf,
        package: &'a Package,
    },
    NotFound,
    Excluded,
}

#[derive(Debug)]
pub struct PackageResolver<'a> {
    source_roots: &'a [PathBuf],
    path_exclusions: &'a PathExclusions,
    package_for_source_root: HashMap<PathBuf, Package>,
}

impl<'a> PackageResolver<'a> {
    pub fn try_new(
        project_root: &'a PathBuf,
        source_roots: &'a [PathBuf],
        path_exclusions: &'a PathExclusions,
    ) -> Result<Self> {
        let package_for_source_root = source_roots
            .iter()
            .map(|source_root| {
                let package_root = find_package_root(project_root, source_root)?;
                let mut package: Package = package_root.try_into()?;
                package.set_source_roots(source_roots.to_vec());
                Ok((source_root.clone(), package))
            })
            .collect::<Result<HashMap<PathBuf, Package>>>()?;
        Ok(Self {
            source_roots,
            path_exclusions,
            package_for_source_root,
        })
    }

    pub fn get_package_for_source_root<P: AsRef<Path>>(
        &'a self,
        source_root: P,
    ) -> Option<&'a Package> {
        self.package_for_source_root.get(source_root.as_ref())
    }

    pub fn get_dependencies_for_package_root(
        &self,
        package_root: &PathBuf,
    ) -> Option<&HashSet<String>> {
        self.package_for_source_root
            .values()
            .find(|package| &package.root == package_root)
            .map(|package| &package.dependencies)
    }

    pub fn resolve_module_path(&self, module_path: &str) -> PackageResolution {
        if let Some(resolved_module) = module_to_file_path(self.source_roots, module_path, true) {
            if self
                .path_exclusions
                .is_path_excluded(&resolved_module.file_path)
            {
                return PackageResolution::Excluded;
            }

            match self.get_package_for_source_root(&resolved_module.source_root) {
                Some(package) => PackageResolution::Found {
                    source_root: resolved_module.source_root,
                    package,
                },
                None => PackageResolution::NotFound,
            }
        } else {
            PackageResolution::NotFound
        }
    }

    pub fn module_path_is_internal<P: AsRef<Path>>(
        &self,
        module_path: &str,
        source_root: P,
    ) -> bool {
        let expected_package_root = match self
            .package_for_source_root
            .get(source_root.as_ref())
            .map(|package| &package.root)
        {
            Some(package_root) => package_root,
            None => return false,
        };

        let package_resolution = self.resolve_module_path(module_path);
        match package_resolution {
            PackageResolution::Found { package, .. } => &package.root == expected_package_root,
            PackageResolution::NotFound | PackageResolution::Excluded => false,
        }
    }

    pub fn module_path_is_external<P: AsRef<Path>>(
        &self,
        module_path: &str,
        source_root: P,
    ) -> bool {
        let expected_package_root = match self
            .package_for_source_root
            .get(source_root.as_ref())
            .map(|package| &package.root)
        {
            Some(package_root) => package_root,
            None => return false,
        };

        let package_resolution = self.resolve_module_path(module_path);
        match package_resolution {
            PackageResolution::Found { package, .. } => &package.root != expected_package_root,
            PackageResolution::NotFound => true,
            PackageResolution::Excluded => false,
        }
    }
}
