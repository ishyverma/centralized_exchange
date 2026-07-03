use actix_cors::Cors;
use actix_web::{middleware::Logger, test, web, App};
use api_gateway::middleware::jwt::JwtAuthMiddleware;
use api_gateway::middleware::rate_limiter::RateLimiter;
use serde_json::json;

const TEST_JWT_SECRET: &str = "test-jwt-secret-for-testing";

macro_rules! setup_app {
    () => {{
        let auth_service_url = "http://localhost:9999".to_string();
        let order_service_url = "http://localhost:9998".to_string();
        let wallet_service_url = "http://localhost:9997".to_string();
        let market_data_service_url = "http://localhost:9996".to_string();
        test::init_service(
            App::new()
                .wrap(RateLimiter::new(None, 6000, 60, 10))
                .wrap(JwtAuthMiddleware::new(TEST_JWT_SECRET.to_string()))
                .wrap(Logger::default())
                .wrap(Cors::permissive())
                .app_data(web::Data::new(auth_service_url))
                .app_data(web::Data::new(order_service_url))
                .app_data(web::Data::new(wallet_service_url))
                .app_data(web::Data::new(market_data_service_url))
                .route("/api/v3/ping", web::get().to(api_gateway::ping))
                .route("/api/v3/time", web::get().to(api_gateway::server_time))
                .route(
                    "/api/v3/auth/{tail:.*}",
                    web::route().to(api_gateway::proxy_to_auth),
                )
                .route(
                    "/api/v3/order",
                    web::route().to(api_gateway::proxy_to_order),
                )
                .route(
                    "/api/v3/allOrders",
                    web::route().to(api_gateway::proxy_to_order),
                )
                .route(
                    "/api/v3/myTrades",
                    web::route().to(api_gateway::proxy_to_order),
                )
                .route(
                    "/api/v3/depth",
                    web::route().to(api_gateway::proxy_to_order),
                )
                .route(
                    "/api/v3/account",
                    web::route().to(api_gateway::proxy_to_wallet),
                )
                .route(
                    "/api/v3/balance",
                    web::route().to(api_gateway::proxy_to_wallet),
                )
                .route(
                    "/api/v3/exchangeInfo",
                    web::route().to(api_gateway::proxy_to_market_data),
                )
                .route(
                    "/api/v3/trades",
                    web::route().to(api_gateway::proxy_to_market_data),
                )
                .route(
                    "/api/v3/ticker/{tail:.*}",
                    web::route().to(api_gateway::proxy_to_market_data),
                )
                .default_service(web::route().to(api_gateway::not_found)),
        )
        .await
    }};
}

fn generate_jwt(secret: &str, sub: &str, email: &str, exp_offset: i64) -> String {
    use api_gateway::middleware::jwt::Claims;
    use jsonwebtoken::{encode, EncodingKey, Header};
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let claims = Claims {
        sub: uuid::Uuid::parse_str(sub).unwrap(),
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

#[actix_web::test]
async fn test_ping() {
    let app = setup_app!();
    let req = test::TestRequest::get().uri("/api/v3/ping").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_server_time() {
    let app = setup_app!();
    let req = test::TestRequest::get().uri("/api/v3/time").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["serverTime"].as_u64().is_some());
}

#[actix_web::test]
async fn test_public_routes_bypass_auth() {
    let app = setup_app!();
    let routes = [
        "/api/v3/ping",
        "/api/v3/time",
        "/api/v3/exchangeInfo",
        "/api/v3/depth/BTCUSDT",
    ];

    for route in routes {
        let req = test::TestRequest::get().uri(route).to_request();
        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status() != 401,
            "Route {} should not return 401",
            route
        );
    }
}

#[actix_web::test]
async fn test_protected_route_rejects_without_auth() {
    let app = setup_app!();
    let routes = [
        "/api/v3/order",
        "/api/v3/account",
        "/api/v3/myTrades",
        "/api/v3/balance",
    ];

    for route in routes {
        let req = test::TestRequest::get().uri(route).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401, "Route {} should return 401", route);
    }
}

#[actix_web::test]
async fn test_protected_route_accepts_valid_jwt() {
    let app = setup_app!();
    let token = generate_jwt(
        TEST_JWT_SECRET,
        "550e8400-e29b-41d4-a716-446655440000",
        "test@example.com",
        3600,
    );

    let req = test::TestRequest::get()
        .uri("/api/v3/order")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(resp.status(), 401);
}

#[actix_web::test]
async fn test_protected_route_rejects_expired_jwt() {
    let app = setup_app!();
    let token = generate_jwt(
        TEST_JWT_SECRET,
        "550e8400-e29b-41d4-a716-446655440000",
        "test@example.com",
        -3600,
    );

    let req = test::TestRequest::get()
        .uri("/api/v3/order")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_protected_route_rejects_malformed_jwt() {
    let app = setup_app!();
    let req = test::TestRequest::get()
        .uri("/api/v3/order")
        .insert_header(("Authorization", "Bearer definitely-not-a-valid-jwt"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_not_found() {
    let app = setup_app!();
    let req = test::TestRequest::get()
        .uri("/api/v3/nonexistent")
        .to_request();
    let resp = test::call_service(&app, req).await;
    // JWT middleware rejects before reaching not_found handler for /api/v3 paths
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_proxy_returns_bad_gateway() {
    let app = setup_app!();
    let req = test::TestRequest::get()
        .uri("/api/v3/auth/register")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 502);
}

#[actix_web::test]
async fn test_proxy_forwards_auth_header() {
    let app = setup_app!();
    let token = generate_jwt(
        TEST_JWT_SECRET,
        "550e8400-e29b-41d4-a716-446655440000",
        "test@example.com",
        3600,
    );

    let req = test::TestRequest::post()
        .uri("/api/v3/auth/api-key")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "permissions": ["READ"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Should reach auth service proxy → returns 502 (backend down) not 401 (auth rejected)
    assert_ne!(resp.status(), 401);
    assert_eq!(resp.status(), 502);
}

#[actix_web::test]
async fn test_order_proxy_returns_bad_gateway() {
    let app = setup_app!();
    let token = generate_jwt(
        TEST_JWT_SECRET,
        "550e8400-e29b-41d4-a716-446655440000",
        "test@example.com",
        3600,
    );

    let req = test::TestRequest::get()
        .uri("/api/v3/order?orderId=550e8400-e29b-41d4-a716-446655440000")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Should reach order service proxy → returns 502 (backend down) not 401 (auth rejected)
    assert_ne!(resp.status(), 401);
    assert_eq!(resp.status(), 502);
}

#[actix_web::test]
async fn test_missing_auth_header() {
    let app = setup_app!();
    let req = test::TestRequest::get().uri("/api/v3/order").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_order_proxy_accepts_valid_jwt() {
    let app = setup_app!();
    let token = generate_jwt(
        TEST_JWT_SECRET,
        "550e8400-e29b-41d4-a716-446655440000",
        "test@example.com",
        3600,
    );

    let req = test::TestRequest::post()
        .uri("/api/v3/order")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "symbol": "BTCUSDT",
            "side": "BUY",
            "type": "LIMIT",
            "quantity": "0.01",
            "price": "50000",
            "timeInForce": "GTC"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(resp.status(), 401);
    assert_eq!(resp.status(), 502);
}
