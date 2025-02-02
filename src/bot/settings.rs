use ar_settings::types::{
    settings_wrap, Column, ColumnSuggestion, ColumnType, InnerColumnType,
    OperationType, Setting, SettingOperations,
};
use ar_settings::types::{
    SettingCreator, SettingDeleter, SettingUpdater, SettingView,
};
use kittycat::perms::Permission;
use serde_json::Value;
use antiraid_types::ar_event::{AntiraidEvent, ExternalKeyUpdateEventData, ExternalKeyUpdateEventDataAction};
use silverpelt::ar_event::AntiraidEventOperations;
use std::sync::LazyLock;
use async_trait::async_trait;
use crate::botlib::settings::SettingsData;
use crate::Error;

async fn check_perms(
    ctx: &SettingsData,
    perm: kittycat::perms::Permission,
) -> Result<(), Error> {
    crate::botlib::permission_checks::member_has_kittycat_perm(
        ctx.guild_id,
        ctx.author,
        &ctx.data.pool,
        &ctx.serenity_context,
        &ctx.data.reqwest,
        &None,
        perm,
    )
    .await
    .map_err(|e| 
        format!("Failed to check permissions: {:?}", e).into()
    )
}

pub static GUILD_ROLES: LazyLock<Setting<SettingsData>> = LazyLock::new(|| {
    Setting {
        id: "guild_roles".to_string(),
        name: "Server Roles".to_string(),
        description: "Configure/setup server roles which can then have permissions on AntiRaid".to_string(),
        primary_key: "role_id".to_string(),
        columns: settings_wrap(vec![
            ar_settings::common_columns::guild_id("guild_id", "Guild ID", "The Guild ID"),
            Column {
                id: "role_id".to_string(),
                name: "Role ID".to_string(),
                description: "The role ID".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "role".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "perms".to_string(),
                name: "Permissions".to_string(),
                description: "What permissions should the role have".to_string(),
                column_type: ColumnType::new_array(InnerColumnType::String {
                    kind: "kittycat".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "index".to_string(),
                name: "Index".to_string(),
                description: "Where in the role hierarchy should this role be on Anti-Raid for permission purposes. Note that a lower index implies higher on the hierarchy and vice versa".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::Integer {}),
                nullable: true,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "display_name".to_string(),
                name: "Display Name".to_string(),
                description: "What should the role be displayed as in API's etc".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec![],
                }),
                nullable: true,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            ar_settings::common_columns::created_at(),
            ar_settings::common_columns::created_by(),
            ar_settings::common_columns::last_updated_at(),
            ar_settings::common_columns::last_updated_by(),
        ]),
        title_template: "{index} - {role_id}".to_string(),
        operations: SettingOperations::from(GuildRolesExecutor),
    }
});

#[derive(Clone)]
pub struct GuildRolesExecutor;

