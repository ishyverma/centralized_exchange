use actix_web::{HttpResponse, ResponseError};
use backpack_common::error::ApiError;
use std::fmt;

#[derive(Debug)]
pub struct WalletError(pub ApiError);

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for WalletError {
    fn error_response(&self) -> HttpResponse {
        match &self.0 {
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

impl From<sqlx::Error> for WalletError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::Protocol(msg) => {
                if msg == "Insufficient balance" || msg == "Insufficient reserved balance" {
                    WalletError(ApiError::InsufficientBalance)
                } else {
                    WalletError(ApiError::Internal(format!("Database error: {}", e)))
                }
            }
            _ => WalletError(ApiError::Internal(format!("Database error: {}", e))),
        }
    }
}
