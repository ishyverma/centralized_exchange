use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use tracing_subscriber::EnvFilter;
use wallet_service::config::Config;
use wallet_service::db::DbPool;

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

    let host = config.host.clone();
    let port = config.port;

    tracing::info!("Wallet Service starting on {}:{}", host, port);

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
                        "/account",
                        web::get().to(wallet_service::handlers::wallet::get_account),
                    )
                    .route(
                        "/balance",
                        web::get().to(wallet_service::handlers::wallet::get_balance),
                    ),
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;

    Ok(())
}
