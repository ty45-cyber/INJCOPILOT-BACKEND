mod config;
mod db;
mod errors;
mod models;
mod routes;
mod handlers;
mod services;

use axum::middleware;
use crate::middleware::auth::require_auth;

use axum::{Router, middleware};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let cfg = config::Config::from_env()?;
    let pool = db::create_pool(&cfg.database_url).await?;
    db::run_migrations(&pool).await?;

    let state = AppState {
        pool,
        cfg: cfg.clone(),
    };
    impl AppState {
    /// Used only as a type placeholder for route-layer middleware binding.
    /// The real state is injected at server startup — this is never called at runtime.
    pub fn default_placeholder() -> Self {
        panic!("default_placeholder must never be called at runtime")
    }
}

    let app = Router::new()
    .merge(routes::auth::router())
    .merge(
        Router::new()
            .merge(routes::portfolio::router())
            .merge(routes::intent::router())
            .merge(routes::activity::router())
            .layer(middleware::from_fn_with_state(state.clone(), require_auth))
    )
    .layer(TraceLayer::new_for_http())
    .layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    )
    .with_state(state);

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("Injective Copilot API running on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::MySqlPool,
    pub cfg: config::Config,
}