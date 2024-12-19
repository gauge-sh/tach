pub mod check_internal;
pub mod lsp_server;
pub mod module;
pub mod test;
#[cfg(test)]
pub mod fixtures {
    use rstest::fixture;

    #[fixture]
    pub fn example_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/tests/example")
    }

    #[fixture]
    pub fn tests_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python/tests")
    }
}
