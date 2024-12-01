mod cmds;
use indexmap::indexmap;

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

    fn raw_commands(&self) -> Vec<modules::modules::CommandObj> {
        vec![(
            cmds::backups(),
            indexmap! {
                "" => modules::types::CommandExtendedData::kittycat_simple("server_backups", "*"),
                "create" => modules::types::CommandExtendedData::kittycat_or_admin("server_backups", "create"),
                "list" => modules::types::CommandExtendedData::kittycat_or_admin("server_backups", "list"),
                "delete" => modules::types::CommandExtendedData::kittycat_or_admin("server_backups", "delete"),
                "restore" => modules::types::CommandExtendedData::kittycat_or_admin("server_backups", "restore"),
            },
        )]
    }
}
