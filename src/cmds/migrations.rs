use log::{error, info};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::io::Write;

use crate::config::CONFIG;

pub async fn start() {
    const POSTGRES_MAX_CONNECTIONS: u32 = 70; // max connections to the database, we don't need too many here

    // Setup logging
    let debug_mode = std::env::var("DEBUG").unwrap_or_default() == "true";
    let debug_opts = std::env::var("DEBUG_OPTS").unwrap_or_default();

    let mut env_builder = env_logger::builder();

    let default_filter =
        "serenity=error,bot=info,bot_binutils=info,rust_rpc_server=info,rust_rpc_server_bot=info,botox=info,sqlx=error".to_string();

    env_builder
        .format(move |buf, record| {
            writeln!(
                buf,
                "({}) {} - {}",
                record.target(),
                record.level(),
                record.args()
            )
        })
        .parse_filters(&default_filter)
        .filter(None, log::LevelFilter::Info);

    // Set custom log levels
    for opt in debug_opts.split(',') {
        let opt = opt.trim();

        if opt.is_empty() {
            continue;
        }

        let (target, level) = if opt.contains('=') {
            let mut split = opt.split('=');
            let target = split.next().unwrap();
            let level = split.next().unwrap();
            (target, level)
        } else {
            (opt, "debug")
        };

        let level = match level {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => {
                error!("Invalid log level: {}", level);
                continue;
            }
        };

        env_builder.filter(Some(target), level);
    }

    if debug_mode {
        env_builder.filter(None, log::LevelFilter::Debug);
    } else {
        env_builder.filter(None, log::LevelFilter::Error);
    }

    env_builder.init();

    info!("Connecting to database");

    let pg_pool = PgPoolOptions::new()
        .max_connections(POSTGRES_MAX_CONNECTIONS)
        .connect(&CONFIG.meta.postgres_url)
        .await
        .expect("Could not initialize connection");

    log::info!("Starting migrations for postgres");

    //* Migration #1 - Template content: text -> jsonb with init.luau containing content
    println!("guild_templates: content text -> jsonb");

    // Check if content is already jsonb
    let content_type: String = sqlx::query(
        "SELECT data_type FROM information_schema.columns WHERE table_name = 'guild_templates' AND column_name = 'content'"
    )
    .fetch_one(&pg_pool)
    .await
    .expect("Could not fetch content type")
    .get("data_type");

    if content_type == "jsonb" {
        log::info!("Content is already jsonb, skipping migration");
    } else {
        log::info!("Migrating content to jsonb");

        #[derive(sqlx::FromRow)]
        struct TemplateRow {
            name: String,
            content: String,
        }

        let mut tx = pg_pool.begin().await.expect("Could not start transaction");

        let contents: Vec<TemplateRow> =
            sqlx::query_as("SELECT name, content FROM guild_templates")
                .fetch_all(&mut *tx)
                .await
                .expect("Could not fetch contents");

        // Drop old column
        sqlx::query("ALTER TABLE guild_templates DROP COLUMN content")
            .execute(&mut *tx)
            .await
            .expect("Could not drop old column");

        // Add new column (nullable for now)
        sqlx::query("ALTER TABLE guild_templates ADD COLUMN content jsonb")
            .execute(&mut *tx)
            .await
            .expect("Could not add new column");

        for content in contents {
            let new_data = indexmap::indexmap! {
                "init.luau".to_string() => content.content
            };

            sqlx::query("UPDATE guild_templates SET content = $1 WHERE name = $2")
                .bind(serde_json::to_value(new_data).expect("Could not serialize data"))
                .bind(content.name)
                .execute(&mut *tx)
                .await
                .expect("Could not update content");
        }

        // Set new column as not nullable
        sqlx::query("ALTER TABLE guild_templates ALTER COLUMN content SET NOT NULL")
            .execute(&mut *tx)
            .await
            .expect("Could not set new column as not nullable");

        tx.commit().await.expect("Could not commit transaction");
    }

    //* Migration #1 - Template shop content: text -> jsonb with init.luau containing content
    println!("template_shop: content text -> jsonb");

    // Check if content is already jsonb
    let content_type: String = sqlx::query(
        "SELECT data_type FROM information_schema.columns WHERE table_name = 'template_shop' AND column_name = 'content'"
    )
    .fetch_one(&pg_pool)
    .await
    .expect("Could not fetch content type")
    .get("data_type");

    if content_type == "jsonb" {
        log::info!("Content is already jsonb, skipping migration");
    } else {
        log::info!("Migrating content to jsonb");

        #[derive(sqlx::FromRow)]
        struct TemplateRow {
            id: uuid::Uuid,
            content: String,
        }

        let mut tx = pg_pool.begin().await.expect("Could not start transaction");

        let contents: Vec<TemplateRow> = sqlx::query_as("SELECT id, content FROM template_shop")
            .fetch_all(&mut *tx)
            .await
            .expect("Could not fetch contents");

        // Drop old column
        sqlx::query("ALTER TABLE template_shop DROP COLUMN content")
            .execute(&mut *tx)
            .await
            .expect("Could not drop old column");

        // Add new column (nullable for now)
        sqlx::query("ALTER TABLE template_shop ADD COLUMN content jsonb")
            .execute(&mut *tx)
            .await
            .expect("Could not add new column");

        for content in contents {
            let new_data = indexmap::indexmap! {
                "init.luau".to_string() => content.content
            };

            sqlx::query("UPDATE template_shop SET content = $1 WHERE id = $2")
                .bind(serde_json::to_value(new_data).expect("Could not serialize data"))
                .bind(content.id)
                .execute(&mut *tx)
                .await
                .expect("Could not update content");
        }

        // Set new column as not nullable
        sqlx::query("ALTER TABLE template_shop ALTER COLUMN content SET NOT NULL")
            .execute(&mut *tx)
            .await
            .expect("Could not set new column as not nullable");

        tx.commit().await.expect("Could not commit transaction");
    }
}
