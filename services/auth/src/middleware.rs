use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, DecodingKey, Validation};
use std::future::{ready, Ready};

use crate::handlers::auth::Claims;

pub struct JwtMiddleware {
    secret: String,
}

impl JwtMiddleware {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl<S> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = JwtMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtMiddlewareService {
            service,
            secret: self.secret.clone(),
        }))
    }
}

pub struct JwtMiddlewareService<S> {
    service: S,
    secret: String,
}

impl<S> Service<ServiceRequest> for JwtMiddlewareService<S>
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
        let method = req.method().to_string();

        let public_routes = [
            ("/api/v3/auth/register", "POST"),
            ("/api/v3/auth/login", "POST"),
            ("/api/v3/ping", "GET"),
            ("/api/v3/time", "GET"),
        ];

        let is_public = public_routes
            .iter()
            .any(|(p, m)| path.starts_with(p) && method == *m);

        if is_public {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            });
        }

        let token = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        match token {
            Some(token) => {
                match decode::<Claims>(
                    &token,
                    &DecodingKey::from_secret(secret.as_bytes()),
                    &Validation::default(),
                ) {
                    Ok(data) => {
                        req.extensions_mut().insert(data.claims.sub);
                        let fut = self.service.call(req);
                        Box::pin(async move {
                            let res = fut.await?;
                            Ok(res)
                        })
                    }
                    Err(_) => {
                        let (http_req, _) = req.into_parts();
                        let response = actix_web::HttpResponse::Unauthorized()
                        .json(serde_json::json!({ "code": -1006, "msg": "Invalid or expired token" }))
                        .map_into_boxed_body();
                        let res = ServiceResponse::new(http_req, response);
                        Box::pin(async move { Ok(res) })
                    }
                }
            }
            None => {
                let (http_req, _) = req.into_parts();
                let response = actix_web::HttpResponse::Unauthorized()
                    .json(
                        serde_json::json!({ "code": -1006, "msg": "Missing authorization header" }),
                    )
                    .map_into_boxed_body();
                let res = ServiceResponse::new(http_req, response);
                Box::pin(async move { Ok(res) })
            }
        }
    }
}
