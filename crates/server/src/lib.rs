//! Server crate provides HTTP server functionality.
//!
//! This module implements an HTTP server for handling order-related requests,
//! including retrieving orders, sending test orders, and serving static content.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use cache::OrderCache;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info, warn};
use prometheus::{
    CounterVec, HistogramOpts, HistogramVec, Opts, Registry,
};

/// Server represents an HTTP server for working with orders.
pub struct Server {
    cache: Arc<OrderCache>,
    static_dir: String,
    port: String,
    metrics: Arc<Metrics>,
}

/// Metrics collects and exposes HTTP server metrics.
struct Metrics {
    registry: Registry,
    http_requests_total: CounterVec,
    http_request_duration_seconds: HistogramVec,
    errors_total: CounterVec,
    network_traffic_bytes: CounterVec,
}

impl Metrics {
    fn new() -> Self {
        let registry = Registry::new();

        let http_requests_total = CounterVec::new(
            Opts::new("http_requests_total", "Total number of HTTP requests"),
            &["method", "endpoint", "status"],
        )
        .expect("Failed to create http_requests_total metric");

        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            ),
            &["method", "endpoint"],
        )
        .expect("Failed to create http_request_duration_seconds metric");

        let errors_total = CounterVec::new(
            Opts::new("errors_total", "Total number of errors"),
            &["source", "endpoint"],
        )
        .expect("Failed to create errors_total metric");

        let network_traffic_bytes = CounterVec::new(
            Opts::new("network_traffic_bytes", "Network traffic in bytes"),
            &["direction"],
        )
        .expect("Failed to create network_traffic_bytes metric");

        registry
            .register(Box::new(http_requests_total.clone()))
            .expect("Failed to register http_requests_total metric");
        registry
            .register(Box::new(http_request_duration_seconds.clone()))
            .expect("Failed to register http_request_duration_seconds metric");
        registry
            .register(Box::new(errors_total.clone()))
            .expect("Failed to register errors_total metric");
        registry
            .register(Box::new(network_traffic_bytes.clone()))
            .expect("Failed to register network_traffic_bytes metric");

        Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            errors_total,
            network_traffic_bytes,
        }
    }

    fn record_request(&self, method: &str, endpoint: &str, status: u16, duration: Duration) {
        self.http_requests_total
            .with_label_values(&[method, endpoint, &status.to_string()])
            .inc();
        self.http_request_duration_seconds
            .with_label_values(&[method, endpoint])
            .observe(duration.as_secs_f64());
    }

    fn record_error(&self, source: &str, endpoint: &str) {
        self.errors_total
            .with_label_values(&[source, endpoint])
            .inc();
    }

    fn record_network_traffic(&self, direction: &str, bytes: usize) {
        self.network_traffic_bytes
            .with_label_values(&[direction])
            .inc_by(bytes as f64);
    }
}

impl Server {
    /// Creates a new Server instance.
    ///
    /// # Arguments
    ///
    /// * `port` - The port on which the server will listen
    /// * `cache` - The order cache for accessing orders
    /// * `static_dir` - The directory for static files (e.g., index.html)
    ///
    /// # Returns
    ///
    /// A new Server instance
    pub fn new(port: String, cache: Arc<OrderCache>, static_dir: String) -> Self {
        info!("Initializing HTTP server on port {}", port);

        Self {
            cache,
            static_dir,
            port,
            metrics: Arc::new(Metrics::new()),
        }
    }

    /// Starts the server and blocks until it's shut down.
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure
    pub async fn start(&self) -> Result<()> {
        let app = self.create_router();

        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await
            .context("Failed to bind to port")?;

        info!("HTTP server listening on port {}", self.port);

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("Server error")?;

        info!("HTTP server shut down gracefully");
        Ok(())
    }

    fn create_router(&self) -> Router {
        let metrics = self.metrics.clone();
        let cache = self.cache.clone();
        let static_dir = self.static_dir.clone();

        Router::new()
            .route("/order/{id}", get(Self::handle_get_order_by_id))
            .route("/api/orders", get(Self::handle_get_orders))
            .route("/api/send-test-order", post(Self::handle_send_test_order))
            .route("/health", get(Self::handle_health))
            .route("/metrics", get(Self::handle_metrics))
            .fallback(Self::handle_static)
            .layer(axum::middleware::from_fn_with_state(
                metrics.clone(),
                Self::metrics_middleware,
            ))
            .with_state(AppState {
                cache,
                static_dir,
                metrics,
            })
    }

