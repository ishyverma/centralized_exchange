#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub wallet_service_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://backpack:backpack_dev@localhost:5432/backpack".into()
            }),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8082),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "backpack-dev-jwt-secret-change-in-production".into()),
            wallet_service_url: std::env::var("WALLET_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8083".into()),
        }
    }
}
