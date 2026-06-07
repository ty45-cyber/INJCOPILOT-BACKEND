use axum::{
    extract::{Path, State},
    Extension, Json,
};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    models::{Claims, ExecuteIntentRequest, IntentRequest},
    services::{ai::AiService, injective::InjectiveService},
    AppState,
};

/// POST /intent/generate
/// Fetches portfolio + market context, calls AI, persists plan
pub async fn generate_intent(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<IntentRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.prompt.trim().is_empty() {
        return Err(AppError::BadRequest("Prompt cannot be empty".into()));
    }

    let injective = InjectiveService::new(&state.cfg);
    let ai = AiService::new(&state.cfg);

    // Gather context in parallel
    let (portfolio_result, market_result) = tokio::join!(
        injective.build_portfolio_context(&claims.wallet),
        injective.fetch_market_summary()
    );

    let portfolio = portfolio_result.unwrap_or_else(|_| crate::models::PortfolioContext {
        total_value_usd: 0.0,
        assets: vec![],
        dominant_exposure: "unknown".into(),
    });

    let market_summary = market_result
        .unwrap_or_else(|_| serde_json::json!({ "status": "unavailable" }));

    // Generate AI plan
    let plan = ai
        .generate_action_plan(&req.prompt, &portfolio, &market_summary)
        .await?;

    // Persist intent
    let intent_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO intents (id, user_id, prompt, status) VALUES (?, ?, ?, 'analyzed')",
    )
    .bind(&intent_id)
    .bind(&claims.sub)
    .bind(&req.prompt)
    .execute(&state.pool)
    .await?;

    // Persist AI output
    let output_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO ai_outputs
           (id, intent_id, intent_summary, portfolio_context, recommended_actions, risk_analysis, injective_tx)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&output_id)
    .bind(&intent_id)
    .bind(&plan.intent_summary)
    .bind(serde_json::to_string(&plan.portfolio_context).unwrap_or_default())
    .bind(serde_json::to_string(&plan.recommended_actions).unwrap_or_default())
    .bind(serde_json::to_string(&plan.risk_analysis).unwrap_or_default())
    .bind(serde_json::to_string(&plan.injective_tx).unwrap_or_default())
    .execute(&state.pool)
    .await?;

    // Log activity
    log_activity(
        &state,
        &claims.sub,
        "intent_generated",
        &format!("AI plan generated for: {}", &req.prompt),
        serde_json::json!({ "intent_id": intent_id }),
    )
    .await;

    Ok(Json(serde_json::json!({
        "intent_id": intent_id,
        "plan": plan
    })))
}

/// GET /intent/review/:intent_id
pub async fn review_intent(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(intent_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let row = sqlx::query!(
        r#"SELECT i.id, i.prompt, i.status, i.user_id,
                  o.intent_summary, o.portfolio_context, o.recommended_actions,
                  o.risk_analysis, o.injective_tx
           FROM intents i
           LEFT JOIN ai_outputs o ON o.intent_id = i.id
           WHERE i.id = ? AND i.user_id = ?"#,
        intent_id,
        claims.sub
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Intent not found".into()))?;

    Ok(Json(serde_json::json!({
        "intent_id": row.id,
        "prompt": row.prompt,
        "status": row.status,
        "plan": {
            "intent_summary": row.intent_summary,
            "portfolio_context": row.portfolio_context,
            "recommended_actions": row.recommended_actions,
            "risk_analysis": row.risk_analysis,
            "injective_tx": row.injective_tx
        }
    })))
}

/// POST /intent/execute
pub async fn execute_intent(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<ExecuteIntentRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Verify ownership
    let intent = sqlx::query!(
        "SELECT id, status FROM intents WHERE id = ? AND user_id = ?",
        req.intent_id,
        claims.sub
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Intent not found".into()))?;

    if intent.status == "executed" {
        return Err(AppError::BadRequest("Intent already executed".into()));
    }
    if intent.status == "rejected" {
        return Err(AppError::BadRequest("Cannot execute a rejected intent".into()));
    }

    // Broadcast to Injective
    let injective = InjectiveService::new(&state.cfg);
    let tx_hash = injective.broadcast_tx(&req.signed_tx).await?;

    // Persist transaction record
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO transactions (id, intent_id, user_id, tx_hash, tx_type, payload, status)
           VALUES (?, ?, ?, ?, 'signed', ?, 'pending')"#,
    )
    .bind(&tx_id)
    .bind(&req.intent_id)
    .bind(&claims.sub)
    .bind(&tx_hash)
    .bind(serde_json::to_string(&req.signed_tx).unwrap_or_default())
    .execute(&state.pool)
    .await?;

    // Mark intent executed
    sqlx::query("UPDATE intents SET status = 'executed' WHERE id = ?")
        .bind(&req.intent_id)
        .execute(&state.pool)
        .await?;

    log_activity(
        &state,
        &claims.sub,
        "tx_broadcast",
        &format!("Transaction submitted: {}", tx_hash),
        serde_json::json!({ "intent_id": req.intent_id, "tx_hash": tx_hash }),
    )
    .await;

    Ok(Json(serde_json::json!({
        "status": "pending",
        "tx_hash": tx_hash,
        "transaction_id": tx_id
    })))
}

/// POST /intent/save/:intent_id
pub async fn save_intent(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(intent_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let updated = sqlx::query(
        "UPDATE intents SET status = 'saved' WHERE id = ? AND user_id = ?",
    )
    .bind(&intent_id)
    .bind(&claims.sub)
    .execute(&state.pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("Intent not found".into()));
    }

    log_activity(
        &state,
        &claims.sub,
        "intent_saved",
        "Intent saved as draft",
        serde_json::json!({ "intent_id": intent_id }),
    )
    .await;

    Ok(Json(serde_json::json!({ "status": "saved", "intent_id": intent_id })))
}

/// POST /intent/reject/:intent_id
pub async fn reject_intent(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(intent_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let updated = sqlx::query(
        "UPDATE intents SET status = 'rejected' WHERE id = ? AND user_id = ?",
    )
    .bind(&intent_id)
    .bind(&claims.sub)
    .execute(&state.pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("Intent not found".into()));
    }

    log_activity(
        &state,
        &claims.sub,
        "intent_rejected",
        "Intent rejected by user",
        serde_json::json!({ "intent_id": intent_id }),
    )
    .await;

    Ok(Json(serde_json::json!({ "status": "rejected", "intent_id": intent_id })))
}

// ── Internal helper ───────────────────────────────────────────────────────────

async fn log_activity(
    state: &AppState,
    user_id: &str,
    event_type: &str,
    description: &str,
    metadata: serde_json::Value,
) {
    let id = Uuid::new_v4().to_string();
    let _ = sqlx::query(
        "INSERT INTO activity_logs (id, user_id, event_type, description, metadata) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(event_type)
    .bind(description)
    .bind(serde_json::to_string(&metadata).unwrap_or_default())
    .execute(&state.pool)
    .await;
    // Fire-and-forget — logging must never crash the main flow
}