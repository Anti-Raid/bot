use crate::{
    canonical::CanonicalModule,
    modules::{Module, PermissionCheck},
};
use std::sync::Arc;

/// The compiler requires some help here with module cache so we use a wrapper struct
pub struct ModuleCacheEntry(pub Arc<dyn Module>);

impl Clone for ModuleCacheEntry {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl std::ops::Deref for ModuleCacheEntry {
    type Target = Arc<dyn Module>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The module cache is a structure that contains the core state for the bots modules
pub struct ModuleCache {
    /// A commonly needed operation is mapping a module id to its respective module
    ///
    /// module_cache is a cache of module id to module
    ///
    /// We use indexmap here to avoid the 'static restriction
    pub module_cache: dashmap::DashMap<String, ModuleCacheEntry>,

    /// Command ID to module map
    pub command_id_module_map: dashmap::DashMap<String, String>,

    /// Command ID to permission check map
    pub command_id_permission_check_map: dashmap::DashMap<String, PermissionCheck>,

    /// Cache of the canonical forms of all modules
    pub canonical_module_cache: dashmap::DashMap<String, CanonicalModule>,

    /// Cache of all known settings
    pub settings_cache: dashmap::DashMap<String, ar_settings::types::Setting>,
}

impl Default for ModuleCache {
    fn default() -> Self {
        Self {
            module_cache: dashmap::DashMap::new(),
            command_id_module_map: dashmap::DashMap::new(),
            command_id_permission_check_map: dashmap::DashMap::new(),
            canonical_module_cache: dashmap::DashMap::new(),
            settings_cache: dashmap::DashMap::new(),
        }
    }
}

impl ModuleCache {
    pub fn add_module(&mut self, module: Box<dyn Module>) {
        // Try validating the module first before adding it
        match module.validate() {
            Ok(_) => {}
            Err(e) => {
                panic!("ModuleCache::add_module - Failed to validate module: {}", e);
            }
        }

        let module: Arc<dyn Module> = module.into();

        // Add the settings to cache
        for setting in module.config_options() {
            self.settings_cache
                .insert(setting.id.clone(), setting.clone());
        }

        // Add commands to map
        for (command, f) in module.raw_commands() {
            self.command_id_module_map
                .insert(command.name.to_string(), module.id().to_string());

            self.command_id_permission_check_map
                .insert(command.name.to_string(), f);
        }

        // Add to canonical cache
        self.canonical_module_cache
            .insert(module.id().to_string(), CanonicalModule::from(&module));

        // Add the module to cache
        self.module_cache
            .insert(module.id().to_string(), ModuleCacheEntry(module));
    }

    pub fn remove_module(&mut self, module_id: &str) {
        if let Some((_, module)) = self.module_cache.remove(module_id) {
            for setting in module.config_options() {
                self.settings_cache.remove(&setting.id);
            }

            for (command, _) in module.raw_commands() {
                self.command_id_module_map.remove(&command.name.to_string());
                self.command_id_permission_check_map
                    .remove(&command.name.to_string());
            }

            self.canonical_module_cache.remove(module_id);
        }
    }
}
