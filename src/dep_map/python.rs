use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::ProjectConfig;

use super::map::{DependentMap, Direction};

#[pyclass(name = "Direction", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyDirection {
    Dependencies = 0,
    Dependents = 1,
}

impl From<PyDirection> for Direction {
    fn from(direction: PyDirection) -> Self {
        match direction {
            PyDirection::Dependencies => Direction::Dependencies,
            PyDirection::Dependents => Direction::Dependents,
        }
    }
}

#[pyclass(name = "DependentMap")]
pub struct PyDependentMap {
    inner: DependentMap,
}

#[pymethods]
impl PyDependentMap {
    #[new]
    #[pyo3(signature = (project_root, project_config, direction = None))]
    fn new(
        project_root: PathBuf,
        project_config: ProjectConfig,
        direction: Option<PyDirection>,
    ) -> PyResult<Self> {
        let direction = direction
            .map(Direction::from)
            .unwrap_or(Direction::Dependents);
        let inner = DependentMap::new(&project_root, &project_config, direction)
            .map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))?;
        Ok(Self { inner })
    }

    fn rebuild(&mut self) -> PyResult<()> {
        self.inner
            .rebuild()
            .map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))
    }

    fn get_closure(&self, paths: Vec<PathBuf>) -> PyResult<HashSet<String>> {
        self.inner
            .get_closure(&paths)
            .map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))
    }

    fn update_files(&mut self, changed_files: Vec<PathBuf>) -> PyResult<()> {
        self.inner
            .update_files(&changed_files)
            .map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))
    }

    fn write_to_file(&self, path: PathBuf) -> PyResult<()> {
        self.inner
            .write_to_file(&path)
            .map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))
    }

    fn write_to_stdout(&self) -> PyResult<()> {
        self.inner
            .write_to_stdout()
            .map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))
    }
}
