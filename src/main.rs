mod db;
mod config;
mod routing;
mod server;
mod events;
mod tui;

use db::DbStore;
use server::AppState;
use std::sync::Arc;
use genai::Client;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Redirect tracing logs to a file so they don't corrupt the Ratatui UI
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("proxy.log")?;
    tracing_subscriber::fmt()
        .with_writer(log_file)
        .init();

    tracing::info!("Starting LLM Harness & Local Router Proxy...");

    // Initialize local database
    let db = DbStore::new("llm_proxy.db")?;
    tracing::info!("Initialized local database");

    // Inject keys from environment
    config::inject_environment_keys(&db)?;

    tracing::info!("Phase 1 setup complete");

    // Initialize GenAI client
    let genai_client = Arc::new(Client::default());

    // Create channel for events
    let (tx, rx) = mpsc::channel(100);

    let state = AppState {
        db: db.clone(),
        genai_client,
        tx,
    };

    // Start Axum proxy server in background
    tracing::info!("Starting Axum Server...");
    tokio::spawn(async move {
        if let Err(e) = server::start_server(state).await {
            tracing::error!("Server error: {}", e);
        }
    });

    // Start TUI
    tracing::info!("Starting TUI dashboard...");
    tui::run_tui(rx, db).await?;

    Ok(())
}
