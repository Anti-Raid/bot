use ar_settings::types::{
    settings_wrap, Column, ColumnSuggestion, ColumnType, HookContext, InnerColumnType,
    InnerColumnTypeStringKind, OperationType, Setting, SettingOperations, SettingsError,
};
use ar_settings::types::{
    SettingCreator, SettingDeleter, SettingUpdater, SettingView, SettingsData,
};
use kittycat::perms::Permission;
use splashcore_rs::value::Value;
use std::sync::LazyLock;
use async_trait::async_trait;

async fn check_perms(
    ctx: &HookContext<'_>,
    perm: &kittycat::perms::Permission,
) -> Result<(), SettingsError> {
    crate::botlib::permission_checks::member_has_kittycat_perm(
        ctx.guild_id,
        ctx.author,
        &ctx.data.data.pool,
        &ctx.data.serenity_context,
        &ctx.data.data.reqwest,
        &None,
        perm,
    )
    .await
    .map_err(|e| SettingsError::Generic {
        message: format!("Failed to check permissions: {:?}", e),
        src: "check_perms".to_string(),
        typ: "internal".to_string(),
    })
}

pub static GUILD_ROLES: LazyLock<Setting> = LazyLock::new(|| {
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
                    kind: InnerColumnTypeStringKind::Role {},
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
                    kind: InnerColumnTypeStringKind::KittycatPermission {},
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
                    kind: InnerColumnTypeStringKind::KittycatPermission {},
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
        operations: GuildRolesExecutor.into(),
    }
});

#[derive(Clone)]
pub struct GuildRolesExecutor;

