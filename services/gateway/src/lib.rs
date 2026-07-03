pub mod config;
pub mod middleware;

use actix_web::{web, HttpRequest, HttpResponse};

pub type RedisConn = redis::aio::ConnectionManager;

async fn proxy_request(req: HttpRequest, body: web::Bytes, base_url: &str) -> HttpResponse {
    let path = req.path();
    let query = req.query_string();
    let method = req.method().clone();

    let url = format!(
        "{}{}{}",
        base_url,
        path,
        if query.is_empty() {
            String::new()
        } else {
            format!("?{}", query)
        }
    );

    let client = reqwest::Client::new();
    let mut proxy_req = client.request(method, &url);

    for (name, value) in req.headers().iter() {
        let name_lower = name.as_str().to_lowercase();
        if name_lower == "host"
            || name_lower == "connection"
            || name_lower == "transfer-encoding"
            || name_lower == "content-length"
        {
            continue;
        }
        proxy_req = proxy_req.header(name.as_str(), value);
    }

    if !body.is_empty() {
        proxy_req = proxy_req.body(body.to_vec());
    }

    match proxy_req.send().await {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();
            let resp_body = response.bytes().await.unwrap_or_default();

            let mut builder = HttpResponse::build(
                actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap(),
            );

            for (name, value) in headers.iter() {
                let name_str = name.as_str().to_lowercase();
                if name_str != "transfer-encoding" && name_str != "connection" {
                    builder.insert_header((name.clone(), value.clone()));
                }
            }

            builder.body(resp_body)
        }
        Err(e) => HttpResponse::BadGateway().json(serde_json::json!({
            "code": -2001,
            "msg": format!("Upstream service error: {}", e)
        })),
    }
}

pub async fn ping() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({}))
}

pub async fn server_time() -> HttpResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    HttpResponse::Ok().json(serde_json::json!({ "serverTime": now }))
}

pub async fn proxy_to_auth(
    req: HttpRequest,
    body: web::Bytes,
    auth_service_url: web::Data<String>,
) -> HttpResponse {
    proxy_request(req, body, auth_service_url.get_ref()).await
}

pub async fn proxy_to_order(
    req: HttpRequest,
    body: web::Bytes,
    order_service_url: web::Data<String>,
) -> HttpResponse {
    proxy_request(req, body, order_service_url.get_ref()).await
}

pub async fn not_found() -> HttpResponse {
    HttpResponse::NotFound().json(serde_json::json!({
        "code": -2002,
        "msg": "Not found"
    }))
}
