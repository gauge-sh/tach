pub mod error;
pub mod glob;
pub mod parsing;
pub mod tree;

pub use glob::ModuleGlobResolver;
pub use parsing::build_module_tree;
pub use tree::{ModuleNode, ModuleTree};