#[async_trait::async_trait]
impl SettingView for GuildRolesExecutor {
    async fn view<'a>(
        &self,
        context: HookContext<'a>,
        _filters: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, splashcore_rs::value::Value>>, SettingsError> {
        let rows = sqlx::query!("SELECT role_id, perms, index, display_name, created_at, created_by, last_updated_at, last_updated_by FROM guild_roles WHERE guild_id = $1", context.guild_id.to_string())
        .fetch_all(&context.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching guild roles: {}", e),
            src: "GuildRolesExecutor".to_string(),
            typ: "value_error".to_string(),
        })?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "guild_id".to_string() => Value::String(context.guild_id.to_string()),
                "role_id".to_string() => Value::String(row.role_id),
                "perms".to_string() => Value::List(row.perms.iter().map(|x| Value::String(x.to_string())).collect()),
                "index".to_string() => Value::Integer(row.index.into()),
                "display_name".to_string() => row.display_name.map(Value::String).unwrap_or(Value::None),
                "created_at".to_string() => Value::TimestampTz(row.created_at),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::TimestampTz(row.last_updated_at),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator for GuildRolesExecutor {
    async fn create<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_roles.create".into()).await?;

        let res = self
            .base_verify_checks(&ctx, &entry, OperationType::Create)
            .await?;

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_roles WHERE guild_id = $1 AND role_id = $2",
            ctx.guild_id.to_string(),
            res.role_id.to_string()
        )
        .fetch_one(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to check if role exists: {:?}", e),
            src: "GuildRolesExecutor->create".to_string(),
            typ: "internal".to_string(),
        })?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err(SettingsError::Generic {
                message: "Role already exists".to_string(),
                src: "GuildRolesExecutor->create".to_string(),
                typ: "internal".to_string(),
            });
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
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to insert role: {:?}", e),
            src: "GuildRolesExecutor->create".to_string(),
            typ: "internal".to_string(),
        })?;

        sqlx::query!(
            "UPDATE guild_members SET needs_perm_rederive = true WHERE guild_id = $1 AND $2 = ANY(roles)",
            ctx.guild_id.to_string(),
            res.role_id.to_string()
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to update guild members cache: {:?}", e),
            src: "GuildRolesExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater for GuildRolesExecutor {
    async fn update<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_roles.update".into()).await?;

        let res = self
            .base_verify_checks(&ctx, &entry, OperationType::Update)
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
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to update role: {:?}", e),
            src: "GuildRolesExecutor->update".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter for GuildRolesExecutor {
    async fn delete<'a>(
        &self,
        ctx: HookContext<'a>,
        primary_key: splashcore_rs::value::Value,
    ) -> Result<(), SettingsError> {
        check_perms(&ctx, &"guild_roles.delete".into()).await?;

        let Some(row) = sqlx::query!("SELECT role_id, perms, index, display_name FROM guild_roles WHERE guild_id = $1 AND role_id = $2", ctx.guild_id.to_string(), primary_key.to_string())
        .fetch_optional(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching roles: {}", e),
            src: "GuildRolesExecutor".to_string(),
            typ: "value_error".to_string(),
        })? else {
            return Err(SettingsError::RowDoesNotExist {
                column_id: "role_id".to_string(),
            });
        };

        let entry = indexmap::indexmap! {
            "guild_id".to_string() => Value::String(ctx.guild_id.to_string()),
            "role_id".to_string() => Value::String(row.role_id),
            "perms".to_string() => Value::List(row.perms.iter().map(|x| Value::String(x.to_string())).collect()),
            "index".to_string() => Value::Integer(row.index.into()),
            "display_name".to_string() => row.display_name.map(Value::String).unwrap_or(Value::None),
        };

        let res = self
            .base_verify_checks(&ctx, &entry, OperationType::Delete)
            .await?;

        sqlx::query!(
            "DELETE FROM guild_roles WHERE guild_id = $1 AND role_id = $2",
            ctx.guild_id.to_string(),
            res.role_id.to_string()
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to delete role: {:?}", e),
            src: "GuildRolesExecutor->delete".to_string(),
            typ: "internal".to_string(),
        })?;

        sqlx::query!(
            "UPDATE guild_members SET needs_perm_rederive = true WHERE guild_id = $1 AND $2 = ANY(roles)",
            ctx.guild_id.to_string(),
            res.role_id.to_string()
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to update guild members cache: {:?}", e),
            src: "GuildRolesExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

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
        ctx: &HookContext<'_>,
        state: &indexmap::IndexMap<String, Value>,
        op: OperationType,
    ) -> Result<GreBaseVerifyChecksResult, SettingsError> {
        let parsed_value = if let Some(new_index_val) = state.get("index") {
            match new_index_val {
                Value::Integer(new_index) => Value::Integer(*new_index),
                Value::None => Value::None,
                _ => {
                    return Err(SettingsError::MissingOrInvalidField {
                        field: "index".to_string(),
                        src: "base_verify_checks".to_string(),
                    });
                }
            }
        } else {
            Value::None
        };

        // Get the index to set to
        let new_index = match parsed_value {
            Value::Integer(new_index_val) => {
                new_index_val
                    .try_into()
                    .map_err(|e| SettingsError::Generic {
                        message: format!("Failed to parse index: {:?}", e),
                        src: "base_verify_checks->match parsed_value".to_string(),
                        typ: "internal".to_string(),
                    })?
            }
            Value::None => {
                let highest_index_rec = sqlx::query!(
                    "SELECT MAX(index) FROM guild_roles WHERE guild_id = $1",
                    ctx.guild_id.to_string()
                )
                .fetch_one(&ctx.data.data.pool)
                .await
                .map_err(|e| SettingsError::Generic {
                    message: format!("Failed to get highest index: {:?}", e),
                    src: "base_verify_checks->match parsed_value".to_string(),
                    typ: "internal".to_string(),
                })?
                .max
                .unwrap_or(0);

                let index: i32 = highest_index_rec + 1;

                index
            }
            _ => {
                return Err(SettingsError::MissingOrInvalidField {
                    field: "index".to_string(),
                    src: "base_verify_checks->match parsed_value, _ result".to_string(),
                });
            }
        };

        let Some(Value::String(settings_role_id_str)) = state.get("role_id") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "role_id".to_string(),
                src: "base_verify_checks".to_string(),
            });
        };

        let settings_role_id = settings_role_id_str
            .parse::<serenity::all::RoleId>()
            .map_err(|e| SettingsError::Generic {
                message: format!(
                    "Failed to parse role id despite already having parsed it: {:?}",
                    e
                ),
                src: "base_verify_checks".to_string(),
                typ: "internal".to_string(),
            })?;

        // Get the new permissions as a Vec<String>
        let Some(Value::List(perms_value)) = state.get("perms") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "perms".to_string(),
                src: "index->NativeAction [default_pre_checks]".to_string(),
            });
        };

        let mut perms = Vec::with_capacity(perms_value.len());

        for perm in perms_value {
            if let Value::String(perm) = perm {
                perms.push(perm.to_string());
            } else {
                return Err(SettingsError::Generic {
                    message: "Failed to parse permissions".to_string(),
                    src: "NativeAction->index".to_string(),
                    typ: "internal".to_string(),
                });
            }
        }

        let display_name = if let Some(Value::String(display_name)) = state.get("display_name") {
            Some(display_name.to_string())
        } else {
            None
        };

        let guild = sandwich_driver::guild(
            &ctx.data.serenity_context.cache,
            &ctx.data.serenity_context.http,
            &ctx.data.data.reqwest,
            ctx.guild_id,
        )
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to get guild: {:?}", e),
            src: "base_verify_checks".to_string(),
            typ: "internal".to_string(),
        })?;

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
            &ctx.data.serenity_context.cache,
            &ctx.data.serenity_context.http,
            &ctx.data.data.reqwest,
            ctx.guild_id,
            ctx.author,
        )
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to get member: {:?}", e),
            src: "NativeAction->index".to_string(),
            typ: "internal".to_string(),
        })?
        else {
            return Err(SettingsError::Generic {
                message: "You must be in the guild to run this command!".to_string(),
                src: "NativeAction->index".to_string(),
                typ: "internal".to_string(),
            });
        };

        let current_roles = {
            let query = sqlx::query!(
                "SELECT index, role_id, perms FROM guild_roles WHERE guild_id = $1",
                ctx.guild_id.to_string()
            )
            .fetch_all(&ctx.data.data.pool)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Failed to get current role configuration: {:?}", e),
                src: "NativeAction->index".to_string(),
                typ: "internal".to_string(),
            })?;

            query
                .into_iter()
                .map(|x| (x.role_id, (x.index, x.perms)))
                .collect::<std::collections::HashMap<String, (i32, Vec<String>)>>()
        };

        let Some(first_role) = member.roles.first() else {
            return Err(SettingsError::Generic {
                message: "You must have at least one role to run this command!".to_string(),
                src: "NativeAction->index".to_string(),
                typ: "internal".to_string(),
            });
        };

        let Some(first_role) = guild.roles.get(first_role) else {
            return Err(SettingsError::Generic {
                message: "Could not find your first role".to_string(),
                src: "NativeAction->index".to_string(),
                typ: "internal".to_string(),
            });
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
            return Err(SettingsError::Generic {
                message: "You do not have any Anti-Raid configured roles yet!".to_string(),
                src: "base_verify_checks".to_string(),
                typ: "index_check".to_string(),
            });
        };

        let Some(settings_role) = guild.roles.get(&settings_role_id) else {
            return Err(SettingsError::SchemaCheckValidationError {
                column: "role_id".to_string(),
                check: "find_role_id_in_guild".to_string(),
                error: "Could not find role in guild".to_string(),
                accepted_range: "Any role in the guild".to_string(),
            });
        };

        if highest_role <= settings_role {
            return Err(SettingsError::Generic {
                message: "You do not have permission to edit this role's permissions as they are higher than you".to_string(),
                src: "NativeAction->index".to_string(),
                typ: "internal".to_string(),
            });
        }

        let author_kittycat_perms = silverpelt::member_permission_calc::get_kittycat_perms(
            &mut *ctx
                .data
                .data
                .pool
                .acquire()
                .await
                .map_err(|e| SettingsError::Generic {
                    message: format!("Failed to get pool: {:?}", e),
                    src: "GuildMembersExecutor".to_string(),
                    typ: "internal".to_string(),
                })?,
            ctx.guild_id,
            guild.owner_id,
            ctx.author,
            &member.roles,
        )
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to get author permissions: {:?}", e),
            src: "NativeAction->index".to_string(),
            typ: "internal".to_string(),
        })?;

        if new_index < lowest_index {
            return Err(SettingsError::Generic {
                message: format!("You do not have permission to edit this role's permissions as the new index would be lower than you: {} < {}", new_index, lowest_index),
                src: "NativeAction->index".to_string(),
                typ: "internal".to_string(),
            });
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
                .map_err(|e| SettingsError::Generic {
                    message: format!(
                        "You do not have permission to add a role with these permissions: {}",
                        e
                    ),
                    src: "base_verify_checks".to_string(),
                    typ: "perm_check_failed".to_string(),
                })?;
            }
            OperationType::Update => {
                let Some((index, current_perms)) = current_roles.get(settings_role_id_str.as_str())
                else {
                    return Err(SettingsError::Generic {
                        message: "Could not find role in guild".to_string(),
                        src: "NativeAction->index".to_string(),
                        typ: "internal".to_string(),
                    });
                };

                if *index < lowest_index {
                    return Err(SettingsError::Generic {
                        message: format!("You do not have permission to edit this role's permissions as the current index is lower than you: {} < {}", *index, lowest_index),
                        src: "base_verify_checks".to_string(),
                        typ: "internal".to_string(),
                    });
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
                .map_err(|e| SettingsError::Generic {
                    message: format!(
                        "You do not have permission to edit this role's permissions: {}",
                        e
                    ),
                    src: "base_verify_checks".to_string(),
                    typ: "perm_check_failed".to_string(),
                })?;
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
                .map_err(|e| SettingsError::Generic {
                    message: format!(
                        "You do not have permission to remove a role with these permissions: {}",
                        e
                    ),
                    src: "base_verify_checks".to_string(),
                    typ: "perm_check_failed".to_string(),
                })?;
            }
            _ => {
                return Err(SettingsError::OperationNotSupported { operation: op });
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

pub static GUILD_MEMBERS: LazyLock<Setting> = LazyLock::new(|| Setting {
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
                kind: InnerColumnTypeStringKind::User {},
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
                kind: InnerColumnTypeStringKind::KittycatPermission {},
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
    operations: GuildMembersExecutor.into(),
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
        conn: &mut sqlx::PgConnection,
        guild_id: serenity::all::GuildId,
        guild_owner_id: serenity::all::UserId,
        user_id: serenity::all::UserId,
    ) -> Result<(Vec<serenity::all::RoleId>, Vec<kittycat::perms::Permission>), SettingsError> {
        let Some(member) = sandwich_driver::member_in_guild(
            &data.serenity_context.cache,
            &data.serenity_context.http,
            &data.data.reqwest,
            guild_id,
            user_id,
        )
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to get user {}: {:?}", user_id, e),
            src: "GuildMembersExecutor".to_string(),
            typ: "internal".to_string(),
        })?
        else {
            return Ok((Vec::new(), Vec::new()));
        };

        let kittycat_perms = silverpelt::member_permission_calc::get_kittycat_perms(
            &mut *conn,
            guild_id,
            guild_owner_id,
            user_id,
            &member.roles,
        )
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to get user permissions: {:?} ({})", e, user_id),
            src: "GuildMembersExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        let roles = member
            .roles
            .iter()
            .copied()
            .collect::<Vec<serenity::all::RoleId>>();

        Ok((roles, kittycat_perms))
    }

    async fn verify(
        &self,
        ctx: &HookContext<'_>,
        state: &indexmap::IndexMap<String, Value>,
        op: OperationType,
    ) -> Result<GmeBaseVerifyChecksResult, SettingsError> {
        // Get the user id as this is required for all operations
        let Some(Value::String(user_id)) = state.get("user_id") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "user_id".to_string(),
                src: "guildmembers->user_id".to_string(),
            });
        };

        // Parse the user id
        let user_id: serenity::all::UserId =
            user_id.parse().map_err(|e| SettingsError::Generic {
                message: format!("Failed to parse user id: {:?}", e),
                src: "guildmembers->user_id".to_string(),
                typ: "external".to_string(),
            })?;

        let Some(Value::Boolean(public)) = state.get("public") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "public".to_string(),
                src: "guildmembers->public".to_string(),
            });
        };

        if op == OperationType::Update {
            let current_public = sqlx::query!(
                "SELECT public FROM guild_members WHERE guild_id = $1 AND user_id = $2",
                ctx.guild_id.to_string(),
                user_id.to_string()
            )
            .fetch_one(&ctx.data.data.pool)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Failed to get current public status: {:?}", e),
                src: "GuildMembersExecutor".to_string(),
                typ: "internal".to_string(),
            })?
            .public;

            if *public != current_public && ctx.author != user_id {
                return Err(SettingsError::Generic {
                    message: "Only the user can change their public status".to_string(),
                    src: "guildmembers->public".to_string(),
                    typ: "external".to_string(),
                });
            }
        }

        // Get perm overrides
        let perm_overrides = {
            let Some(Value::List(perm_overrides_value)) = state.get("perm_overrides") else {
                return Err(SettingsError::MissingOrInvalidField {
                    field: "perm_overrides".to_string(),
                    src: "guildmembers->perm_overrides".to_string(),
                });
            };

            let mut perm_overrides = Vec::with_capacity(perm_overrides_value.len());

            for perm in perm_overrides_value {
                if let Value::String(perm) = perm {
                    perm_overrides.push(kittycat::perms::Permission::from_string(perm));
                } else {
                    return Err(SettingsError::Generic {
                        message: "Failed to parse permissions".to_string(),
                        src: "NativeAction->index".to_string(),
                        typ: "internal".to_string(),
                    });
                }
            }

            perm_overrides
        };

        let guild = sandwich_driver::guild(
            &ctx.data.serenity_context.cache,
            &ctx.data.serenity_context.http,
            &ctx.data.data.reqwest,
            ctx.guild_id,
        )
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to get guild: {:?}", e),
            src: "NativeAction->index".to_string(),
            typ: "internal".to_string(),
        })?;

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
                    ctx.data,
                    &mut *ctx.data.data.pool.acquire().await.map_err(|e| {
                        SettingsError::Generic {
                            message: format!("Failed to get pool: {:?}", e),
                            src: "GuildMembersExecutor".to_string(),
                            typ: "internal".to_string(),
                        }
                    })?,
                    ctx.guild_id,
                    guild.owner_id,
                    ctx.author,
                )
                .await
            {
                Ok((_, author_kittycat_perms)) => author_kittycat_perms,
                Err(e) => {
                    return Err(SettingsError::Generic {
                        message: format!("Failed to get author permissions: {:?}", e),
                        src: "GuildMembersExecutor".to_string(),
                        typ: "internal".to_string(),
                    })
                }
            };

        // Get the target members current kittycat permissions (if any) as well as their roles (for finding new permissions with overrides taken into account)
        let (target_member_roles, current_user_kittycat_perms) =
            match self
                .get_kittycat_perms_for_user(
                    ctx.data,
                    &mut *ctx.data.data.pool.acquire().await.map_err(|e| {
                        SettingsError::Generic {
                            message: format!("Failed to get pool: {:?}", e),
                            src: "GuildMembersExecutor".to_string(),
                            typ: "internal".to_string(),
                        }
                    })?,
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
                    return Err(SettingsError::Generic {
                        message: format!("Failed to get target member permissions: {:?}", e),
                        src: "GuildMembersExecutor".to_string(),
                        typ: "internal".to_string(),
                    })
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
                    &mut *ctx.data.data.pool.acquire().await.map_err(|e| {
                        SettingsError::Generic {
                            message: format!("Failed to get pool: {:?}", e),
                            src: "GuildMembersExecutor".to_string(),
                            typ: "internal".to_string(),
                        }
                    })?,
                    ctx.guild_id,
                    &roles_str,
                )
                .await
                .map_err(|e| SettingsError::Generic {
                    message: format!("Failed to get user positions: {:?}", e),
                    src: "GuildMembersExecutor".to_string(),
                    typ: "internal".to_string(),
                })?;

            silverpelt::member_permission_calc::rederive_perms_impl(
                ctx.guild_id,
                user_id,
                user_positions,
                perm_overrides.clone(),
            )
        };

        // Check permissions
        match op {
            OperationType::Create => {
                kittycat::perms::check_patch_changes(
                    &author_kittycat_perms,
                    &[],
                    &new_user_kittycat_perms,
                )
                .map_err(|e| SettingsError::Generic {
                    message: format!(
                        "You do not have permission to add a role with these permissions: {}",
                        e
                    ),
                    src: "GuildMembersExecutor".to_string(),
                    typ: "perm_check_failed".to_string(),
                })?;
            }
            OperationType::Update => {
                kittycat::perms::check_patch_changes(
                    &author_kittycat_perms,
                    &current_user_kittycat_perms,
                    &new_user_kittycat_perms,
                )
                .map_err(|e| SettingsError::Generic {
                    message: format!(
                        "You do not have permission to edit this role's permissions: {}",
                        e
                    ),
                    src: "GuildMembersExecutor".to_string(),
                    typ: "perm_check_failed".to_string(),
                })?;
            }
            OperationType::Delete => {
                kittycat::perms::check_patch_changes(
                    &author_kittycat_perms,
                    &current_user_kittycat_perms,
                    &[],
                )
                .map_err(|e| SettingsError::Generic {
                    message: format!(
                        "You do not have permission to remove this members permission overrides: {}",
                        e
                    ),
                    src: "GuildMembersExecutor".to_string(),
                    typ: "perm_check_failed".to_string(),
                })?;
            }
            _ => {
                return Err(SettingsError::OperationNotSupported { operation: op });
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
impl SettingView for GuildMembersExecutor {
    async fn view<'a>(
        &self,
        context: HookContext<'a>,
        _filters: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, splashcore_rs::value::Value>>, SettingsError> {
        let rows = sqlx::query!("SELECT user_id, perm_overrides, public, created_at FROM guild_members WHERE guild_id = $1", context.guild_id.to_string())
        .fetch_all(&context.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching guild roles: {}", e),
            src: "GuildRolesExecutor".to_string(),
            typ: "value_error".to_string(),
        })?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "user_id".to_string() => Value::String(row.user_id),
                "perm_overrides".to_string() => Value::List(row.perm_overrides.iter().map(|x| Value::String(x.to_string())).collect()),
                "public".to_string() => Value::Boolean(row.public),
                "created_at".to_string() => Value::TimestampTz(row.created_at),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator for GuildMembersExecutor {
    async fn create<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_members.create".into()).await?;

        let res = self.verify(&ctx, &entry, OperationType::Create).await?;

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_members WHERE guild_id = $1 AND user_id = $2",
            ctx.guild_id.to_string(),
            res.user_id.to_string()
        )
        .fetch_one(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to check if role exists: {:?}", e),
            src: "GuildRolesExecutor->create".to_string(),
            typ: "internal".to_string(),
        })?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err(SettingsError::Generic {
                message: "Role already exists".to_string(),
                src: "GuildRolesExecutor->create".to_string(),
                typ: "internal".to_string(),
            });
        }

        sqlx::query!(
            "INSERT INTO guild_members (guild_id, user_id, perm_overrides, public) VALUES ($1, $2, $3, $4)",
            ctx.guild_id.to_string(),
            res.user_id.to_string(),
            &res.perm_overrides.into_iter().map(|x| x.to_string()).collect::<Vec<String>>(),
            res.public
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to insert role: {:?}", e),
            src: "GuildRolesExecutor->create".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater for GuildMembersExecutor {
    async fn update<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_members.update".into()).await?;

        let res = self.verify(&ctx, &entry, OperationType::Update).await?;

        sqlx::query!(
            "UPDATE guild_members SET perm_overrides = $1, public = $2 WHERE guild_id = $3 AND user_id = $4",
            &res.perm_overrides.into_iter().map(|x| x.to_string()).collect::<Vec<String>>(),
            res.public,
            ctx.guild_id.to_string(),
            res.user_id.to_string()
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to update role: {:?}", e),
            src: "GuildRolesExecutor->update".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter for GuildMembersExecutor {
    async fn delete<'a>(
        &self,
        ctx: HookContext<'a>,
        primary_key: splashcore_rs::value::Value,
    ) -> Result<(), SettingsError> {
        check_perms(&ctx, &"guild_members.delete".into()).await?;

        let Some(row) = sqlx::query!("SELECT user_id, perm_overrides, public FROM guild_members WHERE guild_id = $1 AND user_id = $2", ctx.guild_id.to_string(), primary_key.to_string())
        .fetch_optional(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching roles: {}", e),
            src: "GuildRolesExecutor".to_string(),
            typ: "value_error".to_string(),
        })? else {
            return Err(SettingsError::RowDoesNotExist {
                column_id: "user_id".to_string(),
            });
        };

        let entry = indexmap::indexmap! {
            "guild_id".to_string() => Value::String(ctx.guild_id.to_string()),
            "user_id".to_string() => Value::String(row.user_id),
            "perm_overrides".to_string() => Value::List(row.perm_overrides.iter().map(|x| Value::String(x.to_string())).collect()),
            "public".to_string() => Value::Boolean(row.public),
        };

        let res = self.verify(&ctx, &entry, OperationType::Delete).await?;

        sqlx::query!(
            "DELETE FROM guild_members WHERE guild_id = $1 AND user_id = $2",
            ctx.guild_id.to_string(),
            res.user_id.to_string()
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to delete role: {:?}", e),
            src: "GuildRolesExecutor->delete".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(())
    }
}

pub static GUILD_TEMPLATES: LazyLock<Setting> = LazyLock::new(|| {
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
                    kind: InnerColumnTypeStringKind::Normal {},
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
                id: "content".to_string(),
                name: "Content".to_string(),
                description: "The content of the template".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: InnerColumnTypeStringKind::Textarea {
                        ctx: "template".to_string(),
                    },
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
                column_type: ColumnType::new_array(InnerColumnType::String { min_length: None, max_length: None, allowed_values: vec![], kind: InnerColumnTypeStringKind::Normal {} }),
                nullable: true,
                suggestions: ColumnSuggestion::Static { suggestions: gwevent::core::event_list().iter().copied().map(|x| x.to_string()).collect() },
                ignored_for: vec![],
                secret: false,
            },
            Column {
                id: "error_channel".to_string(),
                name: "Error Channel".to_string(),
                description: "The channel to report errors to. If not specified, an Error event will be omitted instead".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: InnerColumnTypeStringKind::Channel {
                        needed_bot_permissions: serenity::all::Permissions::SEND_MESSAGES,
                        allowed_channel_types: vec![]
                    },
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
        operations: GuildTemplateExecutor.into(),
    }
});

#[derive(Clone)]
pub struct GuildTemplateExecutor;

impl GuildTemplateExecutor {
    async fn validate(&self, ctx: &HookContext<'_>, name: &str) -> Result<(), SettingsError> {
        if name.starts_with("$shop/") {
            let (shop_tname, shop_tversion) = silverpelt::templates::parse_shop_template(name)
                .map_err(|e| SettingsError::Generic {
                    message: format!("Failed to parse shop template: {:?}", e),
                    src: "guild_templates->name".to_string(),
                    typ: "external".to_string(),
                })?;

            let shop_template = sqlx::query!(
                "SELECT COUNT(*) FROM template_shop WHERE name = $1 AND version = $2",
                shop_tname,
                shop_tversion
            )
            .fetch_one(&ctx.data.data.pool)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Failed to get shop template: {:?}", e),
                src: "guild_templates->name".to_string(),
                typ: "internal".to_string(),
            })?;

            if shop_template.count.unwrap_or_default() == 0 {
                return Err(SettingsError::Generic {
                    message: "Could not find shop template".to_string(),
                    src: "guild_templates->name".to_string(),
                    typ: "external".to_string(),
                });
            }
        }

        Ok(())
    }

    async fn post_action(&self, ctx: &HookContext<'_>, name: &str) -> Result<(), SettingsError> {
        // Dispatch a OnStartup event for the template
        silverpelt::ar_event::AntiraidEvent::OnStartup(vec![name.to_string()])
            .dispatch_to_template_worker_and_nowait(&ctx.data.data, ctx.guild_id)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Failed to dispatch OnStartup event: {:?}", e),
                src: "GuildTemplateExecutor".to_string(),
                typ: "internal".to_string(),
            })?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl SettingView for GuildTemplateExecutor {
    async fn view<'a>(
        &self,
        context: HookContext<'a>,
        _filters: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, splashcore_rs::value::Value>>, SettingsError> {
        log::info!("Viewing guild templates for guild id: {}", context.guild_id);

        check_perms(&context, &"guild_templates.view".into()).await?;

        let rows = sqlx::query!("SELECT name, content, events, error_channel, created_at, created_by, last_updated_at, last_updated_by FROM guild_templates WHERE guild_id = $1", context.guild_id.to_string())
        .fetch_all(&context.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching guild templates: {}", e),
            src: "GuildTemplateExecutor".to_string(),
            typ: "value_error".to_string(),
        })?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "name".to_string() => Value::String(row.name),
                "content".to_string() => Value::String(row.content),
                "events".to_string() => {
                    Value::List(row.events.iter().map(|x| Value::String(x.to_string())).collect())
                },
                "error_channel".to_string() => {
                    match row.error_channel {
                        Some(error_channel) => Value::String(error_channel),
                        None => Value::None,
                    }
                },
                "created_at".to_string() => Value::TimestampTz(row.created_at),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::TimestampTz(row.last_updated_at),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator for GuildTemplateExecutor {
    async fn create<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_templates.create".into()).await?;

        let Some(Value::String(name)) = entry.get("name") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "content".to_string(),
                src: "GuildTemplateExecutor".to_string(),
            });
        };

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_templates WHERE guild_id = $1 AND name = $2",
            ctx.guild_id.to_string(),
            name
        )
        .fetch_one(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to check if template exists: {:?}", e),
            src: "GuildTemplateExecutor".to_string(),
            typ: "internal".to_string(),
        })?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err(SettingsError::Generic {
                message: "Template already exists".to_string(),
                src: "GuildTemplateExecutor".to_string(),
                typ: "internal".to_string(),
            });
        }

        self.validate(&ctx, name).await?;

        let Some(Value::String(content)) = entry.get("content") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "content".to_string(),
                src: "GuildTemplateExecutor".to_string(),
            });
        };

        let events = match entry.get("events") {
            Some(Value::List(events)) => Some(
                events
                    .iter()
                    .map(|x| {
                        if let Value::String(x) = x {
                            Ok(x.to_string())
                        } else {
                            Err(SettingsError::Generic {
                                message: "Failed to parse events".to_string(),
                                src: "GuildTemplateExecutor".to_string(),
                                typ: "internal".to_string(),
                            })
                        }
                    })
                    .collect::<Result<Vec<String>, SettingsError>>()?,
            ),
            _ => None,
        };

        let error_channel = match entry.get("error_channel") {
            Some(Value::String(error_channel)) => Some(error_channel.to_string()),
            _ => None,
        };

        sqlx::query!(
            "INSERT INTO guild_templates (guild_id, name, content, events, error_channel, created_by, last_updated_by) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            ctx.guild_id.to_string(),
            name,
            content,
            events.as_deref(),
            error_channel,
            ctx.author.to_string(),
            ctx.author.to_string()
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to insert template: {:?}", e),
            src: "GuildTemplateExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        self.post_action(&ctx, name).await?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater for GuildTemplateExecutor {
    async fn update<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_templates.update".into()).await?;

        let Some(Value::String(name)) = entry.get("name") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "name".to_string(),
                src: "GuildTemplateExecutor".to_string(),
            });
        };

        self.validate(&ctx, name).await?;

        let Some(Value::String(content)) = entry.get("content") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "content".to_string(),
                src: "GuildTemplateExecutor".to_string(),
            });
        };

        let events = match entry.get("events") {
            Some(Value::List(events)) => Some(
                events
                    .iter()
                    .map(|x| {
                        if let Value::String(x) = x {
                            Ok(x.to_string())
                        } else {
                            Err(SettingsError::Generic {
                                message: "Failed to parse events".to_string(),
                                src: "GuildTemplateExecutor".to_string(),
                                typ: "internal".to_string(),
                            })
                        }
                    })
                    .collect::<Result<Vec<String>, SettingsError>>()?,
            ),
            _ => None,
        };

        let error_channel = match entry.get("error_channel") {
            Some(Value::String(error_channel)) => Some(error_channel.to_string()),
            _ => None,
        };

        sqlx::query!(
            "UPDATE guild_templates SET content = $1, events = $2, last_updated_at = NOW(), last_updated_by = $3, error_channel = $4 WHERE guild_id = $5 AND name = $6",
            content,
            events.as_deref(),
            ctx.author.to_string(),
            error_channel,
            ctx.guild_id.to_string(),
            name
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to update template: {:?}", e),
            src: "GuildTemplateExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        self.post_action(&ctx, name).await?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter for GuildTemplateExecutor {
    async fn delete<'a>(
        &self,
        ctx: HookContext<'a>,
        primary_key: splashcore_rs::value::Value,
    ) -> Result<(), SettingsError> {
        check_perms(&ctx, &"guild_templates.delete".into()).await?;

        let Some(row) = sqlx::query!(
            "SELECT name FROM guild_templates WHERE guild_id = $1 AND name = $2",
            ctx.guild_id.to_string(),
            primary_key.to_string()
        )
        .fetch_optional(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching template: {}", e),
            src: "GuildTemplateExecutor".to_string(),
            typ: "value_error".to_string(),
        })?
        else {
            return Err(SettingsError::RowDoesNotExist {
                column_id: "name".to_string(),
            });
        };

        let name = row.name;

        sqlx::query!(
            "DELETE FROM guild_templates WHERE guild_id = $1 AND name = $2",
            ctx.guild_id.to_string(),
            name
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to delete template: {:?}", e),
            src: "GuildTemplateExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        self.post_action(&ctx, &name).await?;

        Ok(())
    }
}

