use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

// ── Auth ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub wallet_address: String,
    pub nonce: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct WalletConnectRequest {
    pub wallet_address: String,
}

#[derive(Debug, Deserialize)]
pub struct WalletVerifyRequest {
    pub wallet_address: String,
    pub signature: String,
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub wallet_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,          // user_id
    pub wallet: String,
    pub exp: usize,
}

// ── Intent ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Intent {
    pub id: String,
    pub user_id: String,
    pub prompt: String,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct IntentRequest {
    pub prompt: String,
}

// ── AI Output ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AiOutput {
    pub id: String,
    pub intent_id: String,
    pub intent_summary: Option<String>,
    pub portfolio_context: Option<serde_json::Value>,
    pub recommended_actions: Option<serde_json::Value>,
    pub risk_analysis: Option<serde_json::Value>,
    pub injective_tx: Option<serde_json::Value>,
}

/// Strict schema enforced in AI prompt
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiPlan {
    pub intent_summary: String,
    pub portfolio_context: PortfolioContext,
    pub recommended_actions: Vec<RecommendedAction>,
    pub risk_analysis: RiskAnalysis,
    pub injective_tx: InjectiveTxDraft,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortfolioContext {
    pub total_value_usd: f64,
    pub assets: Vec<AssetSnapshot>,
    pub dominant_exposure: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetSnapshot {
    pub denom: String,
    pub amount: String,
    pub usd_value: f64,
    pub pct_of_portfolio: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecommendedAction {
    pub action: String,
    pub reason: String,
    pub priority: String,  // "high" | "medium" | "low"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskAnalysis {
    pub low: Vec<String>,
    pub medium: Vec<String>,
    pub high: Vec<String>,
    pub overall_score: u8,  // 1–10
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InjectiveTxDraft {
    pub tx_type: String,
    pub description: String,
    pub payload: serde_json::Value,
    pub estimated_gas: Option<u64>,
}

// ── Transaction ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: String,
    pub intent_id: String,
    pub user_id: String,
    pub tx_hash: Option<String>,
    pub tx_type: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteIntentRequest {
    pub intent_id: String,
    pub signed_tx: serde_json::Value,
}

// ── Activity Log ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ActivityLog {
    pub id: String,
    pub user_id: String,
    pub event_type: String,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: Option<DateTime<Utc>>,
}