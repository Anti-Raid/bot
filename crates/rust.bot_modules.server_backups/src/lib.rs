mod cmds;

pub struct Module;

impl modules::modules::Module for Module {
    fn id(&self) -> &'static str {
        "server_backups"
    }

    fn name(&self) -> &'static str {
        "Server Backups"
    }

    fn description(&self) -> &'static str {
        "Customizable advanced server backup system for your server"
    }

    fn is_default_enabled(&self) -> bool {
        true
    }

    fn raw_commands(&self) -> Vec<(modules::Command, modules::modules::PermissionCheck)> {
        vec![(
            cmds::backups(),
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
        )]
    }
}
