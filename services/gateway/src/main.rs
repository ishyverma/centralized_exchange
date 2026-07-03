use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use api_gateway::config::Config;
use api_gateway::middleware::jwt::JwtAuthMiddleware;
use api_gateway::middleware::rate_limiter::RateLimiter;
use tracing_subscriber::EnvFilter;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env();

    let redis = match redis::Client::open(config.redis_url.as_str()) {
        Ok(client) => redis::aio::ConnectionManager::new(client).await.ok(),
        Err(_) => None,
    };

    if redis.is_some() {
        tracing::info!("Connected to Redis for rate limiting");
    } else {
        tracing::warn!("Redis unavailable, rate limiting disabled");
    }

    let jwt_secret = config.jwt_secret.clone();
    let max_weight = config.rate_limit_max_weight;
    let window_secs = config.rate_limit_weight_window;
    let order_rate = config.order_rate_limit_per_sec;
    let auth_service_url = config.auth_service_url.clone();
    let order_service_url = config.order_service_url.clone();
    let host = config.host.clone();
    let port = config.port;

    tracing::info!(
        "API Gateway starting on {}:{}, auth backend: {}, order backend: {}",
        host,
        port,
        auth_service_url,
        order_service_url
    );

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(RateLimiter::new(
                redis.clone(),
                max_weight,
                window_secs,
                order_rate,
            ))
            .wrap(JwtAuthMiddleware::new(jwt_secret.clone()))
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::new(auth_service_url.clone()))
            .app_data(web::Data::new(order_service_url.clone()))
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
            .default_service(web::route().to(api_gateway::not_found))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;

    Ok(())
}
