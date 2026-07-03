use actix_web::{web, App, HttpServer};
use backpack_common::auth::middleware::JwtAuthMiddleware;
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

    let jwt_secret = config.jwt_secret.clone();
    let public_prefixes = vec![
        "/api/v3/balance/reserve".to_string(),
        "/api/v3/balance/release".to_string(),
    ];

    tracing::info!("Wallet Service starting on {}:{}", host, port);

    HttpServer::new(move || {
        App::new()
            .wrap(JwtAuthMiddleware::new(
                jwt_secret.clone(),
                public_prefixes.clone(),
            ))
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
                    )
                    .route(
                        "/balance/reserve",
                        web::post().to(wallet_service::handlers::wallet::reserve_balance),
                    )
                    .route(
                        "/balance/release",
                        web::post().to(wallet_service::handlers::wallet::release_balance),
                    ),
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;

    Ok(())
}
