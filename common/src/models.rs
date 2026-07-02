use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Limit,
    Market,
    StopLoss,
    TakeProfit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    Gtc,
    Ioc,
    Fok,
    Gtd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub totp_secret: Option<String>,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum UserStatus {
    Active,
    Suspended,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub api_key: String,
    pub permissions: Vec<String>,
    pub ip_whitelist: Option<Vec<String>>,
    pub status: ApiKeyStatus,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ApiKeyStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub user_id: Uuid,
    pub asset: String,
    pub total: rust_decimal::Decimal,
    pub available: rust_decimal::Decimal,
    pub reserved: rust_decimal::Decimal,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<rust_decimal::Decimal>,
    pub quantity: rust_decimal::Decimal,
    pub filled_quantity: rust_decimal::Decimal,
    pub status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub client_order_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: i64,
    pub symbol: String,
    pub price: rust_decimal::Decimal,
    pub quantity: rust_decimal::Decimal,
    pub quote_quantity: rust_decimal::Decimal,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_user_id: Uuid,
    pub seller_user_id: Uuid,
    pub taker_side: Side,
    pub trade_time: DateTime<Utc>,
}
