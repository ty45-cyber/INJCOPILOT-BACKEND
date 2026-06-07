use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub jwt_secret: String,
    pub anthropic_api_key: String,
    pub injective_rest_url: String,
    pub injective_chain_id: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8000".into())
                .parse()
                .context("PORT must be a valid u16")?,
            jwt_secret: std::env::var("JWT_SECRET")
                .context("JWT_SECRET must be set")?,
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY must be set")?,
            injective_rest_url: std::env::var("INJECTIVE_REST_URL")
                .unwrap_or_else(|_| "https://lcd.injective.network".into()),
            injective_chain_id: std::env::var("INJECTIVE_CHAIN_ID")
                .unwrap_or_else(|_| "injective-1".into()),
        })
    }
}