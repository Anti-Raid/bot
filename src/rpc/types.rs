use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::all::{GuildChannel, Permissions, Role, RoleId};

use crate::botlib::settings::SettingsData;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GuildChannelWithPermissions {
    pub user: Permissions,
    pub bot: Permissions,
    pub channel: GuildChannel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BaseGuildUserInfo {
    pub owner_id: String,
    pub name: String,
    pub icon: Option<String>,
    /// List of all roles in the server
    pub roles: Vec<Role>,
    /// List of roles the user has
    pub user_roles: Vec<RoleId>,
    /// List of roles the bot has
    pub bot_roles: Vec<RoleId>,
    /// List of all channels in the server
    pub channels: Vec<GuildChannelWithPermissions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckCommandPermission {
    pub result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
/// Given a guild id, a user id and a command name, check if the user has permission to run the command
pub struct CheckCommandPermissionRequest {
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CanonicalSettingsResult {
    Ok {
        fields: Vec<indexmap::IndexMap<String, Value>>,
    },
    Err {
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsOperationRequest {
    pub fields: indexmap::IndexMap<String, Value>,
    pub op: ar_settings::types::OperationType,
    pub setting: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteTemplateRequest {
    pub args: serde_json::Value,
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecuteTemplateResponse {
    Ok { result: Option<serde_json::Value> },
    ExecErr { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckUserHasKittycatPermissionsRequest {
    pub perm: String,
}

#[derive(Serialize, Deserialize)]
pub struct BotState {
    pub commands: Vec<crate::botlib::canonical::CanonicalCommand>,
    pub settings: Vec<ar_settings::types::Setting<SettingsData>>,
    pub command_permissions: crate::botlib::CommandPermissionMetadata,
}
