mod cmd;

pub struct Module;

impl modules::modules::Module for Module {
    fn id(&self) -> &'static str {
        "moderation"
    }

    fn name(&self) -> &'static str {
        "Moderation"
    }

    fn description(&self) -> &'static str {
        "Simple yet customizable moderation plugin for your server."
    }

    fn is_default_enabled(&self) -> bool {
        true
    }

    fn raw_commands(&self) -> Vec<(modules::Command, modules::modules::PermissionCheck)> {
        vec![(
            cmd::moderation(),
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
        )]
    }
}