pub static GUILD_TEMPLATES_KV: LazyLock<Setting> = LazyLock::new(|| Setting {
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
                kind: InnerColumnTypeStringKind::Normal {},
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
            name: "Content".to_string(),
            description: "The content of the template".to_string(),
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
    operations: GuildTemplatesKVExecutor.into(),
});

#[derive(Clone)]
pub struct GuildTemplatesKVExecutor;

#[async_trait::async_trait]
impl SettingView for GuildTemplatesKVExecutor {
    async fn view<'a>(
        &self,
        context: HookContext<'a>,
        _filters: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, splashcore_rs::value::Value>>, SettingsError> {
        check_perms(&context, &"guild_templates_kv.view".into()).await?;

        let rows = sqlx::query!("SELECT key, value, created_at, last_updated_at FROM guild_templates_kv WHERE guild_id = $1", context.guild_id.to_string())
        .fetch_all(&context.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching guild templates kv: {}", e),
            src: "GuildTemplatesKVExecutor".to_string(),
            typ: "value_error".to_string(),
        })?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "key".to_string() => Value::String(row.key),
                "value".to_string() => row.value.map(Value::Json).unwrap_or(Value::None),
                "created_at".to_string() => Value::TimestampTz(row.created_at),
                "last_updated_at".to_string() => Value::TimestampTz(row.last_updated_at),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator for GuildTemplatesKVExecutor {
    async fn create<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_templates_kv.create".into()).await?;

        let Some(Value::String(key)) = entry.get("key") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "key".to_string(),
                src: "GuildTemplatesKVExecutor".to_string(),
            });
        };

        let total_count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_templates_kv WHERE guild_id = $1",
            ctx.guild_id.to_string()
        )
        .fetch_one(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to check total kv count: {:?}", e),
            src: "GuildTemplatesKVExecutor".to_string(),
            typ: "internal".to_string(),
        })?
        .count
        .unwrap_or_default();

        if total_count
            >= silverpelt::templates::LuaKVConstraints::default()
                .max_keys
                .try_into()
                .map_err(|_| SettingsError::Generic {
                    message: "Failed to convert max_keys to i64".to_string(),
                    src: "GuildTemplatesKVExecutor".to_string(),
                    typ: "internal".to_string(),
                })?
        {
            return Err(SettingsError::Generic {
                message: "Max key-value pairs reached".to_string(),
                src: "GuildTemplatesKVExecutor".to_string(),
                typ: "internal".to_string(),
            });
        }

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM guild_templates_kv WHERE guild_id = $1 AND key = $2",
            ctx.guild_id.to_string(),
            key
        )
        .fetch_one(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to check if kv exists: {:?}", e),
            src: "GuildTemplatesKVExecutor".to_string(),
            typ: "internal".to_string(),
        })?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err(SettingsError::Generic {
                message: "KV already exists".to_string(),
                src: "GuildTemplatesKVExecutor".to_string(),
                typ: "internal".to_string(),
            });
        }

        let Some(Value::Json(value)) = entry.get("value") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "value".to_string(),
                src: "GuildTemplatesKVExecutor".to_string(),
            });
        };

        sqlx::query!(
            "INSERT INTO guild_templates_kv (guild_id, key, value, created_at, last_updated_at) VALUES ($1, $2, $3, NOW(), NOW())",
            ctx.guild_id.to_string(),
            key,
            value
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to insert kv: {:?}", e),
            src: "GuildTemplatesKVExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater for GuildTemplatesKVExecutor {
    async fn update<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_templates_kv.update".into()).await?;

        let Some(Value::String(key)) = entry.get("key") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "key".to_string(),
                src: "GuildTemplatesKVExecutor".to_string(),
            });
        };

        let Some(Value::Json(value)) = entry.get("value") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "value".to_string(),
                src: "GuildTemplatesKVExecutor".to_string(),
            });
        };

        sqlx::query!(
            "UPDATE guild_templates_kv SET value = $1, last_updated_at = NOW() WHERE guild_id = $2 AND key = $3",
            value,
            ctx.guild_id.to_string(),
            key
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to update kv: {:?}", e),
            src: "GuildTemplatesKVExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter for GuildTemplatesKVExecutor {
    async fn delete<'a>(
        &self,
        ctx: HookContext<'a>,
        primary_key: splashcore_rs::value::Value,
    ) -> Result<(), SettingsError> {
        check_perms(&ctx, &"guild_templates_kv.delete".into()).await?;

        if sqlx::query!(
            "SELECT COUNT(*) FROM guild_templates_kv WHERE guild_id = $1 AND key = $2",
            ctx.guild_id.to_string(),
            primary_key.to_string()
        )
        .fetch_one(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching kv: {}", e),
            src: "GuildTemplatesKVExecutor".to_string(),
            typ: "value_error".to_string(),
        })?
        .count
        .unwrap_or_default()
            <= 0
        {
            return Err(SettingsError::RowDoesNotExist {
                column_id: "key".to_string(),
            });
        };

        sqlx::query!(
            "DELETE FROM guild_templates_kv WHERE guild_id = $1 AND key = $2",
            ctx.guild_id.to_string(),
            primary_key.to_string()
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to delete kv: {:?}", e),
            src: "GuildTemplatesKVExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(())
    }
}

