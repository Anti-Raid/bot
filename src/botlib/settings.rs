use ar_settings::{
    serenity::autogen::{subcommand_command, SubcommandCallbackWrapper},
    types::OperationType,
};
use serenity::all::{CreateInteractionResponse, CreateInteractionResponseMessage, InteractionType};
use silverpelt::data::Data;
use std::sync::Arc;

#[derive(Clone)]
pub struct SettingsData {
    pub data: Arc<Data>,
    pub serenity_context: serenity::all::Context,
    pub guild_id: serenity::all::GuildId,
    pub author: serenity::all::UserId,
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
    guild_id: serenity::all::GuildId,
    author: serenity::all::UserId,
) -> SettingsData {
    SettingsData {
        data: serenity_context.data::<Data>(),
        serenity_context,
        guild_id,
        author,
    }
}

/// Executes an interaction if it is a setting
/// using ar_settings' autogen ui
pub async fn execute_setting_interaction(
    ctx: serenity::all::Context,
    interaction: &serenity::all::Interaction,
) -> Result<(), crate::Error> {
    // Check if command or autocomplete
    match interaction.kind() {
        InteractionType::Command | InteractionType::Autocomplete => {}
        _ => return Ok(()),
    }

    if let Some(cmd) = interaction.as_command() {
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
            data: settings_data(ctx.clone(), guild_id, cmd.user.id).into(),
            operation_type: op,
        };

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
    }

    Ok(())
}
