use ruff_text_size::TextSize;

#[derive(Debug)]
pub struct SourceCodeReference {
    pub module_path: String,
    pub offset: TextSize,
}

impl SourceCodeReference {
    pub fn new(module_path: String, offset: TextSize) -> Self {
        Self {
            module_path,
            offset,
        }
    }
}
