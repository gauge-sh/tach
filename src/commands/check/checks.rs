use std::collections::{HashMap, HashSet};

use crate::diagnostics::{CodeDiagnostic, Diagnostic, DiagnosticDetails};
use crate::{
    config::{rules::RuleSetting, ProjectConfig},
    external::parsing::{normalize_package_name, ProjectInfo},
    processors::imports::{DirectiveIgnoredImport, NormalizedImport},
};

#[derive(Debug)]
pub enum ImportProcessResult {
    UndeclaredDependency(String),
    UsedDependencies(Vec<String>),
    Excluded(Vec<String>),
}

pub(super) fn check_import_external(
    import: &NormalizedImport,
    project_info: &ProjectInfo,
    module_mappings: &HashMap<String, Vec<String>>,
    excluded_external_modules: &HashSet<String>,
    stdlib_modules: &HashSet<String>,
) -> ImportProcessResult {
    let top_level_module_name = import.top_level_module_name().to_string();
    let default_distribution_names = vec![top_level_module_name.clone()];
    let distribution_names: Vec<String> = module_mappings
        .get(&top_level_module_name)
        .map(|dist_names| {
            dist_names
                .iter()
                .map(|dist_name| normalize_package_name(dist_name))
                .collect()
        })
        .unwrap_or(default_distribution_names);

    if distribution_names
        .iter()
        .any(|dist_name| excluded_external_modules.contains(dist_name))
        || stdlib_modules.contains(&top_level_module_name)
    {
        return ImportProcessResult::Excluded(distribution_names);
    }

    let is_declared = distribution_names
        .iter()
        .any(|dist_name| project_info.dependencies.contains(dist_name));

    if !is_declared {
        ImportProcessResult::UndeclaredDependency(top_level_module_name.to_string())
    } else {
        ImportProcessResult::UsedDependencies(distribution_names)
    }
}

pub(super) fn check_unused_ignore_directive_external(
    directive_ignored_import: &DirectiveIgnoredImport,
    project_info: &ProjectInfo,
    module_mappings: &HashMap<String, Vec<String>>,
    excluded_external_modules: &HashSet<String>,
    stdlib_modules: &HashSet<String>,
    project_config: &ProjectConfig,
) -> Result<(), Diagnostic> {
    if let ImportProcessResult::UsedDependencies(_) | ImportProcessResult::Excluded(_) =
        check_import_external(
            directive_ignored_import.import,
            project_info,
            module_mappings,
            excluded_external_modules,
            stdlib_modules,
        )
    {
        match project_config.rules.unused_ignore_directives {
            RuleSetting::Error => Err(Diagnostic::new_global(
                (&project_config.rules.unused_ignore_directives)
                    .try_into()
                    .unwrap(),
                DiagnosticDetails::Code(CodeDiagnostic::UnnecessarilyIgnoredImport {
                    import_mod_path: directive_ignored_import.import.module_path.to_string(),
                }),
            )),
            RuleSetting::Warn => Err(Diagnostic::new_global(
                (&project_config.rules.unused_ignore_directives)
                    .try_into()
                    .unwrap(),
                DiagnosticDetails::Code(CodeDiagnostic::UnnecessarilyIgnoredImport {
                    import_mod_path: directive_ignored_import.import.module_path.to_string(),
                }),
            )),
            RuleSetting::Off => Ok(()),
        }
    } else {
        Ok(())
    }
}

pub(super) fn check_missing_ignore_directive_reason(
    directive_ignored_import: &DirectiveIgnoredImport,
    project_config: &ProjectConfig,
) -> Result<(), Diagnostic> {
    if project_config.rules.require_ignore_directive_reasons == RuleSetting::Off {
        return Ok(());
    }

    if directive_ignored_import.reason.is_empty() {
        Err(Diagnostic::new_global(
            (&project_config.rules.require_ignore_directive_reasons)
                .try_into()
                .unwrap(),
            DiagnosticDetails::Code(CodeDiagnostic::MissingIgnoreDirectiveReason()),
        ))
    } else {
        Ok(())
    }
}
