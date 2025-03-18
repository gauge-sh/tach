use std::cmp::Ordering;
use std::fmt::Debug;
use std::io;
use std::path::PathBuf;

use rayon::prelude::*;

use thiserror::Error;

use crate::cli;
use crate::cli::create_clickable_link;
use crate::colors::*;
use crate::config::root_module::RootModuleTreatment;
use crate::config::ProjectConfig;
use crate::dependencies::LocatedImport;
use crate::filesystem;
use crate::interrupt::check_interrupt;
use crate::modules::{ModuleTreeBuilder, ModuleTreeError};
use crate::processors::import::ImportParseError;
use crate::resolvers::{SourceRootResolver, SourceRootResolverError};

use super::helpers::import::get_located_project_imports;

struct Dependency {
    file_path: PathBuf,
    absolute_path: PathBuf,
    import: LocatedImport,
    source_module: String,
    target_module: String,
}

#[derive(Error, Debug)]
pub enum ReportCreationError {
    #[error("I/O failure during report generation:\n{0}")]
    Io(#[from] io::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] filesystem::FileSystemError),
    #[error("Import parsing error: {0}")]
    ImportParse(#[from] ImportParseError),
    #[error("Nothing to report when skipping dependencies and usages.")]
    NothingToReport,
    #[error("Module tree build error: {0}")]
    ModuleTree(#[from] ModuleTreeError),
    #[error("Operation interrupted")]
    Interrupted,
    #[error("Failed to resolve source roots: {0}")]
    SourceRootResolver(#[from] SourceRootResolverError),
}

pub type Result<T> = std::result::Result<T, ReportCreationError>;

// less code than implementing/deriving all necessary traits for Ord
fn compare_dependencies(left: &Dependency, right: &Dependency) -> Ordering {
    let path_cmp = left.file_path.cmp(&right.file_path);
    if path_cmp == Ordering::Equal {
        return left
            .import
            .alias_line_number()
            .cmp(&right.import.alias_line_number());
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

    fn color_if_interactive(&self, color: &'static str) -> &'static str {
        if cli::is_interactive() {
            color
        } else {
            ""
        }
    }

    fn render_dependency(&self, dependency: &Dependency) -> String {
        let clickable_link = create_clickable_link(
            &dependency.file_path,
            &dependency.absolute_path,
            &dependency.import.alias_line_number(),
        );
        format!(
            "{green}{clickable_link}{end_color}: {cyan}Import '{import_mod_path}'{end_color}",
            green = self.color_if_interactive(BColors::OKGREEN),
            clickable_link = clickable_link,
            end_color = self.color_if_interactive(BColors::ENDC),
            cyan = self.color_if_interactive(BColors::OKCYAN),
            import_mod_path = dependency.import.module_path()
        )
    }

    fn render_to_string(
        &mut self,
        skip_dependencies: bool,
        skip_usages: bool,
        raw: bool,
    ) -> String {
        if raw {
            let mut lines = Vec::new();

            if !skip_dependencies && !self.dependencies.is_empty() {
                lines.push("# Module Dependencies".to_string());
                let mut module_paths: Vec<_> = self
                    .dependencies
                    .iter()
                    .map(|dep| dep.target_module.clone())
                    .collect();
                module_paths.sort();
                module_paths.dedup();
                lines.extend(module_paths);
            }

            if !skip_usages && !self.usages.is_empty() {
                lines.push("# Module Usages".to_string());
                let mut using_modules: Vec<_> = self
                    .usages
                    .iter()
                    .map(|usage| usage.source_module.clone())
                    .collect();
                using_modules.sort();
                using_modules.dedup();
                lines.extend(using_modules);
            }

            return lines.join("\n");
        }

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
                0 => format!(
                    "{cyan}No dependencies found.{end_color}",
                    cyan = self.color_if_interactive(BColors::WARNING),
                    end_color = self.color_if_interactive(BColors::ENDC)
                ),
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
                {deps}\n\
                -------------------------------\n",
                deps_title = deps_title,
                deps = deps_display,
            ));
        }

        if !skip_usages {
            let usages_title = format!("Usages of '{path}'", path = self.path.as_str());
            self.usages.sort_by(compare_dependencies);
            let usages_display: String = match self.usages.len() {
                0 => format!(
                    "{cyan}No usages found.{end_color}",
                    cyan = self.color_if_interactive(BColors::WARNING),
                    end_color = self.color_if_interactive(BColors::ENDC)
                ),
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
                {usages}\n\
                -------------------------------\n",
                usages_title = usages_title,
                usages = usages_display,
            ));
        }

        if !self.warnings.is_empty() {
            result.push_str(&format!(
                "[ Warnings ]\n\
                {warning_color}{warnings}{end_color}",
                warning_color = self.color_if_interactive(BColors::WARNING),
                end_color = self.color_if_interactive(BColors::ENDC),
                warnings = self.warnings.join("\n")
            ));
        }

        result
    }
}

fn is_module_prefix(prefix: &str, full_path: &str) -> bool {
    if !full_path.starts_with(prefix) {
        return false;
    }
    full_path.len() == prefix.len() || full_path[prefix.len()..].starts_with('.')
}

pub fn create_dependency_report(
    project_root: &PathBuf,
    project_config: &ProjectConfig,
    path: &PathBuf,
    include_dependency_modules: Option<Vec<String>>,
    include_usage_modules: Option<Vec<String>>,
    skip_dependencies: bool,
    skip_usages: bool,
    raw: bool,
) -> Result<String> {
    if skip_dependencies && skip_usages {
        return Err(ReportCreationError::NothingToReport);
    }

    let file_walker = filesystem::FSWalker::try_new(
        project_root,
        &project_config.exclude,
        project_config.respect_gitignore,
    )?;
    let source_root_resolver = SourceRootResolver::new(project_root, &file_walker);
    let source_roots = source_root_resolver.resolve(&project_config.source_roots)?;
    let module_tree_builder = ModuleTreeBuilder::new(
        &source_roots,
        &file_walker,
        false,                      // skip circular dependency check in report
        RootModuleTreatment::Allow, // skip root module check in report
    );
    let (valid_modules, _) = module_tree_builder.resolve_modules(project_config.all_modules());

    check_interrupt().map_err(|_| ReportCreationError::Interrupted)?;

    let module_tree = module_tree_builder.build(valid_modules)?;

    let absolute_path = project_root.join(path);
    let module_path = filesystem::file_to_module_path(&source_roots, &absolute_path)?;
    let target_module = module_tree.find_nearest(&module_path).ok_or_else(|| {
        ReportCreationError::ModuleTree(ModuleTreeError::ModuleNotFound(module_path.clone()))
    })?;

    let mut report = DependencyReport::new(path.display().to_string());

    for source_root in &source_roots {
        check_interrupt().map_err(|_| ReportCreationError::Interrupted)?;

        let source_root_results: Vec<_> = file_walker
            .walk_pyfiles(&source_root.display().to_string())
            .par_bridge()
            .filter_map(|pyfile| {
                if check_interrupt().is_err() {
                    return None;
                }

                let absolute_pyfile = source_root.join(&pyfile);
                let file_module_path =
                    match filesystem::file_to_module_path(&source_roots, &absolute_pyfile) {
                        Ok(path) => path,
                        Err(_) => return None,
                    };
                let file_module = module_tree.find_nearest(&file_module_path);

                match get_located_project_imports(
                    project_root,
                    &source_roots,
                    &absolute_pyfile,
                    project_config,
                ) {
                    Ok(project_imports) => {
                        let is_in_target_path = is_module_prefix(&module_path, &file_module_path);
                        let mut dependencies = Vec::new();
                        let mut usages = Vec::new();

                        if is_in_target_path && !skip_dependencies {
                            // Add dependencies
                            dependencies.extend(
                                project_imports
                                    .iter()
                                    .filter_map(|import| {
                                        if let Some(import_module) =
                                            module_tree.find_nearest(import.module_path())
                                        {
                                            if import_module == target_module {
                                                return None;
                                            }
                                            include_dependency_modules.as_ref().map_or(
                                                Some((import.clone(), import_module.clone())),
                                                |included_modules| {
                                                    if included_modules
                                                        .contains(&import_module.full_path)
                                                    {
                                                        Some((
                                                            import.clone(),
                                                            import_module.clone(),
                                                        ))
                                                    } else {
                                                        None
                                                    }
                                                },
                                            )
                                        } else {
                                            None
                                        }
                                    })
                                    .map(|(import, import_module)| Dependency {
                                        file_path: pyfile.clone(),
                                        absolute_path: absolute_pyfile.clone(),
                                        import,
                                        source_module: target_module.full_path.clone(),
                                        target_module: import_module.full_path.clone(),
                                    }),
                            );
                        } else if !is_in_target_path && !skip_usages {
                            // Add usages
                            usages.extend(
                                project_imports
                                    .iter()
                                    .filter(|import| {
                                        if !is_module_prefix(&module_path, import.module_path()) {
                                            return false;
                                        }
                                        file_module.as_ref().is_some_and(|m| {
                                            include_usage_modules.as_ref().is_none_or(
                                                |included_modules| {
                                                    included_modules.contains(&m.full_path)
                                                },
                                            )
                                        })
                                    })
                                    .map(|import| Dependency {
                                        file_path: pyfile.clone(),
                                        absolute_path: absolute_pyfile.clone(),
                                        import: import.clone(),
                                        source_module: file_module
                                            .as_ref()
                                            .map_or(String::new(), |m| m.full_path.clone()),
                                        target_module: target_module.full_path.clone(),
                                    }),
                            );
                        }

                        Some((dependencies, usages, None))
                    }
                    Err(err) => Some((Vec::new(), Vec::new(), Some(err.to_string()))),
                }
            })
            .collect();

        check_interrupt().map_err(|_| ReportCreationError::Interrupted)?;

        // Combine results
        for (dependencies, usages, warning) in source_root_results {
            report.dependencies.extend(dependencies);
            report.usages.extend(usages);
            if let Some(warning) = warning {
                report.warnings.push(warning);
            }
        }
    }

    Ok(report.render_to_string(skip_dependencies, skip_usages, raw))
}
