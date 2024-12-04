pub mod error;
pub mod parsing;
pub mod tree;

pub use parsing::build_module_tree;
pub use tree::{ModuleNode, ModuleTree};
