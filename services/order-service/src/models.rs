use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PlaceOrderRequest {
    pub symbol: String,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub quantity: Decimal,
    pub price: Option<Decimal>,
    #[serde(default = "default_time_in_force")]
    pub time_in_force: String,
    pub new_client_order_id: Option<String>,
}

fn default_time_in_force() -> String {
    "GTC".to_string()
}

#[derive(Debug, Deserialize)]
pub struct QueryOrderParams {
    pub symbol: Option<String>,
    pub order_id: Option<Uuid>,
    pub orig_client_order_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CancelOrderParams {
    pub symbol: Option<String>,
    pub order_id: Option<Uuid>,
    pub orig_client_order_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AllOrdersParams {
    pub symbol: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct BookTickerQueryParams {
    pub symbol: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BookTickerResponse {
    pub symbol: String,
    pub bid_price: String,
    pub bid_qty: String,
    pub ask_price: String,
    pub ask_qty: String,
}

#[derive(Debug, Deserialize)]
pub struct DepthParams {
    pub symbol: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct DepthResponse {
    pub last_update_id: u64,
    pub bids: Vec<Vec<String>>,
    pub asks: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct MyTradesParams {
    pub symbol: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub symbol: String,
    pub order_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    pub price: String,
    pub orig_qty: String,
    pub executed_qty: String,
    pub status: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
    pub transact_time: u64,
}

#[derive(Debug, Serialize)]
pub struct TradeResponse {
    pub id: i64,
    pub symbol: String,
    pub price: String,
    pub qty: String,
    pub quote_qty: String,
    pub order_id: Uuid,
    pub is_buyer: bool,
    pub time: u64,
}

impl From<super::db::OrderRow> for OrderResponse {
    fn from(row: super::db::OrderRow) -> Self {
        Self {
            symbol: row.symbol,
            order_id: row.id,
            client_order_id: row.client_order_id,
            price: row
                .price
                .map(|p| p.to_string())
                .unwrap_or_else(|| "0".into()),
            orig_qty: row.quantity.to_string(),
            executed_qty: row.filled_quantity.to_string(),
            status: row.status,
            order_type: row.order_type,
            side: row.side,
            transact_time: row.created_at.timestamp_millis() as u64,
        }
    }
}

pub fn trade_to_response(
    row: super::db::TradeRow,
    requesting_user_id: uuid::Uuid,
) -> TradeResponse {
    let is_buyer = row.buyer_user_id == requesting_user_id;
    let order_id = if is_buyer {
        row.buyer_order_id
    } else {
        row.seller_order_id
    };

    TradeResponse {
        id: row.id,
        symbol: row.symbol,
        price: row.price.to_string(),
        qty: row.quantity.to_string(),
        quote_qty: row.quote_quantity.to_string(),
        order_id,
        is_buyer,
        time: row.trade_time.timestamp_millis() as u64,
    }
}
