use serenity::all::{CreateActionRow, CreateButton, CreateEmbed};

pub async fn load_autocomplete<'a>(
    ctx: crate::Context<'_>,
    partial: &str,
) -> serenity::all::CreateAutocompleteResponse<'a> {
    let data = ctx.data();

    match sqlx::query!(
        "SELECT name, friendly_name FROM template_shop WHERE name ILIKE $1 OR friendly_name ILIKE $1",
        format!("%{}%", partial.replace('%', "\\%").replace('_', "\\_")),
    )
    .fetch_all(&data.pool)
    .await {
        Ok(templates) => {
            let mut choices = serenity::all::CreateAutocompleteResponse::new();

            for template in templates {
                choices = choices.add_choice(serenity::all::AutocompleteChoice::new(
                    template.friendly_name,
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

/// Adds a protection template from the shop to your server
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

    let data = ctx.data();

    let version = version.as_deref().unwrap_or("latest");

    let (version, description, events) = {
        if version == "latest" {
            let rec = sqlx::query!(
                "SELECT version, description, events FROM template_shop WHERE name = $1 ORDER BY version DESC LIMIT 1",
                template_name,
            )
            .fetch_optional(&data.pool)
            .await?;

            if let Some(rec) = rec {
                (rec.version, rec.description, rec.events)
            } else {
                return Err("No template with that name found in the shop".into());
            }
        } else {
            let rec = sqlx::query!(
                "SELECT version, description, events FROM template_shop WHERE name = $1 AND version = $2",
                template_name,
                version,
            )
            .fetch_optional(&data.pool)
            .await?;

            if let Some(rec) = rec {
                (rec.version, rec.description, rec.events)
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
                            "Are you sure you want to load the template `{}`, version `{}`?",
                            template_name, version
                        ))
                        .field(
                            "Description",
                            if description.len() > 300 {
                                format!("{}...", &description[..300])
                            } else {
                                description
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
        return Ok(());
    }

    // Add template to servers list of templates
    sqlx::query!(
        "INSERT INTO guild_templates (guild_id, name, content, events, error_channel, created_by, last_updated_by) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        guild_id.to_string(),
        silverpelt::templates::create_shop_template(&template_name, &version),
        "".to_string(),
        &events,
        match error_channel {
            Some(channel) => channel.id.to_string(),
            None => ctx.channel_id().to_string(),
        },
        ctx.author().id.to_string(),
        ctx.author().id.to_string()
    )
    .execute(&data.pool)
    .await
    .map_err(|e| format!("Failed to add template to guild: {:?}", e))?;

    ctx.send(poise::CreateReply::new().content("Template loaded successfully!"))
        .await?;

    Ok(())
}
