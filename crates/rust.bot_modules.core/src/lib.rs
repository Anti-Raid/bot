mod commands;
mod help;
mod modules;
mod ping;
mod settings;
mod stats;
mod web;
mod whois;

use indexmap::indexmap;

pub struct Module;

#[async_trait::async_trait]
impl ::modules::Module for Module {
    fn id(&self) -> &'static str {
        "core"
    }

    fn name(&self) -> &'static str {
        "Core"
    }

    fn description(&self) -> &'static str {
        "Core module handling pretty much all core functionality"
    }

    fn toggleable(&self) -> bool {
        false
    }

    fn commands_toggleable(&self) -> bool {
        true
    }

    fn is_default_enabled(&self) -> bool {
        true
    }

    fn raw_commands(&self) -> Vec<::modules::modules::CommandObj> {
        vec![
            (
                help::help(),
                ::modules::types::CommandExtendedData::none_map(),
            ),
            (
                stats::stats(),
                ::modules::types::CommandExtendedData::none_map(),
            ),
            (
                ping::ping(),
                ::modules::types::CommandExtendedData::none_map(),
            ),
            (
                whois::whois(),
                ::modules::types::CommandExtendedData::none_map(),
            ),
            (
                modules::modules(),
                indexmap! {
                    "" => ::modules::types::CommandExtendedData::kittycat_or_admin("modules", "*"),
                    "list" => ::modules::types::CommandExtendedData::kittycat_or_admin("modules", "list"),
                    "enable" => ::modules::types::CommandExtendedData::kittycat_or_admin("modules", "enable"),
                    "disable" => ::modules::types::CommandExtendedData::kittycat_or_admin("modules", "disable"),
                },
            ),
            (
                commands::commands(),
                indexmap! {
                    "check" => ::modules::types::CommandExtendedData::kittycat_or_admin("commands", "check"),
                    "enable" => ::modules::types::CommandExtendedData::kittycat_or_admin("commands", "enable"),
                    "disable" => ::modules::types::CommandExtendedData::kittycat_or_admin("commands", "disable"),
                },
            ),
            (
                web::web(),
                indexmap! {
                    "use" => ::modules::types::CommandExtendedData {
                        virtual_command: true,
                        ..::modules::types::CommandExtendedData::kittycat_or_admin("web", "use")
                    },
                },
            ),
        ]
    }

    fn config_options(&self) -> Vec<ar_settings::types::Setting> {
        vec![
            (*settings::GUILD_ROLES).clone(),
            (*settings::GUILD_MEMBERS).clone(),
            (*settings::GUILD_TEMPLATES).clone(),
            (*settings::GUILD_TEMPLATES_KV).clone(),
            (*settings::GUILD_TEMPLATE_SHOP).clone(),
            (*settings::GUILD_TEMPLATE_SHOP_PUBLIC_LIST).clone(),
        ]
    }
}
