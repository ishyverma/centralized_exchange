use actix_web::{web, HttpResponse};
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::AuthError;
use crate::models::*;
use backpack_common::error::ApiError;

type HmacSha256 = Hmac<Sha256>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key_format() {
        let key = generate_api_key();
        assert!(key.starts_with("bp_"));
        assert_eq!(key.len(), 35); // "bp_" + 32 alphanumeric chars
    }

    #[test]
    fn test_generate_api_key_uniqueness() {
        let k1 = generate_api_key();
        let k2 = generate_api_key();
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_generate_api_secret_length() {
        let (secret, hash) = generate_api_secret();
        assert_eq!(secret.len(), 48);
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_generate_api_secret_different() {
        let (s1, h1) = generate_api_secret();
        let (s2, h2) = generate_api_secret();
        assert_ne!(s1, s2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_generate_api_secret_hmac_verification() {
        let (secret, hash) = generate_api_secret();
        // Verify the hash can be recomputed
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(b"backpack-api-key-secret");
        let computed = hex::encode(mac.finalize().into_bytes());
        assert_eq!(hash, computed);
    }
}

fn generate_api_key() -> String {
    let key: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    format!("bp_{}", key)
}

fn generate_api_secret() -> (String, String) {
    let secret: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC should accept any key length");
    mac.update(b"backpack-api-key-secret");

    let hash = hex::encode(mac.finalize().into_bytes());
    (secret, hash)
}

pub async fn create_api_key(
    db: web::Data<DbPool>,
    user_id: web::ReqData<Uuid>,
    body: web::Json<CreateApiKeyRequest>,
) -> Result<HttpResponse, AuthError> {
    let uid = user_id.into_inner();

    let (raw_secret, secret_hash) = generate_api_secret();
    let api_key_value = generate_api_key();

    let permissions = if body.permissions.is_empty() {
        vec!["READ".to_string()]
    } else {
        body.permissions.clone()
    };

    for perm in &permissions {
        if !["READ", "TRADE", "WITHDRAW"].contains(&perm.as_str()) {
            return Err(AuthError(ApiError::ValidationError(format!(
                "Invalid permission: {}. Must be one of: READ, TRADE, WITHDRAW",
                perm
            ))));
        }
    }

    let key_id = db
        .create_api_key(uid, &api_key_value, &secret_hash, &permissions)
        .await?;

    Ok(HttpResponse::Created().json(ApiKeyResponse {
        id: key_id,
        api_key: api_key_value,
        secret: Some(raw_secret),
        permissions,
        status: "ACTIVE".into(),
        created_at: chrono::Utc::now(),
        expires_at: None,
    }))
}

pub async fn list_api_keys(
    db: web::Data<DbPool>,
    user_id: web::ReqData<Uuid>,
) -> Result<HttpResponse, AuthError> {
    let uid = user_id.into_inner();
    let keys = db.list_api_keys(uid).await?;

    let response: Vec<ApiKeyResponse> = keys
        .into_iter()
        .map(|k| ApiKeyResponse {
            id: k.id,
            api_key: k.api_key,
            secret: None,
            permissions: k.permissions,
            status: k.status,
            created_at: k.created_at,
            expires_at: k.expires_at,
        })
        .collect();

    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete_api_key(
    db: web::Data<DbPool>,
    user_id: web::ReqData<Uuid>,
    body: web::Json<DeleteApiKeyRequest>,
) -> Result<HttpResponse, AuthError> {
    let uid = user_id.into_inner();
    let deleted = db.delete_api_key(body.api_key_id, uid).await?;

    if !deleted {
        return Err(AuthError(ApiError::ApiKeyNotFound));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "api_key_id": body.api_key_id,
        "status": "DELETED"
    })))
}
