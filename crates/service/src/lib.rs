//! Business logic layer for order management.
//!
//! This module defines the [`OrderService`] trait and its async implementation [`OrderServiceImpl`].
//! The service coordinates multi-table order persistence and retrieval, providing
//! transactional guarantees, business validation, and repository abstraction.
//!
//! # Features
//! - Atomic saving of [`Order`]s (and related entities) in a single transaction.
//! - Validation of input data before persistence.
//! - Dependency injection for testability and loose coupling.
//! - Async-first API suitable for scalable web applications.
//! - Well-typed error handling via [`ServiceError`].

use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::{Pool, PoolError};
use model::Order;
use repository::{
    DeliveriesRepository, ItemsRepository, OrdersRepository, PaymentsRepository, RepositoryError,
};
use thiserror::Error;
use tracing::instrument;

/// The main error type for all operations in [`OrderService`] and [`OrderServiceImpl`].
#[derive(Debug, Error)]
pub enum ServiceError {
    /// The provided order is structurally or semantically invalid.
    #[error("Invalid order: {0}")]
    InvalidOrder(String),
    /// A repository (database) operation failed.
    #[error("Database error: {0}")]
    Db(#[from] RepositoryError),
    /// Failed to obtain a database connection from the pool.
    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),
    /// Some unexpected or unhandled error.
    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

/// Trait describing business operations for order management.
///
/// Service implementations are expected to guarantee atomicity and data integrity
/// when saving orders and their related entities, typically via a transaction.
#[async_trait]
pub trait OrderService: Send + Sync {
    /// Atomically persists the order and all related data (delivery, payment, items).
    ///
    /// # Arguments
    /// * `order` - The order to save.
    ///
    /// # Errors
    /// Returns [`ServiceError::InvalidOrder`] if validation fails,
    /// [`ServiceError::Db`] for DB-level errors, or [`ServiceError::Pool`] if
    /// a connection cannot be obtained.
    async fn save_order(&self, order: &Order) -> Result<(), ServiceError>;

    /// Retrieves the full order by its unique ID, including all related entities.
    ///
    /// # Arguments
    /// * `order_uid` - The unique identifier for the order.
    ///
    /// # Errors
    /// Returns [`ServiceError::Db`] or [`ServiceError::Pool`] on failure.
    async fn get_order_by_id(&self, order_uid: &str) -> Result<Order, ServiceError>;
}

/// Async implementation of [`OrderService`] using repository pattern.
///
/// This struct wires together concrete repository implementations and a Postgres
/// connection pool to enable atomic, transactional operations on orders.
pub struct OrderServiceImpl<R1, R2, R3, R4> {
    db_pool: Pool,
    orders_repo: R1,
    deliveries_repo: R2,
    payments_repo: R3,
    items_repo: R4,
}

impl<R1, R2, R3, R4> OrderServiceImpl<R1, R2, R3, R4>
where
    R1: OrdersRepository + Send + Sync,
    R2: DeliveriesRepository + Send + Sync,
    R3: PaymentsRepository + Send + Sync,
    R4: ItemsRepository + Send + Sync,
{
    /// Constructs a new [`OrderServiceImpl`] from the provided dependencies.
    ///
    /// # Arguments
    /// * `db_pool` - The Postgres connection pool to use for transactions.
    /// * `orders_repo` - The repository for main order data.
    /// * `deliveries_repo` - The repository for delivery information.
    /// * `payments_repo` - The repository for payment information.
    /// * `items_repo` - The repository for items information.
    ///
    /// This approach enables dependency injection and facilitates mocking/testing.
    pub fn new(
        db_pool: Pool,
        orders_repo: R1,
        deliveries_repo: R2,
        payments_repo: R3,
        items_repo: R4,
    ) -> Self {
        Self {
            db_pool,
            orders_repo,
            deliveries_repo,
            payments_repo,
            items_repo,
        }
    }

    /// Validates the structure and required fields of the order.
    ///
    /// Returns [`ServiceError::InvalidOrder`] if any required field is missing or incorrect.
    fn validate_order(&self, order: &Order) -> Result<(), ServiceError> {
        if order.order_uid.is_empty() {
            return Err(ServiceError::InvalidOrder("order_uid is empty".into()));
        }
        if order.items.is_empty() {
            return Err(ServiceError::InvalidOrder("order has no items".into()));
        }
        if order.delivery.name.is_empty() || order.delivery.phone.is_empty() {
            return Err(ServiceError::InvalidOrder("invalid delivery data".into()));
        }
        Ok(())
    }
}

#[async_trait]
impl<R1, R2, R3, R4> OrderService for OrderServiceImpl<R1, R2, R3, R4>
where
    R1: OrdersRepository + Send + Sync,
    R2: DeliveriesRepository + Send + Sync,
    R3: PaymentsRepository + Send + Sync,
    R4: ItemsRepository + Send + Sync,
{
    /// Atomically saves the order and all related entities in a single DB transaction.
    ///
    /// If validation fails or any repository operation returns an error, the entire
    /// transaction is rolled back and an appropriate error is returned.
    ///
    /// # Arguments
    /// * `order` - The order to be saved.
    #[instrument(skip(self, order))]
    async fn save_order(&self, order: &Order) -> Result<(), ServiceError> {
        self.validate_order(order)?;

        let mut client = self.db_pool.get().await.map_err(ServiceError::from)?;
        let tx = client
            .transaction()
            .await
            .map_err(|e| ServiceError::Unexpected(format!("Begin transaction failed: {e}")))?;

        self.orders_repo.insert_tx(&tx, order).await?;
        self.deliveries_repo
            .insert_tx(&tx, &order.delivery, &order.order_uid)
            .await?;
        self.payments_repo
            .insert_tx(&tx, &order.payment, &order.order_uid)
            .await?;
        self.items_repo
            .insert_tx(&tx, &order.items, &order.order_uid)
            .await?;

        tx.commit()
            .await
            .map_err(|e| ServiceError::Unexpected(format!("Commit failed: {e}")))?;

        Ok(())
    }

    /// Loads a full order with delivery, payment, and items by its unique order_uid.
    ///
    /// Returns [`ServiceError::Db`] if the order or any related entity is not found.
    #[instrument(skip(self))]
    async fn get_order_by_id(&self, order_uid: &str) -> Result<Order, ServiceError> {
        let order = self.orders_repo.get_by_id(order_uid).await?;
        let delivery = self.deliveries_repo.get_by_order_id(order_uid).await?;
        let payment = self.payments_repo.get_by_order_id(order_uid).await?;
        let items = self.items_repo.get_by_order_id(order_uid).await?;

        Ok(Order {
            delivery,
            payment,
            items,
            ..order
        })
    }
}
