use ruff_linter::Locator;

#[derive(Debug)]
pub struct SourceCodeReference<'a> {
    pub content: &'a str,
    pub locator: Locator<'a>,
}

impl<'a> SourceCodeReference<'a> {
    pub fn new(content: &'a str, locator: Locator<'a>) -> Self {
        Self { content, locator }
    }
}
