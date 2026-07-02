use actix_web::{web, HttpResponse};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;
use crate::db::DbPool;
use crate::error::AuthError;
use crate::models::*;
use backpack_common::error::ApiError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

pub async fn register(
    db: web::Data<DbPool>,
    config: web::Data<Config>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AuthError> {
    let email = body.email.trim().to_lowercase();

    if !validator::validate_email(&email) {
        return Err(AuthError(ApiError::ValidationError(
            "Invalid email format".into(),
        )));
    }
    if body.password.len() < 8 {
        return Err(AuthError(ApiError::ValidationError(
            "Password must be at least 8 characters".into(),
        )));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(body.password.as_bytes(), &salt)?
        .to_string();

    let user_id = db.create_user(&email, &hash).await?;

    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id,
        email: email.clone(),
        iat: now,
        exp: now + config.jwt_expiration_secs as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;

    Ok(HttpResponse::Created().json(AuthResponse {
        token,
        token_type: "Bearer".into(),
        expires_in: config.jwt_expiration_secs,
    }))
}

pub async fn login(
    db: web::Data<DbPool>,
    config: web::Data<Config>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AuthError> {
    let email = body.email.trim().to_lowercase();
    let user = db
        .find_user_by_email(&email)
        .await?
        .ok_or(AuthError(ApiError::InvalidCredentials))?;

    if user.status != "ACTIVE" {
        return Err(AuthError(ApiError::InvalidCredentials));
    }

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| AuthError(ApiError::Internal(format!("Hash parse error: {}", e))))?;

    Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError(ApiError::InvalidCredentials))?;

    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user.id,
        email: user.email,
        iat: now,
        exp: now + config.jwt_expiration_secs as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;

    Ok(HttpResponse::Ok().json(AuthResponse {
        token,
        token_type: "Bearer".into(),
        expires_in: config.jwt_expiration_secs,
    }))
}

pub async fn me(
    db: web::Data<DbPool>,
    user_id: web::ReqData<Uuid>,
) -> Result<HttpResponse, AuthError> {
    let uid = user_id.into_inner();
    let user = db
        .find_user_by_id(uid)
        .await?
        .ok_or(AuthError(ApiError::UserNotFound))?;

    Ok(HttpResponse::Ok().json(UserResponse {
        id: user.id,
        email: user.email,
        status: user.status,
        created_at: user.created_at,
    }))
}
