use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use auth_service::config::Config;
use auth_service::db::DbPool;
use auth_service::middleware::JwtMiddleware;
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

    let host = config.host.clone();
    let port = config.port;
    let jwt_secret = config.jwt_secret.clone();

    tracing::info!("Auth Service starting on {}:{}", host, port);

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
                    .wrap(JwtMiddleware::new(jwt_secret.clone()))
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
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;

    Ok(())
}
