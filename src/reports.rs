use std::env::current_dir;
use std::fmt::{self, Debug};
use std::path::MAIN_SEPARATOR_STR;

use crate::filesystem::{
    adjust_path_from_cwd_to_root, file_to_module_path, walk_pyfiles, FileSystemError,
};
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

pub type Result<T> = std::result::Result<T, ReportCreationError>;

struct Dependency {
    file_path: String,
    import: ProjectImport,
}

struct DependencyReport {
    path: String,
    pub external_dependencies: Vec<Dependency>,
    pub external_usages: Vec<Dependency>,
}

impl DependencyReport {
    fn new(path: String) -> Self {
        DependencyReport {
            path,
            external_dependencies: vec![],
            external_usages: vec![],
        }
    }

    fn render_dependency(&self, dependency: &Dependency) -> String {
        format!(
            "{file_path}[L{line_no}]  '{import_mod_path}'",
            file_path = dependency.file_path.as_str(),
            line_no = dependency.import.line_no,
            import_mod_path = dependency.import.mod_path
        )
    }

    fn render_to_string(&self) -> String {
        let title = format!("Dependency Report for {path}", path = self.path.as_str());
        let external_deps_title = format!(
            "{path} has external dependencies:",
            path = self.path.as_str()
        );
        let external_usages_title =
            format!("These modules depend on {path}:", path = self.path.as_str());

        format!(
            "{title}\n\
            -------------------------------\n\
            {deps_title}\n\
            {deps}\n\
            \n\
            {usages_title}\n\
            {usages}",
            title = title,
            deps_title = external_deps_title,
            usages_title = external_usages_title,
            deps = self
                .external_dependencies
                .iter()
                .map(|dep| self.render_dependency(dep))
                .collect::<Vec<String>>()
                .join("\n"),
            usages = self
                .external_usages
                .iter()
                .map(|dep| self.render_dependency(dep))
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}

pub fn create_dependency_report(
    project_root: String,
    path: String,
    ignore_type_checking_imports: bool,
) -> Result<String> {
    let path_relative_to_root = adjust_path_from_cwd_to_root(&project_root, &path)?;
    let module_path = file_to_module_path(path_relative_to_root.to_str().unwrap());
    let mut result = DependencyReport::new(path_relative_to_root.to_string_lossy().to_string()); // TODO: clone shouldnt be necessary

    for pyfile in walk_pyfiles(&project_root) {
        let project_imports = get_project_imports(
            project_root.clone(), // TODO: not necessary, need to update the args
            pyfile.to_string_lossy().to_string(),
            ignore_type_checking_imports,
        )?;

        if pyfile.starts_with(path_relative_to_root.to_str().unwrap()) {
            result
                .external_dependencies
                .extend(project_imports.into_iter().map(|import| Dependency {
                    file_path: pyfile.to_string_lossy().to_string(),
                    import,
                }));
        } else {
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

    Ok(result.render_to_string())
}
