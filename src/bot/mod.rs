use std::vec;

use crate::{botlib::settings::SettingsData, config::CONFIG};

mod backups;
mod help;
mod load;
mod lockdowns;
mod moderation;
mod ping;
mod settings;
mod stats;
mod whois;

pub fn command_permissions_metadata() -> indexmap::IndexMap<String, Vec<String>> {
    indexmap::indexmap! {
        "moderation prune".to_string() => vec!["moderation.prune".to_string()],
        "moderation kick".to_string() => vec!["moderation.kick".to_string()],
        "moderation ban".to_string() => vec!["moderation.ban".to_string()],
        "moderation tempban".to_string() => vec!["moderation.tempban".to_string()],
        "moderation unban".to_string() => vec!["moderation.unban".to_string()],
        "moderation timeout".to_string() => vec!["moderation.timeout".to_string()],
        "lockdowns list".to_string() => vec!["lockdowns.list".to_string()],
        "lockdowns tsl".to_string() => vec!["lockdowns.tsl".to_string()],
        "lockdowns qsl".to_string() => vec!["lockdowns.qsl".to_string()],
        "lockdowns scl".to_string() => vec!["lockdowns.scl".to_string()],
        "lockdowns role".to_string() => vec!["lockdowns.role".to_string()],
        "lockdowns remove".to_string() => vec!["lockdowns.remove".to_string()],
        "backups create".to_string() => vec!["backups.create".to_string()],
        "backups list".to_string() => vec!["backups.list".to_string()],
        "backups delete".to_string() => vec!["backups.delete".to_string()],
        "backups restore".to_string() => vec!["backups.restore".to_string()],
        "load".to_string() => vec!["bot.load".to_string()],
    }
}

