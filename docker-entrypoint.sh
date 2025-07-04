#!/bin/bash
set -e

# Enable debug mode to see all commands being executed
set -x

# Print all environment variables for debugging
echo "=== Environment Variables ==="
env | sort
echo "==========================="

# Check if required directories exist
echo "=== Directory Structure ==="
ls -la /app
echo "==========================="

# Function to wait for a service to be ready
wait_for_service() {
  local host="$1"
  local port="$2"
  local service="$3"
  local max_attempts=30
  local attempt=1

  echo "Waiting for $service at $host:$port..."

  while ! nc -z "$host" "$port" >/dev/null 2>&1; do
    if [ $attempt -ge $max_attempts ]; then
      echo "Error: $service at $host:$port is not available after $max_attempts attempts. Exiting."
      exit 1
    fi

    echo "Attempt $attempt: $service at $host:$port is not ready yet. Waiting..."
    sleep 2
    attempt=$((attempt + 1))
  done

  echo "$service at $host:$port is ready!"
}

# Wait for PostgreSQL
echo "Checking PostgreSQL connection..."
wait_for_service "$DB_HOST" "$DB_PORT" "PostgreSQL"

# Wait for Kafka
echo "Checking Kafka connection..."
KAFKA_HOST=$(echo "$KAFKA_BROKERS" | cut -d ':' -f 1)
KAFKA_PORT=$(echo "$KAFKA_BROKERS" | cut -d ':' -f 2)
wait_for_service "$KAFKA_HOST" "$KAFKA_PORT" "Kafka"

echo "All services are ready. Starting the application..."

# Create migrations directory if it doesn't exist
mkdir -p /app/migrations

# Execute the original command (the app) with debug output
echo "Running command: $@"
echo "Setting RUST_LOG=debug for more verbose logging"
export RUST_LOG=debug
exec "$@"
