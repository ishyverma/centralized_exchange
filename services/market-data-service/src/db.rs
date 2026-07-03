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

    pub async fn get_trades_24hr(&self, symbol: &str) -> Result<Vec<TradeRow>, sqlx::Error> {
        sqlx::query_as::<_, TradeRow>(
            "SELECT id, symbol, price, quantity, quote_quantity, buyer_order_id, seller_order_id, taker_side, trade_time FROM trades WHERE symbol = $1 AND trade_time > NOW() - INTERVAL '24 hours' ORDER BY trade_time ASC",
        )
        .bind(symbol)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_last_price(&self, symbol: &str) -> Result<Option<Decimal>, sqlx::Error> {
        let row: Option<(Decimal,)> = sqlx::query_as(
            "SELECT price FROM trades WHERE symbol = $1 ORDER BY trade_time DESC LIMIT 1",
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0))
    }

    pub async fn get_historical_trades(
        &self,
        symbol: &str,
        limit: i64,
        from_id: Option<i64>,
    ) -> Result<Vec<TradeRow>, sqlx::Error> {
        match from_id {
            Some(fid) => {
                sqlx::query_as::<_, TradeRow>(
                    "SELECT id, symbol, price, quantity, quote_quantity, buyer_order_id, seller_order_id, taker_side, trade_time FROM trades WHERE symbol = $1 AND id < $2 ORDER BY id DESC LIMIT $3",
                )
                .bind(symbol)
                .bind(fid)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, TradeRow>(
                    "SELECT id, symbol, price, quantity, quote_quantity, buyer_order_id, seller_order_id, taker_side, trade_time FROM trades WHERE symbol = $1 ORDER BY id DESC LIMIT $2",
                )
                .bind(symbol)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
            }
        }
    }

    pub async fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: i64,
    ) -> Result<Vec<KlineRow>, sqlx::Error> {
        let interval_secs: i64 = match interval {
            "1m" => 60,
            "5m" => 300,
            "15m" => 900,
            "1h" => 3600,
            "4h" => 14400,
            "1d" => 86400,
            "1w" => 604800,
            "1M" => 2592000,
            _ => 60,
        };

        sqlx::query_as::<_, KlineRow>(
            "SELECT
                FLOOR(EXTRACT(EPOCH FROM trade_time) / $5::float) * $5::float * 1000 as bucket_start,
                (array_agg(price ORDER BY trade_time))[1] as open,
                MAX(price) as high,
                MIN(price) as low,
                (array_agg(price ORDER BY trade_time DESC))[1] as close,
                SUM(quantity) as volume,
                SUM(quote_quantity) as quote_volume,
                COUNT(*) as trade_count
             FROM trades
             WHERE symbol = $1
               AND ($2::bigint IS NULL OR EXTRACT(EPOCH FROM trade_time) * 1000 >= $2)
               AND ($3::bigint IS NULL OR EXTRACT(EPOCH FROM trade_time) * 1000 <= $3)
             GROUP BY bucket_start
             ORDER BY bucket_start DESC
             LIMIT $4",
        )
        .bind(symbol)
        .bind(start_time)
        .bind(end_time)
        .bind(limit)
        .bind(interval_secs)
        .fetch_all(&self.pool)
        .await
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct KlineRow {
    pub bucket_start: f64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub quote_volume: Decimal,
    pub trade_count: i64,
}
