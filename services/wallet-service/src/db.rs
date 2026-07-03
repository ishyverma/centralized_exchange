use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct DbPool {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct BalanceRow {
    pub user_id: Uuid,
    pub asset: String,
    pub total: Decimal,
    pub available: Decimal,
    pub reserved: Decimal,
    pub updated_at: DateTime<Utc>,
}

impl DbPool {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn get_balance(
        &self,
        user_id: Uuid,
        asset: &str,
    ) -> Result<Option<BalanceRow>, sqlx::Error> {
        sqlx::query_as::<_, BalanceRow>(
            "SELECT user_id, asset, total, available, reserved, updated_at FROM balances WHERE user_id = $1 AND asset = $2",
        )
        .bind(user_id)
        .bind(asset)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_balances(&self, user_id: Uuid) -> Result<Vec<BalanceRow>, sqlx::Error> {
        sqlx::query_as::<_, BalanceRow>(
            "SELECT user_id, asset, total, available, reserved, updated_at FROM balances WHERE user_id = $1 ORDER BY asset",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn reserve_balance(
        &self,
        user_id: Uuid,
        asset: &str,
        amount: Decimal,
    ) -> Result<(), sqlx::Error> {
        let result = sqlx::query(
            "UPDATE balances SET available = available - $1, reserved = reserved + $1, updated_at = NOW() WHERE user_id = $2 AND asset = $3 AND available >= $1",
        )
        .bind(amount)
        .bind(user_id)
        .bind(asset)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::Protocol("Insufficient balance".to_string()));
        }

        Ok(())
    }
}

pub struct BalanceData {
    pub asset: String,
    pub free: String,
    pub locked: String,
}
