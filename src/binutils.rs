use log::error;
use modules::Error;
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
                                &config::CONFIG.meta.support_server_invite,
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
                                    &config::CONFIG.meta.support_server_invite,
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
                            &config::CONFIG.meta.support_server_invite,
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
                        config::CONFIG.sites.docs.clone(),
                    )
                    .label("User Guide"),
                    serenity::all::CreateButton::new_link(
                        config::CONFIG.sites.frontend.clone(),
                    )
                    .label("Website"),
                    serenity::all::CreateButton::new_link(
                        config::CONFIG.meta.support_server_invite.clone(),
                    )
                    .label("Support Server")
                ].into()
            )
        ]
    )
}

pub async fn command_check(ctx: modules::Context<'_>) -> Result<bool, Error> {
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

    let command = ctx.command();

    if let Err(res) = modules::permission_checks::check_command(
        &modules::module_cache(&data),
        &command.qualified_name,
        guild_id,
        ctx.author().id,
        &data.pool,
        ctx.serenity_context(),
        &data.reqwest,
        &Some(ctx),
    )
    .await
    {
        ctx.send(
            poise::CreateReply::new().embed(
                serenity::all::CreateEmbed::new()
                    .color(serenity::all::Color::RED)
                    .title("You don't have permission to use this command?")
                    .description(res.to_string()),
            ),
        )
        .await?;

        return Ok(false);
    }

    Ok(true)
}

pub fn get_commands(
    silverpelt_cache: &modules::cache::ModuleCache,
) -> Vec<poise::Command<Data, Error>> {
    let mut cmds = Vec::new();

    let mut _cmd_names = Vec::new();
    for module in silverpelt_cache.module_cache.iter() {
        log::info!("Loading module {}", module.id());

        match module.validate() {
            Ok(_) => {}
            Err(e) => {
                panic!("Error validating module {}: {}", module.id(), e);
            }
        }

        if module.virtual_module() {
            continue;
        }

        for (mut cmd, _) in module.raw_commands() {
            cmd.category = Some(module.id().into());

            let mut subcommands = Vec::new();
            // Ensure subcommands are also linked to a category
            for subcommand in cmd.subcommands {
                subcommands.push(poise::Command {
                    category: Some(module.id().into()),
                    ..subcommand
                });
            }

            cmd.subcommands = subcommands;

            // Check for duplicate command names
            if _cmd_names.contains(&cmd.name) {
                error!("Duplicate command name: {:#?}", cmd);
                panic!("Duplicate command name: {}", cmd.qualified_name);
            }

            _cmd_names.push(cmd.name.clone());

            // Check for duplicate command aliases
            for alias in cmd.aliases.iter() {
                if _cmd_names.contains(alias) {
                    panic!(
                        "Duplicate command alias: {} from command {}",
                        alias, cmd.name
                    );
                }

                _cmd_names.push(alias.clone());
            }

            // Good to go
            cmds.push(cmd);
        }
    }

    cmds
}
