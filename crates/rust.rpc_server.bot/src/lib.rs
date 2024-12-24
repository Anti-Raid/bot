pub mod settings_execute;
pub mod templating_exec;
pub mod types;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rust_rpc_server::AppData;
use std::sync::Arc;

type Response<T> = Result<Json<T>, (StatusCode, String)>;

pub fn create_bot_rpc_server(
    data: Arc<silverpelt::data::Data>,
    ctx: &serenity::all::Context,
) -> axum::routing::IntoMakeService<Router> {
    let router = rust_rpc_server::create_blank_rpc_server()
        // Returns the list of modules [Modules]
        .route("/modules", get(modules))
        // Given a list of guild ids, return a set of 0s and 1s indicating whether each guild exists in cache [GuildsExist]
        .route("/guilds-exist", get(guilds_exist))
        // Returns basic user/guild information [BaseGuildUserInfo]
        .route(
            "/base-guild-user-info/:guild_id/:user_id",
            get(base_guild_user_info),
        )
        // Returns if the user has permission to run a command on a given guild [CheckCommandPermission]
        .route(
            "/check-command-permission/:guild_id/:user_id",
            get(check_command_permission),
        )
        // Checks if a user has a given permission [CheckPermission]
        .route(
            "/check-user-has-permission/:guild_id/:user_id",
            post(check_user_has_permission),
        )
        // Executes a template on a Lua VM
        .route(
            "/template-exec/:guild_id/:user_id",
            post(templating_exec::execute_template),
        )
        // Executes an operation on a setting [SettingsOperation]
        .route(
            "/settings-operation/:guild_id/:user_id",
            post(settings_execute::settings_operation),
        );
    let router: Router<()> = router.with_state(AppData::new(data, ctx));
    router.into_make_service()
}

/// Returns a list of modules [Modules]
async fn modules(
    State(AppData { data, .. }): State<AppData>,
) -> Json<Vec<modules::canonical::CanonicalModule>> {
    let mut modules = Vec::new();

    let modules_cache = modules::module_cache(&data);
    for idm in modules_cache.canonical_module_cache.iter() {
        let module = idm.value();
        modules.push(module.clone());
    }

    Json(modules)
}

/// Given a list of guild ids, return a set of 0s and 1s indicating whether each guild exists in cache [GuildsExist]
#[axum::debug_handler]
async fn guilds_exist(
    State(AppData {
        data,
        serenity_context,
    }): State<AppData>,
    Json(guilds): Json<Vec<serenity::all::GuildId>>,
) -> Response<Vec<i32>> {
    let mut guilds_exist = Vec::with_capacity(guilds.len());

    for guild in guilds {
        let has_guild = sandwich_driver::has_guild(
            &serenity_context.cache,
            &serenity_context.http,
            &data.reqwest,
            guild,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        guilds_exist.push({
            if has_guild {
                1
            } else {
                0
            }
        });
    }

    Ok(Json(guilds_exist))
}

/// Returns basic user/guild information [BaseGuildUserInfo]
async fn base_guild_user_info(
    State(AppData {
        data,
        serenity_context,
        ..
    }): State<AppData>,
    Path((guild_id, user_id)): Path<(serenity::all::GuildId, serenity::all::UserId)>,
) -> Response<crate::types::BaseGuildUserInfo> {
    let bot_user_id = serenity_context.cache.current_user().id;
    let guild = sandwich_driver::guild(
        &serenity_context.cache,
        &serenity_context.http,
        &data.reqwest,
        guild_id,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get guild: {:#?}", e),
        )
    })?;

    // Next fetch the member and bot_user
    let member: serenity::model::prelude::Member = match sandwich_driver::member_in_guild(
        &serenity_context.cache,
        &serenity_context.http,
        &data.reqwest,
        guild_id,
        user_id,
    )
    .await
    {
        Ok(Some(member)) => member,
        Ok(None) => {
            return Err((StatusCode::NOT_FOUND, "User not found".into()));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get member: {:#?}", e),
            ));
        }
    };

    let bot_user: serenity::model::prelude::Member = match sandwich_driver::member_in_guild(
        &serenity_context.cache,
        &serenity_context.http,
        &data.reqwest,
        guild_id,
        bot_user_id,
    )
    .await
    {
        Ok(Some(member)) => member,
        Ok(None) => {
            return Err((StatusCode::NOT_FOUND, "Bot user not found".into()));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get bot user: {:#?}", e),
            ));
        }
    };

    // Fetch the channels
    let channels = sandwich_driver::guild_channels(
        &serenity_context.cache,
        &serenity_context.http,
        &data.reqwest,
        guild_id,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get channels: {:#?}", e),
        )
    })?;

    let mut channels_with_permissions = Vec::with_capacity(channels.len());

    for channel in channels.iter() {
        channels_with_permissions.push(crate::types::GuildChannelWithPermissions {
            user: guild.user_permissions_in(channel, &member),
            bot: guild.user_permissions_in(channel, &bot_user),
            channel: channel.clone(),
        });
    }

    Ok(Json(crate::types::BaseGuildUserInfo {
        name: guild.name.to_string(),
        icon: guild.icon_url(),
        owner_id: guild.owner_id.to_string(),
        roles: guild.roles.into_iter().collect(),
        user_roles: member.roles.to_vec(),
        bot_roles: bot_user.roles.to_vec(),
        channels: channels_with_permissions,
    }))
}

/// Checks if a user has a given permission [CheckPermission]
#[axum::debug_handler]
async fn check_user_has_permission(
    State(AppData {
        data,
        serenity_context,
        ..
    }): State<AppData>,
    Path((guild_id, user_id)): Path<(serenity::all::GuildId, serenity::all::UserId)>,
    Json(perms): Json<types::CheckUserHasKittycatPermissionsRequest>,
) -> Response<String> {
    let perms = modules::permission_checks::member_has_kittycat_perm(
        guild_id,
        user_id,
        &data.pool,
        &serenity_context,
        &data.reqwest,
        &None,
        &kittycat::perms::Permission::from_string(&perms.perm),
    )
    .await;

    match perms {
        Ok(_) => Ok(Json("".to_string())),
        Err(e) => Err((StatusCode::FORBIDDEN, e.to_string())),
    }
}

/// Returns if the user has permission to run a command on a given guild [CheckCommandPermission]
async fn check_command_permission(
    State(AppData {
        data,
        serenity_context,
        ..
    }): State<AppData>,
    Path((guild_id, user_id)): Path<(serenity::all::GuildId, serenity::all::UserId)>,
    Json(req): Json<crate::types::CheckCommandPermissionRequest>,
) -> Response<crate::types::CheckCommandPermission> {
    let modules_cache = modules::module_cache(&data);
    let perm_res = modules::permission_checks::check_command(
        &modules_cache,
        &req.command,
        guild_id,
        user_id,
        &data.pool,
        &serenity_context,
        &data.reqwest,
        &None,
    )
    .await;

    Ok(Json(crate::types::CheckCommandPermission {
        result: match perm_res {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        },
    }))
}
