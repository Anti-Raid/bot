pub mod canonical;
pub mod permission_checks;
pub mod settings;

use silverpelt::data::Data;

pub type Error = Box<dyn std::error::Error + Send + Sync>; // This is constant and should be copy pasted
pub type Command = poise::Command<Data, Error>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type CommandPermissionMetadata = indexmap::IndexMap<String, Vec<String>>;
