use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
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

    tracing::info!("Order Service starting on {}:{}", host, port);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(engine.clone()))
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
                    ),
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;

    Ok(())
}
