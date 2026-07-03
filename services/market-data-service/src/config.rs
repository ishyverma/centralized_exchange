#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("MARKET_DATA_SERVICE_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8084),
        }
    }
}
