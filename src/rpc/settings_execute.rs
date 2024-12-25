use crate::rpc::types::{CanonicalSettingsResult, SettingsOperationRequest};
use crate::rpc::AppData;
use ar_settings::{self, types::OperationType, types::SettingsError};
use axum::{
    extract::{Path, State},
    Json,
};

/// Executes an operation on a setting [SettingsOperation]
pub(crate) async fn settings_operation(
    State(AppData {
        serenity_context, ..
    }): State<AppData>,
    Path((guild_id, user_id)): Path<(serenity::all::GuildId, serenity::all::UserId)>,
    Json(req): Json<SettingsOperationRequest>,
) -> Json<super::types::CanonicalSettingsResult> {
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
