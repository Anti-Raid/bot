mod cmd;

use indexmap::indexmap;
use modules::types::CommandExtendedData;
use permissions::types::PermissionCheck;

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

    fn raw_commands(&self) -> Vec<modules::modules::CommandObj> {
        vec![
            (
                cmd::prune(),
                indexmap! {
                    "" => CommandExtendedData {
                        default_perms: PermissionCheck {
                            kittycat_perms: vec!["moderation.prune".to_string()],
                            native_perms: vec![serenity::model::permissions::Permissions::MANAGE_MESSAGES, serenity::model::permissions::Permissions::MANAGE_GUILD],
                            inner_and: true,
                        },
                        ..Default::default()
                    },
                },
            ),
            (
                cmd::kick(),
                indexmap! {
                    "" => CommandExtendedData {
                        default_perms: PermissionCheck {
                            kittycat_perms: vec!["moderation.kick".to_string()],
                            native_perms: vec![serenity::model::permissions::Permissions::KICK_MEMBERS],
                            inner_and: false,
                        },
                        ..Default::default()
                    },
                },
            ),
            (
                cmd::ban(),
                indexmap! {
                    "" => CommandExtendedData {
                        default_perms: PermissionCheck {
                            kittycat_perms: vec!["moderation.ban".to_string()],
                            native_perms: vec![serenity::model::permissions::Permissions::BAN_MEMBERS],
                            inner_and: false,
                        },
                        ..Default::default()
                    },
                },
            ),
            (
                cmd::tempban(),
                indexmap! {
                    "" => CommandExtendedData {
                        default_perms: PermissionCheck {
                            kittycat_perms: vec!["moderation.tempban".to_string()],
                            native_perms: vec![serenity::model::permissions::Permissions::BAN_MEMBERS],
                            inner_and: false,
                        },
                        ..Default::default()
                    },
                },
            ),
            (
                cmd::unban(),
                indexmap! {
                    "" => CommandExtendedData {
                        default_perms: PermissionCheck {
                            kittycat_perms: vec!["moderation.unban".to_string()],
                            native_perms: vec![serenity::model::permissions::Permissions::BAN_MEMBERS],
                            inner_and: false,
                        },
                        ..Default::default()
                    },
                },
            ),
            (
                cmd::timeout(),
                indexmap! {
                    "" => CommandExtendedData {
                        default_perms: PermissionCheck {
                            kittycat_perms: vec!["moderation.timeout".to_string()],
                            native_perms: vec![serenity::model::permissions::Permissions::MODERATE_MEMBERS],
                            inner_and: false,
                        },
                        ..Default::default()
                    },
                },
            ),
        ]
    }
}
