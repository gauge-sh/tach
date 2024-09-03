use std::cmp::Ordering;
use std::fmt::Debug;
use std::fs;
use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::colors::*;

use crate::cli::create_clickable_link;
use crate::filesystem::{file_to_module_path, walk_pyfiles, FileSystemError};
use crate::imports::{get_project_imports, ImportParseError, NormalizedImport};

struct Dependency {
    file_path: PathBuf,
    absolute_path: PathBuf,
    import: NormalizedImport,
}

#[derive(Error, Debug)]
pub enum ReportCreationError {
    #[error("I/O failure during report generation:\n{0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] FileSystemError),
    #[error("Import parsing error: {0}")]
    ImportParse(#[from] ImportParseError),
    #[error("Nothing to report when skipping dependencies and usages.")]
    NothingToReport,
}

pub type Result<T> = std::result::Result<T, ReportCreationError>;

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
    pub dependencies: Vec<Dependency>,
    pub usages: Vec<Dependency>,
    pub warnings: Vec<String>,
}

impl DependencyReport {
    fn new(path: String) -> Self {
        DependencyReport {
            path,
            dependencies: vec![],
            usages: vec![],
            warnings: vec![],
        }
    }

    fn render_dependency(&self, dependency: &Dependency) -> String {
        let clickable_link = create_clickable_link(
            &dependency.file_path,
            &dependency.absolute_path,
            &dependency.import.line_no,
        );
        format!(
            "{green}{clickable_link}{end_color}: Import '{import_mod_path}'",
            green = BColors::OKGREEN,
            clickable_link = clickable_link,
            end_color = BColors::ENDC,
            import_mod_path = dependency.import.module_path
        )
    }

    fn render_to_string(&mut self, skip_dependencies: bool, skip_usages: bool) -> String {
        let title = format!("Dependency Report for '{path}'", path = self.path.as_str());
        let mut result = format!(
            "[ {title} ]\n\
            -------------------------------\n",
            title = title,
        );

        if !skip_dependencies {
            let deps_title = format!("Dependencies of '{path}'", path = self.path.as_str());
            self.dependencies.sort_by(compare_dependencies);
            let deps_display: String = match self.dependencies.len() {
                0 => "No dependencies found.".to_string(),
                _ => self
                    .dependencies
                    .iter()
                    .map(|dep| self.render_dependency(dep))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .to_string(),
            };
            result.push_str(&format!(
                "[ {deps_title} ]\n\
                {cyan}{deps}{end_color}\n\
                -------------------------------\n",
                deps_title = deps_title,
                deps = deps_display,
                cyan = BColors::OKCYAN,
                end_color = BColors::ENDC,
            ));
        }

        if !skip_usages {
            let usages_title = format!("Usages of '{path}'", path = self.path.as_str());
            self.usages.sort_by(compare_dependencies);
            let usages_display: String = match self.usages.len() {
                0 => "No usages found.".to_string(),
                _ => self
                    .usages
                    .iter()
                    .map(|dep| self.render_dependency(dep))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .to_string(),
            };
            result.push_str(&format!(
                "[ {usages_title} ]\n\
                {cyan}{usages}{end_color}\n\
                -------------------------------\n",
                usages_title = usages_title,
                usages = usages_display,
                cyan = BColors::OKCYAN,
                end_color = BColors::ENDC,
            ));
        }

        if !self.warnings.is_empty() {
            result.push_str(&format!(
                "[ Warnings ]\n\
                {warning_color}{warnings}{end_color}",
                warning_color = BColors::WARNING,
                end_color = BColors::ENDC,
                warnings = self.warnings.join("\n")
            ));
        }

        result
    }
}

pub fn create_dependency_report(
    project_root: &PathBuf,
    source_roots: &[PathBuf],
    path: &PathBuf,
    include_dependency_modules: Option<Vec<String>>,
    include_usage_modules: Option<Vec<String>>,
    skip_dependencies: bool,
    skip_usages: bool,
    ignore_type_checking_imports: bool,
) -> Result<String> {
    if skip_dependencies && skip_usages {
        return Err(ReportCreationError::NothingToReport);
    }
    let absolute_path = fs::canonicalize(path)?;
    let module_path = file_to_module_path(source_roots, &absolute_path)?;
    let mut report = DependencyReport::new(path.to_string_lossy().to_string());

    for pyfile in walk_pyfiles(project_root.to_str().unwrap()) {
        let absolute_pyfile = PathBuf::from(&project_root).join(&pyfile);
        match get_project_imports(source_roots, &absolute_pyfile, ignore_type_checking_imports) {
            Ok(project_imports) => {
                let pyfile_in_target_module = absolute_pyfile.starts_with(&absolute_path);
                if pyfile_in_target_module && !skip_dependencies {
                    // Any import from within the target module which points to an external mod_path
                    // is an external dependency
                    report.dependencies.extend(
                        project_imports
                            .into_iter()
                            .filter(|import| {
                                if import.module_path.starts_with(&module_path) {
                                    // this is an internal import
                                    return false;
                                }

                                // for external imports,
                                // if there is a filter list of dependencies, verify that the import is included
                                include_dependency_modules.as_ref().map_or(
                                    true,
                                    |included_modules| {
                                        included_modules.iter().any(|module_path| {
                                            import.module_path.starts_with(module_path)
                                        })
                                    },
                                )
                            })
                            .map(|import| Dependency {
                                file_path: pyfile.clone(),
                                absolute_path: absolute_pyfile.clone(),
                                import,
                            }),
                    );
                } else if !pyfile_in_target_module && !skip_usages {
                    // We are looking at imports from outside the target module,
                    // so any import which points to the target module is an external usage
                    for import in project_imports {
                        if !import.module_path.starts_with(&module_path) {
                            // this import doesn't point to the target module
                            continue;
                        }

                        let pyfile_mod_path = file_to_module_path(source_roots, &absolute_pyfile);
                        if pyfile_mod_path.is_err() {
                            // the current file doesn't belong to the source root
                            continue;
                        }

                        if include_usage_modules.is_none()
                            || include_usage_modules
                                .as_ref()
                                .is_some_and(|included_modules| {
                                    included_modules.contains(&pyfile_mod_path.unwrap())
                                })
                        {
                            report.usages.push(Dependency {
                                file_path: pyfile.clone(),
                                absolute_path: absolute_pyfile.clone(),
                                import,
                            });
                        }
                    }
                }
            }
            Err(err) => {
                // Failed to parse project imports
                report.warnings.push(err.to_string());
            }
        }
    }
    Ok(report.render_to_string(skip_dependencies, skip_usages))
}
