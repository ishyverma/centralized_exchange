use actix_web::{HttpResponse, ResponseError};
use backpack_common::error::ApiError;
use std::fmt;

#[derive(Debug)]
pub struct MarketDataError(pub ApiError);

impl fmt::Display for MarketDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for MarketDataError {
    fn error_response(&self) -> HttpResponse {
        match &self.0 {
            ApiError::InvalidSymbol => HttpResponse::BadRequest().json(serde_json::json!({
                "code": -1013,
                "msg": "Invalid symbol"
            })),
            ApiError::ValidationError(msg) => HttpResponse::BadRequest().json(serde_json::json!({
                "code": -1013,
                "msg": msg
            })),
            _ => HttpResponse::InternalServerError().json(serde_json::json!({
                "code": -2000,
                "msg": "Internal server error"
            })),
        }
    }
}

impl From<sqlx::Error> for MarketDataError {
    fn from(e: sqlx::Error) -> Self {
        MarketDataError(ApiError::Internal(format!("Database error: {}", e)))
    }
}
