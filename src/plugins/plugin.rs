use crate::config::ProjectConfig;
use crate::diagnostics::Diagnostic;

use super::error::PluginError;

pub trait Plugin<C = ProjectConfig, D = Diagnostic>
where
    Self::Check: Sized + PluginCheck<C, D>,
{
    type Check;
    fn setup(config: &C) -> Result<Self::Check, PluginError>;
}

pub trait PluginCheck<C = ProjectConfig, D = Diagnostic> {
    fn check(&self, config: &C) -> Result<Vec<D>, PluginError>;
}
