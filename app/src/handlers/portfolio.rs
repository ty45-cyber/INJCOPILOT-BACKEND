use axum::{extract::State, Extension, Json};

use crate::{
    errors::AppResult,
    models::Claims,
    services::injective::InjectiveService,
    AppState,
};

pub async fn get_portfolio_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<serde_json::Value>> {
    let injective = InjectiveService::new(&state.cfg);

    let portfolio = injective
        .build_portfolio_context(&claims.wallet)
        .await?;

    let market_summary = injective
        .fetch_market_summary()
        .await
        .unwrap_or_else(|_| serde_json::json!({ "error": "market data unavailable" }));

    Ok(Json(serde_json::json!({
        "wallet": claims.wallet,
        "portfolio": portfolio,
        "market_summary": market_summary
    })))
}