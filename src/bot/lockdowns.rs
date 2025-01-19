use crate::{Context, Error};

pub async fn lockdown_autocomplete<'a>(
    ctx: crate::Context<'_>,
    partial: &str,
) -> serenity::all::CreateAutocompleteResponse<'a> {
    let data = ctx.data();

    let Some(guild_id) = ctx.guild_id() else {
        return serenity::builder::CreateAutocompleteResponse::new();
    };

    match sqlx::query!(
        "SELECT id, type FROM lockdown__guild_lockdowns WHERE guild_id = $1 AND type ILIKE $2",
        guild_id.to_string(),
        format!("%{}%", partial.replace('%', "\\%").replace('_', "\\_")),
    )
    .fetch_all(&data.pool)
    .await
    {
        Ok(lockdowns) => {
            let mut choices = serenity::all::CreateAutocompleteResponse::new();

            for lockdown in lockdowns {
                choices = choices.add_choice(serenity::all::AutocompleteChoice::new(
                    lockdown.r#type,
                    lockdown.id.to_string(),
                ));
            }

            choices
        }
        Err(e) => {
            log::error!("Failed to fetch lockdowns: {:?}", e);
            serenity::builder::CreateAutocompleteResponse::new()
        }
    }
}

/// Lockdowns
#[poise::command(
    slash_command,
    subcommands(
        "lockdowns_list",
        "lockdowns_tsl",
        "lockdowns_qsl",
        "lockdowns_scl",
        "lockdowns_role",
        "lockdowns_remove"
    )
)]
pub async fn lockdowns(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Lists all currently ongoing lockdowns in summary form
#[poise::command(slash_command, guild_only, rename = "list")]
pub async fn lockdowns_list(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let data = ctx.data();

    let lockdowns = lockdowns::LockdownSet::guild(guild_id, &data.pool)
        .await
        .map_err(|e| format!("Error while fetching lockdown set: {}", e))?;

    if lockdowns.lockdowns.is_empty() {
        return Err("No active lockdowns".into());
    }

    let mut msg = String::new();

    for lockdown in lockdowns.lockdowns {
        msg.push_str(&format!(
            "ID: {}, Type: {}, Reason: {}\n",
            lockdown.id,
            lockdown.r#type.string_form(),
            lockdown.reason
        ));
    }

    ctx.send(
        poise::CreateReply::new().embed(
            serenity::all::CreateEmbed::new()
                .title("Active Lockdowns")
                .description(msg),
        ),
    )
    .await?;

    Ok(())
}

/// Starts a traditional server lockdown
#[poise::command(slash_command, guild_only, rename = "tsl")]
pub async fn lockdowns_tsl(ctx: Context<'_>, reason: String) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let data = ctx.data();

    // Get the current lockdown set
    let mut lockdowns = lockdowns::LockdownSet::guild(guild_id, &data.pool)
        .await
        .map_err(|e| format!("Error while fetching lockdown set: {}", e))?;

    // Create the lockdown
    let lockdown_type = lockdowns::tsl::TraditionalServerLockdown {};

    let lockdown_data = lockdowns::LockdownData {
        cache: ctx.cache(),
        http: ctx.http(),
        pool: data.pool.clone(),
        reqwest: data.reqwest.clone(),
    };

    ctx.defer().await?;

    lockdowns
        .easy_apply(Box::new(lockdown_type), &lockdown_data, &reason)
        .await
        .map_err(|e| format!("Error while applying lockdown: {}", e))?;

    ctx.say("Lockdown started").await?;

    Ok(())
}

#[poise::command(slash_command, guild_only, rename = "qsl")]
/// Starts a quick server lockdown
pub async fn lockdowns_qsl(ctx: Context<'_>, reason: String) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let data = ctx.data();

    // Get the current lockdown set
    let mut lockdowns = lockdowns::LockdownSet::guild(guild_id, &data.pool)
        .await
        .map_err(|e| format!("Error while fetching lockdown set: {}", e))?;

    // Create the lockdown
    let lockdown_type = lockdowns::qsl::QuickServerLockdown {};

    let lockdown_data = lockdowns::LockdownData {
        cache: ctx.cache(),
        http: ctx.http(),
        pool: data.pool.clone(),
        reqwest: data.reqwest.clone(),
    };

    ctx.defer().await?;

    lockdowns
        .easy_apply(Box::new(lockdown_type), &lockdown_data, &reason)
        .await
        .map_err(|e| format!("Error while applying lockdown: {}", e))?;

    ctx.say("Lockdown started").await?;

    Ok(())
}

#[poise::command(slash_command, guild_only, rename = "scl")]
/// Starts a single channel lockdown
pub async fn lockdowns_scl(
    ctx: Context<'_>,
    channel: Option<serenity::all::ChannelId>,
    reason: String,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let data = ctx.data();
    let channel = channel.unwrap_or(ctx.channel_id());

    // Get the current lockdown set
    let mut lockdowns = lockdowns::LockdownSet::guild(guild_id, &data.pool)
        .await
        .map_err(|e| format!("Error while fetching lockdown set: {}", e))?;

    // Create the lockdown
    let lockdown_type = lockdowns::scl::SingleChannelLockdown(channel);

    let lockdown_data = lockdowns::LockdownData {
        cache: ctx.cache(),
        http: ctx.http(),
        pool: data.pool.clone(),
        reqwest: data.reqwest.clone(),
    };

    ctx.defer().await?;

    lockdowns
        .easy_apply(Box::new(lockdown_type), &lockdown_data, &reason)
        .await
        .map_err(|e| format!("Error while applying lockdown: {}", e))?;

    ctx.say("Lockdown started").await?;

    Ok(())
}

#[poise::command(slash_command, guild_only, rename = "role")]
/// Starts a single channel lockdown
pub async fn lockdowns_role(
    ctx: Context<'_>,
    role: serenity::all::RoleId,
    reason: String,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let data = ctx.data();

    // Get the current lockdown set
    let mut lockdowns = lockdowns::LockdownSet::guild(guild_id, &data.pool)
        .await
        .map_err(|e| format!("Error while fetching lockdown set: {}", e))?;

    // Create the lockdown
    let lockdown_type = lockdowns::role::RoleLockdown(role);

    let lockdown_data = lockdowns::LockdownData {
        cache: ctx.cache(),
        http: ctx.http(),
        pool: data.pool.clone(),
        reqwest: data.reqwest.clone(),
    };

    ctx.defer().await?;

    lockdowns
        .easy_apply(Box::new(lockdown_type), &lockdown_data, &reason)
        .await
        .map_err(|e| format!("Error while applying lockdown: {}", e))?;

    ctx.say("Lockdown started").await?;

    Ok(())
}

#[poise::command(slash_command, guild_only, rename = "remove")]
/// Remove a lockdown by ID
pub async fn lockdowns_remove(
    ctx: Context<'_>,
    #[autocomplete = "lockdown_autocomplete"] id: String,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let data = ctx.data();

    // Get the current lockdown set
    let mut lockdowns = lockdowns::LockdownSet::guild(guild_id, &data.pool)
        .await
        .map_err(|e| format!("Error while fetching lockdown set: {}", e))?;

    let lockdown_data = lockdowns::LockdownData {
        cache: ctx.cache(),
        http: ctx.http(),
        pool: data.pool.clone(),
        reqwest: data.reqwest.clone(),
    };

    ctx.defer().await?;

    lockdowns
        .easy_remove(id.parse()?, &lockdown_data)
        .await
        .map_err(|e| format!("Error while applying lockdown: {}", e))?;

    ctx.say("Lockdown removed").await?;

    Ok(())
}