pub static GUILD_TEMPLATE_SHOP: LazyLock<Setting> = LazyLock::new(|| {
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
                column_type: ColumnType::new_scalar(InnerColumnType::Uuid {}),
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
                    kind: InnerColumnTypeStringKind::Normal {},
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
                    kind: InnerColumnTypeStringKind::Normal {},
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
                    kind: InnerColumnTypeStringKind::Normal {},
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
                    kind: InnerColumnTypeStringKind::Textarea {
                        ctx: "template".to_string(),
                    },
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
                id: "type".to_string(),
                name: "Type".to_string(),
                description: "The type of the template".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: InnerColumnTypeStringKind::Normal {},
                    min_length: None,
                    max_length: None,
                    allowed_values: vec!["hook".to_string(), "library".to_string()],
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
        title_template: "{name}".to_string(),
        operations: GuildTemplateShopExecutor.into(),
    }
});

#[derive(Clone)]
pub struct GuildTemplateShopExecutor;

#[async_trait::async_trait]
impl SettingView for GuildTemplateShopExecutor {
    async fn view<'a>(
        &self,
        context: HookContext<'a>,
        _filters: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, splashcore_rs::value::Value>>, SettingsError> {
        check_perms(&context, &"guild_templates_shop.view".into()).await?;

        let rows = sqlx::query!("SELECT id, name, version, description, type, created_at, created_by, last_updated_at, last_updated_by FROM template_shop WHERE owner_guild = $1", context.guild_id.to_string())
        .fetch_all(&context.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching shop templates: {}", e),
            src: "GuildTemplateShopExecutor".to_string(),
            typ: "value_error".to_string(),
        })?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "id".to_string() => Value::String(row.id.to_string()),
                "name".to_string() => Value::String(row.name),
                "version".to_string() => Value::String(row.version),
                "description".to_string() => Value::String(row.description),
                "type".to_string() => Value::String(row.r#type),
                "owner_guild".to_string() => Value::String(context.guild_id.to_string()),
                "created_at".to_string() => Value::TimestampTz(row.created_at),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::TimestampTz(row.last_updated_at),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl SettingCreator for GuildTemplateShopExecutor {
    async fn create<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_templates_shop.create".into()).await?;

        let Some(Value::String(name)) = entry.get("name") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "name".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
            });
        };

        // Rules for name:
        // Only namespaced templates can contain @ or /
        // Namespaced templates must use a namespace owned by the server
        // Namespaced templates must be in the format @namespace/<pkgname>. <pkgname> itself cannot contain '@' but may use '/'

        if !name.is_ascii() {
            return Err(SettingsError::Generic {
                message: "Name must be ASCII".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
                typ: "external".to_string(),
            });
        }

        if name.starts_with('@') {
            // This is a namespaced template, check that the server owns the namespace
            if !name.contains('/') {
                return Err(SettingsError::Generic {
                    message: "Please contact support to claim ownership over a specific namespace"
                        .to_string(),
                    src: "GuildTemplateShopExecutor".to_string(),
                    typ: "external".to_string(),
                });
            }

            let namespace = name.split('/').next().unwrap();
            let pkgname = name.replace(&format!("{}{}", namespace, "/"), "");

            if pkgname.contains("@") {
                return Err(SettingsError::Generic {
                    message: "Package name cannot contain '@'".to_string(),
                    src: "GuildTemplateShopExecutor".to_string(),
                    typ: "external".to_string(),
                });
            }

            let count = sqlx::query!(
                "SELECT COUNT(*) FROM template_shop WHERE owner_guild = $1 AND name = $2",
                ctx.guild_id.to_string(),
                namespace
            )
            .fetch_one(&ctx.data.data.pool)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Failed to check if namespace exists: {:?}", e),
                src: "GuildTemplateShopExecutor".to_string(),
                typ: "internal".to_string(),
            })?
            .count
            .unwrap_or_default();

            if count <= 0 {
                return Err(SettingsError::Generic {
                    message: "Namespace does not exist".to_string(),
                    src: "GuildTemplateShopExecutor".to_string(),
                    typ: "internal".to_string(),
                });
            }
        } else if name.contains('@') || name.contains('/') {
            return Err(SettingsError::Generic {
                message: "Name cannot contain '@' or '/' unless it is a namespace".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
                typ: "external".to_string(),
            });
        }

        let Some(Value::String(version)) = entry.get("version") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "version".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
            });
        };

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM template_shop WHERE owner_guild = $1 AND name = $2 AND version = $3",
            ctx.guild_id.to_string(),
            name,
            version
        )
        .fetch_one(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to check if shop template exists: {:?}", e),
            src: "GuildTemplateShopExecutor".to_string(),
            typ: "internal".to_string(),
        })?
        .count
        .unwrap_or_default();

        if count > 0 {
            return Err(SettingsError::Generic {
                message: "Shop template with this name and version already exists".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
                typ: "internal".to_string(),
            });
        }

        let Some(Value::String(description)) = entry.get("description") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "description".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
            });
        };

        let Some(Value::String(content)) = entry.get("content") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "content".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
            });
        };

        let Some(Value::String(r#type)) = entry.get("type") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "type".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
            });
        };

        let id = sqlx::query!(
            "INSERT INTO template_shop (name, version, description, content, type, owner_guild, created_by, last_updated_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
            name,
            version,
            description,
            content,
            r#type,
            ctx.guild_id.to_string(),
            ctx.author.to_string(),
            ctx.author.to_string()
        )
        .fetch_one(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to insert shop template: {:?}", e),
            src: "GuildTemplateShopExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        // Add returned ID to entry
        let mut entry = entry;
        entry.insert("id".to_string(), Value::Uuid(id.id));

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingUpdater for GuildTemplateShopExecutor {
    async fn update<'a>(
        &self,
        ctx: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&ctx, &"guild_templates_shop.update".into()).await?;

        let Some(Value::Uuid(id)) = entry.get("id") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "id".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
            });
        };

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM template_shop WHERE owner_guild = $1 AND id = $2",
            ctx.guild_id.to_string(),
            id
        )
        .fetch_one(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to check if shop template exists: {:?}", e),
            src: "GuildTemplateShopExecutor".to_string(),
            typ: "internal".to_string(),
        })?
        .count
        .unwrap_or_default();

        if count <= 0 {
            return Err(SettingsError::RowDoesNotExist {
                column_id: "id".to_string(),
            });
        }

        let Some(Value::String(description)) = entry.get("description") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "description".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
            });
        };

        let Some(Value::String(r#type)) = entry.get("type") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "type".to_string(),
                src: "GuildTemplateShopExecutor".to_string(),
            });
        };

        sqlx::query!(
            "UPDATE template_shop SET description = $1, type = $2, last_updated_at = NOW(), last_updated_by = $3 WHERE owner_guild = $4 AND id = $5",
            description,
            r#type,
            ctx.author.to_string(),
            ctx.guild_id.to_string(),
            id
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to update shop template: {:?}", e),
            src: "GuildTemplateShopExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(entry)
    }
}

