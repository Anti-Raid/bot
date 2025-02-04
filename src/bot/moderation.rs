use crate::{bot::template_dispatch_data, Context};
use antiraid_types::{
    ar_event::{AntiraidEvent, ModerationAction, ModerationEndEventData, ModerationStartEventData},
    punishments::{PunishmentCreate, PunishmentState, PunishmentTarget},
    stings::{StingCreate, StingState, StingTarget},
};
use futures_util::StreamExt;
use jobserver::embed::{embed as embed_job, get_icon_of_state};
use poise::CreateReply;
use sandwich_driver::{guild, member_in_guild};
use serenity::all::{
    ChannelId, CreateEmbed, EditMember, EditMessage, GuildId, Mentionable, Timestamp, User, UserId,
};
use silverpelt::{
    ar_event::AntiraidEventOperations,
    punishments::{PunishmentCreateOperations, PunishmentOperations},
    stings::{StingCreateOperations, StingOperations},
    Error,
};
use splashcore_rs::utils::{
    create_special_allocation_from_str, parse_duration_string, parse_numeric_list,
    parse_numeric_list_to_str, Unit, REPLACE_CHANNEL,
};
use std::{collections::HashMap, time::Duration};

use super::sandwich_config;

/// Helper method to get the username of a user
fn username(m: &User) -> String {
    if let Some(ref global_name) = m.global_name {
        global_name.to_string()
    } else {
        m.tag()
    }
}

/// Helper method to get the username of a member
fn to_log_format(moderator: &User, member: &User, reason: &str) -> String {
    format!(
        "{} | Handled '{}' for reason '{}'",
        username(moderator),
        username(member),
        reason
    )
}

/*
// Options that can be set when pruning a message
//
// Either one of PruneFrom or MaxMessages must be set. If both are set, then both will be used.
type MessagePruneOpts struct {
    UserID             string         `description:"If set, the user id to prune messages of"`
    Channels           []string       `description:"If set, the channels to prune messages from"`
    IgnoreErrors       bool           `description:"If set, ignore errors while pruning"`
    MaxMessages        int            `description:"The maximum number of messages to prune"`
    PruneFrom          timex.Duration `description:"If set, the time to prune messages from."`
    PerChannel         int            `description:"The minimum number of messages to prune per channel"`
    RolloverLeftovers  bool           `description:"Whether to attempt rollover of leftover message quota to another channels or not"`
    SpecialAllocations map[string]int `description:"Specific channel allocation overrides"`
}
*/

#[allow(clippy::too_many_arguments)]
fn create_message_prune_serde(
    user_id: Option<UserId>,
    guild_id: GuildId,
    channels: &Option<String>,
    ignore_errors: Option<bool>,
    max_messages: Option<i32>,
    prune_from: Option<String>,
    per_channel: Option<i32>,
    rollover_leftovers: Option<bool>,
    special_allocations: Option<String>,
) -> Result<serde_json::Value, Error> {
    let channels = if let Some(ref channels) = channels {
        parse_numeric_list_to_str::<ChannelId>(channels, &REPLACE_CHANNEL)?
    } else {
        vec![]
    };

    let prune_from = if let Some(ref prune_from) = prune_from {
        let (dur, unit) = parse_duration_string(prune_from)?;

        dur * unit.to_seconds()
    } else {
        0
    };

    let special_allocations = if let Some(ref special_allocations) = special_allocations {
        create_special_allocation_from_str(special_allocations)?
    } else {
        HashMap::new()
    };

    Ok(serde_json::json!(
        {
            "ServerID": guild_id.to_string(),
            "Options": {
                "UserID": user_id,
                "Channels": channels,
                "IgnoreErrors": ignore_errors.unwrap_or(false),
                "MaxMessages": max_messages.unwrap_or(1000),
                "PruneFrom": prune_from,
                "PerChannel": per_channel.unwrap_or(100),
                "RolloverLeftovers": rollover_leftovers.unwrap_or(false),
                "SpecialAllocations": special_allocations,
            }
        }
    ))
}

