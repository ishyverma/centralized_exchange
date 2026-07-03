use actix_cors::Cors;
use actix_web::{middleware::Logger, test, web, App};
use auth_service::config::Config;
use auth_service::db::DbPool;
use auth_service::middleware::JwtMiddleware;
use serde_json::json;

macro_rules! setup_app {
    () => {{
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://backpack:backpack_dev@localhost:5432/backpack_test".into()
        });
        let db = DbPool::connect(&database_url).await.unwrap();
        let config = Config::from_env();
        let jwt_secret = config.jwt_secret.clone();
        test::init_service(
            App::new()
                .wrap(Logger::default())
                .wrap(Cors::permissive())
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(db.clone()))
                .service(
                    web::scope("/api/v3")
                        .wrap(JwtMiddleware::new(jwt_secret))
                        .route(
                            "/auth/register",
                            web::post().to(auth_service::handlers::auth::register),
                        )
                        .route(
                            "/auth/login",
                            web::post().to(auth_service::handlers::auth::login),
                        )
                        .route("/auth/me", web::get().to(auth_service::handlers::auth::me))
                        .route(
                            "/auth/api-keys",
                            web::post().to(auth_service::handlers::api_keys::create_api_key),
                        )
                        .route(
                            "/auth/api-keys",
                            web::get().to(auth_service::handlers::api_keys::list_api_keys),
                        )
                        .route(
                            "/auth/api-keys",
                            web::delete().to(auth_service::handlers::api_keys::delete_api_key),
                        )
                        .route("/ping", web::get().to(auth_service::ping))
                        .route("/time", web::get().to(auth_service::server_time)),
                ),
        )
        .await
    }};
}

macro_rules! register_and_get_token {
    ($app:expr, $email:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v3/auth/register")
            .set_json(json!({ "email": $email, "password": "password123" }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        let body: serde_json::Value = test::read_body_json(resp).await;
        body["token"].as_str().unwrap().to_string()
    }};
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
async fn test_register_success() {
    let app = setup_app!();
    let email = format!("test_{}@example.com", uuid::Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/register")
        .set_json(json!({ "email": email, "password": "password123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["token"].as_str().is_some());
    assert_eq!(body["token_type"], "Bearer");
}

#[actix_web::test]
async fn test_register_duplicate_email() {
    let app = setup_app!();
    let email = format!("dup_{}@example.com", uuid::Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/register")
        .set_json(json!({ "email": &email, "password": "password123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/register")
        .set_json(json!({ "email": &email, "password": "password123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 409);
}

#[actix_web::test]
async fn test_register_invalid_email() {
    let app = setup_app!();
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/register")
        .set_json(json!({ "email": "invalid", "password": "password123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn test_register_short_password() {
    let app = setup_app!();
    let email = format!("short_{}@example.com", uuid::Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/register")
        .set_json(json!({ "email": email, "password": "short" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn test_login_success() {
    let app = setup_app!();
    let email = format!("login_test_{}@example.com", uuid::Uuid::new_v4());
    let _ = register_and_get_token!(app, &email);
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/login")
        .set_json(json!({ "email": &email, "password": "password123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["token"].as_str().is_some());
}

#[actix_web::test]
async fn test_login_wrong_password() {
    let app = setup_app!();
    let email = format!("wrong_pw_{}@example.com", uuid::Uuid::new_v4());
    let _ = register_and_get_token!(app, &email);
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/login")
        .set_json(json!({ "email": &email, "password": "wrongpassword" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_login_nonexistent_user() {
    let app = setup_app!();
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/login")
        .set_json(json!({ "email": "nonexistent@test.com", "password": "password123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_me_authenticated() {
    let app = setup_app!();
    let email = format!("me_test_{}@example.com", uuid::Uuid::new_v4());
    let token = register_and_get_token!(app, &email);
    let req = test::TestRequest::get()
        .uri("/api/v3/auth/me")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["email"], email);
    assert!(body["id"].as_str().is_some());
}

#[actix_web::test]
async fn test_me_unauthorized() {
    let app = setup_app!();
    let req = test::TestRequest::get().uri("/api/v3/auth/me").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_me_invalid_token() {
    let app = setup_app!();
    let req = test::TestRequest::get()
        .uri("/api/v3/auth/me")
        .insert_header(("Authorization", "Bearer invalid_token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_api_key_crud() {
    let app = setup_app!();
    let email = format!("apikey_{}@example.com", uuid::Uuid::new_v4());
    let token = register_and_get_token!(app, &email);
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/api-keys")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "permissions": ["READ", "TRADE"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["secret"].as_str().unwrap().len() > 10);
    let api_key = body["api_key"].as_str().unwrap().to_string();
    let req = test::TestRequest::get()
        .uri("/api/v3/auth/api-keys")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let keys: Vec<serde_json::Value> = test::read_body_json(resp).await;
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0]["api_key"], api_key);
    let req = test::TestRequest::delete()
        .uri("/api/v3/auth/api-keys")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "api_key_id": keys[0]["id"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let req = test::TestRequest::get()
        .uri("/api/v3/auth/api-keys")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let keys: Vec<serde_json::Value> = test::read_body_json(resp).await;
    assert_eq!(keys.len(), 0);
}

#[actix_web::test]
async fn test_api_key_invalid_permission() {
    let app = setup_app!();
    let email = format!("invalid_perm_{}@example.com", uuid::Uuid::new_v4());
    let token = register_and_get_token!(app, &email);
    let req = test::TestRequest::post()
        .uri("/api/v3/auth/api-keys")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "permissions": ["INVALID"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}