#[async_trait::async_trait]
impl SettingView<SettingsData> for GuildRolesExecutor {
    async fn view<'a>(
        &self,
        context: &SettingsData,
        _filters: indexmap::IndexMap<String, Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error> {
        let rows = sqlx::query!("SELECT role_id, perms, index, display_name, created_at, created_by, last_updated_at, last_updated_by FROM guild_roles WHERE guild_id = $1", context.guild_id.to_string())
        .fetch_all(&context.data.pool)
        .await
        .map_err(|e| format!("Failed to fetch roles: {:?}", e))?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "guild_id".to_string() => Value::String(context.guild_id.to_string()),
                "role_id".to_string() => Value::String(row.role_id),
                "perms".to_string() => Value::Array(row.perms.iter().map(|x| Value::String(x.to_string())).collect()),
                "index".to_string() => Value::Number(row.index.into()),
                "display_name".to_string() => row.display_name.map(Value::String).unwrap_or(Value::Null),
                "created_at".to_string() => Value::String(row.created_at.to_string()),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::String(row.last_updated_at.to_string()),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator<SettingsData> for GuildRolesExecutor {
    async fn create<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_roles.create".into()).await?;

        let res = self
            .base_verify_checks(ctx, &entry, OperationType::Create)
            .await?;

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_roles WHERE guild_id = $1 AND role_id = $2",
            ctx.guild_id.to_string(),
            res.role_id.to_string()
        )
        .fetch_one(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to check role counts from database: {:?}", e))?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err("Role already exists".into());
        }

        sqlx::query!(
            "INSERT INTO guild_roles (guild_id, role_id, perms, index, display_name, created_by, last_updated_by) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            ctx.guild_id.to_string(),
            res.role_id.to_string(),
            &res.perms,
            res.index,
            res.display_name,
            ctx.author.to_string(),
            ctx.author.to_string()
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to insert role: {:?}", e))?;

        sqlx::query!(
            "UPDATE guild_members SET needs_perm_rederive = true WHERE guild_id = $1 AND $2 = ANY(roles)",
            ctx.guild_id.to_string(),
            res.role_id.to_string()
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to update guild members cache: {:?}", e))?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater<SettingsData> for GuildRolesExecutor {
    async fn update<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_roles.update".into()).await?;

        let res = self
            .base_verify_checks(ctx, &entry, OperationType::Update)
            .await?;

        sqlx::query!(
            "UPDATE guild_roles SET perms = $1, index = $2, display_name = $3, last_updated_at = NOW(), last_updated_by = $4 WHERE guild_id = $5 AND role_id = $6",
            &res.perms,
            res.index,
            res.display_name,
            ctx.author.to_string(),
            ctx.guild_id.to_string(),
            res.role_id.to_string()
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to update role: {:?}", e))?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter<SettingsData> for GuildRolesExecutor {
    async fn delete<'a>(
        &self,
        ctx: &SettingsData,
        primary_key: Value,
    ) -> Result<(), Error> {
        check_perms(ctx, "guild_roles.delete".into()).await?;

        let Some(row) = sqlx::query!("SELECT role_id, perms, index, display_name FROM guild_roles WHERE guild_id = $1 AND role_id = $2", ctx.guild_id.to_string(), primary_key.to_string())
        .fetch_optional(&ctx.data.pool)
        .await
        .map_err(|e| format!("Error while fetching roles: {}", e))? else {
            return Err("Role not found when trying to delete it!".into());
        };

        let entry = indexmap::indexmap! {
            "guild_id".to_string() => Value::String(ctx.guild_id.to_string()),
            "role_id".to_string() => Value::String(row.role_id),
            "perms".to_string() => Value::Array(row.perms.iter().map(|x| Value::String(x.to_string())).collect()),
            "index".to_string() => Value::Number(row.index.into()),
            "display_name".to_string() => row.display_name.map(Value::String).unwrap_or(Value::Null),
        };

        let res = self
            .base_verify_checks(ctx, &entry, OperationType::Delete)
            .await?;

        sqlx::query!(
            "DELETE FROM guild_roles WHERE guild_id = $1 AND role_id = $2",
            ctx.guild_id.to_string(),
            res.role_id.to_string()
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to delete role: {:?}", e))?;

        sqlx::query!(
            "UPDATE guild_members SET needs_perm_rederive = true WHERE guild_id = $1 AND $2 = ANY(roles)",
            ctx.guild_id.to_string(),
            res.role_id.to_string()
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to update guild members cache: {:?}", e))?;

        Ok(())
    }
}

pub struct GreBaseVerifyChecksResult {
    pub role_id: serenity::all::RoleId,
    pub index: i32,
    pub perms: Vec<String>,
    pub display_name: Option<String>,
}

impl GuildRolesExecutor {
    async fn base_verify_checks(
        &self,
        ctx: &SettingsData,
        state: &indexmap::IndexMap<String, Value>,
        op: OperationType,
    ) -> Result<GreBaseVerifyChecksResult, Error> {
        let parsed_value = if let Some(new_index_val) = state.get("index") {
            match new_index_val {
                Value::Number(new_index) => Value::Number(new_index.clone()),
                Value::Null => Value::Null,
                _ => {
                    return Err(format!("Failed to retrieve valid `index`: {:?}", new_index_val).into());
                }
            }
        } else {
            Value::Null
        };

        // Get the index to set to
        let new_index = match parsed_value {
            Value::Number(new_index_val) => {
                let int64 = new_index_val
                    .as_i64()
                    .ok_or_else(|| format!("Failed to parse index as i64: {:?}", new_index_val))?;

                if int64 < i32::MIN as i64 || int64 > i32::MAX as i64 {
                    return Err(format!("Index out of range: {}", int64).into());
                }

                int64 as i32
            }
            Value::Null => {
                let highest_index_rec = sqlx::query!(
                    "SELECT MAX(index) FROM guild_roles WHERE guild_id = $1",
                    ctx.guild_id.to_string()
                )
                .fetch_one(&ctx.data.pool)
                .await
                .map_err(|e| format!("Failed to get highest index: {:?}", e))?
                .max
                .unwrap_or(0);

                let index: i32 = highest_index_rec + 1;

                index
            }
            _ => {
                return Err(format!("Missing or invalid field: `index`: {:?}", parsed_value).into());
            }
        };

        let Some(Value::String(settings_role_id_str)) = state.get("role_id") else {
            return Err("Missing or invalid field: `role_id`".into());
        };

        let settings_role_id = settings_role_id_str
            .parse::<serenity::all::RoleId>()
            .map_err(|e| format!(
                "Internal Error in base_verify_checks: Failed to parse role id despite already having parsed it: {:?}",
                e
            ))?;

        // Get the new permissions as a Vec<String>
        let Some(Value::Array(perms_value)) = state.get("perms") else {
            return Err("Missing or invalid field: `perms`".into());
        };

        let mut perms = Vec::with_capacity(perms_value.len());

        for perm in perms_value {
            if let Value::String(perm) = perm {
                perms.push(perm.to_string());
            } else {
                return Err("Failed to parse permissions".into());
            }
        }

        let display_name = if let Some(Value::String(display_name)) = state.get("display_name") {
            Some(display_name.to_string())
        } else {
            None
        };

        let guild = sandwich_driver::guild(
            &ctx.serenity_context.cache,
            &ctx.serenity_context.http,
            &ctx.data.reqwest,
            ctx.guild_id,
        )
        .await
        .map_err(|e| format!("Failed to get guild: {:?}", e))?;

        // If owner, early return
        if guild.owner_id == ctx.author {
            return Ok(GreBaseVerifyChecksResult {
                index: new_index,
                role_id: settings_role_id,
                perms,
                display_name,
            });
        }

        let Some(member) = sandwich_driver::member_in_guild(
            &ctx.serenity_context.cache,
            &ctx.serenity_context.http,
            &ctx.data.reqwest,
            ctx.guild_id,
            ctx.author,
        )
        .await
        .map_err(|e| format!("Failed to get member: {:?}", e))?
        else {
            return Err("You must be in the guild to change settings!".into());
        };

        let current_roles = {
            let query = sqlx::query!(
                "SELECT index, role_id, perms FROM guild_roles WHERE guild_id = $1",
                ctx.guild_id.to_string()
            )
            .fetch_all(&ctx.data.pool)
            .await
            .map_err(|e| format!("Failed to get current role configuration: {:?}", e))?;

            query
                .into_iter()
                .map(|x| (x.role_id, (x.index, x.perms)))
                .collect::<std::collections::HashMap<String, (i32, Vec<String>)>>()
        };

        let Some(first_role) = member.roles.first() else {
            return Err("You must have at least one role to change this settings!".into());
        };

        let Some(first_role) = guild.roles.get(first_role) else {
            return Err("Could not find first role in guild".into());
        };

        let mut highest_role = first_role;
        let mut lowest_index: Option<i32> = None;

        for r in &member.roles {
            if let Some((index, _)) = current_roles.get(&r.to_string()) {
                match lowest_index {
                    Some(li) => {
                        if *index < li {
                            lowest_index = Some(*index);
                        }
                    }
                    None => {
                        lowest_index = Some(*index);
                    }
                }
            }

            let Some(r) = guild.roles.get(r) else {
                continue;
            };

            if r > highest_role {
                highest_role = r;
            }
        }

        // Check that our index is lower than the targets index
        let Some(lowest_index) = lowest_index else {
            return Err("You do not have any Anti-Raid configured roles yet! [could not find lowest index]".into());
        };

        let Some(settings_role) = guild.roles.get(&settings_role_id) else {
            return Err(format!("Expected a role but could not find it: {}", settings_role_id).into());
        };

        if highest_role <= settings_role {
            return Err("You do not have permission to edit this role's permissions as they are higher than you".into());
        }

        let author_kittycat_perms = silverpelt::member_permission_calc::get_kittycat_perms(
            &ctx.data.pool,
            ctx.guild_id,
            guild.owner_id,
            ctx.author,
            &member.roles,
        )
        .await
        .map_err(|e| format!("Failed to get author permissions: {:?}", e))?
        .resolve();

        if new_index < lowest_index {
            return Err(format!("You do not have permission to edit this role's permissions as the new index would be lower than you: {} < {}", new_index, lowest_index).into());
        }

        match op {
            OperationType::Create => {
                kittycat::perms::check_patch_changes(
                    &author_kittycat_perms,
                    &[],
                    &perms
                        .iter()
                        .map(|x| Permission::from_string(x))
                        .collect::<Vec<Permission>>(),
                )
                .map_err(|e| format!(
                    "You do not have permission to add a role with these permissions: {}",
                    e
                ))?;
            }
            OperationType::Update => {
                let Some((index, current_perms)) = current_roles.get(settings_role_id_str.as_str())
                else {
                    return Err("Could not find role in guild".into());
                };

                if *index < lowest_index {
                    return Err(format!("You do not have permission to edit this role's permissions as the current index is lower than you: {} < {}", *index, lowest_index).into());
                }

                kittycat::perms::check_patch_changes(
                    &author_kittycat_perms,
                    &current_perms
                        .iter()
                        .map(|x| Permission::from_string(x))
                        .collect::<Vec<Permission>>(),
                    &perms
                        .iter()
                        .map(|x| Permission::from_string(x))
                        .collect::<Vec<Permission>>(),
                )
                .map_err(|e| format!(
                    "You do not have permission to edit this role's permissions: {}",
                    e
                ))?;
            }
            OperationType::Delete => {
                kittycat::perms::check_patch_changes(
                    &author_kittycat_perms,
                    &perms
                        .iter()
                        .map(|x| Permission::from_string(x))
                        .collect::<Vec<Permission>>(),
                    &[],
                )
                .map_err(|e| format!(
                    "You do not have permission to remove a role with these permissions: {}",
                    e
                ))?;
            }
            _ => {
                return Err(format!("Invalid operation type: {}", op).into());
            }
        }

        Ok(GreBaseVerifyChecksResult {
            index: new_index,
            role_id: settings_role_id,
            perms,
            display_name,
        })
    }
}

pub static GUILD_MEMBERS: LazyLock<Setting<SettingsData>> = LazyLock::new(|| Setting {
    id: "guild_members".to_string(),
    name: "Server Members".to_string(),
    description: "Manage server members".to_string(),
    primary_key: "user_id".to_string(),
    columns: settings_wrap(vec![
        ar_settings::common_columns::guild_id("guild_id", "Guild ID", "The Guild ID"),
        Column {
            id: "user_id".to_string(),
            name: "User ID".to_string(),
            description: "The user ID. Cannot be updated once set".to_string(),
            column_type: ColumnType::new_scalar(InnerColumnType::String {
                kind: "user".to_string(),
                min_length: None,
                max_length: Some(64),
                allowed_values: vec![],
            }),
            nullable: false,
            suggestions: ColumnSuggestion::None {},
            ignored_for: vec![OperationType::Update],
            secret: false,
        },
        Column {
            id: "perm_overrides".to_string(),
            name: "Permission Overrides".to_string(),
            description:
                "Any permission overrides the member has. This can and should be edited if needed"
                    .to_string(),
            column_type: ColumnType::new_array(InnerColumnType::String {
                kind: "kittycat".to_string(),
                min_length: None,
                max_length: Some(64),
                allowed_values: vec![],
            }),
            nullable: false,
            suggestions: ColumnSuggestion::None {},
            ignored_for: vec![],
            secret: false,
        },
        Column {
            id: "public".to_string(),
            name: "Public".to_string(),
            description: "Whether the member is public or not".to_string(),
            column_type: ColumnType::new_scalar(InnerColumnType::Boolean {}),
            nullable: false,
            suggestions: ColumnSuggestion::None {},
            ignored_for: vec![],
            secret: false,
        },
        ar_settings::common_columns::created_at(),
    ]),
    title_template: "{user_id}, perm_overrides={perm_overrides}".to_string(),
    operations: SettingOperations::from(GuildMembersExecutor),
});

pub struct GmeBaseVerifyChecksResult {
    pub user_id: serenity::all::UserId,
    pub perm_overrides: Vec<kittycat::perms::Permission>,
    pub public: bool,
}

#[derive(Clone)]
pub struct GuildMembersExecutor;

impl GuildMembersExecutor {
    async fn get_kittycat_perms_for_user(
        &self,
        data: &SettingsData,
        pool: &sqlx::PgPool,
        guild_id: serenity::all::GuildId,
        guild_owner_id: serenity::all::UserId,
        user_id: serenity::all::UserId,
    ) -> Result<(Vec<serenity::all::RoleId>, Vec<kittycat::perms::Permission>), Error> {
        let Some(member) = sandwich_driver::member_in_guild(
            &data.serenity_context.cache,
            &data.serenity_context.http,
            &data.data.reqwest,
            guild_id,
            user_id,
        )
        .await
        .map_err(|e| format!("Failed to get user {}: {:?}", user_id, e))?
        else {
            return Ok((Vec::new(), Vec::new()));
        };

        let kittycat_perms = silverpelt::member_permission_calc::get_kittycat_perms(
            pool,
            guild_id,
            guild_owner_id,
            user_id,
            &member.roles,
        )
        .await
        .map_err(|e| format!("Failed to get user permissions: {:?} ({})", e, user_id))?
        .resolve();

        let roles = member
            .roles
            .iter()
            .copied()
            .collect::<Vec<serenity::all::RoleId>>();

        Ok((roles, kittycat_perms))
    }

    async fn verify(
        &self,
        ctx: &SettingsData,
        state: &indexmap::IndexMap<String, Value>,
        op: OperationType,
    ) -> Result<GmeBaseVerifyChecksResult, Error> {
        // Get the user id as this is required for all operations
        let Some(Value::String(user_id)) = state.get("user_id") else {
            return Err("Missing or invalid field: `user_id`".into());
        };

        // Parse the user id
        let user_id: serenity::all::UserId =
            user_id.parse().map_err(|e| format!("Failed to parse user id: {:?}", e))?;

        let Some(Value::Bool(public)) = state.get("public") else {
            return Err("Missing or invalid field: `public`".into());
        };

        if op == OperationType::Update {
            let current_public = sqlx::query!(
                "SELECT public FROM guild_members WHERE guild_id = $1 AND user_id = $2",
                ctx.guild_id.to_string(),
                user_id.to_string()
            )
            .fetch_one(&ctx.data.pool)
            .await
            .map_err(|e| format!("Failed to get current public status: {:?}", e))?
            .public;

            if *public != current_public && ctx.author != user_id {
                return Err("Only the user themselves can change their (own) public status".into());
            }
        }

        // Get perm overrides
        let perm_overrides = {
            let Some(Value::Array(perm_overrides_value)) = state.get("perm_overrides") else {
                return Err("Missing or invalid field: `perm_overrides`".into());
            };

            let mut perm_overrides = Vec::with_capacity(perm_overrides_value.len());

            for perm in perm_overrides_value {
                if let Value::String(perm) = perm {
                    perm_overrides.push(kittycat::perms::Permission::from_string(perm));
                } else {
                    return Err("Failed to parse permission overrides".into());
                }
            }

            perm_overrides
        };

        let guild = sandwich_driver::guild(
            &ctx.serenity_context.cache,
            &ctx.serenity_context.http,
            &ctx.data.reqwest,
            ctx.guild_id,
        )
        .await
        .map_err(|e| format!("Failed to get guild: {:?}", e))?;

        // If owner, early return
        if guild.owner_id == ctx.author {
            return Ok(GmeBaseVerifyChecksResult {
                user_id,
                perm_overrides,
                public: *public,
            });
        }

        // Get the authors kittycat permissions
        let author_kittycat_perms =
            match self
                .get_kittycat_perms_for_user(
                    ctx,
                    &ctx.data.pool,
                    ctx.guild_id,
                    guild.owner_id,
                    ctx.author,
                )
                .await
            {
                Ok((_, author_kittycat_perms)) => author_kittycat_perms,
                Err(e) => {
                    return Err(format!("Failed to get author permissions: {:?}", e).into())
                }
            };

        // Get the target members current kittycat permissions (if any) as well as their roles (for finding new permissions with overrides taken into account)
        let (target_member_roles, current_user_kittycat_perms) =
            match self
                .get_kittycat_perms_for_user(
                    ctx,
                    &ctx.data.pool,
                    ctx.guild_id,
                    guild.owner_id,
                    user_id,
                )
                .await
            {
                Ok((target_member_roles, current_user_kittycat_perms)) => {
                    (target_member_roles, current_user_kittycat_perms)
                }
                Err(e) => {
                    return Err(format!("Failed to get target member permissions: {:?}", e).into())
                }
            };

        // Find new user's permissions with the given perm overrides
        let new_user_kittycat_perms = {
            let roles_str = silverpelt::member_permission_calc::create_roles_list_for_guild(
                &target_member_roles,
                ctx.guild_id,
            );

            let user_positions =
                silverpelt::member_permission_calc::get_user_positions_from_db(
                    &ctx.data.pool,
                    ctx.guild_id,
                    &roles_str,
                )
                .await
                .map_err(|e| format!("Failed to get user positions: {:?}", e))?;

            kittycat::perms::StaffPermissions {
                user_positions,
                perm_overrides: perm_overrides.clone(),
            }
            .resolve()
        };

        // Check permissions
        match op {
            OperationType::Create => {
                kittycat::perms::check_patch_changes(
                    &author_kittycat_perms,
                    &[],
                    &new_user_kittycat_perms,
                )
                .map_err(|e| format!(
                    "You do not have permission to add a role with these permissions: {}",
                    e
                ))?;
            }
            OperationType::Update => {
                kittycat::perms::check_patch_changes(
                    &author_kittycat_perms,
                    &current_user_kittycat_perms,
                    &new_user_kittycat_perms,
                )
                .map_err(|e| format!(
                    "You do not have permission to edit this role's permissions: {}",
                    e
                ))?;
            }
            OperationType::Delete => {
                kittycat::perms::check_patch_changes(
                    &author_kittycat_perms,
                    &current_user_kittycat_perms,
                    &[],
                )
                .map_err(|e| format!(
                    "You do not have permission to remove this members permission overrides: {}",
                    e
                ))?;
            }
            _ => {
                return Err(format!("Invalid operation type: {}", op).into());
            }
        }

        Ok(GmeBaseVerifyChecksResult {
            user_id,
            perm_overrides,
            public: *public,
        })
    }
}

#[async_trait::async_trait]
impl SettingView<SettingsData> for GuildMembersExecutor {
    async fn view<'a>(
        &self,
        context: &SettingsData,
        _filters: indexmap::IndexMap<String, Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error> {
        let rows = sqlx::query!("SELECT user_id, perm_overrides, public, created_at FROM guild_members WHERE guild_id = $1", context.guild_id.to_string())
        .fetch_all(&context.data.pool)
        .await
        .map_err(|e| format!("Error while fetching guild roles: {}", e))?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "guild_id".to_string() => Value::String(context.guild_id.to_string()),
                "user_id".to_string() => Value::String(row.user_id),
                "perm_overrides".to_string() => Value::Array(row.perm_overrides.iter().map(|x| Value::String(x.to_string())).collect()),
                "public".to_string() => Value::Bool(row.public),
                "created_at".to_string() => Value::String(row.created_at.to_string()),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator<SettingsData> for GuildMembersExecutor {
    async fn create<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_members.create".into()).await?;

        let res = self.verify(ctx, &entry, OperationType::Create).await?;

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_members WHERE guild_id = $1 AND user_id = $2",
            ctx.guild_id.to_string(),
            res.user_id.to_string()
        )
        .fetch_one(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to check if role exists: {:?}", e))?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err("Role already exists".into());
        }

        sqlx::query!(
            "INSERT INTO guild_members (guild_id, user_id, perm_overrides, public) VALUES ($1, $2, $3, $4)",
            ctx.guild_id.to_string(),
            res.user_id.to_string(),
            &res.perm_overrides.into_iter().map(|x| x.to_string()).collect::<Vec<String>>(),
            res.public
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to insert role: {:?}", e))?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater<SettingsData> for GuildMembersExecutor {
    async fn update<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_members.update".into()).await?;

        let res = self.verify(ctx, &entry, OperationType::Update).await?;

        sqlx::query!(
            "UPDATE guild_members SET perm_overrides = $1, public = $2 WHERE guild_id = $3 AND user_id = $4",
            &res.perm_overrides.into_iter().map(|x| x.to_string()).collect::<Vec<String>>(),
            res.public,
            ctx.guild_id.to_string(),
            res.user_id.to_string()
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to update role: {:?}", e))?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter<SettingsData> for GuildMembersExecutor {
    async fn delete<'a>(
        &self,
        ctx: &SettingsData,
        primary_key: Value,
    ) -> Result<(), Error> {
        check_perms(ctx, "guild_members.delete".into()).await?;

        let Some(row) = sqlx::query!("SELECT user_id, perm_overrides, public FROM guild_members WHERE guild_id = $1 AND user_id = $2", ctx.guild_id.to_string(), primary_key.to_string())
        .fetch_optional(&ctx.data.pool)
        .await
        .map_err(|e| format!("Error while fetching roles: {}", e))? else {
            return Err("Member not found when trying to delete it!".into());
        };

        let entry = indexmap::indexmap! {
            "guild_id".to_string() => Value::String(ctx.guild_id.to_string()),
            "user_id".to_string() => Value::String(row.user_id),
            "perm_overrides".to_string() => Value::Array(row.perm_overrides.iter().map(|x| Value::String(x.to_string())).collect()),
            "public".to_string() => Value::Bool(row.public),
        };

        let res = self.verify(ctx, &entry, OperationType::Delete).await?;

        sqlx::query!(
            "DELETE FROM guild_members WHERE guild_id = $1 AND user_id = $2",
            ctx.guild_id.to_string(),
            res.user_id.to_string()
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to delete role: {:?}", e))?;

        Ok(())
    }
}

pub static GUILD_TEMPLATES: LazyLock<Setting<SettingsData>> = LazyLock::new(|| {
    Setting {
        id: "guild_templates".to_string(),
        name: "Server Templates".to_string(),
        description: "Configure/Setup Server Templates (Lua/Roblox Luau scripts)".to_string(),
        primary_key: "name".to_string(),
        columns: settings_wrap(vec![
            ar_settings::common_columns::guild_id("guild_id", "Guild ID", "The Guild ID"),
            Column {
                id: "name".to_string(),
                name: "Name".to_string(),
                description: "The name to give to the template".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "language".to_string(),
                name: "Language".to_string(),
                description: "The language of the template. Only Roblox Luau is currently supported here.".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec!["luau".to_string()],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "content".to_string(),
                name: "Content".to_string(),
                description: "The content of the template".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "template".to_string(),
                    min_length: None,
                    max_length: None,
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "events".to_string(),
                name: "Events".to_string(),
                description: "The events that this template can be dispatched on. If empty, this template is never dispatched.".to_string(),
                column_type: ColumnType::new_array(InnerColumnType::String { 
                    min_length: None, 
                    max_length: None, 
                    allowed_values: {
                        let mut vec = AntiraidEvent::variant_names()
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>();
                        
                        vec.extend(gwevent::core::event_list().iter().copied().map(|x| x.to_string()).collect::<Vec<String>>());

                        vec
                    },
                    kind: "normal".to_string()
                }),
                nullable: true,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "allowed_caps".to_string(),
                name: "Capabilities".to_string(),
                description: "The capabilities the template will have.".to_string(),
                column_type: ColumnType::new_array(InnerColumnType::String { min_length: None, max_length: None, allowed_values: vec![], kind: "normal".to_string() }),
                nullable: true,
                suggestions: ColumnSuggestion::Static {
                    suggestions: vec![
                        "discord:create_message".to_string()
                    ]
                },
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "error_channel".to_string(),
                name: "Error Channel".to_string(),
                description: "The channel to report errors to. If not specified, an Error event will be omitted instead".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "channel".to_string(),
                    min_length: None,
                    max_length: None,
                    allowed_values: vec![],
                }),
                nullable: true,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            ar_settings::common_columns::created_at(),
            ar_settings::common_columns::created_by(),
            ar_settings::common_columns::last_updated_at(),
            ar_settings::common_columns::last_updated_by(),
        ]),
        title_template: "{name}".to_string(),
        operations: SettingOperations::from(GuildTemplateExecutor),
    }
});

#[derive(Clone)]
pub struct GuildTemplateExecutor;

impl GuildTemplateExecutor {
    async fn validate_channel(&self, ctx: &SettingsData, channel_field: &str, channel_id: serenity::all::ChannelId) -> Result<(), Error> {
        // Perform required checks
        let channel = sandwich_driver::channel(
            &ctx.serenity_context.cache,
            &ctx.serenity_context.http,
            &ctx.data.reqwest,
            Some(ctx.guild_id),
            channel_id,
        )
        .await
        .map_err(|e| format!("Failed to fetch channel id: {} with field: {}", e, channel_field))?;

        let Some(channel) = channel else {
            return Err(format!("Could not find channel with id: {} and field: {}", channel_id, channel_field).into());
        };

        let Some(guild_channel) = channel.guild() else {
            return Err(format!("Channel with id: {} and field: {} is not in a guild", channel_id, channel_field).into());
        };

        if guild_channel.guild_id != ctx.guild_id {
            return Err(format!("Channel with id: {} and field: {} is not in the same guild as the setting", channel_id, channel_field).into());
        }

        let bot_user_id =
            ctx.serenity_context.cache.current_user().id;

        let bot_user = sandwich_driver::member_in_guild(
            &ctx.serenity_context.cache,
            &ctx.serenity_context.http,
            &ctx.data.reqwest,
            ctx.guild_id,
            bot_user_id,
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to get bot user: {}",
                e
            )
        })?;

        let Some(bot_user) = bot_user else {
            return Err(
                format!(
                    "Could not find bot user: {}",
                    bot_user_id
                )
                .into()
            );
        };

        let guild = sandwich_driver::guild(
            &ctx.serenity_context.cache,
            &ctx.serenity_context.http,
            &ctx.data.reqwest,
            ctx.guild_id,
        )
        .await
        .map_err(|e| 
            format!(
                "Failed to get guild: {}",
                e
            )
        )?;

        let permissions =
            guild.user_permissions_in(&guild_channel, &bot_user);

        if !permissions.contains(serenity::all::Permissions::SEND_MESSAGES) {
            return Err(
                format!("Bot does not have permission to `Send Messages` in channel with id: {} and field: {}", channel_id, channel_field).into()
            );
        }

        Ok(())        
    }

    async fn validate(&self, ctx: &SettingsData, name: &str) -> Result<(), Error> {
        if name.starts_with("$shop/") {
            let (shop_tname, shop_tversion) = silverpelt::templates::parse_shop_template(name)
                .map_err(|e| format!("Failed to parse shop template: {:?}", e))?;

            let shop_template = sqlx::query!(
                "SELECT COUNT(*) FROM template_shop WHERE name = $1 AND version = $2",
                shop_tname,
                shop_tversion
            )
            .fetch_one(&ctx.data.pool)
            .await
            .map_err(|e| format!("Failed to get shop template: {:?}", e))?;

            if shop_template.count.unwrap_or_default() == 0 {
                return Err("Shop template does not exist".into());
            }
        }

        Ok(())
    }

    async fn post_action(&self, ctx: &SettingsData, name: &str) -> Result<(), Error> {
        // Dispatch a OnStartup event for the template
        AntiraidEvent::OnStartup(vec![name.to_string()])
            .dispatch_to_template_worker_and_nowait(&ctx.data, ctx.guild_id)
            .await
            .map_err(|e| format!("Failed to dispatch OnStartup event: {:?}", e))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl SettingView<SettingsData> for GuildTemplateExecutor {
    async fn view<'a>(
        &self,
        context: &SettingsData,
        _filters: indexmap::IndexMap<String, Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error> {
        log::info!("Viewing guild templates for guild id: {}", context.guild_id);

        check_perms(context,"guild_templates.view".into()).await?;

        let rows = sqlx::query!("SELECT name, content, language, allowed_caps, events, error_channel, created_at, created_by, last_updated_at, last_updated_by FROM guild_templates WHERE guild_id = $1", context.guild_id.to_string())
        .fetch_all(&context.data.pool)
        .await
        .map_err(|e| format!("Error while fetching guild templates: {}", e))?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "guild_id".to_string() => Value::String(context.guild_id.to_string()),
                "name".to_string() => Value::String(row.name),
                "content".to_string() => Value::String(row.content),
                "language".to_string() => Value::String(row.language),
                "allowed_caps".to_string() => {
                    Value::Array(row.allowed_caps.iter().map(|x| Value::String(x.to_string())).collect())
                },
                "events".to_string() => {
                    Value::Array(row.events.iter().map(|x| Value::String(x.to_string())).collect())
                },
                "error_channel".to_string() => {
                    match row.error_channel {
                        Some(error_channel) => Value::String(error_channel),
                        None => Value::Null,
                    }
                },
                "created_at".to_string() => Value::String(row.created_at.to_string()),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::String(row.last_updated_at.to_string()),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator<SettingsData> for GuildTemplateExecutor {
    async fn create<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_templates.create".into()).await?;

        let Some(Value::String(name)) = entry.get("name") else {
            return Err("Missing or invalid field: `name`".into());
        };

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_templates WHERE guild_id = $1 AND name = $2",
            ctx.guild_id.to_string(),
            name
        )
        .fetch_one(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to check if template exists: {:?}", e))?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err("Template already exists".into());
        }

        self.validate(ctx, name).await?;

        let Some(Value::String(language)) = entry.get("language") else {
            return Err("Missing or invalid field: `language`".into());
        };

        let Some(Value::String(content)) = entry.get("content") else {
            return Err("Missing or invalid field: `content`".into());
        };

        let events = match entry.get("events") {
            Some(Value::Array(events)) => 
                events
                    .iter()
                    .map(|x| {
                        if let Value::String(x) = x {
                            Ok(x.to_string())
                        } else {
                            Err("Failed to parse events".into())
                        }
                    })
                    .collect::<Result<Vec<String>, Error>>()?,
            _ => {
                vec![]
            },
        };

        let allowed_caps = match entry.get("allowed_caps") {
            Some(Value::Array(allowed_caps)) => 
                allowed_caps
                    .iter()
                    .map(|x| {
                        if let Value::String(x) = x {
                            Ok(x.to_string())
                        } else {
                            Err(format!("Failed to parse allowed capabilities due to invalid capability: {:?}", x).into())
                        }
                    })
                    .collect::<Result<Vec<String>, Error>>()?,
            _ => {
                vec![]
            },
        };

        let error_channel = match entry.get("error_channel") {
            Some(Value::String(error_channel)) => {
                let channel_id: serenity::all::ChannelId = error_channel.parse()
                .map_err(|e| format!("Failed to parse error channel: {:?}", e))?;

                self.validate_channel(ctx, "error_channel", channel_id).await?;

                Some(error_channel.to_string())
            },
            _ => None,
        };

        sqlx::query!(
            "INSERT INTO guild_templates (guild_id, name, language, content, events, allowed_caps, error_channel, created_by, last_updated_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            ctx.guild_id.to_string(),
            name,
            language,
            content,
            &events,
            &allowed_caps,
            error_channel,
            ctx.author.to_string(),
            ctx.author.to_string()
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to insert template: {:?}", e))?;

        self.post_action(ctx, name).await?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater<SettingsData> for GuildTemplateExecutor {
    async fn update<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_templates.update".into()).await?;

        let Some(Value::String(name)) = entry.get("name") else {
            return Err("Missing or invalid field: `name`".into());
        };

        self.validate(ctx, name).await?;

        let Some(Value::String(language)) = entry.get("language") else {
            return Err("Missing or invalid field: `language`".into());
        };

        let Some(Value::String(content)) = entry.get("content") else {
            return Err("Missing or invalid field: `content`".into());
        };

        let events = match entry.get("events") {
            Some(Value::Array(events)) => 
                events
                    .iter()
                    .map(|x| {
                        if let Value::String(x) = x {
                            Ok(x.to_string())
                        } else {
                            Err(format!("Failed to parse events due to invalid event: {:?}", x).into())
                        }
                    })
                    .collect::<Result<Vec<String>, Error>>()?,
            _ => {
                vec![]
            },
        };

        let allowed_caps = match entry.get("allowed_caps") {
            Some(Value::Array(allowed_caps)) => 
                allowed_caps
                    .iter()
                    .map(|x| {
                        if let Value::String(x) = x {
                            Ok(x.to_string())
                        } else {
                            Err(format!("Failed to parse allowed capabilities due to invalid capability: {:?}", x).into())
                        }
                    })
                    .collect::<Result<Vec<String>, Error>>()?,
            _ => {
                vec![]
            },
        };
        

        let error_channel = match entry.get("error_channel") {
            Some(Value::String(error_channel)) => {
                let channel_id: serenity::all::ChannelId = error_channel.parse()
                .map_err(|e| format!("Failed to parse error channel: {:?}", e))?;

                self.validate_channel(ctx, "error_channel", channel_id).await?;

                Some(error_channel.to_string())
            },
            _ => None,
        };

        sqlx::query!(
            "UPDATE guild_templates SET content = $1, events = $2, allowed_caps = $3, language = $4, last_updated_at = NOW(), last_updated_by = $5, error_channel = $6 WHERE guild_id = $7 AND name = $8",
            content,
            &events,
            &allowed_caps,
            language,
            ctx.author.to_string(),
            error_channel,
            ctx.guild_id.to_string(),
            name
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to update template: {:?}", e))?;

        self.post_action(ctx, name).await?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter<SettingsData> for GuildTemplateExecutor {
    async fn delete<'a>(
        &self,
        ctx: &SettingsData,
        primary_key: Value,
    ) -> Result<(), Error> {
        check_perms(ctx, "guild_templates.delete".into()).await?;

        let Value::String(primary_key) = primary_key else {
            return Err("Invalid primary key".into());
        };

        let Some(row) = sqlx::query!(
            "SELECT name FROM guild_templates WHERE guild_id = $1 AND name = $2",
            ctx.guild_id.to_string(),
            primary_key
        )
        .fetch_optional(&ctx.data.pool)
        .await
        .map_err(|e| format!("Error while fetching template: {}", e))?
        else {
            return Err("Template not found when trying to delete it!".into());
        };

        let name = row.name;

        sqlx::query!(
            "DELETE FROM guild_templates WHERE guild_id = $1 AND name = $2",
            ctx.guild_id.to_string(),
            name
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to delete template: {:?}", e))?;

        self.post_action(ctx, &name).await?;

        Ok(())
    }
}

pub static GUILD_TEMPLATES_KV: LazyLock<Setting<SettingsData>> = LazyLock::new(|| Setting {
    id: "guild_templates_kv".to_string(),
    name: "Server Templates (key-value db)".to_string(),
    description: "Key-value database available to templates on this server".to_string(),
    primary_key: "key".to_string(),
    columns: settings_wrap(vec![
        ar_settings::common_columns::guild_id("guild_id", "Guild ID", "The Guild ID"),
        Column {
            id: "key".to_string(),
            name: "Key".to_string(),
            description: "Key".to_string(),
            column_type: ColumnType::new_scalar(InnerColumnType::String {
                kind: "normal".to_string(),
                min_length: None,
                max_length: Some(silverpelt::templates::LuaKVConstraints::default().max_key_length),
                allowed_values: vec![],
            }),
            nullable: false,
            suggestions: ColumnSuggestion::None {},
            ignored_for: vec![],
            secret: false,
        },
        Column {
            id: "value".to_string(),
            name: "Value".to_string(),
            description: "The value of the record".to_string(),
            column_type: ColumnType::new_scalar(InnerColumnType::Json {
                max_bytes: Some(silverpelt::templates::LuaKVConstraints::default().max_value_bytes),
            }),
            nullable: true,
            suggestions: ColumnSuggestion::None {},
            ignored_for: vec![],
            secret: false,
        },
        ar_settings::common_columns::created_at(),
        ar_settings::common_columns::last_updated_at(),
    ]),
    title_template: "{key}".to_string(),
    operations: SettingOperations::from(GuildTemplatesKVExecutor),
});

#[derive(Clone)]
pub struct GuildTemplatesKVExecutor;

#[async_trait::async_trait]
impl SettingView<SettingsData> for GuildTemplatesKVExecutor {
    async fn view<'a>(
        &self,
        context: &SettingsData,
        _filters: indexmap::IndexMap<String, Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error> {
        check_perms(context,"guild_templates_kv.view".into()).await?;

        let rows = sqlx::query!("SELECT key, value, created_at, last_updated_at FROM guild_templates_kv WHERE guild_id = $1", context.guild_id.to_string())
        .fetch_all(&context.data.pool)
        .await
        .map_err(|e| format!("Error while fetching guild templates kv: {}", e))?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "guild_id".to_string() => Value::String(context.guild_id.to_string()),
                "key".to_string() => Value::String(row.key),
                "value".to_string() => row.value.unwrap_or(Value::Null),
                "created_at".to_string() => Value::String(row.created_at.to_string()),
                "last_updated_at".to_string() => Value::String(row.last_updated_at.to_string()),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator<SettingsData> for GuildTemplatesKVExecutor {
    async fn create<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_templates_kv.create".into()).await?;

        let Some(Value::String(key)) = entry.get("key") else {
            return Err("Missing or invalid field: `key`".into());
        };

        let total_count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_templates_kv WHERE guild_id = $1",
            ctx.guild_id.to_string()
        )
        .fetch_one(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to check total kv count: {:?}", e))?
        .count
        .unwrap_or_default();

        if total_count
            >= silverpelt::templates::LuaKVConstraints::default()
                .max_keys
                .try_into()
                .map_err(|_| "Failed to convert max_keys to i64".to_string())?
        {
            return Err("Max key-value pairs reached".into());
        }

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_templates_kv WHERE guild_id = $1 AND key = $2",
            ctx.guild_id.to_string(),
            key
        )
        .fetch_one(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to check if kv exists: {:?}", e))?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err("Key already exists in key-value DB".into());
        }

        let Some(value) = entry.get("value") else {
            return Err("Missing or invalid field: `value`".into());
        };

        sqlx::query!(
            "INSERT INTO guild_templates_kv (guild_id, key, value, created_at, last_updated_at) VALUES ($1, $2, $3, NOW(), NOW())",
            ctx.guild_id.to_string(),
            key,
            value
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to insert kv: {:?}", e))?;

        // Dispatch a ExternalKeyUpdate event for the template
        AntiraidEvent::ExternalKeyUpdate(ExternalKeyUpdateEventData {
            key_modified: key.to_string(),
            author: ctx.author,
            action: ExternalKeyUpdateEventDataAction::Create
        })
        .dispatch_to_template_worker_and_nowait(&ctx.data, ctx.guild_id)
        .await
        .map_err(|e| format!("Failed to dispatch ExternalKeyUpdate event: {:?}", e))?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater<SettingsData> for GuildTemplatesKVExecutor {
    async fn update<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_templates_kv.update".into()).await?;

        let Some(Value::String(key)) = entry.get("key") else {
            return Err("Missing or invalid field: `key`".into());
        };

        let Some(value) = entry.get("value") else {
            return Err("Missing or invalid field: `value`".into());
        };

        sqlx::query!(
            "UPDATE guild_templates_kv SET value = $1, last_updated_at = NOW() WHERE guild_id = $2 AND key = $3",
            value,
            ctx.guild_id.to_string(),
            key
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to update kv: {:?}", e))?;

    // Dispatch a ExternalKeyUpdate event for the template
    AntiraidEvent::ExternalKeyUpdate(ExternalKeyUpdateEventData {
        key_modified: key.to_string(),
        author: ctx.author,
        action: ExternalKeyUpdateEventDataAction::Update
    })
        .dispatch_to_template_worker_and_nowait(&ctx.data, ctx.guild_id)
        .await
        .map_err(|e| format!("Failed to dispatch ExternalKeyUpdate event: {:?}", e))?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter<SettingsData> for GuildTemplatesKVExecutor {
    async fn delete<'a>(
        &self,
        ctx: &SettingsData,
        primary_key: Value,
    ) -> Result<(), Error> {
        check_perms(ctx, "guild_templates_kv.delete".into()).await?;

        if sqlx::query!(
            "SELECT COUNT(*) FROM guild_templates_kv WHERE guild_id = $1 AND key = $2",
            ctx.guild_id.to_string(),
            primary_key.to_string()
        )
        .fetch_one(&ctx.data.pool)
        .await
        .map_err(|e| format!("Error while fetching kv: {}", e))?
        .count
        .unwrap_or_default()
            <= 0
        {
            return Err("Row requested to be deleted does not exist".into());
        };

        sqlx::query!(
            "DELETE FROM guild_templates_kv WHERE guild_id = $1 AND key = $2",
            ctx.guild_id.to_string(),
            primary_key.to_string()
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to delete kv: {:?}", e))?;

        // Dispatch a ExternalKeyUpdate event for the template
        AntiraidEvent::ExternalKeyUpdate(ExternalKeyUpdateEventData {
            key_modified: primary_key.to_string(),
            author: ctx.author,
            action: ExternalKeyUpdateEventDataAction::Delete
        })
        .dispatch_to_template_worker_and_nowait(&ctx.data, ctx.guild_id)
        .await
        .map_err(|e| format!("Failed to dispatch ExternalKeyUpdate event: {:?}", e))?;

        Ok(())
    }
}

pub static GUILD_TEMPLATE_SHOP: LazyLock<Setting<SettingsData>> = LazyLock::new(|| {
    Setting {
        id: "template_shop".to_string(),
        name: "Created/Published Templates".to_string(),
        description: "Publish new templates to the shop that can be used by any other server".to_string(),
        primary_key: "id".to_string(),
        columns: settings_wrap(vec![
            Column {
                id: "id".to_string(),
                name: "ID".to_string(),
                description: "The internal ID of the template".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    min_length: Some(30),
                    max_length: Some(64),
                    allowed_values: vec![],
                    kind: "uuid".to_string(),
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![OperationType::Create],
                secret: false,
            },
            Column {
                id: "name".to_string(),
                name: "Name".to_string(),
                description: "The name of the template on the shop. Cannot be updated once set".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![OperationType::Update],
                secret: false,
            },
            Column {
                id: "friendly_name".to_string(),
                name: "Friendly Name".to_string(),
                description: "The friendly name of the template on the shop.".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "language".to_string(),
                name: "Language".to_string(),
                description: "The language of the template. Only Roblox Luau is currently supported here. Cannot be updated once set".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec!["luau".to_string()],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![OperationType::Update],
                secret: false,
            },
            Column {
                id: "version".to_string(),
                name: "Version".to_string(),
                description: "The version of the template. Cannot be updated once set".to_string(), 
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![OperationType::Update],
                secret: false,
            },
            Column {
                id: "description".to_string(),
                name: "Description".to_string(),
                description: "The description of the template".to_string(), 
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(4096),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "content".to_string(),
                name: "Content".to_string(),
                description: "The content of the template. Cannot be updated once set (use a new version for that)".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "template".to_string(),
                    min_length: None,
                    max_length: None,
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![OperationType::Update],
                secret: false,
            },
            Column {
                id: "events".to_string(),
                name: "Events".to_string(),
                description: "The events this template should have access to, Cannot be changed once set".to_string(),
                column_type: ColumnType::new_array(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: None,
                    allowed_values: {
                        let mut vec = AntiraidEvent::variant_names()
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>();
                        
                        vec.extend(gwevent::core::event_list().iter().copied().map(|x| x.to_string()).collect::<Vec<String>>());

                        vec
                    },
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![OperationType::Update],
                secret: false,
            },
            Column {
                id: "allowed_caps".to_string(),
                name: "Capabilities".to_string(),
                description: "The capabilities the template needs to perform its full functionality. Cannot be changed once set".to_string(),
                column_type: ColumnType::new_array(InnerColumnType::String { min_length: None, max_length: None, allowed_values: vec![], kind: "normal".to_string() }),
                nullable: true,
                suggestions: ColumnSuggestion::Static {
                    suggestions: vec![
                        "discord:create_message".to_string()
                    ]
                },
                ignored_for: vec![OperationType::Update],
                secret: false,
            },
            Column {
                id: "type".to_string(),
                name: "Type".to_string(),
                description: "The type of the template".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: None,
                    allowed_values: vec!["public".to_string(), "hidden".to_string()],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            ar_settings::common_columns::guild_id("owner_guild", "Guild ID", "The Guild ID"),
            ar_settings::common_columns::created_at(),
            ar_settings::common_columns::created_by(),
            ar_settings::common_columns::last_updated_at(),
            ar_settings::common_columns::last_updated_by(),
        ]),
        title_template: "{name}#{version}".to_string(),
        operations: SettingOperations::from(GuildTemplateShopExecutor),
    }
});

#[derive(Clone)]
pub struct GuildTemplateShopExecutor;

#[async_trait::async_trait]
impl SettingView<SettingsData> for GuildTemplateShopExecutor {
    async fn view<'a>(
        &self,
        context: &SettingsData,
        _filters: indexmap::IndexMap<String, Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error> {
        check_perms(context,"guild_templates_shop.view".into()).await?;

        let rows = sqlx::query!("SELECT id, name, friendly_name, language, allowed_caps, version, description, content, type, events, created_at, created_by, last_updated_at, last_updated_by FROM template_shop WHERE owner_guild = $1", context.guild_id.to_string())
        .fetch_all(&context.data.pool)
        .await
        .map_err(|e| format!("Error while fetching shop templates: {}", e))?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "id".to_string() => Value::String(row.id.to_string()),
                "name".to_string() => Value::String(row.name),
                "friendly_name".to_string() => Value::String(row.friendly_name),
                "language".to_string() => Value::String(row.language),
                "allowed_caps".to_string() => {
                    Value::Array(row.allowed_caps.iter().map(|x| Value::String(x.to_string())).collect())
                },
                "version".to_string() => Value::String(row.version),
                "description".to_string() => Value::String(row.description),
                "type".to_string() => Value::String(row.r#type),
                "content".to_string() => Value::String(row.content),
                "events".to_string() => {
                    Value::Array(row.events.iter().map(|x| Value::String(x.to_string())).collect())
                },
                "owner_guild".to_string() => Value::String(context.guild_id.to_string()),
                "created_at".to_string() => Value::String(row.created_at.to_string()),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::String(row.last_updated_at.to_string()),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator<SettingsData> for GuildTemplateShopExecutor {
    async fn create<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_templates_shop.create".into()).await?;

        let Some(Value::String(name)) = entry.get("name") else {
            return Err("Missing or invalid field: `name`".into());
        };

        // Rules for name:
        // Only namespaced templates can contain @ or /
        // Namespaced templates must use a namespace owned by the server
        // Namespaced templates must be in the format @namespace/<pkgname>. <pkgname> itself cannot contain '@' but may use '/'

        if !name.is_ascii() {
            return Err("Name must be ASCII".into());
        }

        if name.starts_with('@') {
            // This is a namespaced template, check that the server owns the namespace
            if !name.contains('/') {
                return Err("Please contact support to claim ownership over a specific namespace".into());
            }

            let namespace = name.split('/').next().unwrap();
            let pkgname = name.replace(&format!("{}{}", namespace, "/"), "");

            if pkgname.contains("@") {
                return Err("Package name cannot contain '@'".into());
            }

            let count = sqlx::query!(
                "SELECT COUNT(*) FROM template_shop WHERE owner_guild = $1 AND name = $2",
                ctx.guild_id.to_string(),
                namespace
            )
            .fetch_one(&ctx.data.pool)
            .await
            .map_err(|e| format!("Failed to check if namespace exists: {:?}", e))?
            .count
            .unwrap_or_default();

            if count <= 0 {
                return Err("Namespace does not exist. Please contact support".into());
            }
        } else if name.contains('@') || name.contains('/') {
            return Err("Name cannot contain '@' or '/' unless it is a namespace".into());
        }

        let Some(Value::String(friendly_name)) = entry.get("friendly_name") else {
            return Err("Missing or invalid field: `friendly_name`".into());
        };

        let Some(Value::String(language)) = entry.get("language") else {
            return Err("Missing or invalid field: `language`".into());
        };

        let Some(Value::String(version)) = entry.get("version") else {
            return Err("Missing or invalid field: `version`".into());
        };

        if version == "latest" {
            return Err("Version cannot be 'latest'".into());
        }

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM template_shop WHERE owner_guild = $1 AND name = $2 AND version = $3",
            ctx.guild_id.to_string(),
            name,
            version
        )
        .fetch_one(&ctx.data.pool)
        .await
        .map_err(|e| 
            format!("Failed to check if shop template exists: {:?}", e)
        )?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err("Shop template with this name and version already exists".into());
        }

        let Some(Value::String(description)) = entry.get("description") else {
            return Err("Missing or invalid field: `description`".into());
        };

        let Some(Value::String(content)) = entry.get("content") else {
            return Err("Missing or invalid field: `content`".into());
        };

        let Some(Value::String(r#type)) = entry.get("type") else {
            return Err("Missing or invalid field: `type`".into());
        };

        let events = match entry.get("events") {
            Some(Value::Array(events)) => 
                events
                    .iter()
                    .map(|x| {
                        if let Value::String(x) = x {
                            Ok(x.to_string())
                        } else {
                            Err("Failed to parse events".into())
                        }
                    })
                    .collect::<Result<Vec<String>, Error>>()?,
            _ => {
                vec![]
            },
        };

        let allowed_caps = match entry.get("allowed_caps") {
            Some(Value::Array(allowed_caps)) => 
                allowed_caps
                    .iter()
                    .map(|x| {
                        if let Value::String(x) = x {
                            Ok(x.to_string())
                        } else {
                            Err(format!("Failed to parse allowed capabilities due to invalid capability: {:?}", x).into())
                        }
                    })
                    .collect::<Result<Vec<String>, Error>>()?,
            _ => {
                vec![]
            },
        };

        let id = sqlx::query!(
            "INSERT INTO template_shop (name, friendly_name, language, version, description, content, type, events, owner_guild, created_by, last_updated_by, allowed_caps) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
            name,
            friendly_name,
            language,
            version,
            description,
            content,
            r#type,
            &events,
            ctx.guild_id.to_string(),
            ctx.author.to_string(),
            ctx.author.to_string(),
            &allowed_caps
        )
        .fetch_one(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to insert shop template: {:?}", e))?;

        // Add returned ID to entry
        let mut entry = entry;
        entry.insert("id".to_string(), Value::String(id.id.to_string()));

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater<SettingsData> for GuildTemplateShopExecutor {
    async fn update<'a>(
        &self,
        ctx: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(ctx, "guild_templates_shop.update".into()).await?;

        let Some(Value::String(id)) = entry.get("id") else {
            return Err("Missing or invalid field: `id`".into());
        };

        let id: uuid::Uuid = id.parse().map_err(|e| format!("Failed to parse ID: {:?}", e))?;

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM template_shop WHERE owner_guild = $1 AND id = $2",
            ctx.guild_id.to_string(),
            id
        )
        .fetch_one(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to check if shop template exists: {:?}", e))?
        .count
        .unwrap_or_default();

        if count <= 0 {
            return Err("Shop template does not exist".into());
        }

        let Some(Value::String(friendly_name)) = entry.get("friendly_name") else {
            return Err("Missing or invalid field: `friendly_name`".into());
        };

        let Some(Value::String(description)) = entry.get("description") else {
            return Err("Missing or invalid field: `description`".into());
        };

        let Some(Value::String(r#type)) = entry.get("type") else {
            return Err("Missing or invalid field: `type`".into());
        };

        sqlx::query!(
            "UPDATE template_shop SET description = $1, type = $2, friendly_name = $3, last_updated_at = NOW(), last_updated_by = $4 WHERE owner_guild = $5 AND id = $6",
            description,
            r#type,
            friendly_name,
            ctx.author.to_string(),
            ctx.guild_id.to_string(),
            id
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to update shop template: {:?}", e))?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter<SettingsData> for GuildTemplateShopExecutor {
    async fn delete<'a>(
        &self,
        ctx: &SettingsData,
        primary_key: Value,
    ) -> Result<(), Error> {
        check_perms(ctx, "guild_templates_shop.delete".into()).await?;

        let Value::String(primary_key) = primary_key else {
            return Err("Missing or invalid field: `id`".into());
        };

        let primary_key = primary_key.parse::<uuid::Uuid>().map_err(|e| format!("Failed to parse ID: {:?}", e))?;

        let Some(row) = sqlx::query!(
            "SELECT id FROM template_shop WHERE owner_guild = $1 AND id = $2",
            ctx.guild_id.to_string(),
            primary_key
        )
        .fetch_optional(&ctx.data.pool)
        .await
        .map_err(|e| format!("Error while fetching shop template: {}", e))?
        else {
            return Err("Shop template not found when trying to delete it!".into());
        };

        let id = row.id;

        sqlx::query!(
            "DELETE FROM template_shop WHERE owner_guild = $1 AND id = $2",
            ctx.guild_id.to_string(),
            id
        )
        .execute(&ctx.data.pool)
        .await
        .map_err(|e| format!("Failed to delete shop template: {:?}", e))?;

        Ok(())
    }
}

pub static GUILD_TEMPLATE_SHOP_PUBLIC_LIST: LazyLock<Setting<SettingsData>> = LazyLock::new(|| {
    Setting {
        id: "template_shop_public_list".to_string(),
        name: "Explore the shop!".to_string(),
        description: "Explore other templates published by other servers".to_string(),
        primary_key: "id".to_string(),
        columns: settings_wrap(vec![
            Column {
                id: "id".to_string(),
                name: "ID".to_string(),
                description: "The internal ID of the template".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    min_length: Some(30),
                    max_length: Some(64),
                    allowed_values: vec![],
                    kind: "uuid".to_string(),
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "name".to_string(),
                name: "Name".to_string(),
                description: "The name of the template on the shop. Cannot be updated once set".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![OperationType::Update],
                secret: false,
            },
            Column {
                id: "version".to_string(),
                name: "Version".to_string(),
                description: "The version of the template. Cannot be updated once set".to_string(), 
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(64),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![OperationType::Update],
                secret: false,
            },
            Column {
                id: "description".to_string(),
                name: "Description".to_string(),
                description: "The description of the template".to_string(), 
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: Some(4096),
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "type".to_string(),
                name: "Type".to_string(),
                description: "The type of the template".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: "normal".to_string(),
                    min_length: None,
                    max_length: None,
                    allowed_values: vec!["public".to_string(), "hidden".to_string()],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            ar_settings::common_columns::guild_id("owner_guild", "Guild ID", "The ID of the server which owns the templaye"),
            ar_settings::common_columns::created_at(),
            ar_settings::common_columns::created_by(),
            ar_settings::common_columns::last_updated_at(),
            ar_settings::common_columns::last_updated_by(),
        ]),
        title_template: "{name}".to_string(),
        operations: SettingOperations::to_view_op(GuildTemplateShopPublicListExecutor),
    }
});

#[derive(Clone)]
pub struct GuildTemplateShopPublicListExecutor;

#[async_trait::async_trait]
impl SettingView<SettingsData> for GuildTemplateShopPublicListExecutor {
    async fn view<'a>(
        &self,
        context: &SettingsData,
        _filters: indexmap::IndexMap<String, Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error> {
        let rows = sqlx::query!("SELECT id, name, version, description, type, owner_guild, created_at, created_by, last_updated_at, last_updated_by FROM template_shop WHERE type = 'public'")
        .fetch_all(&context.data.pool)
        .await
        .map_err(|e| format!("Error while fetching shop templates: {}", e))?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "id".to_string() => Value::String(row.id.to_string()),
                "name".to_string() => Value::String(row.name),
                "version".to_string() => Value::String(row.version),
                "description".to_string() => Value::String(row.description),
                "type".to_string() => Value::String(row.r#type),
                "owner_guild".to_string() => Value::String(row.owner_guild),
                "created_at".to_string() => Value::String(row.created_at.to_string()),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::String(row.last_updated_at.to_string()),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }

        Ok(result)
    }
}

pub static LOCKDOWN_SETTINGS: LazyLock<Setting<SettingsData>> = LazyLock::new(|| {
    Setting {
        id: "lockdown_guilds".to_string(),
        name: "Lockdown Settings".to_string(),
        description: "Setup standard lockdown settings for a server".to_string(),
        primary_key: "guild_id".to_string(),
        columns: settings_wrap(vec![
            ar_settings::common_columns::guild_id(
                "guild_id",
                "Guild ID",
                "Guild ID of the server in question",
            ),
            Column {
                id: "member_roles".to_string(),
                name: "Member Roles".to_string(),
                description: "Which roles to use as member roles for the purpose of lockdown. These roles will be explicitly modified during lockdown".to_string(),
                column_type: ColumnType::new_array(InnerColumnType::String {
                    kind: "role".to_string(),
                    min_length: None,
                    max_length: None,
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "require_correct_layout".to_string(),
                name: "Require Correct Layout".to_string(),
                description: "Whether or not a lockdown can proceed even without correct critical role permissions. May lead to partial lockdowns if disabled".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::Boolean {}),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![],
                secret: false,
            },
            ar_settings::common_columns::created_at(),
            ar_settings::common_columns::created_by(),
            ar_settings::common_columns::last_updated_at(),
            ar_settings::common_columns::last_updated_by(),
        ]),
        title_template: "Lockdown Settings".to_string(),
        operations: SettingOperations::from(LockdownSettingsExecutor),
    }
});

#[derive(Clone)]
pub struct LockdownSettingsExecutor;

#[async_trait]
impl SettingView<SettingsData> for LockdownSettingsExecutor {
    async fn view<'a>(
        &self,
        context: &SettingsData,
        _filters: indexmap::IndexMap<String, Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error> {
        check_perms(context,"lockdown_settings.view".into()).await?;

        let rows = sqlx::query!("SELECT member_roles, require_correct_layout, created_at, created_by, last_updated_at, last_updated_by FROM lockdown__guilds WHERE guild_id = $1", context.guild_id.to_string())
            .fetch_all(&context.data.pool)
            .await
            .map_err(|e| format!("Error while fetching lockdowns: {}", e))?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "guild_id".to_string() => Value::String(context.guild_id.to_string()),
                "member_roles".to_string() => Value::Array(row.member_roles.into_iter().map(Value::String).collect()),
                "require_correct_layout".to_string() => Value::Bool(row.require_correct_layout),
                "created_at".to_string() => Value::String(row.created_at.to_string()),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::String(row.last_updated_at.to_string()),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }
        
        Ok(result)
    }
}

#[async_trait]
impl SettingCreator<SettingsData> for LockdownSettingsExecutor {
    async fn create<'a>(
        &self,
        context: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(context,"lockdown_settings.create".into()).await?;

        let Some(Value::Array(member_roles)) = entry.get("member_roles") else {
            return Err("Missing or invalid field: `member_roles`".into());
        };

        let member_roles: Vec<String> = member_roles.iter().map(|v| match v {
            Value::String(s) => Ok(s.clone()),
            _ => Err("Invalid member role".into()),
        }).collect::<Result<Vec<String>, Error>>()?;
        
        let Some(Value::Bool(require_correct_layout)) = entry.get("require_correct_layout") else {
            return Err("Missing or invalid field: `require_correct_layout`".into());
        };

        sqlx::query!(
            "INSERT INTO lockdown__guilds (guild_id, member_roles, require_correct_layout, created_at, created_by, last_updated_at, last_updated_by) VALUES ($1, $2, $3, NOW(), $4, NOW(), $5)",
            context.guild_id.to_string(),
            &member_roles,
            require_correct_layout,
            context.author.to_string(),
            context.author.to_string(),
        )
        .execute(&context.data.pool)
        .await
        .map_err(|e| format!("Error while creating lockdown settings: {}", e))?;

        Ok(entry)
    }
}

#[async_trait]
impl SettingUpdater<SettingsData> for LockdownSettingsExecutor {
    async fn update<'a>(
        &self,
        context: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(context,"lockdown_settings.uodate".into()).await?;

        let Some(Value::Array(member_roles)) = entry.get("member_roles") else {
            return Err("Missing or invalid field: `member_roles`".into());
        };

        let member_roles: Vec<String> = member_roles.iter().map(|v| match v {
            Value::String(s) => Ok(s.clone()),
            _ => Err("Invalid member role".into()),
        }).collect::<Result<Vec<String>, Error>>()?;
        
        let Some(Value::Bool(require_correct_layout)) = entry.get("require_correct_layout") else {
            return Err("Missing or invalid field: `require_correct_layout`".into());
        };

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM lockdown__guilds WHERE guild_id = $1",
            context.guild_id.to_string(),
        )
        .fetch_one(&context.data.pool)
        .await
        .map_err(|e| format!("Error while updating lockdown settings: {}", e))?;

        if count.count.unwrap_or(0) == 0 {
            return Err("Lockdown settings not found".into());
        }

        sqlx::query!(
            "UPDATE lockdown__guilds SET member_roles = $2, require_correct_layout = $3, last_updated_at = NOW(), last_updated_by = $4 WHERE guild_id = $1",
            context.guild_id.to_string(),
            &member_roles,
            require_correct_layout,
            context.author.to_string(),
        )
        .execute(&context.data.pool)
        .await
        .map_err(|e| format!("Error while creating lockdown settings: {}", e))?;

        Ok(entry)
    }
}

#[async_trait]
impl SettingDeleter<SettingsData> for LockdownSettingsExecutor {
    async fn delete<'a>(
        &self,
        context: &SettingsData,
        _primary_key: Value,
    ) -> Result<(), Error> {
        check_perms(context,"lockdown_settings.delete".into()).await?;

        sqlx::query!("DELETE FROM lockdown__guilds WHERE guild_id = $1", context.guild_id.to_string())
            .execute(&context.data.pool)
            .await
            .map_err(|e| format!("Error while deleting lockdown settings: {}", e))?;

        Ok(())
    }
}

pub static LOCKDOWNS: LazyLock<Setting<SettingsData>> = LazyLock::new(|| Setting {
    id: "lockdowns".to_string(),
    name: "Lockdowns".to_string(),
    description: "Lockdowns".to_string(),
    primary_key: "id".to_string(),
    columns: settings_wrap(vec![
        Column {
            id: "id".to_string(),
            name: "ID".to_string(),
            description: "The ID of the lockdown".to_string(),
            column_type: ColumnType::new_scalar(InnerColumnType::String {
                min_length: Some(30),
                max_length: Some(64),
                allowed_values: vec![],
                kind: "uuid".to_string(),
            }),
            nullable: false,
            suggestions: ColumnSuggestion::None {},
            ignored_for: vec![OperationType::Create],
            secret: false,
        },
        ar_settings::common_columns::guild_id(
            "guild_id",
            "Guild ID",
            "The Guild ID referring to this lockdown",
        ),
        Column {
            id: "type".to_string(),
            name: "Type".to_string(),
            description: "The type of the lockdown.".to_string(),
            column_type: ColumnType::new_scalar(InnerColumnType::String {
                kind: "normal".to_string(),
                min_length: Some(1),
                max_length: Some(256),
                allowed_values: vec![],
            }),
            nullable: false,
            suggestions: ColumnSuggestion::None {},
            ignored_for: vec![],
            secret: false,
        },
        Column {
            id: "data".to_string(),
            name: "Data".to_string(),
            description: "The data stored of the lockdown.".to_string(),
            column_type: ColumnType::new_scalar(InnerColumnType::Json { max_bytes: None }),
            nullable: false,
            suggestions: ColumnSuggestion::None {},
            ignored_for: vec![OperationType::Create, OperationType::Update],
            secret: false,
        },
        Column {
            id: "reason".to_string(),
            name: "Reason".to_string(),
            description: "The reason for starting the lockdown.".to_string(),
            column_type: ColumnType::new_scalar(InnerColumnType::String {
                kind: "normal".to_string(),
                min_length: Some(1),
                max_length: Some(256),
                allowed_values: vec![],
            }),
            nullable: false,
            suggestions: ColumnSuggestion::None {},
            ignored_for: vec![],
            secret: false,
        },
        ar_settings::common_columns::created_at(),
    ]),
    title_template: "Reason: {reason}".to_string(),
    operations: SettingOperations::to_view_create_delete_op(LockdownExecutor),
});

#[derive(Clone)]
pub struct LockdownExecutor;

#[async_trait]
impl SettingView<SettingsData> for LockdownExecutor {
    async fn view<'a>(
        &self,
        context: &SettingsData,
        _filters: indexmap::IndexMap<String, Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error> {
        check_perms(context,"lockdowns.view".into()).await?;

        let rows = sqlx::query!("SELECT id, data, type, reason, created_at FROM lockdown__guild_lockdowns WHERE guild_id = $1", context.guild_id.to_string())
            .fetch_all(&context.data.pool)
            .await
            .map_err(|e| format!("Error while fetching lockdowns: {}", e))?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "id".to_string() => Value::String(row.id.to_string()),
                "guild_id".to_string() => Value::String(context.guild_id.to_string()),
                "data".to_string() => row.data,
                "type".to_string() => Value::String(row.r#type),
                "reason".to_string() => Value::String(row.reason),
                "created_at".to_string() => Value::String(row.created_at.to_string()),
            };

            result.push(map);
        }
        
        Ok(result) // TODO: Implement
    }
}

#[async_trait]
impl SettingCreator<SettingsData> for LockdownExecutor {
    async fn create<'a>(
        &self,
        context: &SettingsData,
        entry: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error> {
        check_perms(context,"lockdowns.create".into()).await?;
    
        let Some(Value::String(typ)) = entry.get("type") else {
            return Err("Missing or invalid field: `type`".into());
        };

        let Some(Value::String(reason)) = entry.get("reason") else {
            return Err("Missing or invalid field: `reason`".into());
        };

        // Get the current lockdown set
        let mut lockdowns = lockdowns::LockdownSet::guild(context.guild_id, &context.data.pool)
            .await
            .map_err(|e| 
                format!("Error while fetching lockdown set: {:?}", e)
            )?;

        // Create the lockdown
        let lockdown_type =
            lockdowns::from_lockdown_mode_string(typ)
            .map_err(
                |e|
                format!(
                    "Invalid lockdown mode: {}.\n\nTIP: The following lockdown modes are supported: {}: {}", 
                    typ, 
                    {
                        let mut supported_lockdown_modes = String::new();

                        for mode in lockdowns::CREATE_LOCKDOWN_MODES.iter() {
                            let creator = mode.value();
                            supported_lockdown_modes.push_str(&format!("\n- {}", creator.syntax()));
                        }

                        supported_lockdown_modes
                    },
                    e
            )
        )?;

        let lockdown_data = lockdowns::LockdownData {
            cache: &context.serenity_context.cache,
            http: &context.serenity_context.http,
            pool: context.data.pool.clone(),
            reqwest: context.data.reqwest.clone(),
        };

        lockdowns
            .easy_apply(lockdown_type, &lockdown_data, reason)
            .await
            .map_err(|e| format!("Error while applying lockdown: {}", e))?;

        let created_lockdown =
        lockdowns
            .lockdowns
            .last()
            .ok_or_else(|| "No lockdowns created!".to_string())?;
        

        Ok(indexmap::indexmap! {
            "id".to_string() => Value::String(created_lockdown.id.to_string()),
            "reason".to_string() => Value::String(created_lockdown.reason.clone()),
            "type".to_string() => Value::String(created_lockdown.r#type.string_form()),
            "data".to_string() => created_lockdown.data.clone(),
        })
    }
}

#[async_trait]
impl SettingDeleter<SettingsData> for LockdownExecutor {
    async fn delete<'a>(
        &self,
        context: &SettingsData,
        primary_key: Value,
    ) -> Result<(), Error> {
        check_perms(context,"lockdowns.delete".into()).await?;
                
        let Value::String(primary_key) = primary_key else {
            return Err("Missing or invalid field: `id`".into());
        };

        let primary_key = primary_key.parse::<uuid::Uuid>().map_err(|e| format!("Failed to parse ID: {:?}", e))?;

        // Get the current lockdown set
        let mut lockdowns = lockdowns::LockdownSet::guild(context.guild_id, &context.data.pool)
            .await
            .map_err(|e| format!("Error while fetching lockdown set: {}", e))?;

        let lockdown_data = lockdowns::LockdownData {
            cache: &context.serenity_context.cache,
            http: &context.serenity_context.http,
            pool: context.data.pool.clone(),
            reqwest: context.data.reqwest.clone(),
        };        

        // Remove the lockdown
        lockdowns
            .easy_remove(primary_key, &lockdown_data)
            .await
            .map_err(|e| format!("Error while removing lockdown: {}", e))?;

        Ok(())
    }
}
