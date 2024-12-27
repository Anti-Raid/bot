pub mod types;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use ar_settings::{self, types::OperationType, types::SettingsError};
use types::{CanonicalSettingsResult, SettingsOperationRequest};

type Response<T> = Result<Json<T>, (StatusCode, String)>;

#[derive(Clone)]
pub struct AppData {
    pub data: Arc<silverpelt::data::Data>,
    pub serenity_context: serenity::all::Context,
}

impl AppData {
    pub fn new(data: Arc<silverpelt::data::Data>, ctx: &serenity::all::Context) -> Self {
        Self {
            data,
            serenity_context: ctx.clone(),
        }
    }
}

pub fn create_bot_rpc_server(
    data: Arc<silverpelt::data::Data>,
    ctx: &serenity::all::Context,
) -> axum::routing::IntoMakeService<Router> {
    let router = Router::new()
        .layer(tower_http::trace::TraceLayer::new_for_http())
        // Returns the bots state [BotState]
        .route("/state", get(state))
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
        // Executes an operation on a setting [SettingsOperation]
        .route(
            "/settings-operation/:guild_id/:user_id",
            post(settings_operation),
        );
    let router: Router<()> = router.with_state(AppData::new(data, ctx));
    router.into_make_service()
}

pub static STATE_CACHE: std::sync::LazyLock<Arc<types::BotState>> =
    std::sync::LazyLock::new(|| {
        let mut state = types::BotState {
            commands: Vec::with_capacity(crate::bot::raw_commands().len()),
            settings: Vec::with_capacity(crate::bot::config_options().len()),
            command_permissions: crate::botlib::CommandPermissionMetadata::new(),
        };

        for (cmd, _, perm) in crate::bot::raw_commands() {
            state.commands.push(cmd.into());
            state.command_permissions.extend(perm.into_iter());
        }

        for setting in crate::bot::config_options() {
            state.settings.push(setting);
        }

        Arc::new(state)
    });

/// Returns a list of modules [Modules]
async fn state(State(AppData { .. }): State<AppData>) -> Json<Arc<types::BotState>> {
    Json(STATE_CACHE.clone())
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
) -> Response<types::BaseGuildUserInfo> {
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
        channels_with_permissions.push(types::GuildChannelWithPermissions {
            user: guild.user_permissions_in(channel, &member),
            bot: guild.user_permissions_in(channel, &bot_user),
            channel: channel.clone(),
        });
    }

    Ok(Json(types::BaseGuildUserInfo {
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
) -> Response<types::CheckCommandPermission> {
    let perms = crate::botlib::permission_checks::member_has_kittycat_perm(
        guild_id,
        user_id,
        &data.pool,
        &serenity_context,
        &data.reqwest,
        &None,
        &kittycat::perms::Permission::from_string(&perms.perm),
    )
    .await;

    Ok(Json(types::CheckCommandPermission {
        result: match perms {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        },
    }))
}

/// Returns if the user has permission to run a command on a given guild [CheckCommandPermission]
async fn check_command_permission(
    State(AppData {
        data,
        serenity_context,
        ..
    }): State<AppData>,
    Path((guild_id, user_id)): Path<(serenity::all::GuildId, serenity::all::UserId)>,
    Json(req): Json<types::CheckCommandPermissionRequest>,
) -> Response<types::CheckCommandPermission> {
    let perm_res = crate::botlib::permission_checks::check_command(
        &req.command,
        guild_id,
        user_id,
        &data.pool,
        &serenity_context,
        &data.reqwest,
        &None,
    )
    .await;

    Ok(Json(types::CheckCommandPermission {
        result: match perm_res {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        },
    }))
}

/// Executes an operation on a setting [SettingsOperation]
pub(crate) async fn settings_operation(
    State(AppData {
        serenity_context, ..
    }): State<AppData>,
    Path((guild_id, user_id)): Path<(serenity::all::GuildId, serenity::all::UserId)>,
    Json(req): Json<SettingsOperationRequest>,
) -> Json<types::CanonicalSettingsResult> {
    let op: OperationType = req.op;

    // Find the setting
    let mut setting = None;

    for setting_obj in crate::bot::config_options() {
        if setting_obj.id == req.setting {
            setting = Some(setting_obj);
            break;
        }
    }

    //if let Some(page_setting) = templating::cache::get_setting(guild_id, &req.setting).await {
    //    setting = Some(page_setting);
    //};

    let Some(setting) = setting else {
        return Json(CanonicalSettingsResult::Err {
            error: SettingsError::Generic {
                message: "Setting not found".to_string(),
                src: "SettingsOperationCore".to_string(),
                typ: "client".to_string(),
            },
        });
    };

    match op {
        OperationType::View => {
            match ar_settings::cfg::settings_view(
                &setting,
                &crate::botlib::helpers::settings_data(serenity_context),
                guild_id,
                user_id,
                req.fields,
            )
            .await
            {
                Ok(res) => Json(CanonicalSettingsResult::Ok { fields: res }),
                Err(e) => Json(CanonicalSettingsResult::Err { error: e }),
            }
        }
        OperationType::Create => {
            match ar_settings::cfg::settings_create(
                &setting,
                &crate::botlib::helpers::settings_data(serenity_context),
                guild_id,
                user_id,
                req.fields,
            )
            .await
            {
                Ok(res) => Json(CanonicalSettingsResult::Ok { fields: vec![res] }),
                Err(e) => Json(CanonicalSettingsResult::Err { error: e }),
            }
        }
        OperationType::Update => {
            match ar_settings::cfg::settings_update(
                &setting,
                &crate::botlib::helpers::settings_data(serenity_context),
                guild_id,
                user_id,
                req.fields,
            )
            .await
            {
                Ok(res) => Json(CanonicalSettingsResult::Ok { fields: vec![res] }),
                Err(e) => Json(CanonicalSettingsResult::Err { error: e }),
            }
        }
        OperationType::Delete => {
            let Some(pkey) = req.fields.get(&setting.primary_key) else {
                return Json(CanonicalSettingsResult::Err {
                    error: SettingsError::MissingOrInvalidField {
                        field: setting.primary_key.to_string(),
                        src: "SettingsOperation".to_string(),
                    },
                });
            };

            match ar_settings::cfg::settings_delete(
                &setting,
                &crate::botlib::helpers::settings_data(serenity_context),
                guild_id,
                user_id,
                pkey.clone(),
            )
            .await
            {
                Ok(_res) => Json(CanonicalSettingsResult::Ok { fields: vec![] }),
                Err(e) => Json(CanonicalSettingsResult::Err { error: e }),
            }
        }
    }
}
