use std::collections::HashSet;
use std::path::Path;

use crate::config::{ProjectConfig, RuleSetting};
use crate::diagnostics::{CodeDiagnostic, Diagnostic, DiagnosticDetails, Severity};
use crate::processors::ignore_directive::{IgnoreDirective, IgnoreDirectives};

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
        severity: Severity,
    ) -> Diagnostic {
        Diagnostic::new_located(
            severity,
            DiagnosticDetails::Code(CodeDiagnostic::UnusedIgnoreDirective()),
            relative_file_path.to_path_buf(),
            ignore_directive.line_no,
            None,
        )
    }

    fn check_unused_ignore_directive(
        &self,
        ignore_directive: &IgnoreDirective,
        diagnostics: &[Diagnostic],
        relative_file_path: &Path,
        severity: Severity,
        matched_diagnostic_indices: &mut HashSet<usize>,
    ) -> Option<Diagnostic> {
        let mut found: bool = false;
        // Tag all indices which match the ignore directive, also note whether we found any
        diagnostics
            .iter()
            .enumerate()
            .for_each(|(index, diagnostic)| {
                if ignore_directive.matches_diagnostic(diagnostic)
                    && !matched_diagnostic_indices.contains(&index)
                {
                    matched_diagnostic_indices.insert(index);
                    found = true;
                }
            });

        // If we didn't find any, this ignore directive is unused
        if !found {
            Some(self.get_unused_ignore_directive_diagnostic(
                ignore_directive,
                relative_file_path,
                severity,
            ))
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
                None,
            ))
        } else {
            None
        }
    }

    fn handle_ignore_directive(
        &self,
        ignore_directive: &IgnoreDirective,
        diagnostics: &mut Vec<Diagnostic>,
        relative_file_path: &Path,
        matched_diagnostic_indices: &mut HashSet<usize>,
    ) {
        if let Some(diagnostic) =
            self.check_missing_ignore_directive_reason(ignore_directive, relative_file_path)
        {
            diagnostics.push(diagnostic)
        }

        if let Ok(severity) = (&self.project_config.rules.unused_ignore_directives).try_into() {
            if let Some(diagnostic) = self.check_unused_ignore_directive(
                ignore_directive,
                diagnostics,
                relative_file_path,
                severity,
                matched_diagnostic_indices,
            ) {
                diagnostics.push(diagnostic);
            }
        }
    }

    fn remove_ignored_diagnostics(
        &self,
        diagnostics: &mut Vec<Diagnostic>,
        matched_diagnostic_indices: &HashSet<usize>,
    ) {
        let mut idx = 0;
        diagnostics.retain(|_| {
            let keep = !matched_diagnostic_indices.contains(&idx);
            idx += 1;
            keep
        });
    }

    pub fn process_diagnostics(
        &self,
        ignore_directives: &IgnoreDirectives,
        diagnostics: &mut Vec<Diagnostic>,
        relative_file_path: &Path,
    ) {
        let mut matched_diagnostic_indices: HashSet<usize> = HashSet::new();
        // Using sorted directives, we can greedily match diagnostics to ignore directives
        // to canonically determine which diagnostics are unused
        for ignore_directive in ignore_directives.sorted_directives() {
            self.handle_ignore_directive(
                ignore_directive,
                diagnostics,
                relative_file_path,
                &mut matched_diagnostic_indices,
            );
        }

        self.remove_ignored_diagnostics(diagnostics, &matched_diagnostic_indices);

        if let Ok(severity) = (&self.project_config.rules.unused_ignore_directives).try_into() {
            for ignore_directive in ignore_directives.redundant_directives() {
                diagnostics.push(self.get_unused_ignore_directive_diagnostic(
                    ignore_directive,
                    relative_file_path,
                    severity,
                ));
            }
        }
    }
}
