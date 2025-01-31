use std::path::Path;

use crate::config::RuleSetting;
use crate::diagnostics::{
    CodeDiagnostic, Diagnostic, DiagnosticDetails, FileChecker, FileContext,
    Result as DiagnosticResult,
};
use crate::processors::imports::{IgnoreDirective, IgnoreDirectives};
pub struct IgnoreDirectiveData<'a> {
    ignore_directives: &'a IgnoreDirectives,
    diagnostics: &'a Vec<Diagnostic>,
}

impl<'a> IgnoreDirectiveData<'a> {
    pub fn new(ignore_directives: &'a IgnoreDirectives, diagnostics: &'a Vec<Diagnostic>) -> Self {
        Self {
            ignore_directives,
            diagnostics,
        }
    }
}

pub struct IgnoreDirectiveChecker;

impl Default for IgnoreDirectiveChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl IgnoreDirectiveChecker {
    pub fn new() -> Self {
        Self {}
    }

    fn get_unused_ignore_directive_diagnostic(
        &self,
        ignore_directive: &IgnoreDirective,
        context: &FileContext,
    ) -> Diagnostic {
        Diagnostic::new_located(
            (&context.project_config.rules.unused_ignore_directives)
                .try_into()
                .unwrap(),
            DiagnosticDetails::Code(CodeDiagnostic::UnusedIgnoreDirective()),
            context.relative_file_path.to_path_buf(),
            ignore_directive.line_no,
        )
    }

    fn check_unused_ignore_directive(
        &self,
        ignore_directive: &IgnoreDirective,
        diagnostics: &Vec<Diagnostic>,
        context: &FileContext,
    ) -> Option<Diagnostic> {
        if context.project_config.rules.unused_ignore_directives == RuleSetting::Off {
            return None;
        }

        if !diagnostics.iter().any(|diagnostic| {
            diagnostic.line_number() == Some(ignore_directive.line_no) && diagnostic.is_code()
        }) {
            Some(self.get_unused_ignore_directive_diagnostic(ignore_directive, context))
        } else {
            None
        }
    }

    fn check_missing_ignore_directive_reason(
        &self,
        ignore_directive: &IgnoreDirective,
        context: &FileContext,
    ) -> Option<Diagnostic> {
        if context
            .project_config
            .rules
            .require_ignore_directive_reasons
            == RuleSetting::Off
        {
            return None;
        }

        if ignore_directive.reason.is_empty() {
            Some(Diagnostic::new_located(
                (&context
                    .project_config
                    .rules
                    .require_ignore_directive_reasons)
                    .try_into()
                    .unwrap(),
                DiagnosticDetails::Code(CodeDiagnostic::MissingIgnoreDirectiveReason()),
                context.relative_file_path.to_path_buf(),
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
        context: &FileContext,
    ) -> Vec<Diagnostic> {
        vec![
            self.check_unused_ignore_directive(ignore_directive, diagnostics, context),
            self.check_missing_ignore_directive_reason(ignore_directive, context),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

impl<'a> FileChecker<'a> for IgnoreDirectiveChecker {
    type IR = IgnoreDirectiveData<'a>;
    type Context = FileContext<'a>;
    type Output = Vec<Diagnostic>;

    fn check(
        &'a self,
        _file_path: &Path,
        ir: &Self::IR,
        context: &Self::Context,
    ) -> DiagnosticResult<Self::Output> {
        let mut diagnostics = Vec::new();
        for ignore_directive in ir.ignore_directives.active_directives() {
            diagnostics.extend(self.check_ignore_directive(
                ignore_directive,
                ir.diagnostics,
                context,
            ));
        }
        for ignore_directive in ir.ignore_directives.redundant_directives() {
            diagnostics
                .push(self.get_unused_ignore_directive_diagnostic(ignore_directive, context));
        }

        Ok(diagnostics)
    }
}