/// Helper method to check the author of a user versus a target
async fn check_hierarchy(ctx: &Context<'_>, user_id: UserId) -> Result<(), Error> {
    let data = ctx.data();
    let sctx = ctx.serenity_context();

    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let guild = guild(
        &sctx.cache,
        &sctx.http,
        &data.reqwest,
        guild_id,
        &sandwich_config(),
    )
    .await?;

    let author_id = ctx.author().id;

    let bot_userid = sctx.cache.current_user().id;
    let Some(bot) = member_in_guild(
        &sctx.cache,
        &sctx.http,
        &data.reqwest,
        guild_id,
        bot_userid,
        &sandwich_config(),
    )
    .await?
    else {
        return Err("Bot member not found".into());
    };

    let Some(author) = member_in_guild(
        &sctx.cache,
        &sctx.http,
        &data.reqwest,
        guild_id,
        author_id,
        &sandwich_config(),
    )
    .await?
    else {
        return Err("Message author not found".into());
    };

    let Some(user) = member_in_guild(
        &sctx.cache,
        &sctx.http,
        &data.reqwest,
        guild_id,
        user_id,
        &sandwich_config(),
    )
    .await?
    else {
        // User is not in the server, so yes, they're below us
        return Ok(());
    };

    if let Some(higher_hierarchy) = guild.greater_member_hierarchy(&bot, &user) {
        if higher_hierarchy != bot_userid {
            log::info!("Roles of lhs: {:?}", bot.roles);
            log::info!("Roles of rhs: {:?}", user.roles);
            return Err(format!("You cannot moderate a user with a higher or equal hierarchy to the bot ({} has higher hierarchy)", higher_hierarchy.mention()).into());
        }
    } else {
        return Err("You cannot moderate a user with equal hierarchy to the bot".into());
    }

    if let Some(higher_hierarchy) = guild.greater_member_hierarchy(&author, &user) {
        if higher_hierarchy != author_id {
            Err("You cannot moderate a user with a higher or equal hierarchy than you".into())
        } else {
            Ok(())
        }
    } else {
        Err("You cannot moderate a user with equal hierarchy to you".into())
    }
}

