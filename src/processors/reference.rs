#[derive(Debug)]
pub struct SourceCodeReference<'a> {
    pub content: &'a str,
}

impl<'a> SourceCodeReference<'a> {
    pub fn new(content: &'a str) -> Self {
        Self { content }
    }
}
