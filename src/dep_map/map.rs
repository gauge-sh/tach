use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::iter;
use std::path::PathBuf;

use super::error::{DependentMapError, Result};

use crate::{
    config::ProjectConfig, filesystem, processors::import::get_normalized_imports_from_ast,
    python::parsing::parse_python_source, resolvers::SourceRootResolver,
};

/// A struct that efficiently handles matching files against extra dependency patterns
#[derive(Debug)]
struct ExtraDependencyMatcher {
    /// Maps source file paths to their extra dependencies (all paths relative to project root)
    source_to_deps: HashMap<String, Vec<String>>,
}

impl ExtraDependencyMatcher {
    fn new(
        project_root: &PathBuf,
        file_walker: &filesystem::FSWalker,
        extra_dependencies: &HashMap<String, Vec<String>>,
    ) -> Result<Self> {
        let mut source_to_deps = HashMap::new();

        for (pattern, dep_patterns) in extra_dependencies {
            let matching_files =
                file_walker.walk_globbed_files(project_root.to_str().unwrap(), iter::once(pattern));

            for source_file in matching_files {
                let source_path = filesystem::relative_to(&source_file, project_root)?
                    .to_string_lossy()
                    .to_string();

                let mut deps = Vec::new();
                for dep_pattern in dep_patterns {
                    let dep_files = file_walker.walk_globbed_files(
                        project_root.to_str().unwrap(),
                        iter::once(dep_pattern),
                    );
                    for dep_file in dep_files {
                        if let Ok(rel_path) = filesystem::relative_to(&dep_file, project_root) {
                            deps.push(rel_path.to_string_lossy().to_string());
                        }
                    }
                }

                source_to_deps
                    .entry(source_path)
                    .or_insert_with(Vec::new)
                    .extend(deps);
            }
        }

        Ok(Self { source_to_deps })
    }

    fn get_extra_dependencies(&self, file_path: &str) -> Option<&Vec<String>> {
        self.source_to_deps.get(file_path)
    }
}

fn process_file(
    project_root: &PathBuf,
    path: &PathBuf,
    source_roots: &[PathBuf],
    ignore_type_checking_imports: bool,
    extra_deps: Option<&Vec<String>>,
) -> Result<HashSet<String>> {
    let mut result = HashSet::new();

    let file_content = filesystem::read_file_content(path)?;
    let python_source = parse_python_source(&file_content)?;

    get_normalized_imports_from_ast(
        source_roots,
        path,
        &python_source,
        ignore_type_checking_imports,
        true,
    )?
    .iter()
    .for_each(|module| {
        if let Some(resolved_module) =
            filesystem::module_to_file_path(source_roots, &module.module_path, true)
        {
            if let Ok(rel_path) = filesystem::relative_to(&resolved_module.file_path, project_root)
            {
                result.insert(rel_path.to_string_lossy().to_string());
            }
        }
    });

    if let Some(deps) = extra_deps {
        result.extend(deps.iter().cloned());
    }

    Ok(result)
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Dependencies,
    Dependents,
}

#[derive(Debug)]
pub struct DependentMap {
    project_root: PathBuf,
    source_roots: Vec<PathBuf>,
    project_config: ProjectConfig,
    map: DashMap<String, Vec<String>>,
    direction: Direction,
    file_walker: filesystem::FSWalker,
    extra_deps: ExtraDependencyMatcher,
}

impl DependentMap {
    pub fn new(
        project_root: &PathBuf,
        project_config: &ProjectConfig,
        direction: Direction,
    ) -> Result<Self> {
        let file_walker = filesystem::FSWalker::try_new(
            project_root,
            &project_config.exclude,
            project_config.respect_gitignore,
        )?;
        let source_root_resolver = SourceRootResolver::new(project_root, &file_walker);
        let source_roots = source_root_resolver.resolve(&project_config.source_roots)?;
        let extra_deps = ExtraDependencyMatcher::new(
            project_root,
            &file_walker,
            &project_config.map.extra_dependencies,
        )?;
        let map = Self::build_map(
            project_root,
            &source_roots,
            project_config,
            direction,
            &file_walker,
            &extra_deps,
        )?;

        Ok(Self {
            project_root: project_root.clone(),
            source_roots,
            project_config: project_config.clone(),
            map,
            direction,
            file_walker,
            extra_deps,
        })
    }

