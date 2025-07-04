# Shopping Cart Backend

A Rust-based backend service for an e-commerce shopping cart system that processes and manages orders through an
event-driven architecture.

## Project Structure

```
shoppingcart/
├── Cargo.toml                # Workspace declaration (workspace only)
├── Cargo.lock
├── Makefile                  # Development routines
├── docker-compose.yml        # Docker Compose configuration
├── prometheus.yml            # Prometheus configuration
│
├── web/                      # Static assets (HTML, JS, CSS)
│
├── crates/
│   ├── app/                  # Binary crate: Server entry point
│   │   └── src/
│   │       └── main.rs
│   │
│   ├── config/               # Library crate: Configuration loader & structures
│   │   └── src/lib.rs
│   │
│   ├── db/                   # Library crate: Database and migrations
│   │   └── src/lib.rs
│   │
│   ├── model/                # Library crate: DTOs, entities
│   │   └── src/lib.rs
│   │
│   ├── repository/           # Library crate: Repository interfaces/implementations
│   │   └── src/lib.rs
│   │
│   ├── service/              # Library crate: Business logic
│   │   └── src/lib.rs
│   │
│   ├── cache/                # Library crate: In-memory cache
│   │   └── src/lib.rs
│   │
│   ├── kafka-consumer/       # Library crate: Kafka consumer for order processing
│   │   └── src/lib.rs
│   │
│   ├── kafka-producer/       # Library crate: Kafka producer for order creation
│   │   └── src/lib.rs
│   │
│   ├── metrics/              # Library crate: Prometheus metrics
│   │   └── src/lib.rs
│   │
│   ├── server/               # Library crate: HTTP server and API endpoints
│   │   └── src/lib.rs
│   │
│   └── tools/                # Binary crate: Utilities (e.g., migrations)
│       └── src/main.rs
│
└── tests/                    # Integration tests for different layers
```

## Overview

This project implements a backend service for a shopping cart system with the following features:

- Order management and processing
- Payment processing
- Delivery tracking
- Item inventory management
- Real-time order updates via Kafka
- In-memory caching for performance
- Comprehensive monitoring with Prometheus and Grafana

## Architecture

The application follows a modular architecture with clear separation of concerns:

- **Model**: Data structures and DTOs for orders, payments, deliveries, and items
- **Repository**: Data access layer with PostgreSQL implementation
- **Service**: Business logic and domain rules
- **API**: HTTP endpoints for client interaction
- **Cache**: In-memory caching for performance
- **Kafka Consumer**: Processes incoming order events from Kafka
- **Kafka Producer**: Publishes order events to Kafka
- **Metrics**: Monitoring and observability with Prometheus

### Event-Driven Flow

1. Orders are received via API or Kafka
2. Orders are validated and processed by the service layer
3. Orders are persisted in PostgreSQL
4. Orders are cached in memory for fast retrieval
5. Order status updates are published to Kafka

## Data Model

The system uses the following data model for orders:

### Order

The main entity representing a customer order:

```json
{
  "order_uid": "b563feb7b2b84b6test",
  "track_number": "WBILMTESTTRACK",
  "entry": "WBIL",
  "delivery": {},
  "payment": {},
  "items": [],
  "locale": "en",
  "internal_signature": "",
  "customer_id": "test",
  "delivery_service": "meest",
  "shardkey": "9",
  "sm_id": 99,
  "date_created": "2021-11-26T06:22:19Z",
  "oof_shard": "1"
}
```

**Fields:**

- `order_uid`: Unique order identifier
- `track_number`: Tracking number for the order
- `entry`: Entry point identifier
- `delivery`: Delivery information (see Delivery model)
- `payment`: Payment details (see Payment model)
- `items`: List of ordered items (see Item model)
- `locale`: Language/locale code
- `internal_signature`: Internal signature for verification
- `customer_id`: Customer identifier
- `delivery_service`: Delivery service provider
- `shardkey`: Sharding key for database partitioning
- `sm_id`: Service manager identifier
- `date_created`: Order creation timestamp
- `oof_shard`: Out-of-stock shard identifier

### Delivery

Information about order delivery:

```json
{
  "name": "Test Testov",
  "phone": "+9720000000",
  "zip": "2639809",
  "city": "Kiryat Mozkin",
  "address": "Ploshad Mira 15",
  "region": "Kraiot",
  "email": "test@gmail.com"
}
```

**Fields:**

- `name`: Recipient's full name
- `phone`: Contact phone number
- `zip`: Postal code
- `city`: City name
- `address`: Street address
- `region`: Region or state
- `email`: Contact email address

### Payment

Information about order payment:

```json
{
  "transaction": "b563feb7b2b84b6test",
  "request_id": "",
  "currency": "USD",
  "provider": "wbpay",
  "amount": 1817,
  "payment_dt": 1637907727,
  "bank": "alpha",
  "delivery_cost": 1500,
  "goods_total": 317,
  "custom_fee": 0
}
```

