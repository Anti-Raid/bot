use crate::rpc::types::{ExecuteTemplateRequest, ExecuteTemplateResponse};
use crate::rpc::AppData;
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
    Json(req): Json<ExecuteTemplateRequest>,
) -> Json<ExecuteTemplateResponse> {
    if let Err(perm_res) = crate::botlib::permission_checks::member_has_kittycat_perm(
        guild_id,
        user_id,
        &data.pool,
        &serenity_context,
        &data.reqwest,
        &None,
        &kittycat::perms::Permission::from_string("templating.eval"),
    )
    .await
    {
        return Json(ExecuteTemplateResponse::ExecErr {
            error: perm_res.to_string(),
        });
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
