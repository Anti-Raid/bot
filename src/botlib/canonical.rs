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

impl From<crate::Command> for CanonicalCommand {
    fn from(cmd: crate::Command) -> Self {
        CanonicalCommand::from(&cmd)
    }
}
