use antiraid_types::{
    ar_event::{AntiraidEvent, BuiltinCommandExecuteData, PermissionCheckData},
    userinfo::UserInfo,
};
use serenity::all::{GuildId, UserId};
use silverpelt::{ar_event::AntiraidEventOperations, userinfo::UserInfoOperations};
use sqlx::PgPool;
use std::time::Duration;

use crate::bot::{kittycat_permission_config_data, sandwich_config, template_dispatch_data};

pub type PermissionCheck = fn(&str, serenity::all::UserId, &UserInfo) -> Result<(), crate::Error>;

pub fn permission_check_none(
    _command: &str,
    _user_id: serenity::all::UserId,
    _user_info: &UserInfo,
) -> Result<(), crate::Error> {
    Ok(())
}

/// Check command checks whether or not a user has permission to run a command
#[allow(clippy::too_many_arguments)]
pub async fn check_command(
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

    let user_info = UserInfo::get(
        guild_id,
        user_id,
        pool,
        serenity_context,
        reqwest,
        kittycat_permission_config_data(),
        &sandwich_config(),
        match poise_ctx {
            Some(crate::Context::Application(a)) => a.interaction.member.as_ref(),
            _ => None,
        },
    )
    .await?;

    let builtin_command_exec = AntiraidEvent::BuiltinCommandExecute(BuiltinCommandExecuteData {
        command: command.to_string(),
        user_id,
        user_info,
    });

    let results = builtin_command_exec
        .dispatch_to_template_worker_and_wait(
            &serenity_context.data::<silverpelt::data::Data>(),
            guild_id,
            &template_dispatch_data(),
            Duration::from_secs(1),
        )
        .await?;

    if results.can_execute() {
        return Ok(());
    }

    // Take back the command data from the event
    let AntiraidEvent::BuiltinCommandExecute(command_data) = builtin_command_exec else {
        unreachable!();
    };

    for (command_obj, check_ptr, _) in crate::bot::raw_commands() {
        if command_obj.name == base_command {
            return (check_ptr)(command, user_id, &command_data.user_info);
        }
    }

    Err("Internal Error: Unknown command not matched in check_command".into())
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
    perm: kittycat::perms::Permission,
) -> Result<(), crate::Error> {
    let user_info = UserInfo::get(
        guild_id,
        user_id,
        pool,
        serenity_context,
        reqwest,
        kittycat_permission_config_data(),
        &sandwich_config(),
        match poise_ctx {
            Some(crate::Context::Application(a)) => a.interaction.member.as_ref(),
            _ => None,
        },
    )
    .await?;

    let perm_check_data = AntiraidEvent::PermissionCheckExecute(PermissionCheckData {
        user_id,
        user_info,
        perm,
    });

    let results = perm_check_data
        .dispatch_to_template_worker_and_wait(
            &serenity_context.data::<silverpelt::data::Data>(),
            guild_id,
            &template_dispatch_data(),
            Duration::from_secs(1),
        )
        .await?;

    if results.can_execute() {
        return Ok(());
    }

    // Take back the permission data from the event
    let AntiraidEvent::PermissionCheckExecute(perm_check_data) = perm_check_data else {
        unreachable!();
    };

    if !kittycat::perms::has_perm(
        &perm_check_data.user_info.kittycat_resolved_permissions,
        &perm_check_data.perm,
    ) {
        return Err(format!("User does not have permission: {}", perm_check_data.perm).into());
    }

    Ok(())
}
