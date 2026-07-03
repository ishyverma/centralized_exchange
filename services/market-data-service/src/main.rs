use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use market_data_service::config::Config;
use market_data_service::db::DbPool;
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
    let db = DbPool::connect(
        &std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://backpack:backpack_dev@localhost:5432/backpack".into()
        }),
    )
    .await?;

    let host = config.host.clone();
    let port = config.port;

    tracing::info!("Market Data Service starting on {}:{}", host, port);

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
            .service(
                web::scope("/api/v3")
                    .route(
                        "/exchangeInfo",
                        web::get().to(market_data_service::handlers::market_data::exchange_info),
                    )
                    .route(
                        "/depth",
                        web::get().to(market_data_service::handlers::market_data::get_depth),
                    )
                    .route(
                        "/trades",
                        web::get().to(
                            market_data_service::handlers::market_data::get_recent_trades,
                        ),
                    )
                    .route(
                        "/ticker/24hr",
                        web::get().to(
                            market_data_service::handlers::market_data::get_ticker_24hr,
                        ),
                    )
                    .route(
                        "/ticker/price",
                        web::get().to(
                            market_data_service::handlers::market_data::get_ticker_price,
                        ),
                    ),
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;

    Ok(())
}
