FROM rustlang/rust:nightly-slim AS builder

# Install required components and dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libsasl2-dev \
    libzstd-dev \
    cmake \
    git \
    && rustup component add rustfmt clippy

WORKDIR /app

# Copy the Cargo.toml files to cache dependencies
COPY Cargo.toml .
COPY crates/app/Cargo.toml crates/app/
COPY crates/model/Cargo.toml crates/model/
COPY crates/repository/Cargo.toml crates/repository/
COPY crates/config/Cargo.toml crates/config/
COPY crates/db/Cargo.toml crates/db/
COPY crates/service/Cargo.toml crates/service/
COPY crates/cache/Cargo.toml crates/cache/
COPY crates/kafka-consumer/Cargo.toml crates/kafka-consumer/
COPY crates/kafka-producer/Cargo.toml crates/kafka-producer/
COPY crates/server/Cargo.toml crates/server/

# Create dummy source files to build dependencies
RUN mkdir -p crates/app/src \
    crates/model/src \
    crates/repository/src \
    crates/config/src \
    crates/db/src \
    crates/service/src \
    crates/cache/src \
    crates/kafka-consumer/src \
    crates/kafka-producer/src \
    crates/server/src

# Create dummy main.rs and lib.rs files
RUN for dir in crates/*/src; do \
    if [ "$dir" = "crates/app/src" ]; then \
        echo "fn main() {}" > "$dir/main.rs"; \
    elif [ "$dir" = "crates/model/src" ]; then \
        echo 'use serde::{Deserialize, Serialize};\nuse chrono::{DateTime, Utc};\n\n#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]\npub struct Delivery {\n    pub name: String,\n    pub phone: String,\n    pub zip: String,\n    pub city: String,\n    pub address: String,\n    pub region: String,\n    pub email: String,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]\npub struct Payment {\n    pub transaction: String,\n    pub request_id: String,\n    pub currency: String,\n    pub provider: String,\n    pub amount: i32,\n    pub payment_dt: i64,\n    pub bank: String,\n    pub delivery_cost: i32,\n    pub goods_total: i32,\n    pub custom_fee: i32,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]\npub struct Item {\n    pub chrt_id: i32,\n    pub track_number: String,\n    pub price: i32,\n    pub rid: String,\n    pub name: String,\n    pub sale: i32,\n    pub size: String,\n    pub total_price: i32,\n    pub nm_id: i32,\n    pub brand: String,\n    pub status: i32,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]\npub struct Order {\n    pub order_uid: String,\n    pub track_number: String,\n    pub entry: String,\n    pub delivery: Delivery,\n    pub payment: Payment,\n    pub items: Vec<Item>,\n    pub locale: String,\n    pub internal_signature: String,\n    pub customer_id: String,\n    pub delivery_service: String,\n    pub shardkey: String,\n    pub sm_id: i32,\n    pub date_created: DateTime<Utc>,\n    pub oof_shard: String,\n}' > "$dir/lib.rs"; \
    else \
        echo "pub fn dummy() {}" > "$dir/lib.rs"; \
    fi; \
done

# Build dependencies
RUN cargo build --release

# Remove the dummy source files
RUN rm -rf crates/*/src

# Copy the actual source code
COPY crates crates/

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    netcat-openbsd \
    strace \
    procps \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/app /app/app

# Copy static files
COPY static /app/static

# Copy the entrypoint script
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Copy the wrapper script
COPY wrapper.sh /app/wrapper.sh
RUN chmod +x /app/wrapper.sh

# Create migrations directory
RUN mkdir -p /app/migrations

# Expose the HTTP port
EXPOSE 8081

# Set the entry point
ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/app/wrapper.sh"]