#[async_trait::async_trait]
impl SettingDeleter for GuildTemplateShopExecutor {
    async fn delete<'a>(
        &self,
        ctx: HookContext<'a>,
        primary_key: splashcore_rs::value::Value,
    ) -> Result<(), SettingsError> {
        check_perms(&ctx, &"guild_templates_shop.delete".into()).await?;

        let primary_key = match primary_key {
            Value::Uuid(id) => id,
            Value::String(id) => id.parse().map_err(|e| SettingsError::Generic {
                message: format!("Failed to parse ID: {:?}", e),
                src: "GuildTemplateShopExecutor".to_string(),
                typ: "internal".to_string(),
            })?,
            _ => {
                return Err(SettingsError::MissingOrInvalidField {
                    field: "id".to_string(),
                    src: "GuildTemplateShopExecutor".to_string(),
                });
            }
        };

        let Some(row) = sqlx::query!(
            "SELECT id FROM template_shop WHERE owner_guild = $1 AND id = $2",
            ctx.guild_id.to_string(),
            primary_key
        )
        .fetch_optional(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching shop template: {}", e),
            src: "GuildTemplateShopExecutor".to_string(),
            typ: "value_error".to_string(),
        })?
        else {
            return Err(SettingsError::RowDoesNotExist {
                column_id: "id".to_string(),
            });
        };

        let id = row.id;

        sqlx::query!(
            "DELETE FROM template_shop WHERE owner_guild = $1 AND id = $2",
            ctx.guild_id.to_string(),
            id
        )
        .execute(&ctx.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Failed to delete shop template: {:?}", e),
            src: "GuildTemplateShopExecutor".to_string(),
            typ: "internal".to_string(),
        })?;

        Ok(())
    }
}

