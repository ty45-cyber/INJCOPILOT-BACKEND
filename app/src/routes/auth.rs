use axum::{routing::post, Router};
use crate::{handlers::auth as handler, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/wallet/connect", post(handler::connect_wallet))
        .route("/auth/wallet/verify", post(handler::verify_signature))
}