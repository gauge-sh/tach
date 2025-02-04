use std::path::Path;

use crate::config::{ProjectConfig, RuleSetting};
use crate::diagnostics::{CodeDiagnostic, Diagnostic, DiagnosticDetails};
use crate::processors::import::{IgnoreDirective, IgnoreDirectives};

pub struct IgnoreDirectivePostProcessor<'a> {
    project_config: &'a ProjectConfig,
}

impl<'a> IgnoreDirectivePostProcessor<'a> {
    pub fn new(project_config: &'a ProjectConfig) -> Self {
        Self { project_config }
    }

    fn get_unused_ignore_directive_diagnostic(
        &self,
        ignore_directive: &IgnoreDirective,
        relative_file_path: &Path,
    ) -> Diagnostic {
        Diagnostic::new_located(
            (&self.project_config.rules.unused_ignore_directives)
                .try_into()
                .unwrap(),
            DiagnosticDetails::Code(CodeDiagnostic::UnusedIgnoreDirective()),
            relative_file_path.to_path_buf(),
            ignore_directive.line_no,
        )
    }

    fn check_unused_ignore_directive(
        &self,
        ignore_directive: &IgnoreDirective,
        diagnostics: &Vec<Diagnostic>,
        relative_file_path: &Path,
    ) -> Option<Diagnostic> {
        if self.project_config.rules.unused_ignore_directives == RuleSetting::Off {
            return None;
        }

        if !diagnostics
            .iter()
            .any(|diagnostic| ignore_directive.matches_diagnostic(diagnostic))
        {
            Some(self.get_unused_ignore_directive_diagnostic(ignore_directive, relative_file_path))
        } else {
            None
        }
    }

    fn check_missing_ignore_directive_reason(
        &self,
        ignore_directive: &IgnoreDirective,
        relative_file_path: &Path,
    ) -> Option<Diagnostic> {
        if self.project_config.rules.require_ignore_directive_reasons == RuleSetting::Off {
            return None;
        }

        if ignore_directive.reason.is_empty() {
            Some(Diagnostic::new_located(
                (&self.project_config.rules.require_ignore_directive_reasons)
                    .try_into()
                    .unwrap(),
                DiagnosticDetails::Code(CodeDiagnostic::MissingIgnoreDirectiveReason()),
                relative_file_path.to_path_buf(),
                ignore_directive.line_no,
            ))
        } else {
            None
        }
    }

    fn check_ignore_directive(
        &self,
        ignore_directive: &IgnoreDirective,
        diagnostics: &Vec<Diagnostic>,
        relative_file_path: &Path,
    ) -> Vec<Diagnostic> {
        vec![
            self.check_unused_ignore_directive(ignore_directive, diagnostics, relative_file_path),
            self.check_missing_ignore_directive_reason(ignore_directive, relative_file_path),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn check_ignore_directives(
        &self,
        ignore_directives: &IgnoreDirectives,
        existing_diagnostics: &Vec<Diagnostic>,
        relative_file_path: &Path,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for ignore_directive in ignore_directives.active_directives() {
            diagnostics.extend(self.check_ignore_directive(
                ignore_directive,
                existing_diagnostics,
                relative_file_path,
            ));
        }
        for ignore_directive in ignore_directives.redundant_directives() {
            diagnostics.push(
                self.get_unused_ignore_directive_diagnostic(ignore_directive, relative_file_path),
            );
        }

        diagnostics
    }

    fn remove_ignored_diagnostics(
        &self,
        ignore_directives: &IgnoreDirectives,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for ignore_directive in ignore_directives.active_directives() {
            diagnostics.retain(|diagnostic| !ignore_directive.matches_diagnostic(diagnostic));
        }
    }

    pub fn process_diagnostics(
        &self,
        ignore_directives: &IgnoreDirectives,
        diagnostics: &mut Vec<Diagnostic>,
        relative_file_path: &Path,
    ) {
        // Check for diagnostics related to ignore directives
        let ignore_directive_diagnostics =
            self.check_ignore_directives(ignore_directives, diagnostics, relative_file_path);

        // Remove ignored diagnostics
        self.remove_ignored_diagnostics(ignore_directives, diagnostics);

        // Add the new diagnostics to the list
        diagnostics.extend(ignore_directive_diagnostics);
    }
}
