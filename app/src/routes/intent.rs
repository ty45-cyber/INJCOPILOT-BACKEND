use axum::{routing::{post, get}, Router, middleware};
use crate::{handlers::intent as handler, middleware::auth::require_auth, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/intent/generate", post(handler::generate_intent))
        .route("/intent/review/:intent_id", get(handler::review_intent))
        .route("/intent/execute", post(handler::execute_intent))
        .route("/intent/save/:intent_id", post(handler::save_intent))
        .route("/intent/reject/:intent_id", post(handler::reject_intent))
        .route_layer(middleware::from_fn_with_state(
            AppState::default_placeholder(),
            require_auth,
        ))
}