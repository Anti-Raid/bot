use std::num::NonZeroU16;

use poise::{serenity_prelude::CreateEmbed, CreateReply};
use sandwich_driver::GetStatusResponse;
use serenity::builder::EditMessage;

type Error = silverpelt::Error;
type Context<'a> = modules::Context<'a>;

#[poise::command(category = "Stats", slash_command, user_cooldown = 1)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let msg = CreateReply::default().embed(
        CreateEmbed::default()
            .title("Pong")
            .field(
                "Local WS Ping",
                format!("{}μs", ctx.ping().await.as_micros()),
                true,
            )
            .field("Edit Latency", "Calculating...", true)
            .field("Real WS Latency", "Finding...", true),
    );

    let st = std::time::Instant::now();

    let mut msg = ctx.send(msg).await?.into_message().await?;

    let new_st = std::time::Instant::now();

    let real_ws_latency = {
        let sandwich_resp = get_sandwich_status(&ctx.data()).await?;
        // Due to Sandwich Virtual Sharding etc, we need to reshard the guild id
        let sid = {
            if let Some(guild_id) = ctx.guild_id() {
                serenity::utils::shard_id(
                    guild_id,
                    NonZeroU16::new(sandwich_resp.shard_conns.len().try_into()?)
                        .unwrap_or(NonZeroU16::new(1).unwrap()),
                )
            } else {
                0 // DMs etc. go to shard 0
            }
        };

        // Convert u16 to i64
        let sid = sid as i64;

        let real_latency = sandwich_resp
            .shard_conns
            .get(&sid)
            .map(|sc| sc.real_latency);

        real_latency
    };

    msg.edit(
        ctx,
        EditMessage::new().embed(
            CreateEmbed::default()
                .title("Pong")
                .field(
                    "Local WS Ping",
                    format!("{}μs", ctx.ping().await.as_micros()),
                    true,
                )
                .field(
                    "Local Edit Ping",
                    format!("{}ms", new_st.duration_since(st).as_millis()),
                    true,
                )
                .field(
                    "Real WS Latency",
                    real_ws_latency
                        .map(|latency| format!("{}ms", latency))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    true,
                ),
        ),
    )
    .await?;

    Ok(())
}

pub async fn get_sandwich_status(
    data: &silverpelt::data::Data,
) -> Result<GetStatusResponse, silverpelt::Error> {
    let status = sandwich_driver::get_status(&data.reqwest).await?;

    if status.shard_conns.len() > data.props.shard_count().await?.into() {
        return Err("Sandwich API returned more shards than the bot has".into());
    }

    Ok(status)
}
