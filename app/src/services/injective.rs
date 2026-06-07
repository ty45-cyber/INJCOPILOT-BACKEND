use crate::{
    config::Config,
    errors::{AppError, AppResult},
    models::{AssetSnapshot, PortfolioContext},
};
use reqwest::Client;
use serde_json::Value;

pub struct InjectiveService {
    client: Client,
    rest_url: String,
}

impl InjectiveService {
    pub fn new(cfg: &Config) -> Self {
        Self {
            client: Client::new(),
            rest_url: cfg.injective_rest_url.clone(),
        }
    }

    /// Fetch all bank balances for a wallet address
    pub async fn fetch_balances(&self, address: &str) -> AppResult<Vec<AssetSnapshot>> {
        let url = format!("{}/cosmos/bank/v1beta1/balances/{}", self.rest_url, address);

        let response: Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::InjectiveService(format!("Balance fetch failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::InjectiveService(format!("Balance parse failed: {e}")))?;

        let balances = response["balances"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let prices = self.fetch_oracle_prices().await.unwrap_or_default();

        let mut assets: Vec<AssetSnapshot> = balances
            .iter()
            .filter_map(|b| {
                let denom = b["denom"].as_str()?.to_string();
                let amount_raw = b["amount"].as_str()?.to_string();
                let amount_f64: f64 = amount_raw.parse().unwrap_or(0.0);

                // Injective native denom is 10^18 inj = 1 INJ
                let human_amount = if denom == "inj" {
                    amount_f64 / 1e18
                } else {
                    amount_f64 / 1e6  // most IBC tokens use 6 decimals
                };

                let usd_price = prices
                    .get(&denom.to_uppercase())
                    .copied()
                    .unwrap_or(0.0);

                Some(AssetSnapshot {
                    denom: denom.clone(),
                    amount: format!("{:.4}", human_amount),
                    usd_value: human_amount * usd_price,
                    pct_of_portfolio: 0.0,  // computed below after total
                })
            })
            .collect();

        // Compute portfolio percentages
        let total: f64 = assets.iter().map(|a| a.usd_value).sum();
        for asset in &mut assets {
            asset.pct_of_portfolio = if total > 0.0 {
                (asset.usd_value / total) * 100.0
            } else {
                0.0
            };
        }

        Ok(assets)
    }

    /// Build PortfolioContext from wallet address
    pub async fn build_portfolio_context(&self, address: &str) -> AppResult<PortfolioContext> {
        let assets = self.fetch_balances(address).await?;
        let total_value_usd: f64 = assets.iter().map(|a| a.usd_value).sum();

        let dominant_exposure = assets
            .iter()
            .max_by(|a, b| a.pct_of_portfolio.partial_cmp(&b.pct_of_portfolio).unwrap())
            .map(|a| a.denom.clone())
            .unwrap_or_else(|| "unknown".into());

        Ok(PortfolioContext {
            total_value_usd,
            assets,
            dominant_exposure,
        })
    }

    /// Fetch top market movers from Injective exchange API
    pub async fn fetch_market_summary(&self) -> AppResult<Value> {
        let url = format!("{}/injective/exchange/v1beta1/spot/markets", self.rest_url);

        let response: Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::InjectiveService(format!("Markets fetch failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::InjectiveService(format!("Markets parse failed: {e}")))?;

        // Extract top 5 markets by ticker for summary
        let markets = response["markets"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .take(5)
                    .map(|m| {
                        serde_json::json!({
                            "ticker": m["ticker"],
                            "status": m["status"],
                            "market_id": m["market_id"]
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(serde_json::json!({
            "top_markets": markets,
            "source": "injective_exchange_v1beta1",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Submit a signed transaction to Injective
    pub async fn broadcast_tx(&self, signed_tx: &Value) -> AppResult<String> {
        let url = format!("{}/cosmos/tx/v1beta1/txs", self.rest_url);

        let body = serde_json::json!({ "tx_bytes": signed_tx, "mode": "BROADCAST_MODE_SYNC" });

        let response: Value = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::InjectiveService(format!("Broadcast failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::InjectiveService(format!("Broadcast parse failed: {e}")))?;

        let tx_hash = response["tx_response"]["txhash"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(tx_hash)
    }

    /// Fetch oracle prices for common Injective denoms
    async fn fetch_oracle_prices(&self) -> AppResult<std::collections::HashMap<String, f64>> {
        let url = format!("{}/injective/oracle/v1beta1/oracle/prices", self.rest_url);

        let response: Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::InjectiveService(format!("Oracle fetch failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::InjectiveService(format!("Oracle parse failed: {e}")))?;

        let mut prices = std::collections::HashMap::new();

        if let Some(arr) = response["prices"].as_array() {
            for entry in arr {
                if let (Some(symbol), Some(price_str)) = (
                    entry["symbol"].as_str(),
                    entry["price"].as_str(),
                ) {
                    if let Ok(price) = price_str.parse::<f64>() {
                        prices.insert(symbol.to_uppercase(), price);
                    }
                }
            }
        }

        Ok(prices)
    }
}