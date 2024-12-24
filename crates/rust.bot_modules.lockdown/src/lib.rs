pub mod cmds;
pub mod settings;

pub struct Module;

impl ::modules::modules::Module for Module {
    fn id(&self) -> &'static str {
        "lockdown"
    }

    fn name(&self) -> &'static str {
        "Lockdown"
    }

    fn description(&self) -> &'static str {
        "Lockdown module for quickly locking/unlocking your whole server or individual channels"
    }

    fn config_options(&self) -> Vec<ar_settings::types::Setting> {
        vec![
            (*settings::LOCKDOWN_SETTINGS).clone(),
            (*settings::LOCKDOWNS).clone(),
        ]
    }

    fn raw_commands(&self) -> Vec<(modules::Command, modules::modules::PermissionCheck)> {
        vec![(
            cmds::lockdowns(),
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
        )]
    }
}