    /// Middleware for collecting metrics on HTTP requests
    async fn metrics_middleware(
        State(metrics): State<Arc<Metrics>>,
        req: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> Response {
        let method = req.method().to_string();
        let path = req.uri().path().to_string();

        // Estimate request size for incoming traffic metrics
        let content_length = req.headers()
            .get(axum::http::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        if content_length > 0 {
            metrics.record_network_traffic("in", content_length);
        }

        // Record start time
        let start = std::time::Instant::now();

        // Process the request
        let response = next.run(req).await;

        // Calculate duration
        let duration = start.elapsed();

        // Get status code
        let status = response.status().as_u16();

        // Record metrics
        metrics.record_request(&method, &path, status, duration);

        // If error status, record error
        if status >= 400 {
            metrics.record_error("http", &path);
        }

        // Estimate response size for outgoing traffic metrics
        let response_size = response.headers()
            .get(axum::http::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        if response_size > 0 {
            metrics.record_network_traffic("out", response_size);
        }

        response
    }

    async fn handle_get_order_by_id(
        State(state): State<AppState>,
        axum::extract::Path(order_id): AxumPath<String>,
    ) -> Response {
        info!("Received order request for ID: {}", order_id);

        if order_id.is_empty() {
            warn!("Order ID is missing in request");
            return (StatusCode::BAD_REQUEST, "order id is required").into_response();
        }

        match state.cache.get(&order_id).await {
            Some(order) => {
                let json = serde_json::to_string(&order).unwrap_or_else(|e| {
                    error!("Failed to serialize order: {}", e);
                    "{}".to_string()
                });
                (StatusCode::OK, json).into_response()
            }
            None => {
                warn!("Order not found: {}", order_id);
                (StatusCode::NOT_FOUND, "order not found").into_response()
            }
        }
    }

    async fn handle_get_orders(State(state): State<AppState>) -> Response {
        info!("Received request to fetch all orders");

        // Access the get_all method on the inner OrderCache by dereferencing the Arc
        let orders = state.cache.as_ref().get_all().await;
        if orders.is_empty() {
            warn!("No orders found in cache");
            return (StatusCode::NOT_FOUND, "no orders available").into_response();
        }

        match serde_json::to_string(&orders) {
            Ok(json) => (StatusCode::OK, json).into_response(),
            Err(e) => {
                error!("Failed to encode orders response: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to encode response",
                )
                    .into_response()
            }
        }
    }

    async fn handle_send_test_order(State(_state): State<AppState>) -> Response {
        info!("Received request to send test order");

        match kafka_producer::produce_test_message().await {
            Ok(order_uid) => {
                (
                    StatusCode::OK,
                    format!("Test order sent successfully! Order UID: {}", order_uid),
                )
                    .into_response()
            }
            Err(e) => {
                error!("Failed to send test order: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to send test order",
                )
                    .into_response()
            }
        }
    }

    async fn handle_health() -> &'static str {
        info!("Health check requested");
        "OK"
    }

    async fn handle_metrics(State(state): State<AppState>) -> Response {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();

        let mut buffer = Vec::new();
        if let Err(e) = encoder.encode(&state.metrics.registry.gather(), &mut buffer) {
            error!("Failed to encode metrics: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics").into_response();
        }

        match String::from_utf8(buffer) {
            Ok(metrics_text) => (StatusCode::OK, metrics_text).into_response(),
            Err(e) => {
                error!("Failed to convert metrics to UTF-8: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Invalid metrics data").into_response()
            }
        }
    }

    async fn handle_static(State(state): State<AppState>, uri: axum::http::Uri) -> Response {
        let path = uri.path().trim_start_matches('/');
        let path = if path.is_empty() { "index.html" } else { path };

        let file_path = Path::new(&state.static_dir).join(path);
        info!("Serving static file: {:?}", file_path);

        match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                let content_type = if path.ends_with(".html") {
                    "text/html"
                } else if path.ends_with(".css") {
                    "text/css"
                } else if path.ends_with(".js") {
                    "application/javascript"
                } else {
                    "text/plain"
                };

                Response::builder()
                    .header("Content-Type", content_type)
                    .body(content.into())
                    .unwrap_or_else(|_| {
                        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create response").into_response()
                    })
            }
            Err(_) => (StatusCode::NOT_FOUND, "File not found").into_response(),
        }
    }
}

/// Application state shared between request handlers
#[derive(Clone)]
struct AppState {
    cache: Arc<OrderCache>,
    static_dir: String,
    metrics: Arc<Metrics>,
}

/// Waits for a shutdown signal (Ctrl+C)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::Order;

    // Helper function to create a test server
    fn create_test_server() -> Server {
        let cache = Arc::new(OrderCache::new());
        Server::new("8080".to_string(), cache, "static".to_string())
    }

    #[test]
    fn test_server_creation() {
        let server = create_test_server();
        assert_eq!(server.port, "8080");
        assert_eq!(server.static_dir, "static");
    }
}
