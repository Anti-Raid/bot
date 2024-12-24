pub type PermissionCheck = fn(
    &str,
    serenity::all::UserId,
    serenity::all::Permissions,
    Vec<kittycat::perms::Permission>,
) -> Result<(), crate::Error>;

pub fn permission_check_none(
    _command: &str,
    _user_id: serenity::all::UserId,
    _native_perms: serenity::all::Permissions,
    _kittycat_perms: Vec<kittycat::perms::Permission>,
) -> Result<(), crate::Error> {
    Ok(())
}

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
    fn raw_commands(&self) -> Vec<(crate::Command, PermissionCheck)> {
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