/// Moderation base command
#[poise::command(
    slash_command,
    guild_only,
    user_cooldown = "5",
    subcommands("prune", "kick", "ban", "tempban", "unban", "timeout",)
)]
#[allow(clippy::too_many_arguments)]
pub async fn moderation(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Customizable pruning of messages
#[poise::command(
    slash_command,
    guild_only,
    user_cooldown = "5",
    required_bot_permissions = "KICK_MEMBERS | MANAGE_MESSAGES"
)]
#[allow(clippy::too_many_arguments)]
async fn prune(
    ctx: Context<'_>,
    #[description = "The reason for the prune"]
    #[max_length = 512]
    reason: String,
    #[description = "The user to prune messages of"] user: Option<serenity::all::User>,
    #[description = "Number of stings to give. Defaults to configured base stings"] stings: Option<
        i32,
    >,
    #[description = "Whether or not to show prune status updates"] prune_debug: Option<bool>,
    #[description = "Channels to prune from, otherwise will prune from all channels"]
    prune_channels: Option<String>,
    #[description = "Whether or not to avoid errors while pruning"] prune_ignore_errors: Option<
        bool,
    >,
    #[description = "How many messages at maximum to prune"] prune_max_messages: Option<i32>,
    #[description = "The duration to prune from. Format: <number> days/hours/minutes/seconds"]
    prune_from: Option<String>,
    #[description = "The minimum number of messages to prune per channel"]
    prune_per_channel: Option<i32>,
    #[description = "Whether to attempt rollover of leftover message quota to another channels or not"]
    prune_rollover_leftovers: Option<bool>,
    #[description = "Specific channel allocation overrides"] prune_special_allocations: Option<
        String,
    >,
) -> Result<(), Error> {
    if reason.len() > 512 {
        return Err("Reason must be less than/equal to 512 characters".into());
    }

    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    // Check user hierarchy before performing moderative actions
    if let Some(ref user) = user {
        check_hierarchy(&ctx, user.id).await?;
    }

    let mut embed = CreateEmbed::new()
        .title("Pruning Messages...")
        .description(format!(
            "{} | Pruning User Messages",
            get_icon_of_state("pending"),
        ));

    let base_message = ctx.send(CreateReply::new().embed(embed)).await?;

    let Some(author) = ctx.author_member().await else {
        return Err("This command can only be used in a guild".into());
    };

    let stings = stings.unwrap_or(0);

    if stings < 0 {
        return Err("Stings must be greater than or equal to 0".into());
    }

    // If we're pruning messages, do that
    let prune_opts = create_message_prune_serde(
        user.as_ref().map(|u| u.id),
        guild_id,
        &prune_channels,
        prune_ignore_errors,
        prune_max_messages,
        prune_from,
        prune_per_channel,
        prune_rollover_leftovers,
        prune_special_allocations,
    )?;

    let data = ctx.data();

    // Fire ModerationStart event
    let author_user_id = author.user.id;
    let target_user_id = user.as_ref().map(|u| u.id);
    let correlation_id = sqlx::types::uuid::Uuid::new_v4();
    let results = AntiraidEvent::ModerationStart(ModerationStartEventData {
        correlation_id,
        reason: Some(reason.clone()),
        author: match author {
            std::borrow::Cow::Borrowed(member) => member.clone(),
            std::borrow::Cow::Owned(member) => member,
        },
        action: ModerationAction::Prune {
            user,
            prune_opts: prune_opts.clone(),
            channels: if let Some(ref channels) = prune_channels {
                parse_numeric_list::<ChannelId>(channels, &REPLACE_CHANNEL)?
            } else {
                Vec::new()
            },
        },
        num_stings: stings,
    })
    .dispatch_to_template_worker_and_wait(
        &data,
        guild_id,
        &template_dispatch_data(),
        Duration::from_secs(1),
    )
    .await?;

    if !results.can_execute() {
        // Check for hierarchy
        if let Some(user) = target_user_id {
            check_hierarchy(&ctx, user).await?;
        }
    }

    let mut tx = ctx.data().pool.begin().await?;

    let mut sting_dispatch = None;

    if stings > 0 {
        sting_dispatch = Some(
            StingCreate {
                src: Some("moderation:prune_user".to_string()),
                stings,
                reason: Some(reason),
                void_reason: None,
                guild_id,
                creator: StingTarget::User(author_user_id),
                target: match target_user_id {
                    Some(id) => StingTarget::User(id),
                    None => StingTarget::System,
                },
                state: StingState::Active,
                duration: None,
                sting_data: None,
            }
            .create_without_dispatch(&mut *tx)
            .await?,
        );
    }

    // Make request to jobserver
    let id = jobserver::spawn::spawn_task(
        &data.reqwest,
        &jobserver::Spawn {
            name: "message_prune".to_string(),
            data: prune_opts,
            create: true,
            execute: true,
            id: None,
            user_id: author_user_id.to_string(),
        },
        &config::CONFIG.base_ports.jobserver_base_addr,
        config::CONFIG.base_ports.jobserver,
    )
    .await?
    .id;

    tx.commit().await?;

    // Lastly, fire sting create event
    if let Some(sting_dispatch) = sting_dispatch {
        sting_dispatch
            .dispatch_create_event(ctx.serenity_context().clone(), &template_dispatch_data())
            .await?;
    };

    embed = CreateEmbed::new()
        .title("Pruning User Messages...")
        .description(format!(
            "{} | Pruning User Messages...",
            get_icon_of_state("pending")
        ))
        .field(
            "Pruning Messages",
            format!(":yellow_circle: Created job with ID of {}", id),
            false,
        );

    base_message
        .edit(ctx, CreateReply::new().embed(embed.clone()))
        .await?;

    let mut stream = Box::pin(jobserver::poll::reactive(
        &ctx.data().pool,
        &id,
        jobserver::poll::PollTaskOptions::default(),
    )?);

    while let Some(job) = stream.next().await {
        match job {
            Ok(Some(job)) => {
                let new_job_msg = embed_job(
                    &config::CONFIG.sites.api,
                    &job,
                    vec![CreateEmbed::default()
                        .title("Pruning User Messages...")
                        .description(format!(
                            "{} | Pruning User Messages",
                            get_icon_of_state(&job.state),
                        ))],
                    prune_debug.unwrap_or(false),
                )?;

                base_message
                    .edit(ctx, {
                        let mut msg = CreateReply::new();
                        for embed in new_job_msg.embeds {
                            msg = msg.embed(embed);
                        }
                        msg = msg.components(new_job_msg.components);

                        msg
                    })
                    .await?;
            }
            Ok(None) => {
                continue; // Go to the next iteration
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    AntiraidEvent::ModerationEnd(ModerationEndEventData { correlation_id })
        .dispatch_to_template_worker_and_nowait(&data, guild_id, &template_dispatch_data())
        .await?;

    Ok(())
}

/// Kicks a member from the server with optional purge/stinging abilities
#[poise::command(
    slash_command,
    guild_only,
    user_cooldown = "5",
    required_bot_permissions = "KICK_MEMBERS | MANAGE_MESSAGES"
)]
async fn kick(
    ctx: Context<'_>,
    #[description = "The member to kick"] member: serenity::all::Member,
    #[description = "The reason for the kick"]
    #[max_length = 384]
    reason: String,
    #[description = "Number of stings to give. Defaults to configured base stings"] stings: Option<
        i32,
    >,
) -> Result<(), Error> {
    if reason.len() > 384 {
        return Err("Reason must be less than/equal to 384 characters".into());
    }

    let stings = stings.unwrap_or(0);

    if stings < 0 {
        return Err("Stings must be greater than or equal to 0".into());
    }

    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let data = ctx.data();

    // Dispatch event to modules, erroring out if the dispatch errors (e.g. limits hit due to a lua template etc)
    let Some(author) = ctx.author_member().await else {
        return Err("This command can only be used in a guild".into());
    };

    let kick_log_msg = to_log_format(&author.user, &member.user, &reason);

    let correlation_id = sqlx::types::Uuid::new_v4();
    let author_user_id = author.user.id;
    let target_user_id = member.user.id;
    let target_mention = member.mention();

    let results = AntiraidEvent::ModerationStart(ModerationStartEventData {
        correlation_id,
        reason: Some(reason.clone()),
        action: ModerationAction::Kick { member },
        author: match author {
            std::borrow::Cow::Borrowed(member) => member.clone(),
            std::borrow::Cow::Owned(member) => member,
        },
        num_stings: stings,
    })
    .dispatch_to_template_worker_and_wait(
        &data,
        guild_id,
        &template_dispatch_data(),
        Duration::from_secs(1),
    )
    .await?;

    if !results.can_execute() {
        // Fallback to simple hierarchy check
        check_hierarchy(&ctx, target_user_id).await?;
    }

    let mut embed = CreateEmbed::new()
        .title("Kicking Member...")
        .description(format!(
            "{} | Kicking {}",
            get_icon_of_state("pending"),
            target_mention
        ));

    let mut base_message = ctx
        .send(CreateReply::new().embed(embed))
        .await?
        .into_message()
        .await?;

    // Try kicking them
    let mut tx = data.pool.begin().await?;

    let mut sting_dispatch = None;

    if stings > 0 {
        sting_dispatch = Some(
            StingCreate {
                src: Some("moderation:kick".to_string()),
                stings,
                reason: Some(reason.clone()),
                void_reason: None,
                guild_id,
                creator: StingTarget::User(author_user_id),
                target: StingTarget::User(target_user_id),
                state: StingState::Active,
                duration: None,
                sting_data: None,
            }
            .create_without_dispatch(&mut *tx)
            .await?,
        );
    }

    // Create new punishment
    let p = PunishmentCreate {
        src: Some("kick".to_string()),
        guild_id,
        punishment: "kick".to_string(),
        creator: PunishmentTarget::User(author_user_id),
        target: PunishmentTarget::User(target_user_id),
        handle_log: serde_json::json!({}),
        duration: None,
        reason: reason.clone(),
        data: None,
        state: PunishmentState::Active,
    }
    .create_without_dispatch(&mut *tx)
    .await?;

    guild_id
        .kick(ctx.http(), target_user_id, Some(&kick_log_msg))
        .await?;

    tx.commit().await?;

    p.dispatch_event(ctx.serenity_context().clone(), &template_dispatch_data())
        .await?;
    if let Some(sting_dispatch) = sting_dispatch {
        sting_dispatch
            .dispatch_create_event(ctx.serenity_context().clone(), &template_dispatch_data())
            .await?;
    };

    AntiraidEvent::ModerationEnd(ModerationEndEventData { correlation_id })
        .dispatch_to_template_worker_and_nowait(&data, guild_id, &template_dispatch_data())
        .await?;

    embed = CreateEmbed::new()
        .title("Kicking Member...")
        .description(format!(
            "{} | Kicked {}",
            get_icon_of_state("completed"),
            target_mention
        ));

    base_message
        .edit(&ctx.http(), EditMessage::new().embed(embed))
        .await?;

    Ok(())
}

/// Bans a member from the server with optional purge/stinging abilities
#[poise::command(
    slash_command,
    guild_only,
    user_cooldown = "5",
    required_bot_permissions = "BAN_MEMBERS | MANAGE_MESSAGES"
)]
async fn ban(
    ctx: Context<'_>,
    #[description = "The member to ban"] user: serenity::all::User,
    #[description = "The reason for the ban"]
    #[max_length = 384]
    reason: String,
    #[description = "Number of stings to give. Defaults to configured base stings"] stings: Option<
        i32,
    >,
    #[description = "How many messages to prune using discords autopruner [dmd] (days)"] prune_dmd: Option<u8>,
) -> Result<(), Error> {
    if reason.len() > 384 {
        return Err("Reason must be less than/equal to 384 characters".into());
    }

    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let stings = stings.unwrap_or(1);

    if stings < 0 {
        return Err("Stings must be greater than or equal to 0".into());
    }

    let dmd = prune_dmd.unwrap_or_default();

    let data = ctx.data();

    // Dispatch event to modules, erroring out if the dispatch errors (e.g. limits hit due to a lua template etc)
    let Some(author) = ctx.author_member().await else {
        return Err("This command can only be used in a guild".into());
    };

    let ban_log_msg = to_log_format(&author.user, &user, &reason);

    let correlation_id = sqlx::types::Uuid::new_v4();
    let author_user_id = author.user.id;
    let target_user_id = user.id;
    let target_mention = user.mention();

    let results = AntiraidEvent::ModerationStart(ModerationStartEventData {
        correlation_id,
        reason: Some(reason.clone()),
        action: ModerationAction::Ban {
            user,
            prune_dmd: dmd,
        },
        author: match author {
            std::borrow::Cow::Borrowed(member) => member.clone(),
            std::borrow::Cow::Owned(member) => member,
        },
        num_stings: stings,
    })
    .dispatch_to_template_worker_and_wait(
        &data,
        guild_id,
        &template_dispatch_data(),
        Duration::from_secs(1),
    )
    .await?;

    if !results.can_execute() {
        // Fallback to simple hierarchy check
        check_hierarchy(&ctx, target_user_id).await?;
    }

    let mut embed = CreateEmbed::new()
        .title("Banning Member...")
        .description(format!(
            "{} | Banning {}",
            get_icon_of_state("pending"),
            target_mention
        ));

    let mut base_message = ctx
        .send(CreateReply::new().embed(embed))
        .await?
        .into_message()
        .await?;

    let mut tx = data.pool.begin().await?;

    let mut sting_dispatch = None;

    if stings > 0 {
        sting_dispatch = Some(
            StingCreate {
                src: Some("moderation:ban".to_string()),
                stings,
                reason: Some(reason.clone()),
                void_reason: None,
                guild_id,
                creator: StingTarget::User(author_user_id),
                target: StingTarget::User(target_user_id),
                state: StingState::Active,
                duration: None,
                sting_data: None,
            }
            .create_without_dispatch(&mut *tx)
            .await?,
        );
    }

    // Create new punishment
    let p = PunishmentCreate {
        src: Some("ban".to_string()),
        guild_id,
        punishment: "ban".to_string(),
        creator: PunishmentTarget::User(author_user_id),
        target: PunishmentTarget::User(target_user_id),
        handle_log: serde_json::json!({}),
        duration: None,
        reason: reason.clone(),
        data: None,
        state: PunishmentState::Active,
    }
    .create_without_dispatch(&mut *tx)
    .await?;

    guild_id
        .ban(ctx.http(), target_user_id, dmd, Some(&ban_log_msg))
        .await?;

    tx.commit().await?;

    p.dispatch_event(ctx.serenity_context().clone(), &template_dispatch_data())
        .await?;
    if let Some(sting_dispatch) = sting_dispatch {
        sting_dispatch
            .dispatch_create_event(ctx.serenity_context().clone(), &template_dispatch_data())
            .await?;
    };

    AntiraidEvent::ModerationEnd(ModerationEndEventData { correlation_id })
        .dispatch_to_template_worker_and_nowait(&data, guild_id, &template_dispatch_data())
        .await?;

    embed = CreateEmbed::new()
        .title("Banning Member...")
        .description(format!(
            "{} | Banned {}",
            get_icon_of_state("completed"),
            target_mention
        ));

    base_message
        .edit(&ctx.http(), EditMessage::new().embed(embed))
        .await?;

    Ok(())
}

