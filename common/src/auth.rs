#[cfg(feature = "auth-middleware")]
pub mod middleware {
    use actix_web::{
        body::BoxBody,
        dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
        Error, HttpMessage,
    };
    use futures_util::future::LocalBoxFuture;
    use jsonwebtoken::{decode, DecodingKey, Validation};
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

    pub struct JwtAuthMiddleware {
        secret: String,
        public_prefixes: Vec<String>,
    }

    impl JwtAuthMiddleware {
        pub fn new(secret: String, public_prefixes: Vec<String>) -> Self {
            Self {
                secret,
                public_prefixes,
            }
        }
    }

    impl<S> Transform<S, ServiceRequest> for JwtAuthMiddleware
    where
        S: Service<
            ServiceRequest,
            Response = ServiceResponse<actix_web::body::BoxBody>,
            Error = Error,
        >,
        S::Future: 'static,
    {
        type Response = ServiceResponse<actix_web::body::BoxBody>;
        type Error = Error;
        type Transform = JwtAuthMiddlewareService<S>;
        type InitError = ();
        type Future = Ready<Result<Self::Transform, Self::InitError>>;

        fn new_transform(&self, service: S) -> Self::Future {
            ready(Ok(JwtAuthMiddlewareService {
                service,
                secret: self.secret.clone(),
                public_prefixes: self.public_prefixes.clone(),
            }))
        }
    }

    pub struct JwtAuthMiddlewareService<S> {
        service: S,
        secret: String,
        public_prefixes: Vec<String>,
    }

    impl<S> Service<ServiceRequest> for JwtAuthMiddlewareService<S>
    where
        S: Service<
            ServiceRequest,
            Response = ServiceResponse<actix_web::body::BoxBody>,
            Error = Error,
        >,
        S::Future: 'static,
    {
        type Response = ServiceResponse<BoxBody>;
        type Error = Error;
        type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

        forward_ready!(service);

        fn call(&self, req: ServiceRequest) -> Self::Future {
            let secret = self.secret.clone();
            let path = req.path().to_string();
            let public = self.public_prefixes.clone();

            let is_public = public.iter().any(|p| path.starts_with(p));

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
                            .json(serde_json::json!({
                                "code": -1006,
                                "msg": "Invalid or expired token"
                            }))
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
}
