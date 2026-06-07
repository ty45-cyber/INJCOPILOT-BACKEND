use axum::{extract::{Query, State}, Extension, Json};
use serde::Deserialize;

use crate::{errors::AppResult, models::Claims, AppState};

#[derive(Deserialize)]
pub struct ActivityQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub async fn get_activity_log(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<ActivityQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let logs = sqlx::query!(
        r#"SELECT id, event_type, description, metadata, created_at
           FROM activity_logs
           WHERE user_id = ?
           ORDER BY created_at DESC
           LIMIT ? OFFSET ?"#,
        claims.sub,
        limit,
        offset
    )
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<serde_json::Value> = logs
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.id,
                "event_type": row.event_type,
                "description": row.description,
                "metadata": row.metadata,
                "created_at": row.created_at
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "items": items,
        "limit": limit,
        "offset": offset
    })))
}