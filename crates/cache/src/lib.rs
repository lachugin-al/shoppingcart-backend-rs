//! In-memory cache for storing orders by order_uid, with thread-safe access
//! and async population from the database.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use model::Order;
use repository::{OrdersRepository, DeliveriesRepository, PaymentsRepository, ItemsRepository};
use anyhow::Result;
use deadpool_postgres::{Pool, Object as DbConn};

/// Thread-safe in-memory order cache.
#[derive(Debug, Default)]
pub struct OrderCache {
    inner: Arc<RwLock<HashMap<String, Order>>>,
}

impl OrderCache {
    /// Create a new, empty order cache.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load all orders from the DB into the cache.
    ///
    /// # Arguments
    /// * `pool` — deadpool Postgres pool for DB connection.
    /// * `orders_repo`, `deliveries_repo`, `payments_repo`, `items_repo` — repositories for full order data.
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

    /// Get a cloned order by its order_uid (None if not found).
    pub async fn get(&self, order_uid: &str) -> Option<Order> {
        let map = self.inner.read().await;
        map.get(order_uid).cloned()
    }

    /// Insert or update an order in the cache.
    pub async fn set(&self, order: Order) {
        let mut map = self.inner.write().await;
        map.insert(order.order_uid.clone(), order);
    }
}

/// Helper to load the full order with all relations from the repositories.
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

/// Helper to get all order_uids from the "orders" table.
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
    use model::{Order, Delivery, Payment, Item};

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
            items: vec![
                Item {
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
                }
            ],
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