    fn build_map(
        project_root: &PathBuf,
        source_roots: &[PathBuf],
        project_config: &ProjectConfig,
        direction: Direction,
        file_walker: &filesystem::FSWalker,
        extra_deps: &ExtraDependencyMatcher,
    ) -> Result<DashMap<String, Vec<String>>> {
        let dependent_map: DashMap<String, Vec<String>> = DashMap::new();
        let ignore_type_checking_imports = project_config.ignore_type_checking_imports;

        source_roots.iter().for_each(|source_root| {
            file_walker
                .walk_pyfiles(&source_root.display().to_string())
                .par_bridge()
                .for_each(|path| {
                    let abs_path = source_root.join(&path);
                    let rel_path = filesystem::relative_to(&abs_path, project_root)
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    let extra_deps = extra_deps.get_extra_dependencies(&rel_path);

                    if let Ok(dependencies) = process_file(
                        project_root,
                        &abs_path,
                        source_roots,
                        ignore_type_checking_imports,
                        extra_deps,
                    ) {
                        for dep in dependencies {
                            match direction {
                                Direction::Dependents => {
                                    dependent_map.entry(dep).or_default().push(rel_path.clone());
                                }
                                Direction::Dependencies => {
                                    dependent_map.entry(rel_path.clone()).or_default().push(dep);
                                }
                            }
                        }
                    }
                });
        });

        Ok(dependent_map)
    }

    pub fn rebuild(&mut self) -> Result<()> {
        self.map = Self::build_map(
            &self.project_root,
            &self.source_roots,
            &self.project_config,
            self.direction,
            &self.file_walker,
            &self.extra_deps,
        )?;
        Ok(())
    }

    pub fn get_closure(&self, paths: &[PathBuf]) -> Result<HashSet<String>> {
        for path in paths.iter() {
            if !self.map.contains_key(path.to_str().unwrap()) {
                return Err(DependentMapError::FileNotFound(path.display().to_string()));
            }
        }
        let mut result = HashSet::new();
        let mut to_visit = Vec::new();
        to_visit.extend_from_slice(paths);
        while let Some(current) = to_visit.pop() {
            result.insert(current.display().to_string());

            if let Some(dependents) = self.map.get(current.to_str().unwrap()) {
                for dep in dependents.value().iter() {
                    if !result.contains(dep) {
                        to_visit.push(PathBuf::from(dep));
                        result.insert(dep.clone());
                    }
                }
            }
        }
        Ok(result)
    }

    pub fn update_files(&mut self, changed_files: &[PathBuf]) -> Result<()> {
        self.map.par_iter_mut().for_each(|mut item| {
            item.value_mut().retain(|dep| {
                !changed_files
                    .iter()
                    .any(|path| path.to_str().unwrap() == dep)
            });
        });
        let ignore_type_checking_imports = self.project_config.ignore_type_checking_imports;

        changed_files.par_iter().for_each(|path| {
            let abs_path = self.project_root.join(path);
            let rel_path = path.to_string_lossy().to_string();

            let extra_deps = self.extra_deps.get_extra_dependencies(&rel_path);

            if let Ok(dependencies) = process_file(
                &self.project_root,
                &abs_path,
                &self.source_roots,
                ignore_type_checking_imports,
                extra_deps,
            ) {
                for dep in dependencies {
                    self.map
                        .entry(dep.clone())
                        .or_default()
                        .push(rel_path.clone());
                }
            }
        });

        Ok(())
    }

    pub fn write_to_file(&self, path: &PathBuf) -> Result<()> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, &self.map)
            .map_err(|e| DependentMapError::Io(e.into()))?;
        Ok(())
    }

    pub fn write_to_stdout(&self) -> Result<()> {
        serde_json::to_writer_pretty(std::io::stdout(), &self.map)
            .map_err(|e| DependentMapError::Io(e.into()))?;
        println!();
        std::io::stdout().flush()?;
        Ok(())
    }
}
