use silverpelt::data::Data;
use std::sync::Arc;

#[derive(Clone)]
pub struct SettingsData {
    pub data: Arc<Data>,
    pub serenity_context: serenity::all::Context,
    pub guild_id: serenity::all::GuildId,
    pub author: serenity::all::UserId,
}

impl Default for SettingsData {
    fn default() -> Self {
        unreachable!("SettingsData::default() should never be called")
    }
}

impl serde::Serialize for SettingsData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

/// Given the Data and a cache_http, returns the settings data
pub fn settings_data(
    serenity_context: serenity::all::Context,
    guild_id: serenity::all::GuildId,
    author: serenity::all::UserId,
) -> SettingsData {
    SettingsData {
        data: serenity_context.data::<Data>(),
        serenity_context,
        guild_id,
        author,
    }
}