/// Temporaily bans a member from the server with optional purge/stinging abilities
#[poise::command(
    slash_command,
    guild_only,
    user_cooldown = "5",
    required_bot_permissions = "BAN_MEMBERS | MANAGE_MESSAGES"
)]
async fn tempban(
    ctx: Context<'_>,
    #[description = "The user to ban"] user: serenity::all::User,
    #[description = "The reason for the ban"]
    #[max_length = 384]
    reason: String,
    #[description = "Number of stings to give. Defaults to configured base stings"] stings: Option<
        i32,
    >,
    #[description = "The duration of the ban"] duration: String,
    #[description = "How many messages to prune using discords autopruner [dmd] (days)"] prune_dmd: Option<u8>,
) -> Result<(), Error> {
    if reason.len() > 384 {
        return Err("Reason must be less than/equal to 384 characters".into());
    }

    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let stings = stings.unwrap_or(1);

    if stings < 0 {
        return Err("Stings must be greater than or equal to 0".into());
    }

    let dmd = prune_dmd.unwrap_or_default();

    let duration = parse_duration_string(&duration)?;

    let data = ctx.data();

    // Dispatch event to modules, erroring out if the dispatch errors (e.g. limits hit due to a lua template etc)
    let Some(author) = ctx.author_member().await else {
        return Err("This command can only be used in a guild".into());
    };

    let tempban_log_msg = to_log_format(&author.user, &user, &reason);

    let correlation_id = sqlx::types::Uuid::new_v4();
    let author_user_id = author.user.id;
    let target_user_id = user.id;
    let target_mention = user.mention();

    let results = AntiraidEvent::ModerationStart(ModerationStartEventData {
        correlation_id,
        reason: Some(reason.clone()),
        action: ModerationAction::TempBan {
            user,
            duration: (duration.0 * duration.1.to_seconds()),
            prune_dmd: dmd,
        },
        author: match author {
            std::borrow::Cow::Borrowed(member) => member.clone(),
            std::borrow::Cow::Owned(member) => member,
        },
        num_stings: stings,
    })
    .dispatch_to_template_worker_and_wait(
        &data,
        guild_id,
        &template_dispatch_data(),
        Duration::from_secs(1),
    )
    .await?;

    if !results.can_execute() {
        // Fallback to simple hierarchy check
        check_hierarchy(&ctx, target_user_id).await?;
    }

    let mut embed = CreateEmbed::new()
        .title("(Temporarily) Banning Member...")
        .description(format!(
            "{} | Banning {}",
            get_icon_of_state("pending"),
            target_mention
        ));

    let mut base_message = ctx
        .send(CreateReply::new().embed(embed))
        .await?
        .into_message()
        .await?;

    let mut tx = data.pool.begin().await?;

    let mut sting_dispatch = None;

    if stings > 0 {
        sting_dispatch = Some(
            StingCreate {
                src: Some("moderation:tempban".to_string()),
                stings,
                reason: Some(reason.clone()),
                void_reason: None,
                guild_id,
                creator: StingTarget::User(author_user_id),
                target: StingTarget::User(target_user_id),
                state: StingState::Active,
                duration: Some(std::time::Duration::from_secs(
                    duration.0 * duration.1.to_seconds(),
                )),
                sting_data: None,
            }
            .create_without_dispatch(&mut *tx)
            .await?,
        );
    }

    // Create new punishment
    let p = PunishmentCreate {
        src: Some("tempban".to_string()),
        guild_id,
        punishment: "ban".to_string(),
        creator: PunishmentTarget::User(author_user_id),
        target: PunishmentTarget::User(target_user_id),
        handle_log: serde_json::json!({}),
        duration: Some(std::time::Duration::from_secs(
            duration.0 * duration.1.to_seconds(),
        )),
        reason: reason.clone(),
        data: None,
        state: PunishmentState::Active,
    }
    .create_without_dispatch(&mut *tx)
    .await?;

    guild_id
        .ban(ctx.http(), target_user_id, dmd, Some(&tempban_log_msg))
        .await?;

    tx.commit().await?;

    p.dispatch_event(ctx.serenity_context().clone(), &template_dispatch_data())
        .await?;
    if let Some(sting_dispatch) = sting_dispatch {
        sting_dispatch
            .dispatch_create_event(ctx.serenity_context().clone(), &template_dispatch_data())
            .await?;
    };

    AntiraidEvent::ModerationEnd(ModerationEndEventData { correlation_id })
        .dispatch_to_template_worker_and_nowait(&data, guild_id, &template_dispatch_data())
        .await?;

    embed = CreateEmbed::new()
        .title("(Temporarily) Banned Member...")
        .description(format!(
            "{} | Banned {}",
            get_icon_of_state("completed"),
            target_mention
        ));

    base_message
        .edit(&ctx.http(), EditMessage::new().embed(embed))
        .await?;

    Ok(())
}

