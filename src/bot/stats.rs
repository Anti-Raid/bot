use crate::{Context, Error};
use poise::{serenity_prelude::CreateEmbed, CreateReply};
use rust_buildstats::{
    BUILD_CPU, CARGO_PROFILE, GIT_COMMIT_MSG, GIT_REPO, GIT_SHA, RUSTC_VERSION, VERSION,
};
use sqlx::types::chrono;

#[poise::command(category = "Stats", slash_command, user_cooldown = 1)]
pub async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    let total_cached_guilds = ctx.cache().guild_count();

    let total_guilds = {
        let sandwich_resp = sandwich_driver::get_status(&ctx.data().reqwest).await?;

        let mut guild_count = 0;
        sandwich_resp.shard_conns.iter().for_each(|(_, sc)| {
            guild_count += sc.guilds;
        });

        guild_count
    };

    let total_users = {
        let mut count = 0;

        for guild in ctx.cache().guilds() {
            {
                let guild = guild.to_guild_cached(ctx.cache());

                if let Some(guild) = guild {
                    count += guild.member_count;
                }
            }
        }

        count
    };

    let msg = CreateReply::default().embed(
        CreateEmbed::default()
            .title("Bot Stats")
            .field(
                "Bot name",
                ctx.serenity_context().cache.current_user().name.to_string(),
                true,
            )
            .field("Bot version", VERSION, true)
            .field("rustc", RUSTC_VERSION, true)
            .field(
                "Git Commit",
                format!("[{}]({}/commit/{})", GIT_SHA, GIT_REPO, GIT_SHA),
                true,
            )
            .field(
                "Uptime",
                {
                    let duration: std::time::Duration = std::time::Duration::from_secs(
                        (chrono::Utc::now().timestamp() - config::CONFIG.start_time) as u64,
                    );

                    let seconds = duration.as_secs() % 60;
                    let minutes = (duration.as_secs() / 60) % 60;
                    let hours = (duration.as_secs() / 60) / 60;

                    format!("{}h{}m{}s", hours, minutes, seconds)
                },
                true,
            )
            .field("Cached Servers", total_cached_guilds.to_string(), true)
            .field("Total Servers", total_guilds.to_string(), true)
            .field("Users", total_users.to_string(), true)
            .field("Commit Message", GIT_COMMIT_MSG, true)
            .field("Built On", BUILD_CPU, true)
            .field("Cargo Profile", CARGO_PROFILE, true),
    );

    ctx.send(msg).await?;
    Ok(())
}
