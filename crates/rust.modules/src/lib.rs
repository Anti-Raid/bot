pub mod cache;
pub mod canonical;
pub mod helpers;
pub mod module_config;
pub mod modules;
pub mod permission_checks;
pub mod types;
pub mod utils;

pub use crate::modules::Module;
pub use crate::types::{CommandExtendedData, CommandExtendedDataMap};
pub use helpers::*;

use silverpelt::data::Data;

pub type Error = Box<dyn std::error::Error + Send + Sync>; // This is constant and should be copy pasted
pub type Command = poise::Command<Data, Error>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
