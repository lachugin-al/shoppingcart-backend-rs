use async_trait::async_trait;
use model::{Delivery, Item, Order, Payment};
use thiserror::Error;
use tokio_postgres::Client;
use chrono::{NaiveDateTime};

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Db(#[from] tokio_postgres::Error),
    #[error("Not found")]
    NotFound,
}

/// --- DeliveriesRepository repository ---

#[async_trait]
pub trait DeliveriesRepository: Send + Sync {
    async fn insert(&self, delivery: &Delivery, order_uid: &str) -> Result<(), RepositoryError>;
    async fn get_by_order_id(&self, order_uid: &str) -> Result<Delivery, RepositoryError>;
}

pub struct PgDeliveriesRepository {
    db: Client,
}

impl PgDeliveriesRepository {
    pub fn new(db: Client) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DeliveriesRepository for PgDeliveriesRepository {
    async fn insert(&self, delivery: &Delivery, order_uid: &str) -> Result<(), RepositoryError> {
        let query = r#"
            INSERT INTO deliveries (order_uid, name, phone, zip, city, address, region, email)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#;
        self.db.execute(
            query,
            &[
                &order_uid,
                &delivery.name,
                &delivery.phone,
                &delivery.zip,
                &delivery.city,
                &delivery.address,
                &delivery.region,
                &delivery.email,
            ]
        ).await?;
        Ok(())
    }

    async fn get_by_order_id(&self, order_uid: &str) -> Result<Delivery, RepositoryError> {
        let query = r#"
            SELECT name, phone, zip, city, address, region, email
            FROM deliveries WHERE order_uid = $1
        "#;
        let row = self.db.query_opt(query, &[&order_uid]).await?;
        match row {
            Some(row) => Ok(Delivery {
                name: row.get("name"),
                phone: row.get("phone"),
                zip: row.get("zip"),
                city: row.get("city"),
                address: row.get("address"),
                region: row.get("region"),
                email: row.get("email"),
            }),
            None => Err(RepositoryError::NotFound),
        }
    }
}

/// --- ItemsRepository repository ---

#[async_trait]
pub trait ItemsRepository: Send + Sync {
    async fn insert(&self, items: &[Item], order_uid: &str) -> Result<(), RepositoryError>;
    async fn get_by_order_id(&self, order_uid: &str) -> Result<Vec<Item>, RepositoryError>;
}

pub struct PgItemsRepository {
    db: Client,
}

impl PgItemsRepository {
    pub fn new(db: Client) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ItemsRepository for PgItemsRepository {
    async fn insert(&self, items: &[Item], order_uid: &str) -> Result<(), RepositoryError> {
        let query = r#"
            INSERT INTO items (order_uid, chrt_id, track_number, price, rid, name, sale, size, total_price, nm_id, brand, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#;
        for it in items {
            self.db.execute(
                query,
                &[
                    &order_uid,
                    &it.chrt_id,
                    &it.track_number,
                    &it.price,
                    &it.rid,
                    &it.name,
                    &it.sale,
                    &it.size,
                    &it.total_price,
                    &it.nm_id,
                    &it.brand,
                    &it.status,
                ]
            ).await?;
        }
        Ok(())
    }

    async fn get_by_order_id(&self, order_uid: &str) -> Result<Vec<Item>, RepositoryError> {
        let query = r#"
            SELECT chrt_id, track_number, price, rid, name, sale, size, total_price, nm_id, brand, status
            FROM items WHERE order_uid = $1
        "#;
        let rows = self.db.query(query, &[&order_uid]).await?;
        let mut items = Vec::new();
        for row in rows {
            items.push(Item {
                chrt_id: row.get("chrt_id"),
                track_number: row.get("track_number"),
                price: row.get("price"),
                rid: row.get("rid"),
                name: row.get("name"),
                sale: row.get("sale"),
                size: row.get("size"),
                total_price: row.get("total_price"),
                nm_id: row.get("nm_id"),
                brand: row.get("brand"),
                status: row.get("status"),
            });
        }
        Ok(items)
    }
}

/// --- OrdersRepository repository ---

#[async_trait]
pub trait OrdersRepository: Send + Sync {
    async fn insert(&self, order: &Order) -> Result<(), RepositoryError>;
    async fn get_by_id(&self, order_uid: &str) -> Result<Order, RepositoryError>;
}

pub struct PgOrdersRepository {
    db: Client,
}

impl PgOrdersRepository {
    pub fn new(db: Client) -> Self {
        Self { db }
    }
}

#[async_trait]
impl OrdersRepository for PgOrdersRepository {
    async fn insert(&self, order: &Order) -> Result<(), RepositoryError> {
        let query = r#"
            INSERT INTO orders (
                order_uid, track_number, entry, locale, internal_signature,
                customer_id, delivery_service, shardkey, sm_id, date_created, oof_shard
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
        "#;
        // Используем .naive_utc() чтобы привести DateTime<Utc> к NaiveDateTime для postgres
        self.db.execute(
            query,
            &[
                &order.order_uid,
                &order.track_number,
                &order.entry,
                &order.locale,
                &order.internal_signature,
                &order.customer_id,
                &order.delivery_service,
                &order.shardkey,
                &order.sm_id,
                &order.date_created.naive_utc(),
                &order.oof_shard,
            ]
        ).await?;
        Ok(())
    }

    async fn get_by_id(&self, order_uid: &str) -> Result<Order, RepositoryError> {
        let query = r#"
            SELECT order_uid, track_number, entry, locale, internal_signature,
                   customer_id, delivery_service, shardkey, sm_id, date_created, oof_shard
            FROM orders WHERE order_uid = $1
        "#;
        let row = self.db.query_opt(query, &[&order_uid]).await?;
        match row {
            Some(row) => {
                // date_created: NaiveDateTime → DateTime<Utc>
                let date_created: NaiveDateTime = row.get("date_created");
                Ok(Order {
                    order_uid: row.get("order_uid"),
                    track_number: row.get("track_number"),
                    entry: row.get("entry"),
                    delivery: Delivery::default(),
                    payment: Payment::default(),
                    items: Vec::new(),
                    locale: row.get("locale"),
                    internal_signature: row.get("internal_signature"),
                    customer_id: row.get("customer_id"),
                    delivery_service: row.get("delivery_service"),
                    shardkey: row.get("shardkey"),
                    sm_id: row.get("sm_id"),
                    date_created: chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(date_created, chrono::Utc),
                    oof_shard: row.get("oof_shard"),
                })
            }
            None => Err(RepositoryError::NotFound),
        }
    }
}

/// --- PaymentsRepository repository ---

#[async_trait]
pub trait PaymentsRepository: Send + Sync {
    async fn insert(&self, payment: &Payment, order_uid: &str) -> Result<(), RepositoryError>;
    async fn get_by_order_id(&self, order_uid: &str) -> Result<Payment, RepositoryError>;
}

pub struct PgPaymentsRepository {
    db: Client,
}

impl PgPaymentsRepository {
    pub fn new(db: Client) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PaymentsRepository for PgPaymentsRepository {
    async fn insert(&self, payment: &Payment, order_uid: &str) -> Result<(), RepositoryError> {
        let query = r#"
            INSERT INTO payments (
                order_uid, transaction, request_id, currency, provider, amount, payment_dt,
                bank, delivery_cost, goods_total, custom_fee
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
        "#;
        self.db.execute(
            query,
            &[
                &order_uid,
                &payment.transaction,
                &payment.request_id,
                &payment.currency,
                &payment.provider,
                &payment.amount,
                &payment.payment_dt,
                &payment.bank,
                &payment.delivery_cost,
                &payment.goods_total,
                &payment.custom_fee,
            ]
        ).await?;
        Ok(())
    }

    async fn get_by_order_id(&self, order_uid: &str) -> Result<Payment, RepositoryError> {
        let query = r#"
            SELECT transaction, request_id, currency, provider, amount, payment_dt,
                   bank, delivery_cost, goods_total, custom_fee
            FROM payments WHERE order_uid = $1
        "#;
        let row = self.db.query_opt(query, &[&order_uid]).await?;
        match row {
            Some(row) => Ok(Payment {
                transaction: row.get("transaction"),
                request_id: row.get("request_id"),
                currency: row.get("currency"),
                provider: row.get("provider"),
                amount: row.get("amount"),
                payment_dt: row.get("payment_dt"),
                bank: row.get("bank"),
                delivery_cost: row.get("delivery_cost"),
                goods_total: row.get("goods_total"),
                custom_fee: row.get("custom_fee"),
            }),
            None => Err(RepositoryError::NotFound),
        }
    }
}
