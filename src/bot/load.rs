use antiraid_types::ar_event::AntiraidEvent;
use serenity::all::{CreateActionRow, CreateButton, CreateEmbed};
use silverpelt::ar_event::AntiraidEventOperations;

use crate::bot::template_dispatch_data;

pub async fn load_autocomplete<'a>(
    ctx: crate::Context<'_>,
    partial: &str,
) -> serenity::all::CreateAutocompleteResponse<'a> {
    let data = ctx.data();

    #[derive(sqlx::FromRow)]
    struct TemplateRecord {
        name: String,
        friendly_name: String,
    }

    match sqlx::query_as(
        "SELECT DISTINCT name, friendly_name FROM template_shop WHERE name ILIKE $1 OR friendly_name ILIKE $1",
    )
    .bind(format!("%{}%", partial.replace('%', "\\%").replace('_', "\\_")))
    .fetch_all(&data.pool)
    .await {
        Ok(templates) => {
            let templates: Vec<TemplateRecord> = templates;
            let mut choices = serenity::all::CreateAutocompleteResponse::new();

            for template in templates {
                choices = choices.add_choice(serenity::all::AutocompleteChoice::new(
                    format!("{} | {}", template.friendly_name, template.name),
                    template.name,
                ));
            }

            choices
        },
        Err(e) => {
            log::error!("Failed to fetch shop templates: {:?}", e);
            serenity::builder::CreateAutocompleteResponse::new()
        }
    }
}

/// Loads an Anti-Raid template/module
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn load(
    ctx: crate::Context<'_>,
    #[autocomplete = "load_autocomplete"] template_name: String,
    #[description = "Channel to send errors to"] error_channel: Option<serenity::all::GuildChannel>,
    #[description = "Version of the template to load. Defaults to latest"] version: Option<String>,
) -> Result<(), crate::Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("You must be in a guild to use this command")?;

    crate::botlib::permission_checks::check_permissions(
        guild_id,
        ctx.author().id,
        &ctx.data().pool,
        ctx.serenity_context(),
        &ctx.data().reqwest,
        &Some(ctx),
        "bot.load".into(),
    )
    .await?;

    let data = ctx.data();

    let version = version.as_deref().unwrap_or("latest");

    #[derive(sqlx::FromRow)]
    struct LoadData {
        version: String,
        description: String,
        events: Vec<String>,
        language: String,
        allowed_caps: Vec<String>,
    }

    let rec = {
        if version == "latest" {
            let rec: Option<LoadData> = sqlx::query_as(
                "SELECT version, description, events, language, allowed_caps FROM template_shop WHERE name = $1 ORDER BY version DESC LIMIT 1",
            )
            .bind(&template_name)
            .fetch_optional(&data.pool)
            .await?;

            if let Some(rec) = rec {
                rec
            } else {
                return Err("No template with that name found in the shop".into());
            }
        } else {
            let rec: Option<LoadData> = sqlx::query_as(
                "SELECT version, description, events, language, allowed_caps FROM template_shop WHERE name = $1 AND version = $2",
            )
            .bind(&template_name)
            .bind(version)
            .fetch_optional(&data.pool)
            .await?;

            if let Some(rec) = rec {
                rec
            } else {
                return Err("No template with that name and version found in the shop".into());
            }
        }
    };

    // Ask the user to confirm that they want to load the template
    let confirm = ctx
        .send(
            poise::CreateReply::new()
                .embed(
                    CreateEmbed::default()
                        .title("Load Template?")
                        .description(format!(
                            "Are you sure you want to load the template `{} v{}`?",
                            template_name.replace('`', "\\`"),
                            rec.version.replace('`', "\\`")
                        ))
                        .field(
                            "Description",
                            if rec.description.len() > 300 {
                                format!("{}...", &rec.description[..300])
                            } else {
                                rec.description
                            },
                            false,
                        )
                        .field(
                            "Language",
                            if rec.language.len() > 300 {
                                format!("{}...", &rec.language[..300])
                            } else {
                                rec.language
                            },
                            false,
                        )
                        .field(
                            "Events",
                            {
                                let events_str = rec
                                    .events
                                    .iter()
                                    .map(|e| e.to_lowercase())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                if events_str.len() > 300 {
                                    format!("``{}...``", &events_str[..300])
                                } else {
                                    format!("``{}``", events_str)
                                }
                            },
                            false,
                        )
                        .field(
                            "Capabilities",
                            {
                                let allowed_caps_str = rec.allowed_caps.join(", ");
                                if allowed_caps_str.len() > 300 {
                                    format!("``{}...``", &allowed_caps_str[..300])
                                } else {
                                    format!("``{}``", allowed_caps_str)
                                }
                            },
                            false,
                        ),
                )
                .components(vec![CreateActionRow::buttons(vec![
                    CreateButton::new("yes")
                        .label("Yes")
                        .style(serenity::all::ButtonStyle::Danger),
                    CreateButton::new("no")
                        .label("No")
                        .style(serenity::all::ButtonStyle::Primary),
                ])]),
        )
        .await?
        .into_message()
        .await?;

    let int_col = confirm
        .id
        .await_component_interactions(ctx.serenity_context().shard.clone())
        .author_id(ctx.author().id);

    let Some(confirm) = int_col.await else {
        return Err("No response".into());
    };

    if confirm.data.custom_id != "yes" {
        confirm
            .create_response(
                ctx.http(),
                serenity::all::CreateInteractionResponse::Message(
                    serenity::all::CreateInteractionResponseMessage::new()
                        .content("Cancelled successfully!"),
                ),
            )
            .await?;
        return Ok(());
    }

    // Add template to servers list of templates
    let name = silverpelt::templates::create_shop_template(&template_name, version);
    sqlx::query(
        "INSERT INTO guild_templates (guild_id, name, content, events, allowed_caps, error_channel, created_by, last_updated_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(guild_id.to_string())
    .bind(&name)
    .bind(serde_json::Value::Null)
    .bind(&rec.events)
    .bind(&rec.allowed_caps)
    .bind(match error_channel {
        Some(channel) => channel.id.to_string(),
        None => ctx.channel_id().to_string(),
    })
    .bind(ctx.author().id.to_string())
    .bind(ctx.author().id.to_string())
    .execute(&data.pool)
    .await
    .map_err(|e| format!("Failed to add template to guild: {:?}", e))?;

    // Dispatch a OnStartup event for the template
    AntiraidEvent::OnStartup(vec![name])
        .dispatch_to_template_worker_and_nowait(&ctx.data(), guild_id, &template_dispatch_data())
        .await
        .map_err(|e| format!("Failed to dispatch OnStartup event: {:?}", e))?;

    confirm
        .create_response(
            ctx.http(),
            serenity::all::CreateInteractionResponse::Message(
                serenity::all::CreateInteractionResponseMessage::new()
                    .content("AntiRaid template loaded successfully!"),
            ),
        )
        .await?;
    Ok(())
}
