use pyo3::prelude::*;

use crate::diagnostics::Diagnostic;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Dependency,
    Interface,
}

impl IntoPy<PyObject> for ErrorKind {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Dependency => "DEPENDENCY".to_object(py),
            Self::Interface => "INTERFACE".to_object(py),
        }
    }
}

#[derive(Debug, Clone)]
#[pyclass(get_all, module = "tach.extension")]
pub struct UsageError {
    pub file: String,
    pub line_number: usize,
    pub member: String,
    pub usage_module: String,
    pub definition_module: String,
    pub error_type: ErrorKind,
    pub usage_layer: Option<String>,
    pub definition_layer: Option<String>,
}

impl TryFrom<Diagnostic> for UsageError {
    type Error = ();

    fn try_from(value: Diagnostic) -> Result<Self, Self::Error> {
        if let (
            is_interface_error,
            is_dependency_error,
            Some(file_path),
            Some(line_number),
            Some(member),
            Some(usage_module),
            Some(definition_module),
            usage_layer,
            definition_layer,
        ) = (
            value.is_interface_error(),
            value.is_dependency_error(),
            value.file_path(),
            value.line_number(),
            value.dependency(),
            value.usage_module(),
            value.definition_module(),
            value.usage_layer(),
            value.definition_layer(),
        ) {
            let error_type = match (is_interface_error, is_dependency_error) {
                (false, false) => {
                    return Err(());
                }
                (true, false) => ErrorKind::Interface,
                (false, true) => ErrorKind::Dependency,
                _ => return Err(()),
            };

            Ok(Self {
                file: file_path.to_string_lossy().to_string(),
                line_number,
                member: member.to_string(),
                usage_module: usage_module.to_string(),
                definition_module: definition_module.to_string(),
                error_type,
                usage_layer: usage_layer.map(|s| s.to_string()),
                definition_layer: definition_layer.map(|s| s.to_string()),
            })
        } else {
            Err(())
        }
    }
}

#[pyfunction]
pub fn into_usage_errors(diagnostics: Vec<Diagnostic>) -> Vec<UsageError> {
    diagnostics
        .into_iter()
        .filter_map(|d| UsageError::try_from(d).ok())
        .collect()
}