pub static GUILD_TEMPLATE_SHOP_PUBLIC_LIST: LazyLock<Setting> = LazyLock::new(|| {
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
                column_type: ColumnType::new_scalar(InnerColumnType::Uuid {}),
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
                    kind: InnerColumnTypeStringKind::Normal {},
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
                    kind: InnerColumnTypeStringKind::Normal {},
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
                    kind: InnerColumnTypeStringKind::Normal {},
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
                    kind: InnerColumnTypeStringKind::Textarea {
                        ctx: "template".to_string(),
                    },
                    min_length: None,
                    max_length: None,
                    allowed_values: vec![],
                }),
                nullable: false,
                suggestions: ColumnSuggestion::None {},
                ignored_for: vec![OperationType::Update, OperationType::View],
                secret: false,
            },
            Column {
                id: "type".to_string(),
                name: "Type".to_string(),
                description: "The type of the template".to_string(),
                column_type: ColumnType::new_scalar(InnerColumnType::String {
                    kind: InnerColumnTypeStringKind::Normal {},
                    min_length: None,
                    max_length: None,
                    allowed_values: vec!["hook".to_string(), "library".to_string()],
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
        title_template: "{name}".to_string(),
        operations: SettingOperations::to_view_op(GuildTemplateShopPublicListExecutor),
    }
});

