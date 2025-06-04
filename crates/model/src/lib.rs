use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Delivery — информация о доставке заказа.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Delivery {
    pub name: String,
    pub phone: String,
    pub zip: String,
    pub city: String,
    pub address: String,
    pub region: String,
    pub email: String,
}

/// Payment — информация об оплате заказа.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Payment {
    pub transaction: String,
    #[serde(rename = "request_id")]
    pub request_id: String,
    pub currency: String,
    pub provider: String,
    pub amount: i32,
    #[serde(rename = "payment_dt")]
    pub payment_dt: i64,
    pub bank: String,
    #[serde(rename = "delivery_cost")]
    pub delivery_cost: i32,
    #[serde(rename = "goods_total")]
    pub goods_total: i32,
    #[serde(rename = "custom_fee")]
    pub custom_fee: i32,
}

/// Item — отдельный элемент заказа.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Item {
    #[serde(rename = "chrt_id")]
    pub chrt_id: i32,
    #[serde(rename = "track_number")]
    pub track_number: String,
    pub price: i32,
    pub rid: String,
    pub name: String,
    pub sale: i32,
    pub size: String,
    #[serde(rename = "total_price")]
    pub total_price: i32,
    #[serde(rename = "nm_id")]
    pub nm_id: i32,
    pub brand: String,
    pub status: i32,
}

/// Order — основной агрегат заказа.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Order {
    #[serde(rename = "order_uid")]
    pub order_uid: String,
    #[serde(rename = "track_number")]
    pub track_number: String,
    pub entry: String,
    pub delivery: Delivery,
    pub payment: Payment,
    pub items: Vec<Item>,
    pub locale: String,
    #[serde(rename = "internal_signature")]
    pub internal_signature: String,
    #[serde(rename = "customer_id")]
    pub customer_id: String,
    #[serde(rename = "delivery_service")]
    pub delivery_service: String,
    pub shardkey: String,
    #[serde(rename = "sm_id")]
    pub sm_id: i32,
    #[serde(rename = "date_created")]
    pub date_created: DateTime<Utc>,
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

        // Проверка chrono 0.4.23+: with_ymd_and_hms
        let expected = Utc.with_ymd_and_hms(2021, 11, 26, 6, 22, 19).unwrap();
        assert_eq!(order.date_created, expected);

        assert_eq!(order.date_created.to_rfc3339(), "2021-11-26T06:22:19+00:00");
    }
}
