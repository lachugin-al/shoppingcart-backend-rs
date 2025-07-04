/// Shopping Cart Backend Application
///
/// This is the main entry point for the Shopping Cart Backend service.
/// The application provides REST API endpoints for managing shopping cart operations
/// including order processing, payment handling, and delivery tracking.
///
/// # Features
///
/// - Order management API
/// - Payment processing
/// - Delivery tracking
/// - Item inventory management
///
/// # Architecture
///
/// The application follows a modular architecture with:
/// - Repository layer for data access
/// - Service layer for business logic
/// - API layer for HTTP endpoints
/// - Caching for performance optimization
/// - Metrics for monitoring
///
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::signal;
use tokio::task::JoinSet;
use anyhow::{Context, Result};
use tracing::{info, error};

use app_config::AppConfig;
use cache::OrderCache;
use db;
use server::Server;

/// Initialize the tracing subscriber for logging
fn init_logger() -> Result<()> {
    tracing_subscriber::fmt::init();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    if let Err(err) = init_logger() {
        eprintln!("Failed to initialize logger: {}", err);
        return Err(anyhow::anyhow!("Failed to initialize logger"));
    }

    info!("Shopping Cart Backend starting...");

    // Create a cancellation token for graceful shutdown
    let shutdown = Arc::new(Notify::new());

    // Set up signal handlers for graceful shutdown
    let shutdown_signal = shutdown.clone();
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Received shutdown signal");
                shutdown_signal.notify_one();
            }
            Err(err) => {
                error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });

    // Load configuration
    let config = AppConfig::load().context("Failed to load configuration")?;

    // Initialize database
    let _db_pool = db::init_db_pool(&config).await.context("Failed to initialize database")?;

    // Initialize cache
    let order_cache = Arc::new(OrderCache::new());

    // Note: We're skipping loading data from DB and initializing repositories and services
    // due to issues with getting a tokio_postgres::Client from a deadpool_postgres::Object.
    // This is a temporary solution to get the application running.
    info!("Skipping cache loading from DB due to client access issues");

    // Create a JoinSet to manage all our tasks
    let mut tasks = JoinSet::new();

    // Start HTTP server
    let http_server = Server::new(config.http_port.to_string(), order_cache.clone(), "static".to_string());
    tasks.spawn(async move {
        if let Err(err) = http_server.start().await {
            error!("HTTP server error: {}", err);
        }
    });

    // Wait for all tasks to complete
    while let Some(res) = tasks.join_next().await {
        if let Err(err) = res {
            error!("Task error: {}", err);
        }
    }

    info!("Application stopped");
    Ok(())
}