#[derive(Clone)]
pub struct GuildTemplateShopPublicListExecutor;

#[async_trait::async_trait]
impl SettingView for GuildTemplateShopPublicListExecutor {
    async fn view<'a>(
        &self,
        context: HookContext<'a>,
        _filters: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, splashcore_rs::value::Value>>, SettingsError> {
        let rows = sqlx::query!("SELECT id, name, version, description, type, owner_guild, created_at, created_by, last_updated_at, last_updated_by FROM template_shop")
        .fetch_all(&context.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while fetching shop templates: {}", e),
            src: "GuildTemplateShopPublicListExecutor".to_string(),
            typ: "value_error".to_string(),
        })?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "id".to_string() => Value::String(row.id.to_string()),
                "name".to_string() => Value::String(row.name),
                "version".to_string() => Value::String(row.version),
                "description".to_string() => Value::String(row.description),
                "type".to_string() => Value::String(row.r#type),
                "owner_guild".to_string() => Value::String(row.owner_guild),
                "created_at".to_string() => Value::TimestampTz(row.created_at),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::TimestampTz(row.last_updated_at),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }

        Ok(result)
    }
}

pub static LOCKDOWN_SETTINGS: LazyLock<Setting> = LazyLock::new(|| {
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
                    kind: InnerColumnTypeStringKind::Role {},
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
        operations: LockdownSettingsExecutor.into(),
    }
});

#[derive(Clone)]
pub struct LockdownSettingsExecutor;

#[async_trait]
impl SettingView for LockdownSettingsExecutor {
    async fn view<'a>(
        &self,
        context: HookContext<'a>,
        _filters: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, splashcore_rs::value::Value>>, SettingsError> {
        check_perms(&context, &"lockdown_settings.view".into()).await?;

        let rows = sqlx::query!("SELECT member_roles, require_correct_layout, created_at, created_by, last_updated_at, last_updated_by FROM lockdown__guilds WHERE guild_id = $1", context.guild_id.to_string())
            .fetch_all(&context.data.data.pool)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Error while fetching lockdowns: {}", e),
                src: "lockdown_view".to_string(),
                typ: "value_error".to_string(),
            })?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "guild_id".to_string() => Value::String(context.guild_id.to_string()),
                "member_roles".to_string() => Value::List(row.member_roles.into_iter().map(Value::String).collect()),
                "require_correct_layout".to_string() => Value::Boolean(row.require_correct_layout),
                "created_at".to_string() => Value::TimestampTz(row.created_at),
                "created_by".to_string() => Value::String(row.created_by),
                "last_updated_at".to_string() => Value::TimestampTz(row.last_updated_at),
                "last_updated_by".to_string() => Value::String(row.last_updated_by),
            };

            result.push(map);
        }
        
        Ok(result) // TODO: Implement
    }
}

#[async_trait]
impl SettingCreator for LockdownSettingsExecutor {
    async fn create<'a>(
        &self,
        context: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&context, &"lockdown_settings.create".into()).await?;

        let Some(splashcore_rs::value::Value::List(member_roles)) = entry.get("member_roles") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "member_roles".to_string(),
                src: "lockdown_create_entry".to_string(),
            });
        };

        let member_roles: Vec<String> = member_roles.iter().map(|v| match v {
            Value::String(s) => Ok(s.clone()),
            _ => Err(SettingsError::Generic {
                message: "Invalid member role".to_string(),
                src: "lockdown_create_entry".to_string(),
                typ: "value_error".to_string(),
            }),
        }).collect::<Result<Vec<String>, SettingsError>>()?;
        
        let Some(splashcore_rs::value::Value::Boolean(require_correct_layout)) = entry.get("require_correct_layout") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "require_correct_layout".to_string(),
                src: "lockdown_create_entry".to_string(),
            });
        };

        sqlx::query!(
            "INSERT INTO lockdown__guilds (guild_id, member_roles, require_correct_layout, created_at, created_by, last_updated_at, last_updated_by) VALUES ($1, $2, $3, NOW(), $4, NOW(), $5)",
            context.guild_id.to_string(),
            &member_roles,
            require_correct_layout,
            context.author.to_string(),
            context.author.to_string(),
        )
        .execute(&context.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while creating lockdown settings: {}", e),
            src: "lockdown_create_entry".to_string(),
            typ: "value_error".to_string(),
        })?;

        Ok(entry)
    }
}

#[async_trait]
impl SettingUpdater for LockdownSettingsExecutor {
    async fn update<'a>(
        &self,
        context: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&context, &"lockdown_settings.uodate".into()).await?;

        let Some(splashcore_rs::value::Value::List(member_roles)) = entry.get("member_roles") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "member_roles".to_string(),
                src: "lockdown_create_entry".to_string(),
            });
        };

        let member_roles: Vec<String> = member_roles.iter().map(|v| match v {
            Value::String(s) => Ok(s.clone()),
            _ => Err(SettingsError::Generic {
                message: "Invalid member role".to_string(),
                src: "lockdown_create_entry".to_string(),
                typ: "value_error".to_string(),
            }),
        }).collect::<Result<Vec<String>, SettingsError>>()?;
        
        let Some(splashcore_rs::value::Value::Boolean(require_correct_layout)) = entry.get("require_correct_layout") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "require_correct_layout".to_string(),
                src: "lockdown_create_entry".to_string(),
            });
        };

        let count = sqlx::query!(
            "SELECT COUNT(*) FROM lockdown__guilds WHERE guild_id = $1",
            context.guild_id.to_string(),
        )
        .fetch_one(&context.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while updating lockdown settings: {}", e),
            src: "lockdown_create_entry".to_string(),
            typ: "value_error".to_string(),
        })?;

        if count.count.unwrap_or(0) == 0 {
            return Err(SettingsError::RowDoesNotExist {
                column_id: "guild_id".to_string(),
            });
        }

        sqlx::query!(
            "UPDATE lockdown__guilds SET member_roles = $2, require_correct_layout = $3, last_updated_at = NOW(), last_updated_by = $4 WHERE guild_id = $1",
            context.guild_id.to_string(),
            &member_roles,
            require_correct_layout,
            context.author.to_string(),
        )
        .execute(&context.data.data.pool)
        .await
        .map_err(|e| SettingsError::Generic {
            message: format!("Error while creating lockdown settings: {}", e),
            src: "lockdown_create_entry".to_string(),
            typ: "value_error".to_string(),
        })?;

        Ok(entry)
    }
}

