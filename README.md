# Shopping Cart Backend

A Rust-based backend service for an e-commerce shopping cart system.

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
│   ├── kafka/                # Library crate: Kafka producer/consumer
│   │   └── src/lib.rs
│   │
│   ├── metrics/              # Library crate: Prometheus metrics
│   │   └── src/lib.rs
│   │
│   ├── util/                 # Library crate: Utilities, common code
│   │   └── src/lib.rs
│   │
│   └── tools/                # Binary crate: Utilities (e.g., migrations)
│       └── src/main.rs
│
└── tests/                    # Integration tests for different layers
```

## Overview

This project implements a backend service for a shopping cart system with the following features:

- Order management
- Payment processing
- Delivery tracking
- Item inventory

## Architecture

The application follows a modular architecture with clear separation of concerns:

- **Model**: Data structures and DTOs
- **Repository**: Data access layer with PostgreSQL implementation
- **Service**: Business logic and domain rules
- **API**: HTTP endpoints for client interaction
- **Cache**: In-memory caching for performance
- **Metrics**: Monitoring and observability

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Docker and Docker Compose
- PostgreSQL

### Setup

1. Clone the repository
2. Run `docker-compose up -d` to start the database
3. Run `cargo build` to build the project
4. Run `cargo run -p app` to start the server

### Environment Variables

Create a `.env` file in the project root with the following variables:

```
DATABASE_URL=postgres://user:password@localhost:5432/shoppingcart
KAFKA_BROKERS=localhost:9092
REDIS_URL=redis://localhost:6379
```

## Development

### Running Tests

```
cargo test
```

### Code Style

Follow the Rust standard code style. Run `cargo fmt` before committing.

## Deployment

The application can be deployed using Docker:

```
docker build -t shoppingcart-backend .
docker run -p 8080:8080 shoppingcart-backend
```
