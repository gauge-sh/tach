use std::path::Path;

use super::diagnostics::Diagnostic;
use super::error::DiagnosticError;

pub type Result<T> = std::result::Result<T, DiagnosticError>;

// Turn input into diagnostics
pub trait DiagnosticPipeline<'a, P> {
    type Output: IntoIterator<Item = Diagnostic>;

    fn diagnostics(&'a self, input: P) -> Result<Self::Output>;
}

// Turn filepaths into ProcessedFile (references, imports, etc.)
pub trait FileProcessor<'a, P>
where
    P: AsRef<Path>,
{
    type ProcessedFile;

    fn process(&'a self, file_path: P) -> Result<Self::ProcessedFile>;
}

// Turn ProcessedFile into diagnostics
pub trait FileChecker<'a> {
    type ProcessedFile;
    type Output: IntoIterator<Item = Diagnostic>;

    fn check(&'a self, processed_file: &Self::ProcessedFile) -> Result<Self::Output>;
}

// If you can turn a filepath into ProcessedFile, and then turn that ProcessedFile into diagnostics,
// then you can turn a filepath into diagnostics.
impl<'a, T, P> DiagnosticPipeline<'a, P> for T
where
    P: AsRef<Path>,
    T: FileProcessor<'a, P> + FileChecker<'a>,
    <T as FileProcessor<'a, P>>::ProcessedFile: AsRef<<T as FileChecker<'a>>::ProcessedFile>,
{
    type Output = <T as FileChecker<'a>>::Output;

    fn diagnostics(&'a self, file_path: P) -> Result<Self::Output> {
        let processed_file = self.process(file_path)?;
        let diagnostics = self.check(processed_file.as_ref())?;
        Ok(diagnostics)
    }
}
