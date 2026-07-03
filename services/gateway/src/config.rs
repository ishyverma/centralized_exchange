#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub redis_url: String,
    pub auth_service_url: String,
    pub order_service_url: String,
    pub rate_limit_weight_window: u64,
    pub rate_limit_max_weight: u32,
    pub order_rate_limit_per_sec: u32,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "backpack-dev-jwt-secret-change-in-production".into()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".into()),
            auth_service_url: std::env::var("AUTH_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8081".into()),
            order_service_url: std::env::var("ORDER_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8082".into()),
            rate_limit_weight_window: 60,
            rate_limit_max_weight: 6000,
            order_rate_limit_per_sec: 10,
        }
    }
}
