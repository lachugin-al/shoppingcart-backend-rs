use anyhow::{Context, Result};
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
use tokio::signal;
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tracing::{error, info};

use app_config::AppConfig;
use cache::OrderCache;
use kafka_consumer::KafkaConsumer;
use repository::{
    PgDeliveriesRepository, PgItemsRepository, PgOrdersRepository, PgPaymentsRepository,
};
use server::Server;
use service::OrderServiceImpl;
use tokio_postgres::NoTls;

/// Initialize the tracing subscriber for logging
fn init_logger() -> Result<()> {
    tracing_subscriber::fmt::init();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    if let Err(err) = init_logger() {
        eprintln!("Failed to initialize logger: {err}");
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
        }
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            error!("Database connection is required for application to function properly");
            return Err(anyhow::anyhow!("Failed to initialize database"));
        }
    };

    // Initialize cache
    let order_cache = Arc::new(OrderCache::new());

    // Get a connection to initialize repositories
    // We need to create separate connections for each repository
    // because tokio_postgres::Client doesn't implement Clone
    let dsn = format!(
        "host={} port={} user={} password={} dbname={} sslmode=disable",
        config.db_host, config.db_port, config.db_user, config.db_password, config.db_name
    );

    // Create clients for each repository
    // Orders repository
    let (orders_client, orders_connection) = match tokio_postgres::connect(&dsn, NoTls).await {
        Ok((client, connection)) => {
            info!("Successfully connected to database for orders repository");
            (client, connection)
        }
        Err(e) => {
            error!("Failed to connect to database for orders repository: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to connect to database for orders repository"
            ));
        }
    };
    tokio::spawn(async move {
        if let Err(e) = orders_connection.await {
            error!("Orders connection error: {}", e);
        }
    });

    // Deliveries repository
    let (deliveries_client, deliveries_connection) =
        match tokio_postgres::connect(&dsn, NoTls).await {
            Ok((client, connection)) => {
                info!("Successfully connected to database for deliveries repository");
                (client, connection)
            }
            Err(e) => {
                error!(
                    "Failed to connect to database for deliveries repository: {}",
                    e
                );
                return Err(anyhow::anyhow!(
                    "Failed to connect to database for deliveries repository"
                ));
            }
        };
    tokio::spawn(async move {
        if let Err(e) = deliveries_connection.await {
            error!("Deliveries connection error: {}", e);
        }
    });

    // Payments repository
    let (payments_client, payments_connection) = match tokio_postgres::connect(&dsn, NoTls).await {
        Ok((client, connection)) => {
            info!("Successfully connected to database for payments repository");
            (client, connection)
        }
        Err(e) => {
            error!(
                "Failed to connect to database for payments repository: {}",
                e
            );
            return Err(anyhow::anyhow!(
                "Failed to connect to database for payments repository"
            ));
        }
    };
    tokio::spawn(async move {
        if let Err(e) = payments_connection.await {
            error!("Payments connection error: {}", e);
        }
    });

    // Items repository
    let (items_client, items_connection) = match tokio_postgres::connect(&dsn, NoTls).await {
        Ok((client, connection)) => {
            info!("Successfully connected to database for items repository");
            (client, connection)
        }
        Err(e) => {
            error!("Failed to connect to database for items repository: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to connect to database for items repository"
            ));
        }
    };
    tokio::spawn(async move {
        if let Err(e) = items_connection.await {
            error!("Items connection error: {}", e);
        }
    });

    // Initialize repositories
    let orders_repo = PgOrdersRepository::new(orders_client);
    let deliveries_repo = PgDeliveriesRepository::new(deliveries_client);
    let payments_repo = PgPaymentsRepository::new(payments_client);
    let items_repo = PgItemsRepository::new(items_client);

    // Initialize order service
    let order_service = Arc::new(OrderServiceImpl::new(
        db_pool.clone(),
        orders_repo,
        deliveries_repo,
        payments_repo,
        items_repo,
    ));

    // Load cache from DB
    info!("Creating additional repository instances for cache loading");

    // Create additional clients for cache loading repositories
    // Orders repository for cache
    let (cache_orders_client, cache_orders_connection) =
        match tokio_postgres::connect(&dsn, NoTls).await {
            Ok((client, connection)) => {
                info!("Successfully connected to database for cache orders repository");
                (client, connection)
            }
            Err(e) => {
                error!(
                    "Failed to connect to database for cache orders repository: {}",
                    e
                );
                return Err(anyhow::anyhow!(
                    "Failed to connect to database for cache orders repository"
                ));
            }
        };
    tokio::spawn(async move {
        if let Err(e) = cache_orders_connection.await {
            error!("Cache orders connection error: {}", e);
        }
    });

    // Deliveries repository for cache
    let (cache_deliveries_client, cache_deliveries_connection) =
        match tokio_postgres::connect(&dsn, NoTls).await {
            Ok((client, connection)) => {
                info!("Successfully connected to database for cache deliveries repository");
                (client, connection)
            }
            Err(e) => {
                error!(
                    "Failed to connect to database for cache deliveries repository: {}",
                    e
                );
                return Err(anyhow::anyhow!(
                    "Failed to connect to database for cache deliveries repository"
                ));
            }
        };
    tokio::spawn(async move {
        if let Err(e) = cache_deliveries_connection.await {
            error!("Cache deliveries connection error: {}", e);
        }
    });

    // Payments repository for cache
    let (cache_payments_client, cache_payments_connection) =
        match tokio_postgres::connect(&dsn, NoTls).await {
            Ok((client, connection)) => {
                info!("Successfully connected to database for cache payments repository");
                (client, connection)
            }
            Err(e) => {
                error!(
                    "Failed to connect to database for cache payments repository: {}",
                    e
                );
                return Err(anyhow::anyhow!(
                    "Failed to connect to database for cache payments repository"
                ));
            }
        };
    tokio::spawn(async move {
        if let Err(e) = cache_payments_connection.await {
            error!("Cache payments connection error: {}", e);
        }
    });

    // Items repository for cache
    let (cache_items_client, cache_items_connection) =
        match tokio_postgres::connect(&dsn, NoTls).await {
            Ok((client, connection)) => {
                info!("Successfully connected to database for cache items repository");
                (client, connection)
            }
            Err(e) => {
                error!(
                    "Failed to connect to database for cache items repository: {}",
                    e
                );
                return Err(anyhow::anyhow!(
                    "Failed to connect to database for cache items repository"
                ));
            }
        };
    tokio::spawn(async move {
        if let Err(e) = cache_items_connection.await {
            error!("Cache items connection error: {}", e);
        }
    });

    // Initialize cache repositories
    let cache_orders_repo = PgOrdersRepository::new(cache_orders_client);
    let cache_deliveries_repo = PgDeliveriesRepository::new(cache_deliveries_client);
    let cache_payments_repo = PgPaymentsRepository::new(cache_payments_client);
    let cache_items_repo = PgItemsRepository::new(cache_items_client);

    // Load cache from DB
    info!("Loading cache from database");
    match order_cache
        .load_from_db(
            &db_pool,
            &cache_orders_repo,
            &cache_deliveries_repo,
            &cache_payments_repo,
            &cache_items_repo,
        )
        .await
    {
        Ok(()) => info!("Cache loaded successfully from database"),
        Err(e) => error!("Failed to load cache from database: {}", e),
    }

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

    // Start Kafka consumer
    info!("Initializing Kafka consumer");
    let kafka_shutdown = shutdown.clone();

    // Initialize KafkaConsumer
    match KafkaConsumer::new(
        &config.kafka_brokers,
        &config.kafka_topic,
        &config.kafka_group_id,
        order_service.clone(),
        order_cache.clone(),
    ) {
        Ok(consumer) => {
            // Start KafkaConsumer in a separate task
            tasks.spawn(async move {
                info!("Starting Kafka consumer");
                if let Err(err) = consumer.run(kafka_shutdown).await {
                    error!("Kafka consumer error: {}", err);
                }
            });
        }
        Err(err) => {
            error!("Failed to initialize Kafka consumer: {}", err);
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
