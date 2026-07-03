use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct DbPool {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TradeRow {
    pub id: i64,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub quote_quantity: Decimal,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub taker_side: String,
    pub trade_time: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Ticker24hr {
    pub symbol: String,
    pub open_price: Decimal,
    pub high_price: Decimal,
    pub low_price: Decimal,
    pub last_price: Decimal,
    pub volume: Decimal,
    pub quote_volume: Decimal,
    pub count: i64,
}

impl DbPool {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn get_recent_trades(
        &self,
        symbol: &str,
        limit: i64,
    ) -> Result<Vec<TradeRow>, sqlx::Error> {
        sqlx::query_as::<_, TradeRow>(
            "SELECT id, symbol, price, quantity, quote_quantity, buyer_order_id, seller_order_id, taker_side, trade_time FROM trades WHERE symbol = $1 ORDER BY trade_time DESC LIMIT $2",
        )
        .bind(symbol)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_trades_24hr(
        &self,
        symbol: &str,
    ) -> Result<Vec<TradeRow>, sqlx::Error> {
        sqlx::query_as::<_, TradeRow>(
            "SELECT id, symbol, price, quantity, quote_quantity, buyer_order_id, seller_order_id, taker_side, trade_time FROM trades WHERE symbol = $1 AND trade_time > NOW() - INTERVAL '24 hours' ORDER BY trade_time ASC",
        )
        .bind(symbol)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_last_price(
        &self,
        symbol: &str,
    ) -> Result<Option<Decimal>, sqlx::Error> {
        let row: Option<(Decimal,)> = sqlx::query_as(
            "SELECT price FROM trades WHERE symbol = $1 ORDER BY trade_time DESC LIMIT 1",
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0))
    }
}
