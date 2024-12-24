mod help;
mod ping;
mod settings;
mod stats;
mod whois;

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

    fn raw_commands(&self) -> Vec<(modules::Command, modules::modules::PermissionCheck)> {
        vec![
            (help::help(), modules::modules::permission_check_none),
            (stats::stats(), modules::modules::permission_check_none),
            (ping::ping(), modules::modules::permission_check_none),
            (whois::whois(), modules::modules::permission_check_none),
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
