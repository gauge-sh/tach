use ruff_text_size::TextSize;

use super::import::NormalizedImport;
use super::reference::SourceCodeReference;

#[derive(Debug)]
pub enum Dependency {
    Import(NormalizedImport),
    Reference(SourceCodeReference),
}

impl Dependency {
    pub fn module_path(&self) -> &str {
        match self {
            Dependency::Import(import) => &import.module_path,
            Dependency::Reference(reference) => &reference.module_path,
        }
    }

    pub fn offset(&self) -> TextSize {
        match self {
            Dependency::Import(import) => import.alias_offset,
            Dependency::Reference(reference) => reference.offset,
        }
    }

    pub fn original_line_offset(&self) -> Option<TextSize> {
        match self {
            Dependency::Import(import) => Some(import.import_offset),
            Dependency::Reference(_) => None,
        }
    }
}

impl From<NormalizedImport> for Dependency {
    fn from(normalized_import: NormalizedImport) -> Self {
        Dependency::Import(normalized_import)
    }
}

impl From<SourceCodeReference> for Dependency {
    fn from(source_code_reference: SourceCodeReference) -> Self {
        Dependency::Reference(source_code_reference)
    }
}
