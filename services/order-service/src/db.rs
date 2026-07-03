use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct DbPool {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrderRow {
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
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
    pub buyer_user_id: Uuid,
    pub seller_user_id: Uuid,
    pub taker_side: String,
    pub trade_time: DateTime<Utc>,
    pub match_id: i64,
}

#[derive(Debug)]
pub struct CreateOrderParams<'a> {
    pub user_id: Uuid,
    pub symbol: &'a str,
    pub side: &'a str,
    pub order_type: &'a str,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub time_in_force: &'a str,
    pub client_order_id: Option<&'a str>,
}

impl DbPool {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn create_order(
        &self,
        params: CreateOrderParams<'_>,
    ) -> Result<OrderRow, sqlx::Error> {
        sqlx::query_as::<_, OrderRow>(
            r#"INSERT INTO orders (user_id, symbol, side, order_type, price, quantity, time_in_force, client_order_id, status)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'NEW')
               RETURNING id, user_id, symbol, side, order_type, price, quantity, filled_quantity, status, time_in_force, client_order_id, created_at, updated_at, expires_at"#
        )
        .bind(params.user_id)
        .bind(params.symbol)
        .bind(params.side)
        .bind(params.order_type)
        .bind(params.price)
        .bind(params.quantity)
        .bind(params.time_in_force)
        .bind(params.client_order_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_order(
        &self,
        order_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<OrderRow>, sqlx::Error> {
        sqlx::query_as::<_, OrderRow>(
            "SELECT id, user_id, symbol, side, order_type, price, quantity, filled_quantity, status, time_in_force, client_order_id, created_at, updated_at, expires_at FROM orders WHERE id = $1 AND user_id = $2",
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_order_by_client_id(
        &self,
        client_order_id: &str,
        user_id: Uuid,
    ) -> Result<Option<OrderRow>, sqlx::Error> {
        sqlx::query_as::<_, OrderRow>(
            "SELECT id, user_id, symbol, side, order_type, price, quantity, filled_quantity, status, time_in_force, client_order_id, created_at, updated_at, expires_at FROM orders WHERE client_order_id = $1 AND user_id = $2",
        )
        .bind(client_order_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_orders(
        &self,
        user_id: Uuid,
        symbol: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrderRow>, sqlx::Error> {
        match symbol {
            Some(sym) => {
                sqlx::query_as::<_, OrderRow>(
                    "SELECT id, user_id, symbol, side, order_type, price, quantity, filled_quantity, status, time_in_force, client_order_id, created_at, updated_at, expires_at FROM orders WHERE user_id = $1 AND symbol = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
                )
                .bind(user_id)
                .bind(sym)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, OrderRow>(
                    "SELECT id, user_id, symbol, side, order_type, price, quantity, filled_quantity, status, time_in_force, client_order_id, created_at, updated_at, expires_at FROM orders WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                )
                .bind(user_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }
    }

    pub async fn update_order_status(
        &self,
        order_id: Uuid,
        status: &str,
        filled_quantity: Decimal,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE orders SET status = $1, filled_quantity = $2, updated_at = NOW() WHERE id = $3",
        )
        .bind(status)
        .bind(filled_quantity)
        .bind(order_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn cancel_order(
        &self,
        order_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<OrderRow>, sqlx::Error> {
        sqlx::query_as::<_, OrderRow>(
            "UPDATE orders SET status = 'CANCELLED', updated_at = NOW() WHERE id = $1 AND user_id = $2 AND status IN ('NEW', 'PARTIALLY_FILLED') RETURNING id, user_id, symbol, side, order_type, price, quantity, filled_quantity, status, time_in_force, client_order_id, created_at, updated_at, expires_at",
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
}
