use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Delivery - Information about order delivery.
///
/// Contains all the necessary details for shipping an order to a customer,
/// including contact information and address details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Delivery {
    /// Recipient's full name
    pub name: String,
    /// Contact phone number
    pub phone: String,
    /// Postal code
    pub zip: String,
    /// City name
    pub city: String,
    /// Street address
    pub address: String,
    /// Region or state
    pub region: String,
    /// Contact email address
    pub email: String,
}

/// Payment - Information about order payment.
///
/// Contains all the details related to a payment transaction,
/// including amounts, transaction IDs, and payment provider information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Payment {
    /// Unique transaction identifier
    pub transaction: String,
    /// Request identifier for the payment
    #[serde(rename = "request_id")]
    pub request_id: String,
    /// Currency code (e.g., USD, EUR)
    pub currency: String,
    /// Payment service provider name
    pub provider: String,
    /// Total payment amount
    pub amount: i32,
    /// Payment date/time as Unix timestamp
    #[serde(rename = "payment_dt")]
    pub payment_dt: i64,
    /// Bank name or identifier
    pub bank: String,
    /// Cost of delivery
    #[serde(rename = "delivery_cost")]
    pub delivery_cost: i32,
    /// Total cost of goods without delivery
    #[serde(rename = "goods_total")]
    pub goods_total: i32,
    /// Any additional fees
    #[serde(rename = "custom_fee")]
    pub custom_fee: i32,
}

/// Item - Individual order item.
///
/// Represents a single product in an order with its details
/// such as price, size, and tracking information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Item {
    /// Chart ID - unique identifier for the item in the chart
    #[serde(rename = "chrt_id")]
    pub chrt_id: i32,
    /// Tracking number for the item shipment
    #[serde(rename = "track_number")]
    pub track_number: String,
    /// Original price of the item
    pub price: i32,
    /// Row identifier
    pub rid: String,
    /// Product name
    pub name: String,
    /// Discount percentage
    pub sale: i32,
    /// Size information (may be numeric or descriptive like "S", "M", "L")
    pub size: String,
    /// Final price after applying discounts
    #[serde(rename = "total_price")]
    pub total_price: i32,
    /// Nomenclature ID - product catalog identifier
    #[serde(rename = "nm_id")]
    pub nm_id: i32,
    /// Brand name
    pub brand: String,
    /// Item status code
    pub status: i32,
}

/// Order - Main order aggregate.
///
/// The central entity in the shopping cart system that combines all information
/// about a customer's purchase, including delivery details, payment information,
/// and the items being ordered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Order {
    /// Unique identifier for the order
    #[serde(rename = "order_uid")]
    pub order_uid: String,
    /// Tracking number for the entire order
    #[serde(rename = "track_number")]
    pub track_number: String,
    /// Entry point identifier
    pub entry: String,
    /// Delivery information
    pub delivery: Delivery,
    /// Payment details
    pub payment: Payment,
    /// List of items in the order
    pub items: Vec<Item>,
    /// Language/locale code
    pub locale: String,
    /// Internal signature for verification
    #[serde(rename = "internal_signature")]
    pub internal_signature: String,
    /// Customer identifier
    #[serde(rename = "customer_id")]
    pub customer_id: String,
    /// Delivery service provider
    #[serde(rename = "delivery_service")]
    pub delivery_service: String,
    /// Sharding key for database partitioning
    pub shardkey: String,
    /// Service manager identifier
    #[serde(rename = "sm_id")]
    pub sm_id: i32,
    /// Order creation timestamp
    #[serde(rename = "date_created")]
    pub date_created: DateTime<Utc>,
    /// Out-of-stock shard identifier
    #[serde(rename = "oof_shard")]
    pub oof_shard: String,
}

#[cfg(test)]
mod tests {
    use super::Order;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_deserialize_order_from_json() {
        let json = r#"
        {
           "order_uid": "b563feb7b2b84b6test",
           "track_number": "WBILMTESTTRACK",
           "entry": "WBIL",
           "delivery": {
              "name": "Test Testov",
              "phone": "+9720000000",
              "zip": "2639809",
              "city": "Kiryat Mozkin",
              "address": "Ploshad Mira 15",
              "region": "Kraiot",
              "email": "test@gmail.com"
           },
           "payment": {
              "transaction": "b563feb7b2b84b6test",
              "request_id": "",
              "currency": "USD",
              "provider": "wbpay",
              "amount": 1817,
              "payment_dt": 1637907727,
              "bank": "alpha",
              "delivery_cost": 1500,
              "goods_total": 317,
              "custom_fee": 0
           },
           "items": [
              {
                 "chrt_id": 9934930,
                 "track_number": "WBILMTESTTRACK",
                 "price": 453,
                 "rid": "ab4219087a764ae0btest",
                 "name": "Mascaras",
                 "sale": 30,
                 "size": "0",
                 "total_price": 317,
                 "nm_id": 2389212,
                 "brand": "Vivienne Sabo",
                 "status": 202
              }
           ],
           "locale": "en",
           "internal_signature": "",
           "customer_id": "test",
           "delivery_service": "meest",
           "shardkey": "9",
           "sm_id": 99,
           "date_created": "2021-11-26T06:22:19Z",
           "oof_shard": "1"
        }
        "#;
        let order: Order = serde_json::from_str(json).unwrap();
        assert_eq!(order.order_uid, "b563feb7b2b84b6test");
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].chrt_id, 9934930);

        // Check for chrono 0.4.23+: with_ymd_and_hms
        let expected = Utc.with_ymd_and_hms(2021, 11, 26, 6, 22, 19).unwrap();
        assert_eq!(order.date_created, expected);

        assert_eq!(order.date_created.to_rfc3339(), "2021-11-26T06:22:19+00:00");
    }
}
