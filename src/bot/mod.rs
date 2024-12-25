mod backups;
mod help;
mod lockdowns;
mod moderation;
mod ping;
mod settings;
mod stats;
mod whois;

pub fn raw_commands() -> Vec<(
    crate::Command,
    crate::botlib::permission_checks::PermissionCheck,
    crate::botlib::CommandPermissionMetadata,
)> {
    vec![
        (
            help::help(),
            crate::botlib::permission_checks::permission_check_none,
            indexmap::indexmap! {},
        ),
        (
            stats::stats(),
            crate::botlib::permission_checks::permission_check_none,
            indexmap::indexmap! {},
        ),
        (
            ping::ping(),
            crate::botlib::permission_checks::permission_check_none,
            indexmap::indexmap! {},
        ),
        (
            whois::whois(),
            crate::botlib::permission_checks::permission_check_none,
            indexmap::indexmap! {},
        ),
        (
            moderation::moderation(),
            |command, _user_id, native_perms, kittycat_perms| {
                if native_perms.administrator() {
                    return Ok(());
                }

                match command {
                        "moderation prune" => {
                            if !native_perms.contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES) && !kittycat::perms::has_perm(&kittycat_perms, &"moderation.prune".to_string().into()) {
                                return Err("Missing required permission: MANAGE_MESSAGES or moderation.prune".into());
                            }

                            Ok(())
                        },
                        "moderation kick" => {
                            if !native_perms.contains(serenity::model::permissions::Permissions::KICK_MEMBERS) && !kittycat::perms::has_perm(&kittycat_perms, &"moderation.kick".to_string().into()) {
                                return Err("Missing required permission: KICK_MEMBERS or moderation.kick".into());
                            }

                            Ok(())
                        },
                        "moderation ban" => {
                            if !native_perms.contains(serenity::model::permissions::Permissions::BAN_MEMBERS) && !kittycat::perms::has_perm(&kittycat_perms, &"moderation.ban".to_string().into()) {
                                return Err("Missing required permission: BAN_MEMBERS or moderation.ban".into());
                            }

                            Ok(())
                        },
                        "moderation tempban" => {
                            if !native_perms.contains(serenity::model::permissions::Permissions::BAN_MEMBERS) && !kittycat::perms::has_perm(&kittycat_perms, &"moderation.tempban".to_string().into()) {
                                return Err("Missing required permission: BAN_MEMBERS or moderation.tempban".into());
                            }

                            Ok(())
                        },
                        "moderation unban" => {
                            if !native_perms.contains(serenity::model::permissions::Permissions::BAN_MEMBERS) && !kittycat::perms::has_perm(&kittycat_perms, &"moderation.unban".to_string().into()) {
                                return Err("Missing required permission: BAN_MEMBERS or moderation.unban".into());
                            }

                            Ok(())
                        },
                        "moderation timeout" => {
                            if !native_perms.contains(serenity::model::permissions::Permissions::MODERATE_MEMBERS) && !kittycat::perms::has_perm(&kittycat_perms, &"moderation.timeout".to_string().into()) {
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
            |command, _user_id, native_perms, kittycat_perms| {
                match command {
                    "lockdowns list" => {
                        if !native_perms.contains(serenity::model::permissions::Permissions::MANAGE_GUILD) && !kittycat::perms::has_perm(&kittycat_perms, &"lockdowns.list".to_string().into()) {
                            return Err("Missing required permission: MANAGE_GUILD or lockdowns.list".into());
                        }

                        Ok(())
                    },
                    "lockdowns tsl" => {
                        if !kittycat::perms::has_perm(&kittycat_perms, &"lockdowns.tsl".to_string().into()) {
                            return Err("Missing required permission: lockdowns.tsl".into());
                        }

                        Ok(())
                    },
                    "lockdowns qsl" => {
                        if !kittycat::perms::has_perm(&kittycat_perms, &"lockdowns.qsl".to_string().into()) {
                            return Err("Missing required permission: lockdowns.qsl".into());
                        }

                        Ok(())
                    },
                    "lockdowns scl" => {
                        if !kittycat::perms::has_perm(&kittycat_perms, &"lockdowns.scl".to_string().into()) {
                            return Err("Missing required permission: lockdowns.scl".into());
                        }

                        Ok(())
                    },
                    "lockdowns role" => {
                        if !kittycat::perms::has_perm(&kittycat_perms, &"lockdowns.role".to_string().into()) {
                            return Err("Missing required permission: lockdowns.role".into());
                        }

                        Ok(())
                    },
                    "lockdowns remove" => {
                        if !kittycat::perms::has_perm(&kittycat_perms, &"lockdowns.remove".to_string().into()) {
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
        ((
            backups::backups(),
            |command, _user_id, native_perms, kittycat_perms| {
                match command {
                    "backups create" => {
                        if !native_perms.contains(serenity::model::permissions::Permissions::MANAGE_GUILD) && !kittycat::perms::has_perm(&kittycat_perms, &"backups.create".to_string().into()) {
                            return Err("Missing required permission: MANAGE_GUILD or backups.create".into());
                        }

                        Ok(())
                    },
                    "backups list" => {
                        if !native_perms.contains(serenity::model::permissions::Permissions::MANAGE_GUILD) && !kittycat::perms::has_perm(&kittycat_perms, &"backups.list".to_string().into()) {
                            return Err("Missing required permission: MANAGE_GUILD or backups.list".into());
                        }

                        Ok(())
                    },
                    "backups delete" => {
                        if !native_perms.contains(serenity::model::permissions::Permissions::MANAGE_GUILD) && !kittycat::perms::has_perm(&kittycat_perms, &"backups.delete".to_string().into()) {
                            return Err("Missing required permission: MANAGE_GUILD or backups.delete".into());
                        }

                        Ok(())
                    },
                    "backups restore" => {
                        if !native_perms.contains(serenity::model::permissions::Permissions::ADMINISTRATOR) && !kittycat::perms::has_perm(&kittycat_perms, &"backups.restore".to_string().into()) {
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
        )),
    ]
}

pub fn config_options() -> Vec<ar_settings::types::Setting> {
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