#[async_trait]
impl SettingDeleter for LockdownSettingsExecutor {
    async fn delete<'a>(
        &self,
        context: HookContext<'a>,
        _primary_key: splashcore_rs::value::Value,
    ) -> Result<(), SettingsError> {
        check_perms(&context, &"lockdown_settings.delete".into()).await?;

        sqlx::query!("DELETE FROM lockdown__guilds WHERE guild_id = $1", context.guild_id.to_string())
            .execute(&context.data.data.pool)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Error while deleting lockdown settings: {}", e),
                src: "lockdown_delete".to_string(),
                typ: "value_error".to_string(),
            })?;

        Ok(())
    }
}

pub static LOCKDOWNS: LazyLock<Setting> = LazyLock::new(|| Setting {
    id: "lockdowns".to_string(),
    name: "Lockdowns".to_string(),
    description: "Lockdowns".to_string(),
    primary_key: "id".to_string(),
    columns: settings_wrap(vec![
        Column {
            id: "id".to_string(),
            name: "ID".to_string(),
            description: "The ID of the lockdown".to_string(),
            column_type: ColumnType::new_scalar(InnerColumnType::Uuid {}),
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
                kind: InnerColumnTypeStringKind::Normal {},
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
                kind: InnerColumnTypeStringKind::Normal {},
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
impl SettingView for LockdownExecutor {
    async fn view<'a>(
        &self,
        context: HookContext<'a>,
        _filters: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, splashcore_rs::value::Value>>, SettingsError> {
        check_perms(&context, &"lockdowns.view".into()).await?;

        let rows = sqlx::query!("SELECT id, data, type, reason, created_at FROM lockdown__guild_lockdowns WHERE guild_id = $1", context.guild_id.to_string())
            .fetch_all(&context.data.data.pool)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Error while fetching lockdowns: {}", e),
                src: "lockdown_view".to_string(),
                typ: "value_error".to_string(),
            })?;

        let mut result = vec![];

        for row in rows {
            let map = indexmap::indexmap! {
                "id".to_string() => Value::Uuid(row.id),
                "guild_id".to_string() => Value::String(context.guild_id.to_string()),
                "data".to_string() => Value::Json(row.data),
                "type".to_string() => Value::String(row.r#type),
                "reason".to_string() => Value::String(row.reason),
                "created_at".to_string() => Value::TimestampTz(row.created_at),
            };

            result.push(map);
        }
        
        Ok(result) // TODO: Implement
    }
}

#[async_trait]
impl SettingCreator for LockdownExecutor {
    async fn create<'a>(
        &self,
        context: HookContext<'a>,
        entry: indexmap::IndexMap<String, splashcore_rs::value::Value>,
    ) -> Result<indexmap::IndexMap<String, splashcore_rs::value::Value>, SettingsError> {
        check_perms(&context, &"lockdowns.create".into()).await?;
    
        let Some(splashcore_rs::value::Value::String(typ)) = entry.get("type") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "type".to_string(),
                src: "lockdown_create_entry".to_string(),
            });
        };

        let Some(splashcore_rs::value::Value::String(reason)) = entry.get("reason") else {
            return Err(SettingsError::MissingOrInvalidField {
                field: "reason".to_string(),
                src: "lockdown_create_entry".to_string(),
            });
        };

        // Get the current lockdown set
        let mut lockdowns = lockdowns::LockdownSet::guild(context.guild_id, &context.data.data.pool)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Error while fetching lockdown set: {}", e),
                src: "lockdown_create_entry".to_string(),
                typ: "value_error".to_string(),
            })?;

        // Create the lockdown
        let lockdown_type =
            lockdowns::from_lockdown_mode_string(typ).map_err(|_| SettingsError::Generic {
                message: format!(
                    "Invalid lockdown mode: {}.\n\nTIP: The following lockdown modes are supported: {}", 
                    typ, 
                    {
                        let mut supported_lockdown_modes = String::new();

                        for mode in lockdowns::CREATE_LOCKDOWN_MODES.iter() {
                            let creator = mode.value();
                            supported_lockdown_modes.push_str(&format!("\n- {}", creator.syntax()));
                        }

                        supported_lockdown_modes
                    }
                ),
                src: "lockdown_create_entry".to_string(),
                typ: "value_error".to_string(),
        })?;

        let lockdown_data = lockdowns::LockdownData {
            cache: &context.data.serenity_context.cache,
            http: &context.data.serenity_context.http,
            pool: context.data.data.pool.clone(),
            reqwest: context.data.data.reqwest.clone(),
            object_store: context.data.data.object_store.clone(),
        };

        lockdowns
            .easy_apply(lockdown_type, &lockdown_data, reason)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Error while applying lockdown: {}", e),
                src: "lockdown_create_entry".to_string(),
                typ: "value_error".to_string(),
            })?;

        let created_lockdown =
        lockdowns
            .lockdowns
            .last()
            .ok_or_else(|| SettingsError::Generic {
                message: "No lockdowns created".to_string(),
                src: "lockdown_create_entry".to_string(),
                typ: "value_error".to_string(),
            })?;
        
        Ok(created_lockdown.to_map())
    }
}

#[async_trait]
impl SettingDeleter for LockdownExecutor {
    async fn delete<'a>(
        &self,
        context: HookContext<'a>,
        primary_key: splashcore_rs::value::Value,
    ) -> Result<(), SettingsError> {
        check_perms(&context, &"lockdowns.delete".into()).await?;
                
        let primary_key = match primary_key {
            Value::Uuid(primary_key) => primary_key,
            Value::String(primary_key) => primary_key
                .parse()
                .map_err(|_| SettingsError::Generic {
                    message: format!("Invalid primary key: {}", primary_key),
                    src: "lockdown_delete".to_string(),
                    typ: "value_error".to_string(),
                })?,
            _ => {
                return Err(SettingsError::Generic {
                    message: "Primary key must be a string or UUID".to_string(),
                    src: "lockdown_delete".to_string(),
                    typ: "value_error".to_string(),
                })
            }
        };

        // Get the current lockdown set
        let mut lockdowns = lockdowns::LockdownSet::guild(context.guild_id, &context.data.data.pool)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Error while fetching lockdown set: {}", e),
                src: "lockdown_delete_matching_entries".to_string(),
                typ: "value_error".to_string(),
            })?;

        let lockdown_data = lockdowns::LockdownData {
            cache: &context.data.serenity_context.cache,
            http: &context.data.serenity_context.http,
            pool: context.data.data.pool.clone(),
            reqwest: context.data.data.reqwest.clone(),
            object_store: context.data.data.object_store.clone(),
        };        

        // Remove the lockdown
        lockdowns
            .easy_remove(primary_key, &lockdown_data)
            .await
            .map_err(|e| SettingsError::Generic {
                message: format!("Error while removing lockdown: {}", e),
                src: "lockdown_delete_matching_entries".to_string(),
                typ: "value_error".to_string(),
            })?;

        Ok(()) // TODO: Implement
    }
}
