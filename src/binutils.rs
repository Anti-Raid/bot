use crate::{config::CONFIG, Context, Error};
use log::error;
use silverpelt::data::Data;

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
        .title("Thank you for adding AntiRaid")
        .description(r#"While you have successfully added AntiRaid to your server, it won't do much until you take some time to configure it to your needs.

Please check out the `User Guide` and use the `Website` to tailor AntiRaid to the needs of your server! And, if you need help, feel free to join our `Support Server`!  

*Note: Feel free to rerun the command you were trying to run once you're content with your AntiRaid configuration*
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

    let guild_onboarding_status = sqlx::query!(
        "SELECT finished_onboarding FROM guilds WHERE id = $1",
        guild_id.to_string()
    )
    .fetch_optional(&data.pool)
    .await?;

    if let Some(guild_onboarding_status) = guild_onboarding_status {
        if !guild_onboarding_status.finished_onboarding {
            // Send setup message instead
            ctx.send(setup_message()).await?;

            // Set onboarding status to true
            sqlx::query!(
                "UPDATE guilds SET finished_onboarding = true WHERE id = $1",
                guild_id.to_string()
            )
            .execute(&data.pool)
            .await?;

            return Ok(false);
        }
    } else {
        // Guild not found, create it
        sqlx::query!(
            "INSERT INTO guilds (id, finished_onboarding) VALUES ($1, true)",
            guild_id.to_string()
        )
        .execute(&data.pool)
        .await?;

        // Send setup message instead
        ctx.send(setup_message()).await?;
        return Ok(false);
    }

    let user = sqlx::query!(
        "SELECT COUNT(*) FROM users WHERE user_id = $1",
        guild_id.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    if user.count.unwrap_or_default() == 0 {
        // User not found, create it
        sqlx::query!(
            "INSERT INTO users (user_id) VALUES ($1)",
            guild_id.to_string()
        )
        .execute(&data.pool)
        .await?;
    }

    Ok(true)
}
