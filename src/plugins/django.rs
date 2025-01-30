use crate::config::ProjectConfig;
use crate::diagnostics::Diagnostic;

use super::error::PluginError;
use super::plugin::CheckPlugin;

pub struct DjangoPlugin {
    settings_module: String,
}

impl CheckPlugin for DjangoPlugin {
    fn setup(config: &ProjectConfig) -> Result<Self, PluginError> {
        config
            .plugins
            .django
            .as_ref()
            .map(|django_config| Self {
                settings_module: django_config.settings_module.clone(),
            })
            .ok_or(PluginError::SetupFailed(
                "Django plugin not configured".to_string(),
            ))
    }

    fn check(&self) -> Result<Vec<Diagnostic>, PluginError> {
        Ok(vec![])
    }
}
