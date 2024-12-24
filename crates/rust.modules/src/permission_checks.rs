use crate::cache::ModuleCache;
use serenity::all::{GuildId, UserId};
use serenity::small_fixed_array::FixedArray;
use sqlx::PgPool;

#[inline]
pub async fn get_user_discord_info(
    guild_id: GuildId,
    user_id: UserId,
    serenity_context: &serenity::all::Context,
    reqwest: &reqwest::Client,
    poise_ctx: &Option<crate::Context<'_>>,
) -> Result<
    (
        bool,                              // is_owner
        UserId,                            // owner_id
        serenity::all::Permissions,        // member_perms
        FixedArray<serenity::all::RoleId>, // roles
    ),
    crate::Error,
> {
    #[cfg(test)]
    {
        // Check for env var CHECK_MODULES_TEST_ENABLED, if so, return dummy data
        if std::env::var("CHECK_MODULES_TEST_ENABLED").unwrap_or_default() == "true" {
            return Ok((
                true,
                UserId::new(1),
                serenity::all::Permissions::all(),
                FixedArray::new(),
            ));
        }
    }

    if let Some(cached_guild) = guild_id.to_guild_cached(&serenity_context.cache) {
        // OPTIMIZATION: if owner, we dont need to continue further
        if user_id == cached_guild.owner_id {
            return Ok((
                true,                              // is_owner
                cached_guild.owner_id,             // owner_id
                serenity::all::Permissions::all(), // member_perms
                FixedArray::new(), // OPTIMIZATION: no role data is needed for perm checks for owners
            ));
        }

        // OPTIMIZATION: If we have a poise_ctx which is also a ApplicationContext, we can directly use it
        if let Some(poise::Context::Application(ref a)) = poise_ctx {
            if let Some(ref mem) = a.interaction.member {
                return Ok((
                    mem.user.id == cached_guild.owner_id,
                    cached_guild.owner_id,
                    mem.permissions
                        .unwrap_or(splashcore_rs::serenity_backport::user_permissions(
                            mem.user.id,
                            &mem.roles,
                            cached_guild.id,
                            &cached_guild.roles,
                            cached_guild.owner_id,
                        )),
                    mem.roles.clone(),
                ));
            }
        }

        // Now fetch the member, here calling member automatically tries to find in its cache first
        if let Some(member) = cached_guild.members.get(&user_id) {
            return Ok((
                member.user.id == cached_guild.owner_id,
                cached_guild.owner_id,
                splashcore_rs::serenity_backport::user_permissions(
                    member.user.id,
                    &member.roles,
                    cached_guild.id,
                    &cached_guild.roles,
                    cached_guild.owner_id,
                ),
                member.roles.clone(),
            ));
        }
    }

    let guild = guild_id.to_partial_guild(&serenity_context).await?;

    // OPTIMIZATION: if owner, we dont need to continue further
    if user_id == guild.owner_id {
        return Ok((
            true,
            guild.owner_id,
            serenity::all::Permissions::all(),
            FixedArray::new(),
        ));
    }

    // OPTIMIZATION: If we have a poise_ctx which is also a ApplicationContext, we can directly use it
    if let Some(poise::Context::Application(ref a)) = poise_ctx {
        if let Some(ref mem) = a.interaction.member {
            return Ok((
                mem.user.id == guild.owner_id,
                guild.owner_id,
                mem.permissions
                    .unwrap_or(splashcore_rs::serenity_backport::user_permissions(
                        mem.user.id,
                        &mem.roles,
                        guild.id,
                        &guild.roles,
                        guild.owner_id,
                    )),
                mem.roles.clone(),
            ));
        }
    }

    let member = {
        let member = sandwich_driver::member_in_guild(
            &serenity_context.cache,
            &serenity_context.http,
            reqwest,
            guild_id,
            user_id,
        )
        .await?;

        let Some(member) = member else {
            return Err("Member could not fetched".into());
        };

        member
    };

    Ok((
        member.user.id == guild.owner_id,
        guild.owner_id,
        splashcore_rs::serenity_backport::user_permissions(
            member.user.id,
            &member.roles,
            guild.id,
            &guild.roles,
            guild.owner_id,
        ),
        member.roles.clone(),
    ))
}

