pub type CommandObj = (crate::Command, crate::CommandExtendedDataMap);

/// The `Module` trait can be used to create/define modules that run on Anti-Raid
///
/// A trait is used here to avoid a ton of complicated BoxFuture's, make Default handling more explicit and customizable and makes creating new Modules easier
pub trait Module: Send + Sync {
    /// The ID of the module
    fn id(&self) -> &'static str;

    /// The name of the module
    fn name(&self) -> &'static str;

    /// The description of the module
    fn description(&self) -> &'static str;

    /// Whether or not the module should be visible on the websites command lists
    fn web_hidden(&self) -> bool {
        false
    }

    /// Whether or the module can be enabled and/or disabled
    fn toggleable(&self) -> bool {
        true
    }

    /// Whether or not individual commands in the module can be toggled
    fn commands_toggleable(&self) -> bool {
        true
    }

    /// Virtual module. These modules allow controlling functionality of the bot without having its commands loaded into the bot
    ///
    /// Note that commands on a virtual module must also be virtual as well
    fn virtual_module(&self) -> bool {
        false
    }

    /// Whether the module is enabled or disabled by default
    fn is_default_enabled(&self) -> bool {
        false // Don't enable new modules by default unless modules explicitly opt in to this behavior
    }

    /// The commands in the module
    fn raw_commands(&self) -> Vec<CommandObj> {
        Vec::new()
    }

    /// Modules may store files on seaweed, in order to allow for usage tracking,
    /// s3_paths should be set to the paths of the files on seaweed
    fn s3_paths(&self) -> Vec<String> {
        Vec::new()
    }

    /// Config options for this module
    fn config_options(&self) -> Vec<ar_settings::types::Setting> {
        Vec::new()
    }

    /// Performs any sanity/validation checks on the module
    ///
    /// Should not be overrided by modules unless absolutely necessary
    fn validate(&self) -> Result<(), crate::Error> {
        validate_module(self)
    }
}

/// Validates a module to ensure it is set up correctly
pub fn validate_module<T: Module + ?Sized>(module: &T) -> Result<(), crate::Error> {
    let commands = module.raw_commands();

    // If virtual module, all commands must also be virtual, if root command is virtual, all subcommands must be virtual
    for command in commands.iter() {
        let root_is_virtual = {
            match command.1.get("") {
                Some(root) => root.virtual_command,
                None => false,
            }
        };
        for (sub_name, extended_data) in command.1.iter() {
            if module.virtual_module() && !extended_data.virtual_command {
                return Err(format!(
                    "Module {} is a virtual module, but has a non-virtual command {}",
                    module.id(),
                    command.0.name
                )
                .into());
            }

            if root_is_virtual && !extended_data.virtual_command {
                return Err(format!(
                    "Module {} has a virtual root command, but a non-virtual subcommand {} {}",
                    module.id(),
                    command.0.name,
                    sub_name
                )
                .into());
            }
        }
    }

    // Check: Ensure all command extended data's have valid subcommands listed
    for (command, extended_data) in commands.iter() {
        let mut listed_subcommands = Vec::new();
        let mut actual_subcommands = Vec::new();

        for (subcommand, _) in extended_data.iter() {
            listed_subcommands.push(subcommand.to_string());
        }

        for subcommand in &command.subcommands {
            actual_subcommands.push(subcommand.name.clone());
        }

        // We don't care about omission of "" (rootcmd) here
        if !listed_subcommands.contains(&"".to_string()) {
            listed_subcommands.insert(0, "".to_string());
        }

        if !actual_subcommands.contains(&"".to_string().into()) {
            actual_subcommands.insert(0, "".to_string().into());
        }

        if listed_subcommands != actual_subcommands {
            return Err(
                format!(
                    "Module {} has a command {} with subcommands that do not match the actual subcommands [{} != {}]",
                    module.id(),
                    command.name,
                    listed_subcommands.join(", "),
                    actual_subcommands.join(", ")
                ).into()
            );
        }
    }

    // Check that all config_opts have unique ids
    let mut config_ids = Vec::new();

    for config_opt in &module.config_options() {
        if config_ids.contains(&config_opt.id) {
            panic!(
                "Module {} has a duplicate config option id: {}",
                module.id(),
                config_opt.id
            );
        }

        config_ids.push(config_opt.id.clone());
    }

    Ok(())
}
