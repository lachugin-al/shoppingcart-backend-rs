//! Kafka producer module for generating and sending test order messages.
//!
//! This module provides functionality to generate random order data
//! and publish it to a Kafka topic.

use anyhow::{Context, Result};
use app_config::AppConfig;
use chrono::Utc;
use fake::{Fake, Faker};
use model::{Delivery, Item, Order, Payment};
use rand::seq::SliceRandom;
use rdkafka::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::{Duration, SystemTime};
use tracing::{error, info};
use uuid::Uuid;

/// Generates a test order message, serializes it to JSON, and sends it to Kafka.
///
/// # Returns
/// - `Result<String>`: The unique identifier (OrderUID) of the order sent to Kafka,
///   or an error if the message could not be sent.
pub async fn produce_test_message() -> Result<String> {
    info!("Starting Kafka producer");

    // Load configuration
    let config = AppConfig::load().context("Failed to load config")?;

    // Create Kafka producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", config.kafka_brokers.join(","))
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create Kafka producer")?;

    info!(
        topic = %config.kafka_topic,
        "Kafka producer initialized"
    );

    // Generate and publish message
    let order = generate_order();
    let order_uid = order.order_uid.clone();

    // Serialize message to JSON
    let data = serde_json::to_string(&order).context("Failed to serialize order to JSON")?;

    // Publish message to Kafka
    let record = FutureRecord::to(&config.kafka_topic)
        .key(&order_uid)
        .payload(&data);

    match producer
        .send(record, Duration::from_secs(5))
        .await
        .map_err(|(kafka_err, owned_msg)| {
            anyhow::anyhow!("Kafka error: {:?}, Message: {:?}", kafka_err, owned_msg)
        })
        .context("Failed to send message to Kafka")
    {
        Ok(_) => {
            info!(order_uid = %order_uid, "Message published successfully");
            Ok(order_uid)
        }
        Err(e) => {
            error!(error = ?e, "Failed to publish message to Kafka");
            Err(e)
        }
    }
}

/// Generates a random order with all associated data.
///
/// # Returns
/// - `Order`: A structure containing all the necessary order data.
fn generate_order() -> Order {
    // Generate data for order
    let order_uid = Uuid::new_v4().to_string();
    let track_number = Faker.fake::<String>();

    // Generate delivery data
    let delivery = Delivery {
        name: Faker.fake::<String>(),
        phone: Faker.fake::<String>(),
        zip: Faker.fake::<String>(),
        city: Faker.fake::<String>(),
        address: Faker.fake::<String>(),
        region: Faker.fake::<String>(),
        email: Faker.fake::<String>(),
    };

    // Generate payment data
    let payment = Payment {
        transaction: Uuid::new_v4().to_string(),
        request_id: Uuid::new_v4().to_string(),
        currency: ["USD", "EUR", "GBP", "JPY"]
            .choose(&mut rand::thread_rng())
            .unwrap()
            .to_string(),
        provider: Faker.fake::<String>(),
        amount: (100..10000).fake(),
        payment_dt: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        bank: Faker.fake::<String>(),
        delivery_cost: (10..500).fake(),
        goods_total: (50..5000).fake(),
        custom_fee: (0..100).fake(),
    };

    // Generate items
    let item_count = (1..5).fake::<usize>();
    let mut items = Vec::with_capacity(item_count);

    for _ in 0..item_count {
        items.push(Item {
            chrt_id: (1000..9999).fake(),
            track_number: track_number.clone(),
            price: (100..1000).fake(),
            rid: Uuid::new_v4().to_string(),
            name: Faker.fake::<String>(),
            sale: (0..50).fake(),
            size: ["XS", "S", "M", "L", "XL"]
                .choose(&mut rand::thread_rng())
                .unwrap()
                .to_string(),
            total_price: (100..2000).fake(),
            nm_id: (100000..999999).fake(),
            brand: Faker.fake::<String>(),
            status: (1..3).fake(),
        });
    }

    Order {
        order_uid,
        track_number,
        entry: Faker.fake::<String>(),
        delivery,
        payment,
        items,
        locale: ["en", "ru", "de", "fr"]
            .choose(&mut rand::thread_rng())
            .unwrap()
            .to_string(),
        internal_signature: Uuid::new_v4().to_string(),
        customer_id: Uuid::new_v4().to_string(),
        delivery_service: Faker.fake::<String>(),
        shardkey: Faker.fake::<String>(),
        sm_id: (1..100).fake(),
        date_created: Utc::now(),
        oof_shard: Faker.fake::<String>(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_order() {
        let order = generate_order();

        // Basic validation
        assert!(!order.order_uid.is_empty());
        assert!(!order.track_number.is_empty());
        assert!(!order.items.is_empty());

        // Check that all items have the same track number as the order
        for item in &order.items {
            assert_eq!(item.track_number, order.track_number);
        }
    }
}
