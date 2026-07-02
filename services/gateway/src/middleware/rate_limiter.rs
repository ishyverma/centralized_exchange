use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::sync::Arc;

struct RateLimitState {
    _max_weight: u32,
    _window_secs: u64,
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
                _max_weight: max_weight,
                _window_secs: window_secs,
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
            _state: self.state.clone(),
        }))
    }
}

pub struct RateLimiterService<S> {
    service: S,
    _state: Arc<RateLimitState>,
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
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}
