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
    let db_pool = match db::init_db_pool(&config).await {
        Ok(pool) => {
            info!("Database initialized successfully");
            pool
        },
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            error!("Database connection is required for application to function properly");
            return Err(anyhow::anyhow!("Failed to initialize database"));
        }
    };

    // Initialize cache
    let order_cache = Arc::new(OrderCache::new());

    // Note: We're skipping loading data from DB and initializing repositories and services
    // due to issues with getting a tokio_postgres::Client from a deadpool_postgres::Object.
    // This is a temporary solution to get the application running.
    info!("Skipping cache loading from DB due to client access issues");

    // Create a JoinSet to manage all our tasks
    let mut tasks = JoinSet::new();

    // Start HTTP server
    let http_port = config.http_port.to_string();
    info!("Using HTTP port: {}", http_port);

    // Try to find the static directory in multiple locations
    let static_paths = vec!["./static", "/app/static"];
    let mut static_dir = "./static".to_string(); // Default to current directory

    for path in static_paths {
        info!("Checking static directory: {}", path);
        if std::path::Path::new(path).exists() {
            static_dir = path.to_string();
            info!("Using static directory: {}", static_dir);
            break;
        }
    }

    let http_server = Server::new(http_port, order_cache.clone(), static_dir, db_pool);
    tasks.spawn(async move {
        if let Err(err) = http_server.start().await {
            error!("HTTP server error: {}", err);
            // Exit the application if the server fails to start
            std::process::exit(1);
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
