use crate::{config::CONFIG, Context, Error};
use log::error;
use silverpelt::data::Data;

const BOT_ONBOARDING_VERSION: i32 = 2;

/// Standard error handler for Anti-Raid
pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!("Error in command `{}`: {:?}", ctx.command().name, error,);

            let err = ctx
                .send(
                    poise::CreateReply::new()
                        .embed(
                            serenity::all::CreateEmbed::new()
                                .color(serenity::all::Color::RED)
                                .title("An error has occurred")
                                .description(error.to_string()),
                        )
                        .components(vec![serenity::all::CreateActionRow::Buttons(
                            vec![serenity::all::CreateButton::new_link(
                                &CONFIG.meta.support_server_invite,
                            )
                            .label("Support Server")]
                            .into(),
                        )]),
                )
                .await;

            if let Err(e) = err {
                error!("Message send error for FrameworkError::Command: {}", e);
            }
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx, .. } => {
            error!(
                "[Possible] error in command `{}`: {:?}",
                ctx.command().qualified_name,
                error,
            );

            if let Some(error) = error {
                error!("Error in command `{}`: {:?}", ctx.command().name, error,);

                let err = ctx
                    .send(
                        poise::CreateReply::new()
                            .embed(
                                serenity::all::CreateEmbed::new()
                                    .color(serenity::all::Color::RED)
                                    .title("Command Check Failed")
                                    .description(error.to_string()),
                            )
                            .components(vec![serenity::all::CreateActionRow::Buttons(
                                vec![serenity::all::CreateButton::new_link(
                                    &CONFIG.meta.support_server_invite,
                                )
                                .label("Support Server")]
                                .into(),
                            )]),
                    )
                    .await;

                if let Err(e) = err {
                    error!(
                        "Message send error for FrameworkError::CommandCheckFailed: {}",
                        e
                    );
                }
            }
        }
        poise::FrameworkError::CommandPanic { payload, ctx, .. } => {
            error!(
                "Command `{}` panicked: {:?}",
                ctx.command().qualified_name,
                payload,
            );

            let err = ctx
                .send(
                    poise::CreateReply::new()
                    .embed(
                        serenity::all::CreateEmbed::new()
                            .color(serenity::all::Color::RED)
                            .title("Command Panic")
                            .description(format!("The command panicked. Please report this on our support server.\n\n```{}`", payload.unwrap_or("No payload provided".to_string()))),
                    )
                    .components(vec![serenity::all::CreateActionRow::Buttons(vec![
                        serenity::all::CreateButton::new_link(
                            &CONFIG.meta.support_server_invite,
                        )
                        .label("Support Server"),
                    ].into())]),
                )
                .await;

            if let Err(e) = err {
                error!("Message send error for FrameworkError::CommandPanic: {}", e);
            }
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e);
            }
        }
    }
}

fn setup_message<'a>() -> poise::CreateReply<'a> {
    poise::CreateReply::new()
    .embed(
        serenity::all::CreateEmbed::new()
        .title("Hi there!")
        .description(r#"Here are some of the cool things you can do with AntiRaid:

**Scripting:** AntiRaid allows you to write custom scripts to for total flexibility and control over your server...
**Server Backups:** AntiRaid features downloadable backups which can then be restored even in the event of disaster!
**Lockdowns:** AntiRaid can lockdown your server in the event of a raid. And for automation, scripts can make lockdowns automatically as well!!!

Please check out the `User Guide` and use the `Website` to tailor AntiRaid to the needs of your server! And, if you need help, feel free to join our `Support Server`!  

*Note: Feel free to rerun the command you were trying to run once you're content with your servers' configuration*
        "#)
    )
    .components(
        vec![
            serenity::all::CreateActionRow::Buttons(
                vec![
                    serenity::all::CreateButton::new_link(
                        CONFIG.sites.docs.clone(),
                    )
                    .label("User Guide"),
                    serenity::all::CreateButton::new_link(
                        CONFIG.sites.frontend.clone(),
                    )
                    .label("Website"),
                    serenity::all::CreateButton::new_link(
                        CONFIG.meta.support_server_invite.clone(),
                    )
                    .label("Support Server")
                ].into()
            )
        ]
    )
}

pub async fn command_check(ctx: Context<'_>) -> Result<bool, Error> {
    let guild_id = ctx.guild_id();

    let Some(guild_id) = guild_id else {
        return Err("This command can only be run from servers".into());
    };

    let data = ctx.data();

    #[derive(sqlx::FromRow)]
    struct GOSRecord {
        bot_onboarding_seen_ver: i32,
    }

    let guild_onboarding_status: Option<GOSRecord> =
        sqlx::query_as("SELECT bot_onboarding_seen_ver FROM guilds WHERE id = $1")
            .bind(guild_id.to_string())
            .fetch_optional(&data.pool)
            .await?;

    if let Some(guild_onboarding_status) = guild_onboarding_status {
        if guild_onboarding_status.bot_onboarding_seen_ver != BOT_ONBOARDING_VERSION {
            // Send setup message instead
            ctx.send(setup_message()).await?;

            // Set onboarding status to true
            sqlx::query("UPDATE guilds SET bot_onboarding_seen_ver = $1 WHERE id = $2")
                .bind(BOT_ONBOARDING_VERSION)
                .bind(guild_id.to_string())
                .execute(&data.pool)
                .await?;

            return Ok(false);
        }
    } else {
        // Guild not found, create it
        sqlx::query("INSERT INTO guilds (id, bot_onboarding_seen_ver) VALUES ($1, $2)")
            .bind(guild_id.to_string())
            .bind(BOT_ONBOARDING_VERSION)
            .execute(&data.pool)
            .await?;

        // Send setup message instead
        ctx.send(setup_message()).await?;
        return Ok(false);
    }

    #[derive(sqlx::FromRow)]
    struct UserCountRecord {
        count: Option<i64>,
    }

    let user: UserCountRecord = sqlx::query_as("SELECT COUNT(*) FROM users WHERE user_id = $1")
        .bind(guild_id.to_string())
        .fetch_one(&data.pool)
        .await?;

    if user.count.unwrap_or_default() == 0 {
        // User not found, create it
        sqlx::query("INSERT INTO users (user_id) VALUES ($1)")
            .bind(guild_id.to_string())
            .execute(&data.pool)
            .await?;
    }

    Ok(true)
}