**Fields:**

- `transaction`: Unique transaction identifier
- `request_id`: Request identifier for the payment
- `currency`: Currency code (e.g., USD, EUR)
- `provider`: Payment service provider name
- `amount`: Total payment amount
- `payment_dt`: Payment date/time as Unix timestamp
- `bank`: Bank name or identifier
- `delivery_cost`: Cost of delivery
- `goods_total`: Total cost of goods without delivery
- `custom_fee`: Any additional fees

### Item

Individual order item:

```json
{
  "chrt_id": 9934930,
  "track_number": "WBILMTESTTRACK",
  "price": 453,
  "rid": "ab4219087a764ae0btest",
  "name": "Mascaras",
  "sale": 30,
  "size": "0",
  "total_price": 317,
  "nm_id": 2389212,
  "brand": "Vivienne Sabo",
  "status": 202
}
```

**Fields:**

- `chrt_id`: Chart ID - unique identifier for the item
- `track_number`: Tracking number for the item shipment
- `price`: Original price of the item
- `rid`: Row identifier
- `name`: Product name
- `sale`: Discount percentage
- `size`: Size information (may be numeric or descriptive)
- `total_price`: Final price after applying discounts
- `nm_id`: Nomenclature ID - product catalog identifier
- `brand`: Brand name
- `status`: Item status code

## API Endpoints

The service provides the following REST API endpoints:

- `GET /api/orders` - Get all orders
- `GET /api/orders/:id` - Get order by ID
- `POST /api/orders/test` - Send a test order
- `GET /health` - Health check endpoint
- `GET /metrics` - Prometheus metrics endpoint

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Docker and Docker Compose
- PostgreSQL
- Kafka

### Setup

1. Clone the repository
2. Run `docker-compose up -d` to start the required services:
    - PostgreSQL database
    - Kafka and Zookeeper
    - Prometheus and Grafana for monitoring
    - Kafka UI for message inspection
3. Run `cargo build` to build the project
4. Run `cargo run -p app` to start the server

### Using Makefile

The project includes a Makefile that provides convenient commands for common development tasks:

```bash
# Build the project
make build                # Build in debug mode
make build-release        # Build in release mode

# Run the project
make run                  # Run in debug mode
make run-release          # Run in release mode
make dev                  # Start Docker services and run the project
make start-fresh          # Setup project (Docker + migrations) and run it

# Testing
make test                 # Run tests
make test-coverage        # Run tests with coverage report

# Docker management
make docker-up            # Start all Docker containers
make docker-down          # Stop all Docker containers
make docker-build         # Build Docker images
make docker-logs          # Show Docker container logs
make docker-restart       # Restart Docker containers

# Database operations
make setup                # Start Docker services and run migrations
make db-migrate           # Run database migrations
make db-reset             # Reset database and run migrations

# Code quality
make fmt                  # Format code with rustfmt
make lint                 # Run clippy linter

# Utilities
make clean                # Clean build artifacts
make help                 # Show help message with all available commands
```

For a quick start with a fresh environment, use:

```bash
make start-fresh
```

### Environment Variables

The application uses environment variables for configuration. There are two ways to set these variables:

1. **Docker Compose**: The `.env` file in the project root is used by Docker Compose to configure the services. This
   file uses Docker service names as hostnames (e.g., `postgres`, `kafka`).

2. **Local Development**: For running the application directly on your machine (not in Docker), use the `.env.local`
   file which configures services to use `localhost` instead of Docker service names.

To run locally:

# Load .env directly

Key environment variables:

```
# Database
DB_HOST=localhost  # Use 'postgres' for Docker
DB_PORT=5432
DB_USER=orders_user
DB_PASSWORD=securepassword
DB_NAME=orders_db

# Kafka
KAFKA_BROKERS=localhost:9092  # Use 'kafka:9092' for Docker
KAFKA_TOPIC=orders
KAFKA_GROUP_ID=orders_group

# Server
SERVER_PORT=8080
STATIC_DIR=./static

# Monitoring
PROMETHEUS_PORT=9090
GRAFANA_PORT=3000
POSTGRES_EXPORTER_PORT=9187
KAFKA_EXPORTER_PORT=9308
```

## Development

### Running Tests

```
cargo test
```

### Code Style

Follow the Rust standard code style. Run `cargo fmt` before committing.

### Monitoring

The application includes comprehensive monitoring:

1. **Prometheus**: Collects metrics from the application, PostgreSQL, and Kafka
2. **Grafana**: Visualizes metrics with pre-configured dashboards
3. **Exporters**: Dedicated exporters for PostgreSQL and Kafka metrics

Access Grafana at `http://localhost:3000` with default credentials (admin/admin).

## Deployment

The application can be deployed using Docker:

```
docker build -t shoppingcart-backend .
docker run -p 8080:8080 shoppingcart-backend
```

For production deployment, consider using Kubernetes with the provided Docker image.
