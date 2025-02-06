use futures_util::StreamExt;
use poise::serenity_prelude::{
    self as serenity, ChannelId, ComponentInteraction, ComponentInteractionDataKind,
    CreateActionRow, CreateButton, CreateEmbed, CreateSelectMenuOption, MessageId,
};
use poise::{Command, CreateReply};
use silverpelt::data::Data;
use silverpelt::Error;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;

async fn _filter(
    ctx: &crate::Context<'_>,
    cmd: &poise::Command<silverpelt::data::Data, silverpelt::Error>,
) -> Result<bool, Error> {
    let Some(guild_id) = ctx.guild_id() else {
        return Err("You must be in a guild to use ``filter_by_perms``".into());
    };

    let data = ctx.data();

    match crate::botlib::permission_checks::check_command(
        &cmd.qualified_name,
        guild_id,
        ctx.author().id,
        &ctx.data().pool,
        ctx.serenity_context(),
        &data.reqwest,
        &Some(*ctx),
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Struct to store embed data for the help command
struct EmbedHelp {
    category: String,
    desc: String,
}

async fn _embed_help(
    pctx: crate::Context<'_>,
    ctx: poise::FrameworkContext<'_, Data, crate::Error>,
    filter_by_perms: Option<bool>,
) -> Result<Vec<EmbedHelp>, Error> {
    let mut categories = indexmap::IndexMap::<Option<String>, Vec<&Command<Data, Error>>>::new();
    for cmd in &ctx.options().commands {
        // Check if category exists
        let category = cmd.category.as_ref().map(|x| x.to_string());

        if categories.contains_key(&category) {
            categories.get_mut(&category).unwrap().push(cmd);
        }
        // If category doesn't exist, create it
        else {
            categories.insert(category, vec![cmd]);
        }
    }

    let mut help_arr = Vec::new();

    for (category, commands) in categories {
        let cat_name = category.unwrap_or("Misc Commands".to_string());

        let mut menu = "".to_string();
        for command in commands {
            if command.hide_in_help {
                continue;
            }

            let mut flag = true;

            for check in command.checks.iter() {
                let res = check(pctx).await;

                // User may not run this command
                if res.is_err() {
                    continue;
                }

                let res = res.unwrap();

                if !res {
                    flag = false;
                    break;
                }
            }

            if !flag {
                continue;
            }

            let _ = writeln!(
                menu,
                "/{cmd_name} - {desc}",
                cmd_name = command.name,
                desc = command
                    .description
                    .as_deref()
                    .unwrap_or("*No description available yet*")
            );

            if command.context_menu_action.is_some() {
                let _ = writeln!(
                    menu,
                    "*This command is a context menu command of type {type:#?}*",
                    r#type = command.context_menu_action.unwrap()
                );
                continue;
            }

            if !command.subcommands.is_empty() {
                let _ = writeln!(menu, "**Subcommands**",);

                for subcmd in command.subcommands.iter() {
                    if subcmd.hide_in_help {
                        continue;
                    }

                    if filter_by_perms.unwrap_or(false) {
                        let res = _filter(&pctx, subcmd).await?;

                        if !res {
                            continue;
                        }
                    }

                    let _ = writeln!(
                        menu,
                        "``/{cmd_name} {subcmd_name}``: {desc}",
                        cmd_name = command.name,
                        subcmd_name = subcmd.name,
                        desc = subcmd
                            .description
                            .as_deref()
                            .unwrap_or("*No description available yet*")
                    );
                }
            }
        }

        help_arr.push(EmbedHelp {
            category: cat_name.to_string(),
            desc: menu.clone(),
        });
    }

    Ok(help_arr)
}

/// Instead of cloning a large Message struct, we use a temporary MsgInfo struct to store just the info we need
pub struct MsgInfo {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

/// Internal function that creates a select menu
fn _create_select_menu(data: &[EmbedHelp], index: usize) -> serenity::builder::CreateSelectMenu {
    let mut options = Vec::new();

    for (i, pane) in data.iter().enumerate() {
        if i == index {
            options.push(CreateSelectMenuOption::new(
                pane.category.clone() + " (current)",
                i.to_string(),
            ))
        } else {
            options.push(CreateSelectMenuOption::new(
                pane.category.clone(),
                i.to_string(),
            ));
        }
    }

    serenity::builder::CreateSelectMenu::new(
        "hnav:selectmenu",
        serenity::builder::CreateSelectMenuKind::String {
            options: options.into(),
        },
    )
    .custom_id("hnav:selectmenu")
}

fn _create_reply<'a>(
    data: &'a EmbedHelp,
    l_data: &'a [EmbedHelp],
    index: usize,
    prev_disabled: bool,
    next_disabled: bool,
) -> CreateReply<'a> {
    CreateReply::default()
        .embed(
            CreateEmbed::default()
                .title(format!("{} (Page {})", data.category, index + 1))
                .description(&data.desc),
        )
        .components(vec![
            CreateActionRow::Buttons(
                vec![
                    CreateButton::new("hnav:".to_string() + &(index - 1).to_string())
                        .label("Previous")
                        .disabled(prev_disabled),
                    CreateButton::new("hnav:cancel")
                        .label("Cancel")
                        .style(serenity::ButtonStyle::Danger),
                    CreateButton::new("hnav:".to_string() + &(index + 1).to_string())
                        .label("Next")
                        .disabled(next_disabled),
                ]
                .into(),
            ),
            CreateActionRow::SelectMenu(_create_select_menu(l_data, index)),
        ])
}

async fn _help_send_index<Data: Send + Sync + 'static>(
    ctx: Option<poise::Context<'_, Data, crate::Error>>,
    old_msg: Option<MsgInfo>,
    http: &Arc<serenity::Http>,
    l_data: &[EmbedHelp],
    index: usize,
    interaction: Option<Arc<ComponentInteraction>>,
) -> Result<Option<serenity::Message>, crate::Error> {
    let next_disabled = index >= l_data.len() - 1;

    let data = l_data.get(index);

    let prev_disabled = index == 0;

    match data {
        None => return Ok(None),
        Some(data) => {
            if let Some(old_msg) = old_msg {
                if interaction.is_none() {
                    old_msg
                        .channel_id
                        .edit_message(
                            http,
                            old_msg.message_id,
                            _create_reply(data, l_data, index, prev_disabled, next_disabled)
                                .to_prefix_edit(serenity::EditMessage::new()),
                        )
                        .await?;
                } else {
                    let interaction = interaction.unwrap();

                    interaction
                        .edit_response(
                            http,
                            _create_reply(data, l_data, index, prev_disabled, next_disabled)
                                .to_slash_initial_response_edit(
                                    poise::serenity_prelude::EditInteractionResponse::new(),
                                ),
                        )
                        .await?;
                }

                return Ok(None);
            }

            if let Some(ctx) = ctx {
                let msg = ctx
                    .send(_create_reply(
                        data,
                        l_data,
                        index,
                        prev_disabled,
                        next_disabled,
                    ))
                    .await?
                    .into_message()
                    .await?;

                return Ok(Some(msg));
            }
        }
    }

    Ok(None)
}

