pub mod config;
pub mod db;
pub mod engine_client;
pub mod error;
pub mod handlers;
pub mod models;

use actix_web::HttpResponse;

pub async fn ping() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({}))
}

pub async fn server_time() -> HttpResponse {
    use std::time::SystemTime;
    let millis = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    HttpResponse::Ok().json(serde_json::json!({"serverTime": millis}))
}