/// Unbans a member from the server with optional purge/stinging abilities
#[poise::command(
    slash_command,
    guild_only,
    user_cooldown = "5",
    required_bot_permissions = "BAN_MEMBERS | MANAGE_MESSAGES"
)]
async fn unban(
    ctx: Context<'_>,
    #[description = "The user to unban"] user: serenity::all::User,
    #[description = "The reason/justification for unbanning"]
    #[max_length = 384]
    reason: String,
    #[description = "Number of stings to give. Defaults to 0"] stings: Option<i32>,
) -> Result<(), Error> {
    if reason.len() > 384 {
        return Err("Reason must be less than/equal to 384 characters".into());
    }

    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let stings = stings.unwrap_or(1);

    if stings < 0 {
        return Err("Stings must be greater than or equal to 0".into());
    }

    let data = ctx.data();

    // Dispatch event to modules, erroring out if the dispatch errors (e.g. limits hit due to a lua template etc)
    let Some(author) = ctx.author_member().await else {
        return Err("This command can only be used in a guild".into());
    };

    let unban_log_msg = to_log_format(&author.user, &user, &reason);

    let correlation_id = sqlx::types::Uuid::new_v4();
    let author_user_id = author.user.id;
    let target_user_id = user.id;
    let target_mention = user.mention();

    AntiraidEvent::ModerationStart(ModerationStartEventData {
        correlation_id,
        reason: Some(reason.clone()),
        action: ModerationAction::Unban { user },
        author: match author {
            std::borrow::Cow::Borrowed(member) => member.clone(),
            std::borrow::Cow::Owned(member) => member,
        },
        num_stings: stings,
    })
    .dispatch_to_template_worker_and_wait(
        &data,
        guild_id,
        &template_dispatch_data(),
        Duration::from_secs(1),
    )
    .await?;

    let mut embed = CreateEmbed::new()
        .title("Unbanning Member...")
        .description(format!(
            "{} | Unbanning {}",
            get_icon_of_state("pending"),
            target_mention
        ));

    let mut base_message = ctx
        .send(CreateReply::new().embed(embed))
        .await?
        .into_message()
        .await?;

    let mut tx = data.pool.begin().await?;

    let mut sting_dispatch = None;

    if stings > 0 {
        sting_dispatch = Some(
            StingCreate {
                src: Some("moderation:unban".to_string()),
                stings,
                reason: Some(reason.clone()),
                void_reason: None,
                guild_id,
                creator: StingTarget::User(author_user_id),
                target: StingTarget::User(target_user_id),
                state: StingState::Active,
                duration: None,
                sting_data: None,
            }
            .create_without_dispatch(&mut *tx)
            .await?,
        );
    }

    ctx.http()
        .remove_ban(guild_id, target_user_id, Some(&unban_log_msg))
        .await?;

    tx.commit().await?;

    if let Some(sting_dispatch) = sting_dispatch {
        sting_dispatch
            .dispatch_create_event(ctx.serenity_context().clone(), &template_dispatch_data())
            .await?;
    };

    AntiraidEvent::ModerationEnd(ModerationEndEventData { correlation_id })
        .dispatch_to_template_worker_and_nowait(&data, guild_id, &template_dispatch_data())
        .await?;

    embed = CreateEmbed::new()
        .title("Unbanning Member...")
        .description(format!(
            "{} | Unbanned {}",
            get_icon_of_state("completed"),
            target_mention
        ));

    base_message
        .edit(&ctx.http(), EditMessage::new().embed(embed))
        .await?;

    Ok(())
}