#[poise::command(slash_command)]
/// Help command implementation
pub async fn help(
    ctx: crate::Context<'_>,
    command: Option<String>,
    #[description = "Only show commands you have permission to use"] filter_by_perms: Option<bool>,
) -> Result<(), Error> {
    if let Some(cmd) = command {
        // They just want the parameters for a specific command
        for botcmd in &ctx.framework().options().commands {
            if botcmd.name == cmd {
                let params_str = botcmd
                    .parameters
                    .iter()
                    .map(|p| {
                        format!(
                            "{} - {}",
                            p.name,
                            p.description
                                .as_deref()
                                .unwrap_or("No description available yet")
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                let mut embed = CreateEmbed::default()
                    .title(format!("Help for {}", botcmd.name))
                    .description(
                        botcmd
                            .description
                            .as_deref()
                            .unwrap_or("No description available yet"),
                    )
                    .field("Parameters", params_str, false);

                for subcmd in botcmd.subcommands.iter() {
                    embed = embed.field(
                        subcmd.name.clone(),
                        format!(
                            "{}\n{}",
                            subcmd
                                .description
                                .as_deref()
                                .unwrap_or("No description available yet"),
                            subcmd
                                .parameters
                                .iter()
                                .map(|p| format!(
                                    "*{}* - {}",
                                    p.name,
                                    p.description
                                        .as_deref()
                                        .unwrap_or("No description available yet")
                                ))
                                .collect::<Vec<String>>()
                                .join("\n")
                        ),
                        false,
                    );
                }

                ctx.send(CreateReply::default().embed(embed)).await?;

                return Ok(());
            }
        }

        ctx.say("Command not found!").await?;
        return Ok(());
    }

    let eh = _embed_help(ctx, ctx.framework(), filter_by_perms).await?;

    let msg = _help_send_index(Some(ctx), None, &ctx.serenity_context().http, &eh, 0, None).await?;

    if let Some(msg) = msg {
        // Create a collector
        let interaction = msg
            .id
            .await_component_interactions(ctx.serenity_context().shard.clone())
            .author_id(ctx.author().id)
            .timeout(Duration::from_secs(120));

        let mut collect_stream = interaction.stream();

        while let Some(item) = collect_stream.next().await {
            item.defer(&ctx.serenity_context().http).await?;

            let id = &item.data.custom_id;

            if id == "hnav:cancel" {
                item.delete_response(&ctx.serenity_context().http).await?;
                return Ok(());
            }

            if id == "hnav:selectmenu" {
                // This is a select menu, get the value using modal_get
                let value = match item.data.kind {
                    ComponentInteractionDataKind::StringSelect { ref values, .. } => {
                        if values.is_empty() {
                            return Err("Internal error: No value selected".into());
                        }

                        &values[0]
                    }
                    _ => {
                        return Err("Internal error: Invalid interaction type".into());
                    }
                };

                let value = value.parse::<usize>()?;

                _help_send_index::<Data>(
                    None,
                    Some(MsgInfo {
                        channel_id: msg.channel_id,
                        message_id: msg.id,
                    }),
                    &ctx.serenity_context().http,
                    &eh,
                    value,
                    Some(Arc::new(item.clone())),
                )
                .await?;

                continue;
            }

            if id.starts_with("hnav:") {
                let id = id.replace("hnav:", "");
                let id = id.parse::<usize>()?;

                _help_send_index::<Data>(
                    None,
                    Some(MsgInfo {
                        channel_id: msg.channel_id,
                        message_id: msg.id,
                    }),
                    &ctx.serenity_context().http,
                    &eh,
                    id,
                    Some(Arc::new(item.clone())),
                )
                .await?;
            }
        }
    } else {
        return Err("No help message found".into());
    }

    Ok(())
}
