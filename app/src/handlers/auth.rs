use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    models::{AuthResponse, Claims, User, WalletConnectRequest, WalletVerifyRequest},
    AppState,
};

/// Step 1 — Return a nonce for the wallet to sign
pub async fn connect_wallet(
    State(state): State<AppState>,
    Json(req): Json<WalletConnectRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let nonce = Uuid::new_v4().to_string();
    let wallet = req.wallet_address.to_lowercase();

    // Upsert user record with fresh nonce
    let existing: Option<User> = sqlx::query_as(
        "SELECT id, wallet_address, nonce, created_at FROM users WHERE wallet_address = ?",
    )
    .bind(&wallet)
    .fetch_optional(&state.pool)
    .await?;

    match existing {
        Some(user) => {
            sqlx::query("UPDATE users SET nonce = ? WHERE id = ?")
                .bind(&nonce)
                .bind(&user.id)
                .execute(&state.pool)
                .await?;
        }
        None => {
            let id = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO users (id, wallet_address, nonce) VALUES (?, ?, ?)")
                .bind(&id)
                .bind(&wallet)
                .bind(&nonce)
                .execute(&state.pool)
                .await?;
        }
    }

    Ok(Json(serde_json::json!({
        "nonce": nonce,
        "message": format!("Sign this nonce to authenticate: {}", nonce)
    })))
}

/// Step 2 — Verify signature, issue JWT
/// NOTE: Full EIP-191 / Injective sig verification requires the injective-std
/// crate or cosmrs. For MVP we validate nonce ownership (nonce must match DB).
/// Replace inner block with real sig verify pre-mainnet.
pub async fn verify_signature(
    State(state): State<AppState>,
    Json(req): Json<WalletVerifyRequest>,
) -> AppResult<Json<AuthResponse>> {
    let wallet = req.wallet_address.to_lowercase();

    let user: User = sqlx::query_as(
        "SELECT id, wallet_address, nonce, created_at FROM users WHERE wallet_address = ?",
    )
    .bind(&wallet)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::Auth("Wallet not registered. Call /auth/wallet/connect first".into()))?;

    // Nonce must match what we issued
    if user.nonce != req.nonce {
        return Err(AppError::Auth("Nonce mismatch".into()));
    }

    // Rotate nonce immediately after use — prevents replay
    let fresh_nonce = Uuid::new_v4().to_string();
    sqlx::query("UPDATE users SET nonce = ? WHERE id = ?")
        .bind(&fresh_nonce)
        .bind(&user.id)
        .execute(&state.pool)
        .await?;

    let expiry = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        sub: user.id.clone(),
        wallet: wallet.clone(),
        exp: expiry,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.cfg.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Auth(format!("Token generation failed: {e}")))?;

    Ok(Json(AuthResponse {
        token,
        wallet_address: wallet,
    }))
}