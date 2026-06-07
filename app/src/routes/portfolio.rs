use axum::{routing::get, Router, middleware};
use crate::{handlers::portfolio as handler, middleware::auth::require_auth, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/portfolio/summary", get(handler::get_portfolio_summary))
        .route_layer(middleware::from_fn_with_state(
            AppState::default_placeholder(),
            require_auth,
        ))
}