pub mod canonical;
pub mod durationstring;
pub mod numericlistparser;
pub mod permission_checks;
pub mod specialchannelallocs;

use silverpelt::data::Data;

pub type Error = Box<dyn std::error::Error + Send + Sync>; // This is constant and should be copy pasted
pub type Command = poise::Command<Data, Error>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
