use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

use super::error::{DependentMapError, Result};

use crate::{
    config::ProjectConfig,
    filesystem::{self, module_to_file_path},
    processors::import::get_normalized_imports_from_ast,
    python::parsing::parse_python_source,
    resolvers::SourceRootResolver,
};

fn process_file(
    path: &PathBuf,
    source_roots: &[PathBuf],
    ignore_type_checking_imports: bool,
) -> Result<HashSet<String>> {
    let file_content = filesystem::read_file_content(path)?;
    let python_source = parse_python_source(&file_content)?;
    let mut result = HashSet::new();

    get_normalized_imports_from_ast(
        source_roots,
        path,
        &python_source,
        ignore_type_checking_imports,
        true,
    )?
    .iter()
    .for_each(|module| {
        if let Some(resolved_module) = module_to_file_path(source_roots, &module.module_path, true)
        {
            result.insert(
                resolved_module
                    .relative_file_path()
                    .to_string_lossy()
                    .to_string(),
            );
        }
    });

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
    project_config: ProjectConfig,
    map: DashMap<String, Vec<String>>,
    direction: Direction,
}

impl DependentMap {
    pub fn new(
        project_root: &PathBuf,
        project_config: &ProjectConfig,
        direction: Direction,
    ) -> Result<Self> {
        let map = Self::build(project_root, project_config, direction)?;
        Ok(Self {
            project_root: project_root.clone(),
            project_config: project_config.clone(),
            map,
            direction,
        })
    }

    pub fn build(
        project_root: &PathBuf,
        project_config: &ProjectConfig,
        direction: Direction,
    ) -> Result<DashMap<String, Vec<String>>> {
        let file_walker = filesystem::FSWalker::try_new(
            project_root,
            &project_config.exclude,
            project_config.respect_gitignore,
        )?;
        let source_root_resolver = SourceRootResolver::new(project_root, &file_walker);
        let source_roots = source_root_resolver.resolve(&project_config.source_roots)?;

        let dependent_map: DashMap<String, Vec<String>> = DashMap::new();
        let ignore_type_checking_imports = project_config.ignore_type_checking_imports;

        source_roots.iter().for_each(|source_root| {
            file_walker
                .walk_pyfiles(&source_root.display().to_string())
                .par_bridge()
                .for_each(|path| {
                    let abs_path = source_root.join(&path);
                    if let Ok(dependencies) =
                        process_file(&abs_path, &source_roots, ignore_type_checking_imports)
                    {
                        for dep in dependencies {
                            match direction {
                                Direction::Dependents => {
                                    dependent_map
                                        .entry(dep)
                                        .or_default()
                                        .push(path.display().to_string());
                                }
                                Direction::Dependencies => {
                                    dependent_map
                                        .entry(path.display().to_string())
                                        .or_default()
                                        .push(dep);
                                }
                            }
                        }
                    }
                });
        });

        Ok(dependent_map)
    }

    pub fn rebuild(&mut self) -> Result<()> {
        let map = Self::build(&self.project_root, &self.project_config, self.direction)?;
        self.map = map;
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
        let source_roots = self
            .project_config
            .source_roots
            .iter()
            .map(|root| self.project_root.join(root))
            .collect::<Vec<PathBuf>>();
        let ignore_type_checking_imports = self.project_config.ignore_type_checking_imports;
        changed_files.par_iter().for_each(|path| {
            let abs_path = self.project_root.join(path);
            if let Ok(dependencies) =
                process_file(&abs_path, &source_roots, ignore_type_checking_imports)
            {
                for dep in dependencies {
                    self.map
                        .entry(dep.clone())
                        .or_default()
                        .push(path.display().to_string());
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
