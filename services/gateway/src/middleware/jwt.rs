use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, DecodingKey, Validation};
#[cfg(test)]
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_token(secret: &str, sub: &str, email: &str, exp_offset: i64) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let claims = Claims {
            sub: Uuid::parse_str(sub).unwrap(),
            email: email.to_string(),
            exp: (now + exp_offset) as usize,
            iat: now as usize,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn test_valid_token_decodes() {
        let secret = "test-secret";
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let token = create_token(secret, uuid, "test@example.com", 3600);

        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .unwrap();
        assert_eq!(decoded.claims.sub.to_string(), uuid);
        assert_eq!(decoded.claims.email, "test@example.com");
    }

    #[test]
    fn test_expired_token_rejected() {
        let secret = "test-secret";
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let token = create_token(secret, uuid, "test@example.com", -3600);

        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret_rejected() {
        let secret = "correct-secret";
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let token = create_token(secret, uuid, "test@example.com", 3600);

        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret("wrong-secret".as_bytes()),
            &Validation::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_token_rejected() {
        let result = decode::<Claims>(
            "not-a-valid-jwt",
            &DecodingKey::from_secret("secret".as_bytes()),
            &Validation::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "user@example.com".to_string(),
            exp: 2000000000,
            iat: 1000000000,
        };
        let json = serde_json::to_value(&claims).unwrap();
        assert_eq!(json["email"], "user@example.com");
        assert!(json["sub"].is_string());
        assert_eq!(json["exp"], 2000000000);
    }
}

pub struct JwtAuthMiddleware {
    secret: String,
}

impl JwtAuthMiddleware {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl<S> Transform<S, ServiceRequest> for JwtAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = JwtAuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddlewareService {
            service,
            secret: self.secret.clone(),
        }))
    }
}

pub struct JwtAuthMiddlewareService<S> {
    service: S,
    secret: String,
}

impl<S> Service<ServiceRequest> for JwtAuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let secret = self.secret.clone();
        let path = req.path().to_string();

        let public_prefixes = [
            "/api/v3/ping",
            "/api/v3/time",
            "/api/v3/exchangeInfo",
            "/api/v3/depth",
            "/api/v3/trades",
            "/api/v3/historicalTrades",
            "/api/v3/klines",
            "/api/v3/ticker",
            "/api/v3/auth/register",
            "/api/v3/auth/login",
        ];

        let is_public = public_prefixes.iter().any(|p| path.starts_with(p));

        if is_public {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            });
        }

        let bearer_token = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        if let Some(token) = bearer_token {
            match decode::<Claims>(
                &token,
                &DecodingKey::from_secret(secret.as_bytes()),
                &Validation::default(),
            ) {
                Ok(data) => {
                    req.extensions_mut().insert(data.claims.sub);
                    req.extensions_mut().insert(data.claims.email);
                    let fut = self.service.call(req);
                    return Box::pin(async move {
                        let res = fut.await?;
                        Ok(res)
                    });
                }
                Err(_) => {
                    let (http_req, _) = req.into_parts();
                    let response = actix_web::HttpResponse::Unauthorized()
                        .json(
                            serde_json::json!({ "code": -1006, "msg": "Invalid or expired token" }),
                        )
                        .map_into_boxed_body();
                    let res = ServiceResponse::new(http_req, response);
                    return Box::pin(async move { Ok(res) });
                }
            }
        }

        let (http_req, _) = req.into_parts();
        let response = actix_web::HttpResponse::Unauthorized()
            .json(serde_json::json!({ "code": -1006, "msg": "Missing authorization" }))
            .map_into_boxed_body();
        let res = ServiceResponse::new(http_req, response);
        Box::pin(async move { Ok(res) })
    }
}
