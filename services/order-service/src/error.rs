use actix_web::{HttpResponse, ResponseError};
use backpack_common::error::ApiError;
use std::fmt;

#[derive(Debug)]
pub struct OrderError(pub ApiError);

impl fmt::Display for OrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for OrderError {
    fn error_response(&self) -> HttpResponse {
        match &self.0 {
            ApiError::OrderNotFound => HttpResponse::NotFound().json(serde_json::json!({
                "code": -2013,
                "msg": "Order does not exist"
            })),
            ApiError::InvalidSymbol => HttpResponse::BadRequest().json(serde_json::json!({
                "code": -1013,
                "msg": "Invalid symbol"
            })),
            ApiError::InvalidOrderType => HttpResponse::BadRequest().json(serde_json::json!({
                "code": -1013,
                "msg": "Invalid order type"
            })),
            ApiError::InsufficientBalance => HttpResponse::BadRequest().json(serde_json::json!({
                "code": -2010,
                "msg": "Insufficient balance"
            })),
            ApiError::ValidationError(msg) => HttpResponse::BadRequest().json(serde_json::json!({
                "code": -1013,
                "msg": msg
            })),
            ApiError::Unauthorized => HttpResponse::Unauthorized().json(serde_json::json!({
                "code": -1006,
                "msg": "Unauthorized"
            })),
            _ => HttpResponse::InternalServerError().json(serde_json::json!({
                "code": -2000,
                "msg": "Internal server error"
            })),
        }
    }
}

impl From<sqlx::Error> for OrderError {
    fn from(e: sqlx::Error) -> Self {
        OrderError(ApiError::Internal(format!("Database error: {}", e)))
    }
}
