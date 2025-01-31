use std::path::Path;

use super::diagnostics::Diagnostic;
use super::error::DiagnosticError;

pub type Result<T> = std::result::Result<T, DiagnosticError>;

// Turn filepaths into diagnostics
pub trait DiagnosticPipeline<'a, P>
where
    P: AsRef<Path>,
{
    type Context;
    type Output: IntoIterator<Item = Diagnostic>;

    fn diagnostics(&'a self, input: P, context: &'a Self::Context) -> Result<Self::Output>;
}

// Turn filepaths into IR (references, imports, etc.)
pub trait FileProcessor<'a> {
    type IR;
    type Context;

    fn process(&'a self, file_path: &Path, context: &'a Self::Context) -> Result<Self::IR>;
}

// Turn IR into diagnostics
pub trait FileChecker<'a> {
    type IR;
    type Context;
    type Output: IntoIterator<Item = Diagnostic>;

    fn check(
        &'a self,
        file_path: &Path,
        input: &Self::IR,
        context: &'a Self::Context,
    ) -> Result<Self::Output>;
}

// If you can turn a filepath into IR, and then turn that IR into diagnostics,
// then you can turn a filepath into diagnostics.
impl<'a, T, P> DiagnosticPipeline<'a, P> for T
where
    P: AsRef<Path>,
    T: FileProcessor<'a> + FileChecker<'a>,
    <T as FileProcessor<'a>>::IR: AsRef<<T as FileChecker<'a>>::IR>,
    <T as FileProcessor<'a>>::Context: AsRef<<T as FileChecker<'a>>::Context>,
{
    type Context = <T as FileProcessor<'a>>::Context;
    type Output = <T as FileChecker<'a>>::Output;

    fn diagnostics(&'a self, input: P, context: &'a Self::Context) -> Result<Self::Output> {
        let ir = self.process(input.as_ref(), context)?;
        let diagnostics = self.check(input.as_ref(), ir.as_ref(), context.as_ref())?;
        Ok(diagnostics)
    }
}
