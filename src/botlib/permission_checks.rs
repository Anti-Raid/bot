use antiraid_types::{
    ar_event::{AntiraidEvent, PermissionCheckData},
    userinfo::UserInfo,
};
use serenity::all::{GuildId, UserId};
use silverpelt::{ar_event::AntiraidEventOperations, userinfo::UserInfoOperations};
use sqlx::PgPool;
use std::time::Duration;

use crate::bot::{kittycat_permission_config_data, sandwich_config, template_dispatch_data};

/// Returns whether a member has a kittycat permission
///
/// Note that in opts, only custom_resolved_kittycat_perms is used
pub async fn check_permissions(
    guild_id: GuildId,
    user_id: UserId,
    pool: &PgPool,
    serenity_context: &serenity::all::Context,
    reqwest: &reqwest::Client,
    // If a poise::Context is available and originates from a Application Command, we can fetch the guild+member from cache itself
    poise_ctx: &Option<crate::Context<'_>>,
    kc_perm: kittycat::perms::Permission,
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
        perm: kc_perm,
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
        // Get users top role based on position and created_at
        let mut member_roles = perm_check_data
            .user_info
            .member_roles
            .iter()
            .filter_map(|r| perm_check_data.user_info.guild_roles.get(r))
            .collect::<Vec<_>>();

        // Sort the member_roles
        member_roles.sort();

        // Get top role
        let top_role = member_roles.last().unwrap();

        let perm = perm_check_data.perm;
        return Err(
            format!("You need the ``{perm}`` permission to use this command!
            
Please ask the server administrator to run ``/settings roles create role:{top_role} perms:{perm}`` to give you this permission.", 
            ).into()
        );
    }

    Ok(())
}
