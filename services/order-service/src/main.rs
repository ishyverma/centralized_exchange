use actix_web::{web, App, HttpServer};
use backpack_common::auth::middleware::JwtAuthMiddleware;
use order_service::config::Config;
use order_service::db::DbPool;
use order_service::engine_client::EngineClient;
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
    let db = DbPool::connect(&config.database_url).await?;
    let engine = EngineClient::new();

    let host = config.host.clone();
    let port = config.port;

    let jwt_secret = config.jwt_secret.clone();
    let public_prefixes = vec![
        "/api/v3/ping".to_string(),
        "/api/v3/time".to_string(),
        "/api/v3/depth".to_string(),
        "/api/v3/ticker/bookTicker".to_string(),
        "/ws".to_string(),
    ];

    tracing::info!("Order Service starting on {}:{}", host, port);

    HttpServer::new(move || {
        App::new()
            .wrap(JwtAuthMiddleware::new(
                jwt_secret.clone(),
                public_prefixes.clone(),
            ))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(engine.clone()))
            .route("/api/v3/ping", web::get().to(order_service::ping))
            .route("/api/v3/time", web::get().to(order_service::server_time))
            .service(
                web::scope("/api/v3")
                    .route(
                        "/order",
                        web::post().to(order_service::handlers::orders::place_order),
                    )
                    .route(
                        "/order",
                        web::get().to(order_service::handlers::orders::get_order),
                    )
                    .route(
                        "/order",
                        web::delete().to(order_service::handlers::orders::cancel_order),
                    )
                    .route(
                        "/allOrders",
                        web::get().to(order_service::handlers::orders::all_orders),
                    )
                    .route(
                        "/myTrades",
                        web::get().to(order_service::handlers::orders::my_trades),
                    )
                    .route(
                        "/depth",
                        web::get().to(order_service::handlers::orders::get_depth),
                    )
                    .route(
                        "/ticker/bookTicker",
                        web::get().to(order_service::handlers::orders::get_book_ticker),
                    ),
            )
            .route(
                "/ws",
                web::get().to(order_service::handlers::ws::ws_handler),
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;

    Ok(())
}
