use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct DbPool {
    pub pool: PgPool,
}

impl DbPool {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn create_user(&self, email: &str, password_hash: &str) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<super::models::UserRow>, sqlx::Error> {
        sqlx::query_as::<_, super::models::UserRow>(
            "SELECT id, email, password_hash, totp_secret, status, created_at, updated_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_user_by_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<super::models::UserRow>, sqlx::Error> {
        sqlx::query_as::<_, super::models::UserRow>(
            "SELECT id, email, password_hash, totp_secret, status, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_api_key(
        &self,
        user_id: Uuid,
        api_key: &str,
        api_secret_hash: &str,
        permissions: &[String],
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO api_keys (user_id, api_key, api_secret_hash, permissions) VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(user_id)
        .bind(api_key)
        .bind(api_secret_hash)
        .bind(permissions)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    #[allow(dead_code)]
    pub async fn find_api_key(
        &self,
        api_key: &str,
    ) -> Result<Option<super::models::ApiKeyRow>, sqlx::Error> {
        sqlx::query_as::<_, super::models::ApiKeyRow>(
            "SELECT id, user_id, api_key, api_secret_hash, permissions, ip_whitelist, status, last_used_at, created_at, expires_at FROM api_keys WHERE api_key = $1 AND status = 'ACTIVE'",
        )
        .bind(api_key)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_api_keys(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<super::models::ApiKeyRow>, sqlx::Error> {
        sqlx::query_as::<_, super::models::ApiKeyRow>(
            "SELECT id, user_id, api_key, api_secret_hash, permissions, ip_whitelist, status, last_used_at, created_at, expires_at FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_api_key(&self, key_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM api_keys WHERE id = $1 AND user_id = $2")
            .bind(key_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    #[allow(dead_code)]
    pub async fn update_api_key_last_used(&self, key_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(key_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
