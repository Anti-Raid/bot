use silverpelt::data::Data;
use std::sync::Arc;

/// Given the Data and a cache_http, returns the settings data
pub fn settings_data(serenity_context: serenity::all::Context) -> ar_settings::types::SettingsData {
    ar_settings::types::SettingsData {
        data: serenity_context.data::<Data>(),
        serenity_context,
    }
}

// Returns the module cache from Data
pub fn module_cache(data: &Data) -> Arc<crate::cache::ModuleCache> {
    data.props
        .slot()
        .expect("ModuleCache not initialized")
        .downcast::<crate::cache::ModuleCache>()
        .expect("ModuleCache not initialized")
}
