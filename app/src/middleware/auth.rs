use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::{errors::AppError, models::Claims, AppState};

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer_token(&request)
        .ok_or_else(|| AppError::Auth("Missing Authorization header".into()))?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.cfg.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Auth("Invalid or expired token".into()))?;

    request.extensions_mut().insert(token_data.claims);
    Ok(next.run(request).await)
}

fn extract_bearer_token(request: &Request) -> Option<&str> {
    let header = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
    header.strip_prefix("Bearer ")
}