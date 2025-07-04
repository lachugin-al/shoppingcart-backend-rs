FROM rustlang/rust:nightly-slim as builder

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
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/app /app/app

# Copy static files
COPY static /app/static

# Expose the HTTP port
EXPOSE 8081

# Set the entry point
CMD ["/app/app"]
