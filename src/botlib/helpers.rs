use silverpelt::data::Data;

/// Given the Data and a cache_http, returns the settings data
pub fn settings_data(serenity_context: serenity::all::Context) -> ar_settings::types::SettingsData {
    ar_settings::types::SettingsData {
        data: serenity_context.data::<Data>(),
        serenity_context,
    }
}
