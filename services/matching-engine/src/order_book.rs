use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: String,
    pub time_in_force: String,
    pub client_order_id: Option<String>,
    pub created_at: u64,
}

impl Order {
    pub fn remaining_qty(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }

    pub fn is_filled(&self) -> bool {
        self.filled_quantity >= self.quantity
    }
}

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: Decimal,
    pub orders: Vec<OrderRef>,
}

#[derive(Debug, Clone)]
pub struct OrderRef {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub created_at: u64,
}

impl PriceLevel {
    pub fn new(price: Decimal) -> Self {
        Self {
            price,
            orders: Vec::new(),
        }
    }

    pub fn add_order(&mut self, order: &Order) {
        self.orders.push(OrderRef {
            order_id: order.id,
            user_id: order.user_id,
            quantity: order.quantity,
            filled_quantity: order.filled_quantity,
            created_at: order.created_at,
        });
    }

    pub fn total_quantity(&self) -> Decimal {
        self.orders
            .iter()
            .map(|o| o.quantity - o.filled_quantity)
            .sum()
    }

    pub fn has_orders(&self) -> bool {
        self.orders.iter().any(|o| o.filled_quantity < o.quantity)
    }
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: BTreeMap<Decimal, PriceLevel>,
    pub asks: BTreeMap<Decimal, PriceLevel>,
    pub sequence: u64,
}

impl OrderBook {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            sequence: 0,
        }
    }

    pub fn add_order(&mut self, order: &Order) {
        let levels = match order.side.to_uppercase().as_str() {
            "BUY" => &mut self.bids,
            "SELL" => &mut self.asks,
            _ => return,
        };

        if let Some(price) = order.price {
            let level = levels
                .entry(price)
                .or_insert_with(|| PriceLevel::new(price));
            level.add_order(order);
            self.sequence += 1;
        }
    }

    pub fn remove_order(&mut self, order_id: Uuid, side: &str, price: Decimal) {
        let levels = match side.to_uppercase().as_str() {
            "BUY" => &mut self.bids,
            "SELL" => &mut self.asks,
            _ => return,
        };

        if let Some(level) = levels.get_mut(&price) {
            level.orders.retain(|o| o.order_id != order_id);
            if level.orders.is_empty() {
                levels.remove(&price);
            }
            self.sequence += 1;
        }
    }

    pub fn update_filled(
        &mut self,
        order_id: Uuid,
        side: &str,
        price: Decimal,
        filled_qty: Decimal,
    ) {
        let levels = match side.to_uppercase().as_str() {
            "BUY" => &mut self.bids,
            "SELL" => &mut self.asks,
            _ => return,
        };

        if let Some(level) = levels.get_mut(&price) {
            for order in &mut level.orders {
                if order.order_id == order_id {
                    order.filled_quantity += filled_qty;
                    break;
                }
            }
            level.orders.retain(|o| o.filled_quantity < o.quantity);
            if level.orders.is_empty() {
                levels.remove(&price);
            }
            self.sequence += 1;
        }
    }

    pub fn get_best_bid(&self) -> Option<(&Decimal, &PriceLevel)> {
        self.bids.iter().next_back()
    }

    pub fn get_best_ask(&self) -> Option<(&Decimal, &PriceLevel)> {
        self.asks.iter().next()
    }
}
