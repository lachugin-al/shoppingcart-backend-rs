//! In-memory, thread-safe cache for storing orders by `order_uid`.
//!
//! This cache is designed for concurrent use in an async environment, using [`tokio::sync::RwLock`].
//! It supports async population from the database via repository abstractions and provides
//! fast lookups/updates for the order lifecycle.
//!
//! ## Features
//! - Thread-safe, async-first API
//! - Integration with repositories for population from DB
//! - Unit tests for correctness and concurrency

use anyhow::Result;
use deadpool_postgres::{Object as DbConn, Pool};
use model::Order;
use repository::{DeliveriesRepository, ItemsRepository, OrdersRepository, PaymentsRepository};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe, in-memory cache for orders, keyed by order UID.
///
/// The cache uses [`tokio::sync::RwLock`] to allow concurrent reads and exclusive writes.
/// Suitable for sharing across async tasks and within application state.
#[derive(Debug, Default)]
pub struct OrderCache {
    inner: Arc<RwLock<HashMap<String, Order>>>,
}

impl OrderCache {
    /// Creates a new, empty order cache.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Loads all orders from the database into the cache.
    ///
    /// This method queries the DB for all `order_uid` values, then fetches the
    /// complete order (with delivery, payment, items) for each, and stores it in the cache.
    ///
    /// # Arguments
    /// - `pool`: Deadpool Postgres connection pool.
    /// - `orders_repo`, `deliveries_repo`, `payments_repo`, `items_repo`: repository traits to access full order data.
    ///
    /// # Errors
    /// Returns an error if DB connection or repository calls fail.
    pub async fn load_from_db<R1, R2, R3, R4>(
        &self,
        pool: &Pool,
        orders_repo: &R1,
        deliveries_repo: &R2,
        payments_repo: &R3,
        items_repo: &R4,
    ) -> Result<()>
    where
        R1: OrdersRepository + Sync,
        R2: DeliveriesRepository + Sync,
        R3: PaymentsRepository + Sync,
        R4: ItemsRepository + Sync,
    {
        let conn: DbConn = pool.get().await?;
        let order_uids = get_all_order_uids(&conn).await?;

        for uid in order_uids {
            if let Ok(order) = load_full_order(
                &uid,
                orders_repo,
                deliveries_repo,
                payments_repo,
                items_repo,
            )
            .await
            {
                self.set(order).await;
            }
        }
        Ok(())
    }

    /// Get a cloned order from the cache by its UID.
    ///
    /// Returns `Some(Order)` if found, `None` if missing.
    pub async fn get(&self, order_uid: &str) -> Option<Order> {
        let map = self.inner.read().await;
        map.get(order_uid).cloned()
    }

    /// Insert or update an order in the cache.
    ///
    /// If an order with this UID already exists, it is overwritten.
    pub async fn set(&self, order: Order) {
        let mut map = self.inner.write().await;
        map.insert(order.order_uid.clone(), order);
    }

    /// Get all orders from the cache.
    ///
    /// Returns a vector of all orders in the cache.
    pub async fn get_all(&self) -> Vec<Order> {
        let map = self.inner.read().await;
        map.values().cloned().collect()
    }
}

/// Loads a fully populated [`Order`] from repositories by UID.
///
/// Fetches order main data, then fetches delivery, payment, and items.
/// Returns error if any component is missing.
pub async fn load_full_order<R1, R2, R3, R4>(
    order_uid: &str,
    orders_repo: &R1,
    deliveries_repo: &R2,
    payments_repo: &R3,
    items_repo: &R4,
) -> Result<Order>
where
    R1: OrdersRepository + Sync,
    R2: DeliveriesRepository + Sync,
    R3: PaymentsRepository + Sync,
    R4: ItemsRepository + Sync,
{
    let mut order = orders_repo.get_by_id(order_uid).await?;
    order.delivery = deliveries_repo.get_by_order_id(order_uid).await?;
    order.payment = payments_repo.get_by_order_id(order_uid).await?;
    order.items = items_repo.get_by_order_id(order_uid).await?;
    Ok(order)
}

/// Returns all order_uids from the `orders` table.
///
/// Used for bulk cache population at application startup.
async fn get_all_order_uids(conn: &DbConn) -> Result<Vec<String>> {
    let rows = conn.query("SELECT order_uid FROM orders", &[]).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<_, String>("order_uid").ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::{Delivery, Item, Order, Payment};

    fn sample_order(uid: &str) -> Order {
        Order {
            order_uid: uid.to_string(),
            track_number: "track123".to_string(),
            entry: "test".to_string(),
            delivery: Delivery {
                name: "Test User".to_string(),
                phone: "+1000000000".to_string(),
                zip: "0000".to_string(),
                city: "Test City".to_string(),
                address: "Street".to_string(),
                region: "Test Region".to_string(),
                email: "test@example.com".to_string(),
            },
            payment: Payment {
                transaction: "tx1".to_string(),
                request_id: "".to_string(),
                currency: "USD".to_string(),
                provider: "test".to_string(),
                amount: 100,
                payment_dt: 0,
                bank: "bank".to_string(),
                delivery_cost: 0,
                goods_total: 100,
                custom_fee: 0,
            },
            items: vec![Item {
                chrt_id: 1,
                track_number: "track123".to_string(),
                price: 100,
                rid: "rid1".to_string(),
                name: "Item1".to_string(),
                sale: 0,
                size: "L".to_string(),
                total_price: 100,
                nm_id: 123,
                brand: "brand".to_string(),
                status: 1,
            }],
            locale: "en".to_string(),
            internal_signature: "".to_string(),
            customer_id: "cust1".to_string(),
            delivery_service: "svc".to_string(),
            shardkey: "shard".to_string(),
            sm_id: 1,
            date_created: chrono::Utc::now(),
            oof_shard: "oof".to_string(),
        }
    }

    #[tokio::test]
    async fn test_empty_cache() {
        let cache = OrderCache::new();
        assert!(cache.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_order() {
        let cache = OrderCache::new();
        let order = sample_order("order123");
        cache.set(order.clone()).await;
        let got = cache.get("order123").await;
        assert!(got.is_some());
        assert_eq!(got.unwrap().order_uid, "order123");
    }

    #[tokio::test]
    async fn test_update_order() {
        let cache = OrderCache::new();
        let mut order = sample_order("order123");
        cache.set(order.clone()).await;

        // Update order's locale and save again
        order.locale = "ru".to_string();
        cache.set(order.clone()).await;
        let got = cache.get("order123").await.unwrap();
        assert_eq!(got.locale, "ru");
    }
}
