# Makefile for Shopping Cart Backend

# Variables
CARGO := cargo
DOCKER_COMPOSE := docker-compose
PSQL := psql
DB_URL := postgres://$(DB_USER):$(DB_PASSWORD)@localhost:$(DB_PORT)/$(DB_NAME)

# Required environment variables
REQUIRED_VARS := DB_USER DB_PASSWORD DB_PORT DB_NAME

# Check environment variables
.PHONY: check-env
check-env:
	@echo "Checking required environment variables..."
	@missing=0; \
	for var in $(REQUIRED_VARS); do \
		value=`printenv $$var || echo ""`; \
		if [ -z "$$value" ]; then \
			echo "Error: Required environment variable $$var is not set."; \
			missing=1; \
		fi; \
	done; \
	if [ $$missing -eq 1 ]; then \
		echo "Please set missing variables in your environment or create a .env file."; \
		exit 1; \
	fi
	@echo "All required environment variables are set."

# Default target
.PHONY: all
all: build

# Build targets
.PHONY: build
build:
	$(CARGO) build

.PHONY: build-release
build-release:
	$(CARGO) build --release

# Run targets
.PHONY: run
run:
	$(CARGO) run -p app

.PHONY: run-release
run-release:
	$(CARGO) run --release -p app

.PHONY: dev
dev: docker-up
	@echo "Starting development environment..."
	$(CARGO) run -p app

.PHONY: start-fresh
start-fresh: setup
	@echo "Starting fresh development environment..."
	$(CARGO) run -p app

# Test targets
.PHONY: test
test:
	$(CARGO) test

.PHONY: test-coverage
test-coverage:
	$(CARGO) tarpaulin --workspace

# Docker targets
.PHONY: docker-up
docker-up:
	$(DOCKER_COMPOSE) up -d

.PHONY: docker-down
docker-down:
	$(DOCKER_COMPOSE) down

.PHONY: docker-build
docker-build:
	$(DOCKER_COMPOSE) build

.PHONY: docker-logs
docker-logs:
	$(DOCKER_COMPOSE) logs -f

.PHONY: docker-restart
docker-restart:
	$(DOCKER_COMPOSE) restart

# Database targets
.PHONY: db-migrate
db-migrate: check-env
	@echo "Running database migrations..."
	@for file in migrations/*.sql; do \
		echo "Applying $$file..."; \
		$(PSQL) "$(DB_URL)" -f $$file; \
	done

.PHONY: setup
setup: docker-up check-env
	@echo "Setting up the project..."
	@echo "Waiting for database to be ready..."
	@sleep 5
	$(MAKE) db-migrate

.PHONY: db-reset
db-reset: check-env
	@echo "Resetting database..."
	$(PSQL) "$(DB_URL)" -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
	$(MAKE) db-migrate

# Code quality targets
.PHONY: fmt
fmt:
	$(CARGO) fmt

.PHONY: lint
lint:
	$(CARGO) clippy

# Clean targets
.PHONY: clean
clean:
	$(CARGO) clean

# Help target
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  all            - Build the project (default)"
	@echo "  build          - Build the project in debug mode"
	@echo "  build-release  - Build the project in release mode"
	@echo "  run            - Run the project in debug mode"
	@echo "  run-release    - Run the project in release mode"
	@echo "  dev            - Start Docker services and run the project"
	@echo "  start-fresh    - Setup project (Docker + migrations) and run it"
	@echo "  test           - Run tests"
	@echo "  test-coverage  - Run tests with coverage report"
	@echo "  docker-up      - Start all Docker containers"
	@echo "  docker-down    - Stop all Docker containers"
	@echo "  docker-build   - Build Docker images"
	@echo "  docker-logs    - Show Docker container logs"
	@echo "  docker-restart - Restart Docker containers"
	@echo "  setup          - Start Docker services and run migrations"
	@echo "  db-migrate     - Run database migrations"
	@echo "  db-reset       - Reset database and run migrations"
	@echo "  check-env      - Check if required environment variables are set"
	@echo "  fmt            - Format code with rustfmt"
	@echo "  lint           - Run clippy linter"
	@echo "  clean          - Clean build artifacts"
	@echo "  help           - Show this help message"