pub async fn get_user_kittycat_perms(
    pool: &PgPool,
    guild_id: GuildId,
    guild_owner_id: UserId,
    user_id: UserId,
    roles: &FixedArray<serenity::all::RoleId>,
) -> Result<Vec<kittycat::perms::Permission>, silverpelt::Error> {
    silverpelt::member_permission_calc::get_kittycat_perms(
        &mut *pool.acquire().await?,
        guild_id,
        guild_owner_id,
        user_id,
        roles,
    )
    .await
}

/// Check command checks whether or not a user has permission to run a command
#[allow(clippy::too_many_arguments)]
pub async fn check_command(
    silverpelt_cache: &ModuleCache,
    command: &str,
    guild_id: GuildId,
    user_id: UserId,
    pool: &PgPool,
    serenity_context: &serenity::all::Context,
    reqwest: &reqwest::Client,
    // If a poise::Context is available and originates from a Application Command, we can fetch the guild+member from cache itself
    poise_ctx: &Option<crate::Context<'_>>,
) -> Result<(), crate::Error> {
    let base_command = command.split(' ').next().unwrap_or_default();
    let Some(check_ptr) = silverpelt_cache
        .command_id_permission_check_map
        .get(base_command)
    else {
        return Err(format!("Command `{}` not found", base_command).into());
    };

    // Try getting guild+member from cache to speed up response times first
    let (is_owner, guild_owner_id, member_perms, roles) =
        get_user_discord_info(guild_id, user_id, serenity_context, reqwest, poise_ctx).await?;

    if is_owner {
        return Ok(()); // owner
    }

    let kittycat_perms =
        get_user_kittycat_perms(pool, guild_id, guild_owner_id, user_id, &roles).await?;

    match silverpelt::ar_event::AntiraidEvent::Custom(silverpelt::ar_event::CustomEvent {
        event_name: "AR/CheckCommand".to_string(),
        event_titlename: "(Anti-Raid) Check Command".to_string(),
        event_data: serde_json::json!({
            "command": command,
            "user_id": user_id,
            "member_native_perms": member_perms,
            "member_kittycat_perms": kittycat_perms,
            "is_owner": is_owner,
            "guild_owner_id": guild_owner_id,
            "roles": roles,
        }),
    })
    .dispatch_to_template_worker(&serenity_context.data::<silverpelt::data::Data>(), guild_id)
    .await
    {
        Ok(_) => {}
        Err(e) => {
            if e.to_string() == "AR/CheckCommand/Skip" {
                return Ok(()); // SKIP
            }

            return Err(e);
        }
    };

    (check_ptr)(command, user_id, member_perms, kittycat_perms)
}

/// Returns whether a member has a kittycat permission
///
/// Note that in opts, only custom_resolved_kittycat_perms is used
pub async fn member_has_kittycat_perm(
    guild_id: GuildId,
    user_id: UserId,
    pool: &PgPool,
    serenity_context: &serenity::all::Context,
    reqwest: &reqwest::Client,
    // If a poise::Context is available and originates from a Application Command, we can fetch the guild+member from cache itself
    poise_ctx: &Option<crate::Context<'_>>,
    perm: &kittycat::perms::Permission,
) -> Result<(), crate::Error> {
    // Try getting guild+member from cache to speed up response times first
    let (is_owner, guild_owner_id, member_perms, roles) =
        get_user_discord_info(guild_id, user_id, serenity_context, reqwest, poise_ctx).await?;

    if is_owner {
        return Ok(()); // owner
    }

    let kittycat_perms =
        get_user_kittycat_perms(pool, guild_id, guild_owner_id, user_id, &roles).await?;

    match silverpelt::ar_event::AntiraidEvent::Custom(silverpelt::ar_event::CustomEvent {
        event_name: "AR/CheckKittycatPermissions".to_string(),
        event_titlename: "(Anti-Raid) Check Kittycat Permissions".to_string(),
        event_data: serde_json::json!({
            "user_id": user_id,
            "member_native_perms": member_perms,
            "member_kittycat_perms": kittycat_perms,
            "perm": perm,
            "is_owner": is_owner,
            "guild_owner_id": guild_owner_id,
            "roles": roles,
        }),
    })
    .dispatch_to_template_worker(&serenity_context.data::<silverpelt::data::Data>(), guild_id)
    .await
    {
        Ok(_) => {}
        Err(e) => {
            if e.to_string() == "AR/CheckKittycatPermissions/Skip" {
                return Ok(()); // SKIP
            }

            return Err(e);
        }
    }

    if !kittycat::perms::has_perm(&kittycat_perms, perm) {
        return Err(format!("User does not have permission: {}", perm).into());
    }

    Ok(())
}
