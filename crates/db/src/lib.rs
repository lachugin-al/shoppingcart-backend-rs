//! Database initialization and migration logic for the shoppingcart backend.
//!
//! Provides `init_db_pool` for creating a connection pool and
//! auto-applying SQL migrations from the migrations directory.

use anyhow::{Context, Result};
use app_config::AppConfig;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio::fs;
use tokio_postgres::{Client, Config as PgConfig, NoTls};
use tracing::info;

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

    let pg_config: PgConfig = dsn.parse().context("Failed to parse Postgres DSN")?;

    let mgr = Manager::from_config(
        pg_config,
        NoTls,
        ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        },
    );
    let pool = Pool::builder(mgr)
        .max_size(16)
        .runtime(Runtime::Tokio1)
        .build()
        .context("Failed to create database pool")?;

    // Try to get a connection with retries
    let max_retries = 5;
    let mut retry_count = 0;
    let mut last_error = None;

    while retry_count < max_retries {
        match pool.get().await {
            Ok(client) => {
                // Successfully got a connection, now run migrations
                info!(
                    "Successfully connected to database after {} retries",
                    retry_count
                );

                // Use relative path for migrations when running outside Docker
                // First try the current directory, then try the Docker path
                let migrations_paths = vec!["./migrations", "/app/migrations"];
                let mut migrations_found = false;

                for migrations_dir in migrations_paths {
                    info!("Trying migrations directory: {}", migrations_dir);
                    if tokio::fs::metadata(migrations_dir).await.is_ok() {
                        info!("Using migrations directory: {}", migrations_dir);
                        run_migrations(&client, migrations_dir).await?;
                        migrations_found = true;
                        break;
                    }
                }

                if !migrations_found {
                    info!("No migrations directory found. Skipping migrations.");
                }
                return Ok(pool);
            }
            Err(e) => {
                retry_count += 1;
                last_error = Some(e);
                info!(
                    "Failed to connect to database (attempt {}/{}), retrying in 1 second...",
                    retry_count, max_retries
                );
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }

    // If we get here, all retries failed
    Err(anyhow::anyhow!(
        "Failed to get DB connection after {} retries: {:?}",
        max_retries,
        last_error.unwrap()
    ))
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
    let mut entries = fs::read_dir(migrations_dir)
        .await
        .context("Failed to read migrations directory")?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "sql" {
                let file_name = path.file_name().unwrap().to_string_lossy();
                info!("Applying migration: {}", file_name);
                let content = fs::read_to_string(&path)
                    .await
                    .with_context(|| format!("Failed to read migration file {file_name}"))?;

                client
                    .batch_execute(&content)
                    .await
                    .with_context(|| format!("Failed to execute migration {file_name}"))?;
            }
        }
    }
    Ok(())
}
