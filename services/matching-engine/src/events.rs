use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum MatchingEvent {
    OrderPlaced {
        order_id: Uuid,
        user_id: Uuid,
        symbol: String,
        side: String,
        price: String,
        quantity: String,
        order_type: String,
        time_in_force: String,
        client_order_id: Option<String>,
        timestamp: u64,
    },
    OrderMatched {
        taker_order_id: Uuid,
        maker_order_id: Uuid,
        taker_user_id: Uuid,
        maker_user_id: Uuid,
        symbol: String,
        price: String,
        quantity: String,
        match_id: u64,
        timestamp: u64,
    },
    OrderCancelled {
        order_id: Uuid,
        symbol: String,
        reason: String,
        timestamp: u64,
    },
}

pub const KAFKA_TOPIC_MATCHING_EVENTS: &str = "matching.events";
pub const KAFKA_TOPIC_ORDER_COMMANDS: &str = "order.commands";
