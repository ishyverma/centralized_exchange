use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TradesQueryParams {
    pub symbol: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct HistoricalTradesQueryParams {
    pub symbol: String,
    pub limit: Option<i64>,
    pub from_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct KlineQueryParams {
    pub symbol: String,
    pub interval: String,
    pub limit: Option<i64>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DepthQueryParams {
    pub symbol: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TickerQueryParams {
    pub symbol: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TradeResponse {
    pub id: i64,
    pub price: String,
    pub qty: String,
    pub quote_qty: String,
    pub time: u64,
    pub is_buyer_maker: bool,
}

#[derive(Debug, Serialize)]
pub struct DepthResponse {
    pub last_update_id: u64,
    pub bids: Vec<Vec<String>>,
    pub asks: Vec<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct Ticker24hrResponse {
    pub symbol: String,
    pub price_change: String,
    pub price_change_percent: String,
    pub last_price: String,
    pub high_price: String,
    pub low_price: String,
    pub volume: String,
    pub quote_volume: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct TickerPriceResponse {
    pub symbol: String,
    pub price: String,
}

#[derive(Debug, Serialize)]
pub struct ExchangeInfoResponse {
    pub timezone: String,
    pub server_time: u64,
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Serialize)]
pub struct SymbolInfo {
    pub symbol: String,
    pub status: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub order_types: Vec<String>,
    pub is_spot_trading_allowed: bool,
}

#[derive(Debug, Serialize)]
pub struct BookTickerResponse {
    pub symbol: String,
    pub bid_price: String,
    pub bid_qty: String,
    pub ask_price: String,
    pub ask_qty: String,
}

impl From<crate::db::TradeRow> for TradeResponse {
    fn from(row: crate::db::TradeRow) -> Self {
        Self {
            id: row.id,
            price: row.price.to_string(),
            qty: row.quantity.to_string(),
            quote_qty: row.quote_quantity.to_string(),
            time: row.trade_time.timestamp_millis() as u64,
            is_buyer_maker: row.taker_side == "SELL",
        }
    }
}
