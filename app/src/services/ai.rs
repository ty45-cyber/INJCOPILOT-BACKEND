use crate::{
    config::Config,
    errors::{AppError, AppResult},
    models::{AiPlan, AssetSnapshot, PortfolioContext},
};
use reqwest::Client;
use serde_json::{json, Value};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-20250514";

pub struct AiService {
    client: Client,
    api_key: String,
}

impl AiService {
    pub fn new(cfg: &Config) -> Self {
        Self {
            client: Client::new(),
            api_key: cfg.anthropic_api_key.clone(),
        }
    }

    pub async fn generate_action_plan(
        &self,
        user_prompt: &str,
        portfolio: &PortfolioContext,
        market_summary: &Value,
    ) -> AppResult<AiPlan> {
        let system_prompt = build_system_prompt();
        let user_message = build_user_message(user_prompt, portfolio, market_summary);

        let body = json!({
            "model": MODEL,
            "max_tokens": 2048,
            "system": system_prompt,
            "messages": [
                { "role": "user", "content": user_message }
            ]
        });

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::AiService(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(AppError::AiService(format!("Anthropic error: {err_text}")));
        }

        let raw: Value = response
            .json()
            .await
            .map_err(|e| AppError::AiService(format!("Parse failed: {e}")))?;

        let content = raw["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .ok_or_else(|| AppError::AiService("Empty AI response".into()))?;

        // Strip possible markdown fences before parsing
        let clean = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let plan: AiPlan = serde_json::from_str(clean)
            .map_err(|e| AppError::AiService(format!("Schema mismatch: {e}. Raw: {clean}")))?;

        Ok(plan)
    }
}

fn build_system_prompt() -> String {
    r#"
You are an AI financial decision assistant for the Injective blockchain protocol.
Your role is to analyze user intent, their current portfolio, and market conditions,
then return a structured action plan as STRICT JSON — no markdown, no preamble, no explanation outside the JSON.

OUTPUT SCHEMA (return exactly this shape):
{
  "intent_summary": "<1-2 sentence interpretation of user goal>",
  "portfolio_context": {
    "total_value_usd": <number>,
    "assets": [
      { "denom": "<string>", "amount": "<string>", "usd_value": <number>, "pct_of_portfolio": <number> }
    ],
    "dominant_exposure": "<string>"
  },
  "recommended_actions": [
    { "action": "<specific actionable step>", "reason": "<why this helps>", "priority": "<high|medium|low>" }
  ],
  "risk_analysis": {
    "low": ["<risk description>"],
    "medium": ["<risk description>"],
    "high": ["<risk description>"],
    "overall_score": <integer 1-10>
  },
  "injective_tx": {
    "tx_type": "<MsgSend|MsgPlaceOrder|MsgWithdraw|MsgDelegate|none>",
    "description": "<human-readable explanation of what this tx does>",
    "payload": {},
    "estimated_gas": <integer or null>
  }
}

RULES:
- If no on-chain action is warranted, set tx_type to "none" and payload to {}
- Never fabricate token prices; use the market data provided
- Keep recommended_actions to 3 maximum
- overall_score: 1 = minimal risk, 10 = extreme risk
- Return ONLY the JSON object. No surrounding text.
"#
    .into()
}

fn build_user_message(
    user_prompt: &str,
    portfolio: &PortfolioContext,
    market_summary: &Value,
) -> String {
    format!(
        "USER INTENT:\n{}\n\nPORTFOLIO STATE:\n{}\n\nMARKET SUMMARY:\n{}",
        user_prompt,
        serde_json::to_string_pretty(portfolio).unwrap_or_default(),
        serde_json::to_string_pretty(market_summary).unwrap_or_default(),
    )
}