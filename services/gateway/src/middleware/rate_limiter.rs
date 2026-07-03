use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::collections::HashMap;
use std::future::{ready, Ready};
use std::sync::Arc;
use tokio::sync::Mutex;

struct RateLimitState {
    max_weight: u32,
    window_secs: u64,
    counters: Mutex<HashMap<String, (u64, u32)>>,
}

pub struct RateLimiter {
    state: Arc<RateLimitState>,
}

impl RateLimiter {
    pub fn new(
        _redis: Option<crate::RedisConn>,
        max_weight: u32,
        window_secs: u64,
        _order_rate_per_sec: u32,
    ) -> Self {
        Self {
            state: Arc::new(RateLimitState {
                max_weight,
                window_secs,
                counters: Mutex::new(HashMap::new()),
            }),
        }
    }
}

impl<S> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = RateLimiterService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimiterService {
            service,
            state: self.state.clone(),
        }))
    }
}

pub struct RateLimiterService<S> {
    service: S,
    state: Arc<RateLimitState>,
}

impl<S> Service<ServiceRequest> for RateLimiterService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let state = self.state.clone();
        let client_ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();
        let path = req.path().to_string();
        let key = format!("{}:{}", client_ip, path);

        let exceeded = {
            let mut counters = match state.counters.try_lock() {
                Ok(c) => c,
                Err(_) => {
                    return Box::pin(self.service.call(req));
                }
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let entry = counters.entry(key).or_insert((now, 0));
            if now - entry.0 >= state.window_secs {
                *entry = (now, 1);
            } else {
                entry.1 += 1;
            }
            entry.1 > state.max_weight
        };

        if exceeded {
            let resp = actix_web::HttpResponse::TooManyRequests()
                .json(serde_json::json!({"code": -1015, "msg": "Too many requests"}));
            return Box::pin(async move { Ok(req.into_response(resp).map_into_boxed_body()) });
        }

        Box::pin(self.service.call(req))
    }
}
