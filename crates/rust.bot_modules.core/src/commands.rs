use modules::Context;
use silverpelt::Error;

/// Settings related to commands
#[poise::command(
    slash_command,
    user_cooldown = 1,
    guild_cooldown = 1,
    subcommands("commands_check", "commands_enable", "commands_disable",)
)]
pub async fn commands(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Checks if a command is usable
#[poise::command(slash_command, user_cooldown = 1, guild_cooldown = 1, rename = "check")]
pub async fn commands_check(
    ctx: Context<'_>,
    #[description = "The command to check"] command: String,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command must be run in a guild".into());
    };

    let data = ctx.data();

    // Check if the user has permission to use the command
    let perm_res = modules::permission_checks::check_command(
        &modules::module_cache(&data),
        &command,
        guild_id,
        ctx.author().id,
        &data.pool,
        ctx.serenity_context(),
        &data.reqwest,
        &Some(ctx),
        modules::permission_checks::CheckCommandOptions {
            ignore_command_disabled: true,
            channel_id: Some(ctx.channel_id()),
            ..Default::default()
        },
    )
    .await;

    if !perm_res.is_ok() {
        return Err(format!(
            "You do NOT have permission to use this command?\n{}",
            perm_res.to_markdown()
        )
        .into());
    }

    ctx.say("You have permission to use this command").await?;

    Ok(())
}

/// Enables a module. Note that globally disabled modules cannot be used even if enabled
#[poise::command(
    slash_command,
    user_cooldown = 1,
    guild_cooldown = 1,
    rename = "enable"
)]
pub async fn commands_enable(
    ctx: Context<'_>,
    #[description = "The command to enable"] command: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command must be run in a guild".into());
    };

    if command.is_empty() {
        return Err("No command provided".into());
    }

    // Find command in cache
    let command_permutations = modules::utils::permute_command_names(&command);

    let module_cache = modules::module_cache(&data);

    let Some(module) = module_cache
        .command_id_module_map
        .get(&command_permutations[0])
    else {
        return Err("Command not found".into());
    };

    let Some(module) = module_cache.module_cache.get(module.value()) else {
        return Err("Module not found".into());
    };

    if !module.commands_toggleable() {
        return Err(format!(
            "Commands within the module `{}` cannot be toggled (enabled/disable) at this time!",
            module.id()
        )
        .into());
    }

    // Check if the user has permission to use the command
    let perm_res = modules::permission_checks::check_command(
        &module_cache,
        &command,
        guild_id,
        ctx.author().id,
        &data.pool,
        ctx.serenity_context(),
        &data.reqwest,
        &Some(ctx),
        modules::permission_checks::CheckCommandOptions {
            ignore_command_disabled: true,
            channel_id: Some(ctx.channel_id()),
            ..Default::default()
        },
    )
    .await;

    if !perm_res.is_ok() {
        return Err(format!(
            "You can only modify commands that you have permission to use?\n{}",
            perm_res.to_markdown()
        )
        .into());
    }

    // Check if command is already enabled
    let mut tx = data.pool.begin().await?;

    let disabled = sqlx::query!(
        "SELECT disabled FROM guild_command_configurations WHERE guild_id = $1 AND command = $2",
        guild_id.to_string(),
        command
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(disabled) = disabled {
        // We have a module, now check
        if disabled.disabled.is_some() && !disabled.disabled.unwrap_or_default() {
            return Err("Command is already enabled".into());
        }

        sqlx::query!(
            "UPDATE guild_command_configurations SET disabled = false, last_updated_by = $3, last_updated_at = NOW() WHERE guild_id = $1 AND command = $2",
            guild_id.to_string(),
            command,
            ctx.author().id.to_string()
        )
        .execute(&mut *tx)
        .await?;
    } else {
        // No module, create it
        sqlx::query!(
            "INSERT INTO guild_command_configurations (guild_id, command, disabled, created_by) VALUES ($1, $2, false, $3)",
            guild_id.to_string(),
            command,
            ctx.author().id.to_string()
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    ctx.say("Command enabled").await?;

    Ok(())
}

/// Enables a module. Note that globally disabled modules cannot be used even if enabled
#[poise::command(
    slash_command,
    user_cooldown = 1,
    guild_cooldown = 1,
    rename = "disable"
)]
pub async fn commands_disable(
    ctx: Context<'_>,
    #[description = "The command to disable"] command: String,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command must be run in a guild".into());
    };

    if command.is_empty() {
        return Err("No command provided".into());
    }

    let data = ctx.data();
    let modules_cache = modules::module_cache(&data);

    // Find command in cache
    let command_permutations = modules::utils::permute_command_names(&command);

    let Some(module) = modules_cache
        .command_id_module_map
        .get(&command_permutations[0])
    else {
        return Err("Command not found".into());
    };

    let Some(module) = modules_cache.module_cache.get(module.value()) else {
        return Err("Module not found".into());
    };

    if !module.commands_toggleable() {
        return Err(format!(
            "Commands within the module `{}` cannot be toggled (enabled/disable) at this time!",
            module.id()
        )
        .into());
    }

    // Check if the user has permission to use the command
    let perm_res = modules::permission_checks::check_command(
        &modules_cache,
        &command,
        guild_id,
        ctx.author().id,
        &ctx.data().pool,
        ctx.serenity_context(),
        &data.reqwest,
        &Some(ctx),
        modules::permission_checks::CheckCommandOptions {
            ignore_command_disabled: true,
            channel_id: Some(ctx.channel_id()),
            ..Default::default()
        },
    )
    .await;

    if !perm_res.is_ok() {
        return Err(format!(
            "You can only modify commands that you have permission to use?\n{}",
            perm_res.to_markdown()
        )
        .into());
    }

    // Check if command is already enabled
    let mut tx = ctx.data().pool.begin().await?;

    let disabled = sqlx::query!(
        "SELECT disabled FROM guild_command_configurations WHERE guild_id = $1 AND command = $2",
        guild_id.to_string(),
        command
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(disabled) = disabled {
        // We have a command, now check
        if disabled.disabled.is_some() && disabled.disabled.unwrap_or_default() {
            return Err("Command is already disabled".into());
        }

        sqlx::query!(
            "UPDATE guild_command_configurations SET disabled = true, last_updated_by = $3, last_updated_at = NOW() WHERE guild_id = $1 AND command = $2",
            guild_id.to_string(),
            command,
            ctx.author().id.to_string()
        )
        .execute(&mut *tx)
        .await?;
    } else {
        // No module, create it
        sqlx::query!(
            "INSERT INTO guild_command_configurations (guild_id, command, disabled, created_by) VALUES ($1, $2, true, $3)",
            guild_id.to_string(),
            command,
            ctx.author().id.to_string()
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    ctx.say("Command disabled").await?;

    Ok(())
}
