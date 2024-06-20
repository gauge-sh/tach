use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::filesystem::{file_to_module_path, walk_pyfiles, FileSystemError};
use crate::imports::{get_project_imports, ImportParseError, ProjectImport};

#[derive(Debug)]
pub struct ReportCreationError {
    pub message: String,
}

impl fmt::Display for ReportCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl From<ImportParseError> for ReportCreationError {
    fn from(value: ImportParseError) -> Self {
        ReportCreationError {
            message: value.message,
        }
    }
}

impl From<FileSystemError> for ReportCreationError {
    fn from(value: FileSystemError) -> Self {
        ReportCreationError {
            message: value.message,
        }
    }
}

impl From<io::Error> for ReportCreationError {
    fn from(_: io::Error) -> Self {
        ReportCreationError {
            message: "I/O failure during report generation.".to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, ReportCreationError>;

struct Dependency {
    file_path: String,
    import: ProjectImport,
}

// less code than implementing/deriving all necessary traits for Ord
fn compare_dependencies(left: &Dependency, right: &Dependency) -> Ordering {
    let path_cmp = left.file_path.cmp(&right.file_path);
    if path_cmp == Ordering::Equal {
        return left.import.line_no.cmp(&right.import.line_no);
    }
    path_cmp
}

struct DependencyReport {
    path: String,
    pub external_dependencies: Vec<Dependency>,
    pub external_usages: Vec<Dependency>,
    pub warnings: Vec<String>,
}

impl DependencyReport {
    fn new(path: String) -> Self {
        DependencyReport {
            path,
            external_dependencies: vec![],
            external_usages: vec![],
            warnings: vec![],
        }
    }

    fn render_dependency(&self, dependency: &Dependency) -> String {
        format!(
            "{file_path}[L{line_no}]: Import '{import_mod_path}'",
            file_path = dependency.file_path.as_str(),
            line_no = dependency.import.line_no,
            import_mod_path = dependency.import.mod_path
        )
    }

    fn render_to_string(&mut self) -> String {
        let title = format!("Dependency Report for {path}", path = self.path.as_str());
        let subtitle = format!(
            "The report below shows all instances of imports which cross the boundary of '{path}'",
            path = self.path.as_str()
        );
        let external_deps_title = format!("Dependencies of '{path}'", path = self.path.as_str());
        let external_usages_title = format!("Usages of '{path}'", path = self.path.as_str());

        self.external_dependencies
            .sort_by(|l, r| compare_dependencies(l, r));
        self.external_usages
            .sort_by(|l, r| compare_dependencies(l, r));

        let deps_display: String = match self.external_dependencies.len() {
            0 => "No dependencies found.".to_string(),
            _ => self
                .external_dependencies
                .iter()
                .map(|dep| self.render_dependency(dep))
                .collect::<Vec<String>>()
                .join("\n")
                .to_string(),
        };
        let usages_display: String = match self.external_usages.len() {
            0 => "No usages found.".to_string(),
            _ => self
                .external_usages
                .iter()
                .map(|dep| self.render_dependency(dep))
                .collect::<Vec<String>>()
                .join("\n")
                .to_string(),
        };

        let mut result = format!(
            "[{title}]\n\
            {subtitle}\n\
            -------------------------------\n\
            [{deps_title}]\n\
            {deps}\n\
            -------------------------------\n\
            [{usages_title}]\n\
            {usages}",
            title = title,
            deps_title = external_deps_title,
            usages_title = external_usages_title,
            deps = deps_display,
            usages = usages_display
        );
        if !self.warnings.is_empty() {
            result.push_str(
                format!(
                    "\n\
                    -------------------------------\n\
                    [Warnings]\n\
                    {}",
                    self.warnings.join("\n")
                )
                .as_str(),
            );
        }
        result
    }
}

pub fn create_dependency_report(
    project_root: String,
    source_root: String,
    path: String,
    ignore_type_checking_imports: bool,
) -> Result<String> {
    let absolute_path = fs::canonicalize(&path)?;
    let absolute_source_root = PathBuf::from(&project_root).join(&source_root);
    let module_path = file_to_module_path(
        absolute_source_root.to_str().unwrap(),
        absolute_path.to_str().unwrap(),
    )?;
    let mut result = DependencyReport::new(path.clone()); // TODO: clone shouldnt be necessary

    for pyfile in walk_pyfiles(&project_root) {
        match get_project_imports(
            project_root.clone(), // TODO: clones shouldn't be necessary, need to update the args
            source_root.clone(),
            pyfile.to_string_lossy().to_string(),
            ignore_type_checking_imports,
        ) {
            Ok(project_imports) => {
                let absolute_pyfile = PathBuf::from(&project_root).join(&pyfile);
                let pyfile_in_target_module = absolute_pyfile.starts_with(&absolute_path);
                if pyfile_in_target_module {
                    // Any import from within the target module which points to an external mod_path
                    // is an external dependency
                    result.external_dependencies.extend(
                        project_imports
                            .into_iter()
                            .filter(|import| !import.mod_path.starts_with(&module_path))
                            .map(|import| Dependency {
                                file_path: pyfile.to_string_lossy().to_string(),
                                import,
                            }),
                    );
                } else {
                    // We are looking at imports from outside the target module,
                    // so any import which points to the target module is an external usage
                    for import in project_imports {
                        if import.mod_path.starts_with(&module_path) {
                            result.external_usages.push(Dependency {
                                file_path: pyfile.to_string_lossy().to_string(),
                                import,
                            });
                        }
                    }
                }
            }
            Err(err) => {
                // Failed to parse project imports
                result.warnings.push(err.message);
            }
        }
    }
    Ok(result.render_to_string())
}
