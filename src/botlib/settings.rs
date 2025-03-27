use ar_settings::{
    serenity::autogen::{subcommand_autocomplete, subcommand_command, SubcommandCallbackWrapper},
    types::OperationType,
};
use serenity::all::{CreateInteractionResponse, CreateInteractionResponseMessage};
use silverpelt::data::Data;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum RequestScope {
    Guild((serenity::all::GuildId, serenity::all::UserId)),
    Anonymous,
}

impl RequestScope {
    pub fn guild_id(&self) -> Result<serenity::all::GuildId, crate::Error> {
        match self {
            RequestScope::Guild((guild_id, _)) => Ok(*guild_id),
            RequestScope::Anonymous => {
                Err("This setting cannot be used in an anonymous context".into())
            }
        }
    }

    pub fn user_id(&self) -> Result<serenity::all::UserId, crate::Error> {
        match self {
            RequestScope::Guild((_, user_id)) => Ok(*user_id),
            RequestScope::Anonymous => {
                Err("This setting cannot be used in an anonymous context".into())
            }
        }
    }
}

#[derive(Clone)]
pub struct SettingsData {
    pub data: Arc<Data>,
    pub serenity_context: serenity::all::Context,
    pub scope: RequestScope,
}

impl Default for SettingsData {
    fn default() -> Self {
        unreachable!("SettingsData::default() should never be called")
    }
}

impl serde::Serialize for SettingsData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

/// Given the Data and a cache_http, returns the settings data
pub fn settings_data(
    serenity_context: serenity::all::Context,
    scope: RequestScope,
) -> SettingsData {
    SettingsData {
        data: serenity_context.data::<Data>(),
        serenity_context,
        scope,
    }
}

/// Executes an interaction if it is a setting
/// using ar_settings' autogen ui
pub async fn execute_setting_interaction(
    ctx: serenity::all::Context,
    interaction: &serenity::all::Interaction,
) -> Result<(), crate::Error> {
    let (cmd, is_autocomplete) = match interaction {
        serenity::all::Interaction::Command(cmd) => (cmd, false),
        serenity::all::Interaction::Autocomplete(cmd) => (cmd, true),
        _ => return Ok(()),
    };

    if cmd.data.name != "settings" {
        return Ok(());
    }

    let Some(guild_id) = cmd.guild_id else {
        return Ok(());
    };

    // Extract setting name. Format of the command is `settings SETTING_NAME OPTION`
    let resolved_option = cmd.data.options();
    println!("{:?}", resolved_option);
    let setting_option = resolved_option.into_iter().next().unwrap();

    let name = setting_option.name;
    let value = match setting_option.value {
        serenity::all::ResolvedValue::SubCommandGroup(s) => s,
        _ => return Ok(()),
    };

    let op = value.first().unwrap().name;

    println!("Setting name: {}, Option: {}", name, op);

    let op = match op {
        "view" => OperationType::View,
        "create" => OperationType::Create,
        "update" => OperationType::Update,
        "delete" => OperationType::Delete,
        _ => return Ok(()),
    };

    // Find the config option
    let mut setting = None;
    for setting_obj in crate::bot::config_options() {
        if setting_obj.id == name {
            setting = Some(setting_obj);
            break;
        }
    }

    let Some(setting) = setting else {
        return Ok(());
    };

    let subcommand_wrapper = SubcommandCallbackWrapper {
        config_option: setting,
        data: settings_data(ctx.clone(), RequestScope::Guild((guild_id, cmd.user.id))).into(),
        operation_type: op,
    };

    if is_autocomplete {
        if let Err(error) = subcommand_autocomplete(&ctx, interaction, subcommand_wrapper).await {
            log::error!("Failed to execute setting autocomplete: {:?}", error);
        }

        return Ok(());
    }

    if let Err(error) = subcommand_command(&ctx, interaction, &subcommand_wrapper).await {
        log::error!("Failed to execute setting command: {:?}", error);
        cmd.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(
                        serenity::all::CreateEmbed::new()
                            .color(serenity::all::Color::RED)
                            .title("An error has occurred")
                            .description(error.to_string()),
                    )
                    .components(vec![serenity::all::CreateActionRow::Buttons(
                        vec![serenity::all::CreateButton::new_link(
                            &crate::config::CONFIG.meta.support_server_invite,
                        )
                        .label("Support Server")]
                        .into(),
                    )]),
            ),
        )
        .await?;
    }

    Ok(())
}
