use chrono::Utc;
use matching_engine::engine::MatchingEngine;
use matching_engine::order_book::Order;
use rust_decimal::Decimal;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TradeData {
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub quote_quantity: Decimal,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_user_id: Uuid,
    pub seller_user_id: Uuid,
    pub taker_side: String,
    pub trade_time: chrono::DateTime<Utc>,
    pub match_id: i64,
}

#[derive(Debug, Clone)]
pub struct MatchSummary {
    pub taker_order_id: Uuid,
    pub maker_order_id: Uuid,
    pub taker_filled: Decimal,
    pub maker_filled: Decimal,
    pub trade: Option<TradeData>,
}

#[derive(Clone)]
pub struct EngineClient {
    engine: std::sync::Arc<Mutex<MatchingEngine>>,
}

impl Default for EngineClient {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineClient {
    pub fn new() -> Self {
        Self {
            engine: std::sync::Arc::new(Mutex::new(MatchingEngine::new())),
        }
    }

    pub async fn process_order(
        &self,
        order_row: &crate::db::OrderRow,
    ) -> Result<Vec<MatchSummary>, String> {
        let order = Order {
            id: order_row.id,
            user_id: order_row.user_id,
            symbol: order_row.symbol.clone(),
            side: order_row.side.clone(),
            order_type: order_row.order_type.clone(),
            price: order_row.price,
            quantity: order_row.quantity,
            filled_quantity: Decimal::ZERO,
            status: order_row.status.clone(),
            time_in_force: order_row.time_in_force.clone(),
            client_order_id: order_row.client_order_id.clone(),
            created_at: order_row.created_at.timestamp_millis() as u64,
        };

        let mut engine = self.engine.lock().map_err(|e| e.to_string())?;
        let (matches, _events) = engine.place_limit_order(&order);

        let mut summaries = Vec::new();

        if order_row.side.to_uppercase() == "BUY" {
            let mut total_filled = Decimal::ZERO;
            for m in &matches {
                total_filled += m.quantity;
                let quote_qty = m.price * m.quantity;
                summaries.push(MatchSummary {
                    taker_order_id: order_row.id,
                    maker_order_id: m.maker_order_id,
                    taker_filled: total_filled,
                    maker_filled: m.quantity,
                    trade: Some(TradeData {
                        symbol: order_row.symbol.clone(),
                        price: m.price,
                        quantity: m.quantity,
                        quote_quantity: quote_qty,
                        buyer_order_id: order_row.id,
                        seller_order_id: m.maker_order_id,
                        buyer_user_id: order_row.user_id,
                        seller_user_id: m.maker_user_id,
                        taker_side: "BUY".to_string(),
                        trade_time: Utc::now(),
                        match_id: m.match_id as i64,
                    }),
                });
            }
        } else {
            let mut total_filled = Decimal::ZERO;
            for m in &matches {
                total_filled += m.quantity;
                let quote_qty = m.price * m.quantity;
                summaries.push(MatchSummary {
                    taker_order_id: order_row.id,
                    maker_order_id: m.maker_order_id,
                    taker_filled: total_filled,
                    maker_filled: m.quantity,
                    trade: Some(TradeData {
                        symbol: order_row.symbol.clone(),
                        price: m.price,
                        quantity: m.quantity,
                        quote_quantity: quote_qty,
                        buyer_order_id: m.maker_order_id,
                        seller_order_id: order_row.id,
                        buyer_user_id: m.maker_user_id,
                        seller_user_id: order_row.user_id,
                        taker_side: "SELL".to_string(),
                        trade_time: Utc::now(),
                        match_id: m.match_id as i64,
                    }),
                });
            }
        }

        Ok(summaries)
    }

    pub async fn cancel_order(&self, order_id: Uuid, symbol: &str, side: &str, price: Decimal) {
        if let Ok(mut engine) = self.engine.lock() {
            engine.cancel_order(order_id, symbol, side, price);
        }
    }
}
