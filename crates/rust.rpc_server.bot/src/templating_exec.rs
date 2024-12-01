use crate::types::ExecuteTemplateResponse;
use crate::AppData;
use axum::{
    extract::{Path, State},
    Json,
};

/// Executes a template on a Lua VM
pub(crate) async fn execute_template(
    State(AppData {
        data,
        serenity_context,
        ..
    }): State<AppData>,
    Path((guild_id, user_id)): Path<(serenity::all::GuildId, serenity::all::UserId)>,
    Json(req): Json<crate::types::ExecuteTemplateRequest>,
) -> Json<ExecuteTemplateResponse> {
    let modules_cache = modules::module_cache(&data);
    let perm_res = modules::permission_checks::check_command(
        &modules_cache,
        "exec_template",
        guild_id,
        user_id,
        &data.pool,
        &serenity_context,
        &data.reqwest,
        &None,
        modules::permission_checks::CheckCommandOptions::default(),
    )
    .await;

    if !perm_res.is_ok() {
        return Json(ExecuteTemplateResponse::PermissionError { res: perm_res });
    }

    let resp = silverpelt::ar_event::AntiraidEvent::Custom(silverpelt::ar_event::CustomEvent {
        event_titlename: "(Anti-Raid) Evaluate Template".to_string(),
        event_name: "AR/Virtual_ExecTemplate".to_string(),
        event_data: req.args,
    })
    .dispatch_to_template_worker(&data, guild_id)
    .await;

    match resp {
        Ok(reply) => Json(ExecuteTemplateResponse::Ok {
            result: Some(reply),
        }),
        Err(e) => Json(ExecuteTemplateResponse::ExecErr {
            error: e.to_string(),
        }),
    }
}
