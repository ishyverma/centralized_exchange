use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use crate::events::MatchingEvent;
use crate::kafka::EventProducer;
use crate::order_book::{Order, OrderBook};

#[derive(Debug)]
pub struct MatchResult {
    pub maker_order_id: Uuid,
    pub maker_user_id: Uuid,
    pub price: Decimal,
    pub quantity: Decimal,
    pub match_id: u64,
}

#[derive(Debug, Clone)]
pub struct OrderBookState {
    pub bids: Vec<LevelSnapshot>,
    pub asks: Vec<LevelSnapshot>,
    pub sequence: u64,
}

#[derive(Debug, Clone)]
pub struct LevelSnapshot {
    pub price: Decimal,
    pub quantity: Decimal,
}

#[derive(Debug)]
pub struct PlaceOrderOutput {
    pub matches: Vec<MatchResult>,
    pub events: Vec<MatchingEvent>,
    pub remaining_quantity: Decimal,
    pub filled_quantity: Decimal,
    pub final_status: String,
}

#[derive(Debug)]
pub struct MatchingEngine {
    pub order_books: HashMap<String, OrderBook>,
    match_counter: u64,
    event_producer: EventProducer,
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            order_books: HashMap::new(),
            match_counter: 0,
            event_producer: EventProducer::from_kafka_hosts(None),
        }
    }

    pub fn with_kafka(hosts: Option<String>) -> Self {
        Self {
            order_books: HashMap::new(),
            match_counter: 0,
            event_producer: EventProducer::from_kafka_hosts(hosts),
        }
    }

    pub fn get_or_create_order_book(&mut self, symbol: &str) -> &mut OrderBook {
        self.order_books
            .entry(symbol.to_string())
            .or_insert_with(|| OrderBook::new(symbol))
    }

    pub fn get_order_book(&self, symbol: &str) -> Option<&OrderBook> {
        self.order_books.get(symbol)
    }

    pub fn get_order_book_state(&self, symbol: &str) -> Option<OrderBookState> {
        self.order_books.get(symbol).map(|book| OrderBookState {
            bids: book
                .bids
                .iter()
                .rev()
                .map(|(price, level)| LevelSnapshot {
                    price: *price,
                    quantity: level.total_quantity(),
                })
                .collect(),
            asks: book
                .asks
                .iter()
                .map(|(price, level)| LevelSnapshot {
                    price: *price,
                    quantity: level.total_quantity(),
                })
                .collect(),
            sequence: book.sequence,
        })
    }

    pub fn place_order(&mut self, order: &Order) -> PlaceOrderOutput {
        let order_type = order.order_type.to_uppercase();
        match order_type.as_str() {
            "MARKET" => self.place_market_order(order),
            _ => self.place_limit_order(order),
        }
    }

    fn place_market_order(&mut self, order: &Order) -> PlaceOrderOutput {
        let taker_side = order.side.to_uppercase();
        let maker_side = if taker_side == "BUY" { "SELL" } else { "BUY" };

        let maker_levels: Vec<Decimal> = {
            let book = match self.order_books.get(&order.symbol) {
                Some(b) => b,
                None => {
                    return PlaceOrderOutput {
                        matches: Vec::new(),
                        events: Vec::new(),
                        remaining_quantity: order.quantity,
                        filled_quantity: Decimal::ZERO,
                        final_status: "EXPIRED".to_string(),
                    };
                }
            };

            let levels_map = match maker_side {
                "SELL" => &book.asks,
                "BUY" => &book.bids,
                _ => return PlaceOrderOutput {
                    matches: Vec::new(),
                    events: Vec::new(),
                    remaining_quantity: order.quantity,
                    filled_quantity: Decimal::ZERO,
                    final_status: "EXPIRED".to_string(),
                },
            };

            match taker_side.as_str() {
                "BUY" => levels_map.keys().copied().collect(),
                "SELL" => levels_map.keys().rev().copied().collect(),
                _ => return PlaceOrderOutput {
                    matches: Vec::new(),
                    events: Vec::new(),
                    remaining_quantity: order.quantity,
                    filled_quantity: Decimal::ZERO,
                    final_status: "EXPIRED".to_string(),
                },
            }
        };

        self.execute_matches(order, &maker_levels, &taker_side, maker_side, true)
    }

    fn place_limit_order(&mut self, order: &Order) -> PlaceOrderOutput {
        let taker_side = order.side.to_uppercase();
        let maker_side = if taker_side == "BUY" { "SELL" } else { "BUY" };

        let taker_price = match order.price {
            Some(p) => p,
            None => {
                return PlaceOrderOutput {
                    matches: Vec::new(),
                    events: Vec::new(),
                    remaining_quantity: order.quantity,
                    filled_quantity: Decimal::ZERO,
                    final_status: "NEW".to_string(),
                };
            }
        };

        let maker_levels: Vec<Decimal> = {
            let book = match self.order_books.get(&order.symbol) {
                Some(b) => b,
                None => {
                    let is_ioc_or_fok = order.time_in_force == "IOC" || order.time_in_force == "FOK";
                    if is_ioc_or_fok {
                        return PlaceOrderOutput {
                            matches: Vec::new(),
                            events: Vec::new(),
                            remaining_quantity: order.quantity,
                            filled_quantity: Decimal::ZERO,
                            final_status: "EXPIRED".to_string(),
                        };
                    }
                    let placed_event = MatchingEvent::OrderPlaced {
                        order_id: order.id,
                        user_id: order.user_id,
                        symbol: order.symbol.clone(),
                        side: order.side.clone(),
                        price: order.price.map(|p| p.to_string()).unwrap_or_default(),
                        quantity: order.quantity.to_string(),
                        order_type: order.order_type.clone(),
                        time_in_force: order.time_in_force.clone(),
                        client_order_id: order.client_order_id.clone(),
                        timestamp: order.created_at,
                    };
                    self.add_order_to_book(order);
                    self.event_producer.publish(&placed_event);
                    return PlaceOrderOutput {
                        matches: Vec::new(),
                        events: vec![placed_event],
                        remaining_quantity: order.quantity,
                        filled_quantity: Decimal::ZERO,
                        final_status: "NEW".to_string(),
                    };
                }
            };

            let levels_map = match maker_side {
                "SELL" => &book.asks,
                "BUY" => &book.bids,
                _ => return PlaceOrderOutput {
                    matches: Vec::new(),
                    events: Vec::new(),
                    remaining_quantity: order.quantity,
                    filled_quantity: Decimal::ZERO,
                    final_status: "NEW".to_string(),
                },
            };

            match taker_side.as_str() {
                "BUY" => levels_map
                    .keys()
                    .take_while(|p| **p <= taker_price)
                    .copied()
                    .collect(),
                "SELL" => levels_map
                    .keys()
                    .rev()
                    .take_while(|p| **p >= taker_price)
                    .copied()
                    .collect(),
                _ => return PlaceOrderOutput {
                    matches: Vec::new(),
                    events: Vec::new(),
                    remaining_quantity: order.quantity,
                    filled_quantity: Decimal::ZERO,
                    final_status: "NEW".to_string(),
                },
            }
        };

        self.execute_matches(order, &maker_levels, &taker_side, maker_side, false)
    }

    fn execute_matches(
        &mut self,
        order: &Order,
        maker_levels: &[Decimal],
        _taker_side: &str,
        maker_side: &str,
        is_market: bool,
    ) -> PlaceOrderOutput {
        let mut matches = Vec::new();
        let mut events = Vec::new();
        let mut remaining = order.quantity - order.filled_quantity;
        let mut local_counter = self.match_counter;

        let is_ioc = order.time_in_force == "IOC";
        let is_fok = order.time_in_force == "FOK";

        let book_snapshot = if is_fok {
            self.order_books.get(&order.symbol).cloned()
        } else {
            None
        };

        for level_price in maker_levels {
            if remaining <= Decimal::ZERO {
                break;
            }

            loop {
                let should_continue = {
                    let book = match self.order_books.get_mut(&order.symbol) {
                        Some(b) => b,
                        None => break,
                    };

                    let levels_map = match maker_side {
                        "SELL" => &mut book.asks,
                        "BUY" => &mut book.bids,
                        _ => break,
                    };

                    let level = match levels_map.get_mut(level_price) {
                        Some(l) => l,
                        None => break,
                    };

                    let maker_order = match level.orders.first_mut() {
                        Some(o) => o,
                        None => break,
                    };

                    let remaining_maker = maker_order.quantity - maker_order.filled_quantity;
                    if remaining_maker <= Decimal::ZERO {
                        level.orders.remove(0);
                        continue;
                    }

                    let match_qty = remaining.min(remaining_maker);
                    maker_order.filled_quantity += match_qty;

                    local_counter += 1;

                    matches.push(MatchResult {
                        maker_order_id: maker_order.order_id,
                        maker_user_id: maker_order.user_id,
                        price: *level_price,
                        quantity: match_qty,
                        match_id: local_counter,
                    });

                    let match_event = MatchingEvent::OrderMatched {
                        taker_order_id: order.id,
                        maker_order_id: maker_order.order_id,
                        taker_user_id: order.user_id,
                        maker_user_id: maker_order.user_id,
                        symbol: order.symbol.clone(),
                        price: level_price.to_string(),
                        quantity: match_qty.to_string(),
                        match_id: local_counter,
                        timestamp: order.created_at,
                    };

                    self.event_producer.publish(&match_event);
                    events.push(match_event);

                    info!(
                        match_id = local_counter,
                        symbol = %order.symbol,
                        price = %level_price,
                        qty = %match_qty,
                        "Order matched"
                    );

                    remaining -= match_qty;

                    if maker_order.filled_quantity >= maker_order.quantity {
                        level.orders.remove(0);
                    }

                    remaining > Decimal::ZERO
                };

                if !should_continue {
                    break;
                }
            }

            let book = match self.order_books.get_mut(&order.symbol) {
                Some(b) => b,
                None => break,
            };

            let levels_map = match maker_side {
                "SELL" => &mut book.asks,
                "BUY" => &mut book.bids,
                _ => break,
            };

            if let Some(level) = levels_map.get_mut(level_price) {
                level.orders.retain(|o| o.filled_quantity < o.quantity);
                if level.orders.is_empty() {
                    levels_map.remove(level_price);
                }
            }
        }

        self.match_counter = local_counter;

        let filled_quantity = order.quantity - remaining;
        let final_status = if is_fok && remaining > Decimal::ZERO {
            if let Some(snapshot) = book_snapshot {
                self.order_books.insert(order.symbol.clone(), snapshot);
            }
            matches.clear();
            events.clear();
            "EXPIRED".to_string()
        } else if remaining <= Decimal::ZERO {
            "FILLED".to_string()
        } else if filled_quantity > Decimal::ZERO {
            "PARTIALLY_FILLED".to_string()
        } else if is_market || is_ioc || is_fok {
            "EXPIRED".to_string()
        } else {
            "NEW".to_string()
        };

        if remaining > Decimal::ZERO {
            if is_market || is_ioc || is_fok {
                self.event_producer.publish(&MatchingEvent::OrderCancelled {
                    order_id: order.id,
                    symbol: order.symbol.clone(),
                    reason: "IMMEDIATE_OR_CANCEL".to_string(),
                    timestamp: order.created_at,
                });
            } else {
                let updated_order = Order {
                    id: order.id,
                    user_id: order.user_id,
                    symbol: order.symbol.clone(),
                    side: order.side.clone(),
                    order_type: order.order_type.clone(),
                    price: order.price,
                    quantity: order.quantity,
                    filled_quantity,
                    status: final_status.clone(),
                    time_in_force: order.time_in_force.clone(),
                    client_order_id: order.client_order_id.clone(),
                    created_at: order.created_at,
                };
                let placed_event = MatchingEvent::OrderPlaced {
                    order_id: updated_order.id,
                    user_id: updated_order.user_id,
                    symbol: updated_order.symbol.clone(),
                    side: updated_order.side.clone(),
                    price: updated_order.price.map(|p| p.to_string()).unwrap_or_default(),
                    quantity: updated_order.remaining_qty().to_string(),
                    order_type: updated_order.order_type.clone(),
                    time_in_force: updated_order.time_in_force.clone(),
                    client_order_id: updated_order.client_order_id.clone(),
                    timestamp: updated_order.created_at,
                };
                self.event_producer.publish(&placed_event);
                events.push(placed_event);
                self.add_order_to_book(&updated_order);
            }
        }

        if let Some(book) = self.order_books.get_mut(&order.symbol) {
            book.sequence += 1;
        }

        PlaceOrderOutput {
            matches,
            events,
            remaining_quantity: remaining,
            filled_quantity,
            final_status,
        }
    }

    fn add_order_to_book(&mut self, order: &Order) {
        let book = self.get_or_create_order_book(&order.symbol);
        book.add_order(order);
    }

    pub fn cancel_order(
        &mut self,
        order_id: Uuid,
        symbol: &str,
        side: &str,
        price: Decimal,
    ) -> Vec<MatchingEvent> {
        if let Some(book) = self.order_books.get_mut(symbol) {
            book.remove_order(order_id, side, price);
        }

        let event = MatchingEvent::OrderCancelled {
            order_id,
            symbol: symbol.to_string(),
            reason: "USER_CANCELLED".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        self.event_producer.publish(&event);
        vec![event]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn create_limit_order(id: Uuid, side: &str, price: &str, qty: &str) -> Order {
        Order {
            id,
            user_id: Uuid::new_v4(),
            symbol: "BTCUSDT".to_string(),
            side: side.to_string(),
            order_type: "LIMIT".to_string(),
            price: Some(Decimal::from_str(price).unwrap()),
            quantity: Decimal::from_str(qty).unwrap(),
            filled_quantity: Decimal::ZERO,
            status: "NEW".to_string(),
            time_in_force: "GTC".to_string(),
            client_order_id: None,
            created_at: 1_000_000,
        }
    }

    fn create_market_order(id: Uuid, side: &str, qty: &str) -> Order {
        Order {
            id,
            user_id: Uuid::new_v4(),
            symbol: "BTCUSDT".to_string(),
            side: side.to_string(),
            order_type: "MARKET".to_string(),
            price: None,
            quantity: Decimal::from_str(qty).unwrap(),
            filled_quantity: Decimal::ZERO,
            status: "NEW".to_string(),
            time_in_force: "GTC".to_string(),
            client_order_id: None,
            created_at: 1_000_000,
        }
    }

    #[test]
    fn test_basic_limit_match() {
        let mut engine = MatchingEngine::new();
        let sell_id = Uuid::new_v4();
        let buy_id = Uuid::new_v4();

        let sell_order = create_limit_order(sell_id, "SELL", "50000", "1");
        let buy_order = create_limit_order(buy_id, "BUY", "50000", "1");

        engine.place_order(&sell_order);
        let output = engine.place_order(&buy_order);

        assert_eq!(output.matches.len(), 1);
        assert_eq!(output.matches[0].price, Decimal::from_str("50000").unwrap());
        assert_eq!(output.matches[0].quantity, Decimal::from_str("1").unwrap());
        assert_eq!(output.final_status, "FILLED");
    }

    #[test]
    fn test_bid_higher_than_ask_matches() {
        let mut engine = MatchingEngine::new();
        let sell_id = Uuid::new_v4();
        let buy_id = Uuid::new_v4();

        let sell_order = create_limit_order(sell_id, "SELL", "49900", "1");
        let buy_order = create_limit_order(buy_id, "BUY", "50000", "1");

        engine.place_order(&sell_order);
        let output = engine.place_order(&buy_order);

        assert_eq!(output.matches.len(), 1);
        assert_eq!(output.matches[0].price, Decimal::from_str("49900").unwrap());
    }

    #[test]
    fn test_no_match_price_too_low() {
        let mut engine = MatchingEngine::new();
        let sell_id = Uuid::new_v4();
        let buy_id = Uuid::new_v4();

        let sell_order = create_limit_order(sell_id, "SELL", "50000", "1");
        let buy_order = create_limit_order(buy_id, "BUY", "49900", "1");

        engine.place_order(&sell_order);
        let output = engine.place_order(&buy_order);

        assert_eq!(output.matches.len(), 0);
        assert_eq!(output.events.len(), 1);
        assert_eq!(output.final_status, "NEW");
    }

    #[test]
    fn test_partial_fill() {
        let mut engine = MatchingEngine::new();
        let sell_id = Uuid::new_v4();
        let buy_id = Uuid::new_v4();

        let sell_order = create_limit_order(sell_id, "SELL", "50000", "0.5");
        let buy_order = create_limit_order(buy_id, "BUY", "50000", "1");

        engine.place_order(&sell_order);
        let output = engine.place_order(&buy_order);

        assert_eq!(output.matches.len(), 1);
        assert_eq!(output.matches[0].quantity, Decimal::from_str("0.5").unwrap());
        assert_eq!(output.final_status, "PARTIALLY_FILLED");

        let state = engine.get_order_book_state("BTCUSDT").unwrap();
        assert_eq!(state.bids.len(), 1);
        assert_eq!(state.bids[0].quantity, Decimal::from_str("0.5").unwrap());
    }

    #[test]
    fn test_multiple_levels() {
        let mut engine = MatchingEngine::new();
        let ask1 = Uuid::new_v4();
        let ask2 = Uuid::new_v4();
        let buy = Uuid::new_v4();

        engine.place_order(&create_limit_order(ask1, "SELL", "49900", "1"));
        engine.place_order(&create_limit_order(ask2, "SELL", "50000", "1"));

        let output = engine.place_order(&create_limit_order(buy, "BUY", "50000", "1.5"));
        assert_eq!(output.matches.len(), 2);
        let total: Decimal = output.matches.iter().map(|m| m.quantity).sum();
        assert_eq!(total, Decimal::from_str("1.5").unwrap());
    }

    #[test]
    fn test_cancel_order() {
        let mut engine = MatchingEngine::new();
        let order_id = Uuid::new_v4();
        let order = create_limit_order(order_id, "BUY", "50000", "1");

        engine.place_order(&order);
        let state = engine.get_order_book_state("BTCUSDT").unwrap();
        assert_eq!(state.bids.len(), 1);

        let events = engine.cancel_order(
            order_id,
            "BTCUSDT",
            "BUY",
            Decimal::from_str("50000").unwrap(),
        );
        assert_eq!(events.len(), 1);

        let state = engine.get_order_book_state("BTCUSDT").unwrap();
        assert_eq!(state.bids.len(), 0);
    }

    #[test]
    fn test_order_book_bid_ask_query() {
        let mut engine = MatchingEngine::new();
        engine.place_order(&create_limit_order(Uuid::new_v4(), "BUY", "50000", "1"));
        engine.place_order(&create_limit_order(Uuid::new_v4(), "BUY", "49900", "2"));
        engine.place_order(&create_limit_order(Uuid::new_v4(), "SELL", "50100", "1"));

        let state = engine.get_order_book_state("BTCUSDT").unwrap();
        assert_eq!(state.bids.len(), 2);
        assert_eq!(state.bids[0].price, Decimal::from_str("50000").unwrap());
        assert_eq!(state.bids[0].quantity, Decimal::from_str("1").unwrap());
        assert_eq!(state.asks.len(), 1);
        assert_eq!(state.asks[0].price, Decimal::from_str("50100").unwrap());
    }

    #[test]
    fn test_market_order_buy() {
        let mut engine = MatchingEngine::new();
        let sell_id = Uuid::new_v4();
        let buy_id = Uuid::new_v4();

        engine.place_order(&create_limit_order(sell_id, "SELL", "50000", "1"));
        let output = engine.place_order(&create_market_order(buy_id, "BUY", "1"));

        assert_eq!(output.matches.len(), 1);
        assert_eq!(output.final_status, "FILLED");
        assert_eq!(output.filled_quantity, Decimal::from_str("1").unwrap());
    }

    #[test]
    fn test_market_order_buy_multiple_levels() {
        let mut engine = MatchingEngine::new();
        engine.place_order(&create_limit_order(Uuid::new_v4(), "SELL", "49900", "0.5"));
        engine.place_order(&create_limit_order(Uuid::new_v4(), "SELL", "50000", "0.5"));

        let output = engine.place_order(&create_market_order(Uuid::new_v4(), "BUY", "1"));
        assert_eq!(output.matches.len(), 2);
        assert_eq!(output.final_status, "FILLED");
    }

    #[test]
    fn test_market_order_sell() {
        let mut engine = MatchingEngine::new();
        engine.place_order(&create_limit_order(Uuid::new_v4(), "BUY", "50000", "1"));

        let output = engine.place_order(&create_market_order(Uuid::new_v4(), "SELL", "1"));
        assert_eq!(output.matches.len(), 1);
        assert_eq!(output.final_status, "FILLED");
    }

    #[test]
    fn test_market_order_no_liquidity() {
        let mut engine = MatchingEngine::new();
        let output = engine.place_order(&create_market_order(Uuid::new_v4(), "BUY", "1"));
        assert_eq!(output.matches.len(), 0);
        assert_eq!(output.final_status, "EXPIRED");
    }

    #[test]
    fn test_ioc_limit_partial_fill_does_not_rest_on_book() {
        let mut engine = MatchingEngine::new();
        engine.place_order(&create_limit_order(Uuid::new_v4(), "SELL", "50000", "0.3"));

        let buy_id = Uuid::new_v4();
        let mut ioc_order = create_limit_order(buy_id, "BUY", "50000", "1");
        ioc_order.time_in_force = "IOC".to_string();

        let output = engine.place_order(&ioc_order);
        assert_eq!(output.matches.len(), 1);
        assert_eq!(output.final_status, "PARTIALLY_FILLED");

        let state = engine.get_order_book_state("BTCUSDT").unwrap();
        assert_eq!(state.bids.len(), 0);
    }

    #[test]
    fn test_fok_fails_if_not_fully_filled() {
        let mut engine = MatchingEngine::new();
        engine.place_order(&create_limit_order(Uuid::new_v4(), "SELL", "50000", "0.3"));

        let buy_id = Uuid::new_v4();
        let mut fok_order = create_limit_order(buy_id, "BUY", "50000", "1");
        fok_order.time_in_force = "FOK".to_string();

        let output = engine.place_order(&fok_order);
        assert_eq!(output.matches.len(), 0);
        assert_eq!(output.final_status, "EXPIRED");
    }

    #[test]
    fn test_fok_succeeds_if_fully_filled() {
        let mut engine = MatchingEngine::new();
        engine.place_order(&create_limit_order(Uuid::new_v4(), "SELL", "50000", "1"));

        let buy_id = Uuid::new_v4();
        let mut fok_order = create_limit_order(buy_id, "BUY", "50000", "1");
        fok_order.time_in_force = "FOK".to_string();

        let output = engine.place_order(&fok_order);
        assert_eq!(output.matches.len(), 1);
        assert_eq!(output.final_status, "FILLED");
    }

    #[test]
    fn test_limit_ioc_no_match_does_not_rest() {
        let mut engine = MatchingEngine::new();
        engine.place_order(&create_limit_order(Uuid::new_v4(), "SELL", "50100", "1"));

        let buy_id = Uuid::new_v4();
        let mut ioc_order = create_limit_order(buy_id, "BUY", "50000", "1");
        ioc_order.time_in_force = "IOC".to_string();

        let output = engine.place_order(&ioc_order);
        assert_eq!(output.matches.len(), 0);
        assert_eq!(output.final_status, "EXPIRED");
        let state = engine.get_order_book_state("BTCUSDT").unwrap();
        assert_eq!(state.bids.len(), 0);
    }
}
