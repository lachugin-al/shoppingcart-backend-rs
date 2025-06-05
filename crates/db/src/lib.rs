//! Database initialization and migration logic for the shoppingcart backend.
//!
//! Provides `init_db_pool` for creating a connection pool and
//! auto-applying SQL migrations from the migrations directory.

use anyhow::{Result, Context};
use deadpool_postgres::{Pool, Manager, ManagerConfig, RecyclingMethod, Runtime};
use tokio_postgres::{NoTls, Config as PgConfig, Client};
use tokio::fs;
use tracing::info;
use app_config::AppConfig;

/// Initializes the database connection pool and runs migrations.
///
/// # Arguments
/// * `cfg` - The loaded application configuration.
///
/// # Returns
/// * `Pool` - A pool of PostgreSQL connections, ready for async use.
///
/// # Errors
/// Returns an error if the pool cannot be created or migrations fail.
pub async fn init_db_pool(cfg: &AppConfig) -> Result<Pool> {
    let dsn = format!(
        "host={} port={} user={} password={} dbname={} sslmode=disable",
        cfg.db_host, cfg.db_port, cfg.db_user, cfg.db_password, cfg.db_name
    );

    let pg_config: PgConfig = dsn.parse()
        .context("Failed to parse Postgres DSN")?;

    let mgr = Manager::from_config(pg_config, NoTls, ManagerConfig { recycling_method: RecyclingMethod::Fast });
    let pool = Pool::builder(mgr)
        .max_size(16)
        .runtime(Runtime::Tokio1)
        .build()
        .context("Failed to create database pool")?;

    // Apply migrations
    let client = pool.get().await.context("Failed to get DB connection for migrations")?;
    run_migrations(&client, "migrations").await?;

    Ok(pool)
}

/// Applies all SQL migrations from the given directory to the provided database client.
///
/// # Arguments
/// * `client` - An active Postgres client.
/// * `migrations_dir` - Path to the folder containing .sql migration files.
///
/// # Errors
/// Returns an error if migration files cannot be read or applied.
pub async fn run_migrations(client: &Client, migrations_dir: &str) -> Result<()> {
    let mut entries = fs::read_dir(migrations_dir).await
        .context("Failed to read migrations directory")?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "sql" {
                let file_name = path.file_name().unwrap().to_string_lossy();
                info!("Applying migration: {}", file_name);
                let content = fs::read_to_string(&path).await
                    .with_context(|| format!("Failed to read migration file {}", file_name))?;

                client.batch_execute(&content)
                    .await
                    .with_context(|| format!("Failed to execute migration {}", file_name))?;
            }
        }
    }
    Ok(())
}
