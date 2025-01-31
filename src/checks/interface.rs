use std::path::PathBuf;

use crate::config::root_module::RootModuleTreatment;
use crate::config::{InterfaceConfig, ModuleConfig};
use crate::diagnostics::{
    CodeDiagnostic, ConfigurationDiagnostic, Diagnostic, DiagnosticDetails, FileChecker,
    FileContext, Result as DiagnosticResult,
};
use crate::interfaces::compiled::CompiledInterfaces;
use crate::interfaces::data_types::{TypeCheckCache, TypeCheckResult};
use crate::interfaces::error::InterfaceError;
use crate::processors::imports::{NormalizedImport, NormalizedImports, ProjectImports};

#[derive(Debug)]
pub enum InterfaceCheckResult {
    Exposed { type_check_result: TypeCheckResult },
    NotExposed,
    NoInterfaces,
    TopLevelModule,
}

pub struct InterfaceChecker {
    interfaces: CompiledInterfaces,
    type_check_cache: Option<TypeCheckCache>,
}

impl InterfaceChecker {
    pub fn new(interfaces: &[InterfaceConfig]) -> Self {
        let compiled = CompiledInterfaces::build(interfaces);

        Self {
            interfaces: compiled,
            type_check_cache: None,
        }
    }

    pub fn with_type_check_cache(
        mut self,
        modules: &[ModuleConfig],
        source_roots: &[PathBuf],
    ) -> Result<Self, InterfaceError> {
        let type_check_cache = TypeCheckCache::build(&self.interfaces, modules, source_roots)?;
        self.type_check_cache = Some(type_check_cache);
        Ok(self)
    }

    fn check_member(&self, member: &str, module_path: &str) -> InterfaceCheckResult {
        if member.is_empty() {
            return InterfaceCheckResult::TopLevelModule;
        }

        let matching_interfaces = self.interfaces.get_interfaces(module_path);

        if matching_interfaces.is_empty() {
            return InterfaceCheckResult::NoInterfaces;
        }

        let mut is_exposed = false;
        for interface in matching_interfaces {
            if interface.expose.iter().any(|re| re.is_match(member)) {
                is_exposed = true;
            }
        }

        if !is_exposed {
            return InterfaceCheckResult::NotExposed;
        }

        InterfaceCheckResult::Exposed {
            type_check_result: self
                .type_check_cache
                .as_ref()
                .map(|cache| cache.get_result(member))
                .unwrap_or(TypeCheckResult::Unknown),
        }
    }

    fn check_interfaces(
        &self,
        import: &NormalizedImport,
        context: &FileContext,
    ) -> DiagnosticResult<Vec<Diagnostic>> {
        if let Some(import_module_config) = context
            .module_tree
            .find_nearest(&import.module_path)
            .as_ref()
            .and_then(|module| module.config.as_ref())
        {
            if import_module_config == context.file_module_config {
                return Ok(vec![]);
            }

            if import_module_config.is_root()
                && context.project_config.root_module == RootModuleTreatment::Ignore
            {
                return Ok(vec![]);
            }

            let import_member = import
                .module_path
                .strip_prefix(&import_module_config.path)
                .and_then(|s| s.strip_prefix('.'))
                .unwrap_or("");
            let check_result = self.check_member(import_member, &import_module_config.path);
            match check_result {
                InterfaceCheckResult::NotExposed => Ok(vec![Diagnostic::new_located_error(
                    context.relative_file_path.to_path_buf(),
                    import.line_no,
                    DiagnosticDetails::Code(CodeDiagnostic::PrivateImport {
                        import_mod_path: import.module_path.to_string(),
                        usage_module: context.file_module_config.path.to_string(),
                        definition_module: import_module_config.path.to_string(),
                    }),
                )]),
                InterfaceCheckResult::Exposed {
                    type_check_result: TypeCheckResult::DidNotMatchInterface { expected },
                } => Ok(vec![Diagnostic::new_located_error(
                    context.relative_file_path.to_path_buf(),
                    import.line_no,
                    DiagnosticDetails::Code(CodeDiagnostic::InvalidDataTypeExport {
                        import_mod_path: import.module_path.to_string(),
                        usage_module: context.file_module_config.path.to_string(),
                        definition_module: import_module_config.path.to_string(),
                        expected_data_type: expected.to_string(),
                    }),
                )]),
                InterfaceCheckResult::Exposed {
                    type_check_result: TypeCheckResult::MatchedInterface { .. },
                }
                | InterfaceCheckResult::Exposed {
                    type_check_result: TypeCheckResult::Unknown,
                }
                | InterfaceCheckResult::NoInterfaces
                | InterfaceCheckResult::TopLevelModule => Ok(vec![]),
            }
        } else {
            Ok(vec![Diagnostic::new_global_error(
                DiagnosticDetails::Configuration(ConfigurationDiagnostic::ModuleConfigNotFound {
                    module_path: import.module_path.to_string(),
                }),
            )])
        }
    }
}

impl<'a> FileChecker<'a> for InterfaceChecker {
    type IR = NormalizedImports<ProjectImports>;
    type Context = FileContext<'a>;
    type Output = Vec<Diagnostic>;

    fn check(
        &'a self,
        _file_path: &std::path::Path,
        input: &Self::IR,
        context: &'a Self::Context,
    ) -> DiagnosticResult<Self::Output> {
        let mut diagnostics = vec![];
        for import in input.active_imports() {
            diagnostics.extend(self.check_interfaces(import, context)?);
        }

        Ok(diagnostics)
    }
}