/// Times out a member from the server with optional purge/stinging abilities
#[poise::command(
    slash_command,
    guild_only,
    user_cooldown = "5",
    required_bot_permissions = "MODERATE_MEMBERS | MANAGE_MESSAGES"
)]
async fn timeout(
    ctx: Context<'_>,
    #[description = "The member to timeout"] member: serenity::all::Member,
    #[description = "The duration of the timeout"] duration: String,
    #[description = "The reason for the timeout"]
    #[max_length = 384]
    reason: String,
    #[description = "Number of stings to give. Defaults to configured base stings"] stings: Option<
        i32,
    >,
) -> Result<(), Error> {
    if reason.len() > 384 {
        return Err("Reason must be less than/equal to 384 characters".into());
    }

    let Some(guild_id) = ctx.guild_id() else {
        return Err("This command can only be used in a guild".into());
    };

    let data = ctx.data();

    // Try timing them out
    let duration = parse_duration_string(&duration)?;

    // Ensure less than 28 days = 4 weeks = 672 hours = 40320 minutes = 2419200 seconds
    if duration.0 > 7 && duration.1 == Unit::Weeks {
        return Err("Timeout duration must be less than 28 days (4 weeks)".into());
    } else if duration.0 > 28 && duration.1 == Unit::Days {
        return Err("Timeout duration must be less than 28 days".into());
    } else if duration.0 > 672 && duration.1 == Unit::Hours {
        return Err("Timeout duration must be less than 28 days (672 hours)".into());
    } else if duration.0 > 40320 && duration.1 == Unit::Minutes {
        return Err("Timeout duration must be less than 28 days (40320 minutes)".into());
    } else if duration.0 > 2419200 && duration.1 == Unit::Seconds {
        return Err("Timeout duration must be less than 28 days (2419200 seconds)".into());
    }

    let time = (duration.0 * duration.1.to_seconds() * 1000) as i64;

    let stings = stings.unwrap_or(1);

    if stings < 0 {
        return Err("Stings must be greater than or equal to 0".into());
    }

    // Dispatch event to modules, erroring out if the dispatch errors (e.g. limits hit due to a lua template etc)
    let Some(author) = ctx.author_member().await else {
        return Err("This command can only be used in a guild".into());
    };

    let timeout_log_msg = to_log_format(&author.user, &member.user, &reason);

    let correlation_id = sqlx::types::Uuid::new_v4();
    let author_user_id = author.user.id;
    let target_user_id = member.user.id;
    let target_mention = member.user.mention();

    let results = AntiraidEvent::ModerationStart(ModerationStartEventData {
        correlation_id,
        reason: Some(reason.clone()),
        action: ModerationAction::Timeout {
            member,
            duration: (duration.0 * duration.1.to_seconds()),
        },
        author: match author {
            std::borrow::Cow::Borrowed(member) => member.clone(),
            std::borrow::Cow::Owned(member) => member,
        },
        num_stings: stings,
    })
    .dispatch_to_template_worker_and_wait(
        &data,
        guild_id,
        &template_dispatch_data(),
        Duration::from_secs(1),
    )
    .await?;

    if !results.can_execute() {
        // Fallback to simple hierarchy check
        check_hierarchy(&ctx, target_user_id).await?;
    }

    let mut embed = CreateEmbed::new()
        .title("Timing out Member...")
        .description(format!(
            "{} | Timing out {}",
            get_icon_of_state("pending"),
            target_mention
        ));

    let mut base_message = ctx
        .send(CreateReply::new().embed(embed))
        .await?
        .into_message()
        .await?;

    let mut tx = data.pool.begin().await?;

    let mut sting_dispatch = None;

    if stings > 0 {
        sting_dispatch = Some(
            StingCreate {
                src: Some("moderation:timeout".to_string()),
                stings,
                reason: Some(reason.clone()),
                void_reason: None,
                guild_id,
                creator: StingTarget::User(author_user_id),
                target: StingTarget::User(target_user_id),
                state: StingState::Active,
                duration: Some(std::time::Duration::from_secs(
                    duration.0 * duration.1.to_seconds(),
                )),
                sting_data: None,
            }
            .create_without_dispatch(&mut *tx)
            .await?,
        );
    }

    // Create new punishment
    let p = PunishmentCreate {
        src: Some("timeout".to_string()),
        guild_id,
        punishment: "timeout".to_string(),
        creator: PunishmentTarget::User(author_user_id),
        target: PunishmentTarget::User(target_user_id),
        handle_log: serde_json::json!({}),
        duration: Some(std::time::Duration::from_secs(
            duration.0 * duration.1.to_seconds(),
        )),
        reason: reason.clone(),
        data: None,
        state: PunishmentState::Active,
    }
    .create_without_dispatch(&mut *tx)
    .await?;

    guild_id
        .edit_member(
            ctx.http(),
            target_user_id,
            EditMember::new()
                .disable_communication_until(Timestamp::from_millis(
                    Timestamp::now().unix_timestamp() * 1000 + time,
                )?)
                .audit_log_reason(&timeout_log_msg),
        )
        .await?;

    tx.commit().await?;

    p.dispatch_event(ctx.serenity_context().clone(), &template_dispatch_data())
        .await?;
    if let Some(sting_dispatch) = sting_dispatch {
        sting_dispatch
            .dispatch_create_event(ctx.serenity_context().clone(), &template_dispatch_data())
            .await?;
    };

    AntiraidEvent::ModerationEnd(ModerationEndEventData { correlation_id })
        .dispatch_to_template_worker_and_nowait(&data, guild_id, &template_dispatch_data())
        .await?;

    embed = CreateEmbed::new()
        .title("Timed Out Member...")
        .description(format!(
            "{} | Timing out {}",
            get_icon_of_state("completed"),
            target_mention
        ));

    base_message
        .edit(&ctx.http(), EditMessage::new().embed(embed))
        .await?;

    Ok(())
}
