/// Canonical representation of a module for external use
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct CanonicalModule {
    /// The ID of the module
    pub id: String,

    /// The name of the module
    pub name: String,

    /// The description of the module
    pub description: String,

    /// Whether or not the module should be visible on the websites command lists
    pub web_hidden: bool,

    /// Whether or the module can be enabled and/or disabled
    pub toggleable: bool,

    /// Whether or not individual commands in the module can be configured
    pub commands_toggleable: bool,

    /// Virtual module. These modules allow controlling certain functionality of the bot without being loaded into the actual bot
    pub virtual_module: bool,

    /// Whether the module is enabled or disabled by default
    pub is_default_enabled: bool,

    /// The commands in the module
    pub commands: Vec<CanonicalCommand>,

    /// Modules may store files on seaweed, in order to allow for usage tracking,
    /// s3_paths should be set to the paths of the files on seaweed
    pub s3_paths: Vec<String>,

    /// Config options for this module
    pub config_options: Vec<ar_settings::types::Setting>,
}

/// Canonical representation of a command argument for external use
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct CanonicalCommandArgument {
    /// The name of the argument
    pub name: String,

    /// The description of the argument
    pub description: Option<String>,

    /// Whether or not the argument is required
    pub required: bool,

    /// The choices available for the argument
    pub choices: Vec<String>,
}

/// Canonical representation of a command (data section) for external use
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct CanonicalCommand {
    /// The name of the command
    pub name: String,

    /// The qualified name of the command
    pub qualified_name: String,

    /// The description of the command
    pub description: Option<String>,

    /// NSFW status
    pub nsfw: bool,

    /// The subcommands of the command
    pub subcommands: Vec<CanonicalCommand>,

    /// Whether or not a subcommand is required or not
    pub subcommand_required: bool,

    /// The arguments of the command
    pub arguments: Vec<CanonicalCommandArgument>,
}

/// Given command data, return its canonical representation
impl From<&crate::Command> for CanonicalCommand {
    fn from(cmd: &crate::Command) -> Self {
        CanonicalCommand {
            name: cmd.name.to_string(),
            qualified_name: cmd.qualified_name.to_string(),
            description: cmd.description.as_ref().map(|x| x.to_string()),
            nsfw: cmd.nsfw_only,
            subcommands: cmd.subcommands.iter().map(CanonicalCommand::from).collect(),
            subcommand_required: cmd.subcommand_required,
            arguments: cmd
                .parameters
                .iter()
                .map(|arg| CanonicalCommandArgument {
                    name: arg.name.to_string(),
                    description: arg.description.as_ref().map(|x| x.to_string()),
                    required: arg.required,
                    choices: arg
                        .choices
                        .iter()
                        .map(|choice| choice.name.to_string())
                        .collect(),
                })
                .collect(),
        }
    }
}

/// Given a module, return its canonical representation
impl From<&dyn crate::modules::Module> for CanonicalModule {
    fn from(module: &dyn crate::modules::Module) -> Self {
        CanonicalModule {
            id: module.id().to_string(),
            name: module.name().to_string(),
            description: module.description().to_string(),
            toggleable: module.toggleable(),
            commands_toggleable: module.commands_toggleable(),
            virtual_module: module.virtual_module(),
            web_hidden: module.web_hidden(),
            is_default_enabled: module.is_default_enabled(),
            commands: module
                .raw_commands()
                .iter()
                .map(|(c, _)| CanonicalCommand::from(c))
                .collect(),
            s3_paths: module.s3_paths().clone(),
            config_options: module.config_options(),
        }
    }
}

/// Allow &Arc<dyn Module> to be converted to CanonicalModule
impl From<&std::sync::Arc<dyn crate::modules::Module>> for CanonicalModule {
    fn from(module: &std::sync::Arc<dyn crate::modules::Module>) -> Self {
        CanonicalModule::from(&**module)
    }
}
