#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_secs: u64,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://backpack:backpack_dev@localhost:5432/backpack".into()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "backpack-dev-jwt-secret-change-in-production".into()),
            jwt_expiration_secs: std::env::var("JWT_EXPIRATION_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(900),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8081),
        }
    }
}
