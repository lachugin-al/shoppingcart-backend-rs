use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;

/// `AppConfig` holds all configuration parameters required by the application.
///
/// The configuration is loaded from environment variables (optionally via a `.env` file)
/// or uses default values if the variable is not set. Fields include database, Kafka,
/// HTTP server, observability, and exporter settings. This struct is deserializable via Serde.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AppConfig {
    // --- Database settings ---
    /// Database hostname or service name (e.g. "postgres" in Docker Compose, "localhost" for local runs).
    pub db_host: String,
    /// Database port (default: 5432).
    pub db_port: u16,
    /// Database user.
    pub db_user: String,
    /// Database password.
    pub db_password: String,
    /// Database name.
    pub db_name: String,

    // --- Kafka settings ---
    /// List of Kafka brokers (comma-separated string in env, parsed to Vec<String>).
    pub kafka_brokers: Vec<String>,
    /// Kafka topic for processing orders.
    pub kafka_topic: String,
    /// Kafka consumer group ID.
    pub kafka_group_id: String,

    // --- HTTP server ---
    /// The port on which the HTTP server will listen.
    pub http_port: u16,

    // --- Shutdown timeout ---
    /// Graceful shutdown timeout (human-friendly format, e.g. "5s", "1m").
    #[serde(deserialize_with = "deserialize_duration_secs")]
    pub shutdown_timeout: Duration,

    // --- Grafana ---
    /// Initial admin password for Grafana UI.
    pub gf_security_admin_password: String,
    /// Exposed port for Grafana dashboard.
    pub grafana_port: u16,

    // --- Prometheus ---
    /// Exposed port for Prometheus UI.
    pub prometheus_port: u16,
    /// Path to Prometheus configuration file inside the container.
    pub prometheus_config_path: String,

    // --- Postgres exporter ---
    /// Connection string for Postgres exporter (DSN).
    pub data_source_name: String,
    /// Exposed port for Postgres exporter metrics endpoint.
    pub postgres_exporter_port: u16,

    // --- Kafka exporter ---
    /// Exposed port for Kafka exporter metrics endpoint.
    pub kafka_exporter_port: u16,
}

/// Custom deserializer for graceful shutdown timeout.
/// Accepts human-readable formats like "5s", "1m", etc.
fn deserialize_duration_secs<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let val = String::deserialize(deserializer)?;
    humantime::parse_duration(&val)
        .map_err(|e| D::Error::custom(format!("Invalid duration '{val}': {e}")))
}

impl AppConfig {
    /// Loads configuration from environment variables (and optionally from `.env` file).
    ///
    /// Fields not set via env will be filled with default values.
    ///
    /// # Errors
    /// Returns an error if environment variables are invalid or missing required values.
    pub fn load() -> Result<Self> {
        // Load from .env file (for Docker environment)
        dotenvy::dotenv().ok();

        // Note: These default values are for Docker Compose compatibility.
        // When running locally, these values should be overridden by environment variables
        // with localhost as hostname.
        let settings = config::Config::builder()
            // Database
            .set_default("db_host", "localhost")? // Use localhost for local development
            .set_default("db_port", 5432)?
            .set_default("db_user", "orders_user")?
            .set_default("db_password", "securepassword")?
            .set_default("db_name", "orders_db")?
            // Kafka
            .set_default("kafka_brokers", vec!["localhost:9092"])? // Use localhost for local development
            .set_default("kafka_topic", "orders")?
            .set_default("kafka_group_id", "orders_group")?
            // HTTP
            .set_default("http_port", 8081)?
            // Shutdown
            .set_default("shutdown_timeout", "5s")?
            // Grafana
            .set_default("gf_security_admin_password", "admin")?
            .set_default("grafana_port", 3000)?
            // Prometheus
            .set_default("prometheus_port", 9090)?
            .set_default("prometheus_config_path", "/etc/prometheus/prometheus.yml")?
            // Postgres exporter
            .set_default(
                "data_source_name",
                "postgresql://orders_user:securepassword@localhost:5432/orders_db?sslmode=disable", // Use localhost for local development
            )?
            .set_default("postgres_exporter_port", 9187)?
            // Kafka exporter
            .set_default("kafka_exporter_port", 9308)?
            .add_source(config::Environment::default().separator("_"))
            .build()?;

        settings
            .try_deserialize()
            .context("Failed to load configuration")
    }
}
