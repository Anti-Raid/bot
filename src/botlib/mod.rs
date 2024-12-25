pub mod canonical;
pub mod helpers;
pub mod permission_checks;

use silverpelt::data::Data;

pub type Error = Box<dyn std::error::Error + Send + Sync>; // This is constant and should be copy pasted
pub type Command = poise::Command<Data, Error>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type CommandPermissionMetadata = indexmap::IndexMap<String, Vec<String>>;
