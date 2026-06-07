use axum::{routing::get, Router, middleware};
use crate::{handlers::activity as handler, middleware::auth::require_auth, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/activity", get(handler::get_activity_log))
        .route_layer(middleware::from_fn_with_state(
            AppState::default_placeholder(),
            require_auth,
        ))
}