//! Kafka consumer for ingesting orders and persisting them via OrderService.
//!
//! Reads JSON-encoded order messages from a Kafka topic, saves them to the DB
//! using `OrderService`, and updates the in-memory cache.

use anyhow::Result;
use cache::OrderCache;
use model::Order;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::error::KafkaError;
use rdkafka::message::{BorrowedMessage, Message};
use serde_json::from_slice;
use service::OrderService;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tracing::{debug, error, info};

/// KafkaConsumer wraps the underlying StreamConsumer and business dependencies.
pub struct KafkaConsumer<S: OrderService + Send + Sync + 'static> {
    consumer: StreamConsumer,
    order_service: Arc<S>,
    order_cache: Arc<OrderCache>,
}

impl<S: OrderService + Send + Sync + 'static> KafkaConsumer<S> {
    /// Create a new Kafka consumer for the specified brokers/topic/group.
    pub fn new(
        brokers: &[String],
        topic: &str,
        group_id: &str,
        order_service: Arc<S>,
        order_cache: Arc<OrderCache>,
    ) -> Result<Self, KafkaError> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers.join(","))
            .set("group.id", group_id)
            .set("enable.partition.eof", "false")
            .set("auto.offset.reset", "earliest")
            .set("enable.auto.commit", "true")
            .create()?;

        consumer.subscribe(&[topic])?;
        Ok(Self {
            consumer,
            order_service,
            order_cache,
        })
    }

    /// Runs the main consumption loop until the given context is cancelled.
    ///
    /// # Arguments
    /// * `shutdown`: a signal for graceful shutdown (e.g., tokio::sync::Notify).
    pub async fn run(&self, shutdown: Arc<tokio::sync::Notify>) -> Result<()> {
        let mut stream = self.consumer.stream();

        loop {
            tokio::select! {
                maybe_msg = stream.next() => {
                    match maybe_msg {
                        Some(Ok(msg)) => {
                            if let Err(e) = self.handle_message(&msg).await {
                                error!("Failed to handle Kafka message: {e}");
                            }
                        }
                        Some(Err(e)) => {
                            error!("Kafka error: {e}");
                        }
                        None => {
                            debug!("Kafka stream ended.");
                            break;
                        }
                    }
                }
                _ = shutdown.notified() => {
                    info!("Kafka consumer received shutdown signal.");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Handles a single message from Kafka: parses JSON, saves to DB, and caches.
    async fn handle_message(&self, msg: &BorrowedMessage<'_>) -> Result<()> {
        let payload = msg
            .payload()
            .ok_or_else(|| anyhow::anyhow!("Empty Kafka message payload"))?;

        let order: Order = match from_slice(payload) {
            Ok(order) => order,
            Err(e) => {
                error!("Failed to deserialize order JSON: {e}");
                return Ok(()); // Skip bad message, don't crash
            }
        };

        // Save to DB via OrderService
        match self.order_service.save_order(&order).await {
            Ok(()) => {
                // Only cache the order if it was successfully saved to the database
                self.order_cache.set(order).await;
                info!("Order processed and cached: {}", msg.offset());
            }
            Err(e) => {
                error!("Failed to save order to DB: {e}");
                // Skip caching if DB save failed
            }
        }

        Ok(())
    }

    /// Close the consumer, flushing resources.
    pub async fn close(&self) {
        // rdkafka automatically closes on drop, but you may want to call consumer.commit or flush here.
        // We'll just log for completeness.
        info!("Kafka consumer closed.");
    }
}
