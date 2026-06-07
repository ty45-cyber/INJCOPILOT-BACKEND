use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use anyhow::Result;

pub async fn create_pool(database_url: &str) -> Result<MySqlPool> {
    let pool = MySqlPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &MySqlPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id VARCHAR(36) PRIMARY KEY,
            wallet_address VARCHAR(64) UNIQUE NOT NULL,
            nonce VARCHAR(64) NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS intents (
            id VARCHAR(36) PRIMARY KEY,
            user_id VARCHAR(36) NOT NULL,
            prompt TEXT NOT NULL,
            status ENUM('pending','analyzed','executed','rejected','saved') DEFAULT 'pending',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ai_outputs (
            id VARCHAR(36) PRIMARY KEY,
            intent_id VARCHAR(36) NOT NULL,
            intent_summary TEXT,
            portfolio_context JSON,
            recommended_actions JSON,
            risk_analysis JSON,
            injective_tx JSON,
            raw_response LONGTEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (intent_id) REFERENCES intents(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS transactions (
            id VARCHAR(36) PRIMARY KEY,
            intent_id VARCHAR(36) NOT NULL,
            user_id VARCHAR(36) NOT NULL,
            tx_hash VARCHAR(128),
            tx_type VARCHAR(64),
            payload JSON,
            status ENUM('draft','pending','confirmed','failed') DEFAULT 'draft',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (intent_id) REFERENCES intents(id),
            FOREIGN KEY (user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS activity_logs (
            id VARCHAR(36) PRIMARY KEY,
            user_id VARCHAR(36) NOT NULL,
            event_type VARCHAR(64) NOT NULL,
            description TEXT,
            metadata JSON,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("Database migrations complete");
    Ok(())
}