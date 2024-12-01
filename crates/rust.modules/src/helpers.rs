use silverpelt::data::Data;
use std::sync::Arc;

/// Given the Data and a cache_http, returns the settings data
pub fn settings_data(
    data: &Data,
    serenity_context: serenity::all::Context,
) -> ar_settings::types::SettingsData {
    ar_settings::types::SettingsData {
        pool: data.pool.clone(),
        reqwest: data.reqwest.clone(),
        object_store: data.object_store.clone(),
        cache_http: botox::cache::CacheHttpImpl::from_ctx(&serenity_context),
        serenity_context,
    }
}

/// Given a settings data, return the data
///
/// This is just a wrapper for settings_data.serenity_context.data::<Data>().clone()
pub fn get_data(settings_data: &ar_settings::types::SettingsData) -> Arc<Data> {
    settings_data.serenity_context.data::<Data>().clone()
}

// Returns the module cache from Data
pub fn module_cache(data: &Data) -> Arc<crate::cache::ModuleCache> {
    data.props
        .slot()
        .expect("ModuleCache not initialized")
        .downcast::<crate::cache::ModuleCache>()
        .expect("ModuleCache not initialized")
}
