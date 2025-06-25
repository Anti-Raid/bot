pub mod types;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

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
        // Checks if a user has a given permission [CheckPermission]
        .route(
            "/check-user-has-permission/:guild_id/:user_id",
            post(check_user_has_permission),
        );
    let router: Router<()> = router.with_state(AppData::new(data, ctx));
    router.into_make_service()
}

pub static STATE_CACHE: std::sync::LazyLock<Arc<types::BotState>> =
    std::sync::LazyLock::new(|| {
        let mut state = types::BotState {
            commands: Vec::with_capacity(crate::bot::raw_commands().len()),
            command_permissions: crate::bot::command_permissions_metadata(),
        };

        for cmd in crate::bot::raw_commands() {
            state.commands.push(cmd.into());
        }

        Arc::new(state)
    });

/// Returns a list of modules [Modules]
async fn state(State(AppData { .. }): State<AppData>) -> Json<Arc<types::BotState>> {
    Json(STATE_CACHE.clone())
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
    let perms = crate::botlib::permission_checks::check_permissions(
        guild_id,
        user_id,
        &data.pool,
        &serenity_context,
        &data.reqwest,
        &None,
        kittycat::perms::Permission::from_string(&perms.perm),
    )
    .await;

    Ok(Json(types::CheckCommandPermission {
        result: match perms {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        },
    }))
}