pub fn raw_commands() -> Vec<crate::Command> {
    vec![
        help::help(),
        stats::stats(),
        ping::ping(),
        whois::whois(),
        moderation::moderation(),
        lockdowns::lockdowns(),
        backups::backups(),
        load::load(),
    ]

    /*vec![
        (
            moderation::moderation(),
            |command, _user_id, user_info| {
                if user_info.discord_permissions.administrator() {
                    return Ok(());
                }

                match command {
                        "moderation prune" => {
                            if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"moderation.prune".to_string().into()) {
                                return Err("Missing required permission: MANAGE_MESSAGES or moderation.prune".into());
                            }

                            Ok(())
                        },
                        "moderation kick" => {
                            if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::KICK_MEMBERS) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"moderation.kick".to_string().into()) {
                                return Err("Missing required permission: KICK_MEMBERS or moderation.kick".into());
                            }

                            Ok(())
                        },
                        "moderation ban" => {
                            if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::BAN_MEMBERS) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"moderation.ban".to_string().into()) {
                                return Err("Missing required permission: BAN_MEMBERS or moderation.ban".into());
                            }

                            Ok(())
                        },
                        "moderation tempban" => {
                            if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::BAN_MEMBERS) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"moderation.tempban".to_string().into()) {
                                return Err("Missing required permission: BAN_MEMBERS or moderation.tempban".into());
                            }

                            Ok(())
                        },
                        "moderation unban" => {
                            if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::BAN_MEMBERS) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"moderation.unban".to_string().into()) {
                                return Err("Missing required permission: BAN_MEMBERS or moderation.unban".into());
                            }

                            Ok(())
                        },
                        "moderation timeout" => {
                            if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::MODERATE_MEMBERS) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"moderation.timeout".to_string().into()) {
                                return Err("Missing required permission: MODERATE_MEMBERS or moderation.timeout".into());
                            }

                            Ok(())
                        },
                        _ => Err("Internal Error: No permissions needed found for command. Please contact support".into()),
                    }
            },
            indexmap::indexmap! {
                "moderation prune".to_string() => vec!["MANAGE_MESSAGES (Discord) *OR* moderation.prune (Kittycat)".to_string()],
                "moderation kick".to_string() => vec!["KICK_MEMBERS (Discord) *OR* moderation.kick (Kittycat)".to_string()],
                "moderation ban".to_string() => vec!["BAN_MEMBERS (Discord) *OR* moderation.ban (Kittycat)".to_string()],
                "moderation tempban".to_string() => vec!["BAN_MEMBERS (Discord) *OR* moderation.tempban (Kittycat)".to_string()],
                "moderation unban".to_string() => vec!["BAN_MEMBERS (Discord) *OR* moderation.unban (Kittycat)".to_string()],
                "moderation timeout".to_string() => vec!["MODERATE_MEMBERS (Discord) *OR* moderation.timeout (Kittycat)".to_string()],
            },
        ),
        (
            lockdowns::lockdowns(),
            |command, _user_id, user_info| {
                match command {
                    "lockdowns list" => {
                        if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::MANAGE_GUILD) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"lockdowns.list".to_string().into()) {
                            return Err("Missing required permission: MANAGE_GUILD or lockdowns.list".into());
                        }

                        Ok(())
                    },
                    "lockdowns tsl" => {
                        if !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"lockdowns.tsl".to_string().into()) {
                            return Err("Missing required permission: lockdowns.tsl".into());
                        }

                        Ok(())
                    },
                    "lockdowns qsl" => {
                        if !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"lockdowns.qsl".to_string().into()) {
                            return Err("Missing required permission: lockdowns.qsl".into());
                        }

                        Ok(())
                    },
                    "lockdowns scl" => {
                        if !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"lockdowns.scl".to_string().into()) {
                            return Err("Missing required permission: lockdowns.scl".into());
                        }

                        Ok(())
                    },
                    "lockdowns role" => {
                        if !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"lockdowns.role".to_string().into()) {
                            return Err("Missing required permission: lockdowns.role".into());
                        }

                        Ok(())
                    },
                    "lockdowns remove" => {
                        if !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"lockdowns.remove".to_string().into()) {
                            return Err("Missing required permission: lockdowns.remove".into());
                        }

                        Ok(())
                    },
                    _ => Err("Internal Error: No permissions needed found for command. Please contact support".into()),
                }
            },
            indexmap::indexmap! {
                "lockdowns list".to_string() => vec!["MANAGE_GUILD (Discord) *OR* lockdowns.list (Kittycat)".to_string()],
                "lockdowns tsl".to_string() => vec!["lockdowns.tsl (Kittycat)".to_string()],
                "lockdowns qsl".to_string() => vec!["lockdowns.qsl (Kittycat)".to_string()],
                "lockdowns scl".to_string() => vec!["lockdowns.scl (Kittycat)".to_string()],
                "lockdowns role".to_string() => vec!["lockdowns.role (Kittycat)".to_string()],
                "lockdowns remove".to_string() => vec!["lockdowns.remove (Kittycat)".to_string()],
            },
        ),
        (
            backups::backups(),
            |command, _user_id, user_info| {
                match command {
                    "backups create" => {
                        if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::MANAGE_GUILD) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"backups.create".to_string().into()) {
                            return Err("Missing required permission: MANAGE_GUILD or backups.create".into());
                        }

                        Ok(())
                    },
                    "backups list" => {
                        if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::MANAGE_GUILD) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"backups.list".to_string().into()) {
                            return Err("Missing required permission: MANAGE_GUILD or backups.list".into());
                        }

                        Ok(())
                    },
                    "backups delete" => {
                        if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::MANAGE_GUILD) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"backups.delete".to_string().into()) {
                            return Err("Missing required permission: MANAGE_GUILD or backups.delete".into());
                        }

                        Ok(())
                    },
                    "backups restore" => {
                        if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::ADMINISTRATOR) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"backups.restore".to_string().into()) {
                            return Err("Missing required permission: ADMINISTRATOR or backups.restore".into());
                        }

                        Ok(())
                    },
                    _ => Err("Internal Error: No permissions needed found for command. Please contact support".into()),
                }
            },
            indexmap::indexmap! {
                "backups create".to_string() => vec!["MANAGE_GUILD (Discord) *OR* backups.create (Kittycat)".to_string()],
                "backups list".to_string() => vec!["MANAGE_GUILD (Discord) *OR* backups.list (Kittycat)".to_string()],
                "backups delete".to_string() => vec!["MANAGE_GUILD (Discord) *OR* backups.delete (Kittycat)".to_string()],
                "backups restore".to_string() => vec!["ADMINISTRATOR (Discord) *OR* backups.restore (Kittycat)".to_string()],
            },
        ),
        (
            load::load(),
            |command, _user_id, user_info| {
                match command {
                    "load" => {
                        if !user_info.discord_permissions.contains(serenity::model::permissions::Permissions::MANAGE_GUILD) && !kittycat::perms::has_perm(&user_info.kittycat_resolved_permissions, &"bot.load".to_string().into()) {
                            return Err("Missing required permission: MANAGE_GUILD or bot.load".into());
                        }

                        Ok(())
                    },
                    _ => Err("Internal Error: No permissions needed found for command. Please contact support".into()),
                }
            },
            indexmap::indexmap! {
                "load".to_string() => vec!["MANAGE_GUILD (Discord) *OR* bot.load (Kittycat)".to_string()],
            },
        ),
    ]*/
}

pub fn config_options() -> Vec<ar_settings::types::Setting<SettingsData>> {
    vec![
        (*settings::GUILD_ROLES).clone(),
        (*settings::GUILD_MEMBERS).clone(),
        (*settings::GUILD_TEMPLATES).clone(),
        (*settings::GUILD_TEMPLATES_KV).clone(),
        (*settings::GUILD_TEMPLATE_SHOP).clone(),
        (*settings::GUILD_TEMPLATE_SHOP_PUBLIC_LIST).clone(),
        (*settings::LOCKDOWN_SETTINGS).clone(),
        (*settings::LOCKDOWNS).clone(),
    ]
}

/// Provides the config data involving template dispatch
pub(crate) fn template_dispatch_data() -> silverpelt::ar_event::DispatchEventData {
    silverpelt::ar_event::DispatchEventData {
        template_worker_addr: CONFIG.base_ports.template_worker_addr.as_str(),
        template_worker_port: CONFIG.base_ports.template_worker_port,
    }
}

/// Provides the config data involving kittycat permissions
pub(crate) fn kittycat_permission_config_data(
) -> silverpelt::member_permission_calc::GetKittycatPermsConfigData {
    silverpelt::member_permission_calc::GetKittycatPermsConfigData {
        main_server_id: CONFIG.servers.main,
        root_users: CONFIG.discord_auth.root_users.as_ref(),
    }
}

/// Provides the config data involving sandwich http api
pub(crate) fn sandwich_config() -> sandwich_driver::SandwichConfigData {
    sandwich_driver::SandwichConfigData {
        http_api: CONFIG.meta.sandwich_http_api.as_str(),
    }
}
