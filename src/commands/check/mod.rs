pub mod check_external;
pub mod check_internal;
pub mod checks;
pub mod diagnostics;
pub mod error;

pub use check_external::check as check_external;
pub use check_internal::check as check_internal;
pub use diagnostics::Diagnostic;
pub use error::{CheckError, ExternalCheckError};
