use actix_web::{HttpResponse, ResponseError};
use backpack_common::error::ApiError;
use std::fmt;

#[derive(Debug)]
pub struct AuthError(pub ApiError);

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for AuthError {
    fn error_response(&self) -> HttpResponse {
        match &self.0 {
            ApiError::InvalidCredentials => HttpResponse::Unauthorized().json(serde_json::json!({
                "code": -1001,
                "msg": "Invalid credentials"
            })),
            ApiError::EmailAlreadyRegistered => HttpResponse::Conflict().json(serde_json::json!({
                "code": -1002,
                "msg": "Email already registered"
            })),
            ApiError::UserNotFound => HttpResponse::NotFound().json(serde_json::json!({
                "code": -1003,
                "msg": "User not found"
            })),
            ApiError::InvalidApiKey => HttpResponse::Unauthorized().json(serde_json::json!({
                "code": -1004,
                "msg": "Invalid API key"
            })),
            ApiError::ValidationError(msg) => HttpResponse::BadRequest().json(serde_json::json!({
                "code": -1005,
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

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::Database(dbe) if dbe.constraint() == Some("users_email_key") => {
                AuthError(ApiError::EmailAlreadyRegistered)
            }
            _ => AuthError(ApiError::Internal(format!("Database error: {}", e))),
        }
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        AuthError(ApiError::Internal(format!("JWT error: {}", e)))
    }
}

impl From<argon2::password_hash::Error> for AuthError {
    fn from(e: argon2::password_hash::Error) -> Self {
        AuthError(ApiError::Internal(format!("Password hash error: {}", e)))
    }
}
