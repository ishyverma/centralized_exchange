use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Email already registered")]
    EmailAlreadyRegistered,

    #[error("User not found")]
    UserNotFound,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("API key not found")]
    ApiKeyNotFound,

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Order not found")]
    OrderNotFound,

    #[error("Invalid symbol")]
    InvalidSymbol,

    #[error("Invalid order type")]
    InvalidOrderType,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
