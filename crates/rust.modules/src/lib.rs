pub mod cache;
pub mod canonical;
pub mod helpers;
pub mod modules;
pub mod permission_checks;

pub use crate::modules::Module;
pub use helpers::*;

use silverpelt::data::Data;

pub type Error = Box<dyn std::error::Error + Send + Sync>; // This is constant and should be copy pasted
pub type Command = poise::Command<Data, Error>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
