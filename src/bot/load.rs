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
) -> Result<(), crate::Error> {
    let data = ctx.data();

    let Some(tdata) = sqlx::query!(
        "SELECT version, events FROM template_shop WHERE name = $1 ORDER BY version DESC LIMIT 1",
        template_name,
    )
    .fetch_optional(&data.pool)
    .await?
    else {
        return Err("No template with that name found in the shop".into());
    };

    let guild_id = ctx
        .guild_id()
        .ok_or("You must be in a guild to use this command")?;

    // Add template to servers list of templates
    sqlx::query!(
        "INSERT INTO guild_templates (guild_id, name, content, events, error_channel, created_by, last_updated_by) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        guild_id.to_string(),
        silverpelt::templates::create_shop_template(&template_name, &tdata.version),
        "".to_string(),
        &tdata.events,
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

    Ok(())
}
