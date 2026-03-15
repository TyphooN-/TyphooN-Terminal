//! TyphooN Terminal — Risk management trading terminal for Alpaca Markets.
//!
//! Full port of TyphooN EA v1.420 from MQL5:
//! - 4 order modes (Standard, Fixed, Dynamic, VaR)
//! - Forward-looking TRIM martingale
//! - Dynamic PROTECT with urgency scaling
//! - VaR calculation with configurable confidence
//! - Draggable SL/TP lines, one-click order placement
//! - Real-time dashboard (position, risk, P/L, margin level)
//! - Discord webhook notifications

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)] // Scaffold — functions will be wired up incrementally

mod broker;
mod core;
mod notifications;
mod strategies;

use broker::alpaca::AlpacaBroker;
use core::risk::RiskConfig;
use strategies::martingale::{MartingaleConfig, MartingaleState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

/// Shared application state.
struct AppState {
    broker: Option<AlpacaBroker>,
    risk_config: RiskConfig,
    martingale: MartingaleState,
}

type SharedState = Arc<Mutex<AppState>>;

// ── Tauri Commands (called from frontend JS) ─────────────────────────

#[tauri::command]
async fn connect(
    state: State<'_, SharedState>,
    api_key: String,
    secret_key: String,
    paper: bool,
) -> Result<String, String> {
    let broker = AlpacaBroker::new(api_key, secret_key, paper);
    let account = broker.get_account().await?;
    let mut s = state.lock().await;
    s.broker = Some(broker);
    Ok(serde_json::to_string(&account).unwrap())
}

#[tauri::command]
async fn get_account(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let account = broker.get_account().await?;
    Ok(serde_json::to_string(&account).unwrap())
}

#[tauri::command]
async fn get_positions(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let positions = broker.get_positions().await?;
    Ok(serde_json::to_string(&positions).unwrap())
}

#[tauri::command]
async fn get_bars(
    state: State<'_, SharedState>,
    symbol: String,
    timeframe: String,
    limit: u32,
) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let bars = broker.get_bars(&symbol, &timeframe, limit).await?;
    Ok(serde_json::to_string(&bars).unwrap())
}

#[tauri::command]
async fn place_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let result = broker.market_order(&symbol, qty, &side).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn close_position(
    state: State<'_, SharedState>,
    symbol: String,
    qty: Option<f64>,
) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let result = broker.close_position(&symbol, qty).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn close_all(state: State<'_, SharedState>) -> Result<(), String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    broker.close_all_positions().await
}

#[tauri::command]
async fn get_asset(state: State<'_, SharedState>, symbol: String) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let asset = broker.get_asset(&symbol).await?;
    Ok(serde_json::to_string(&asset).unwrap())
}

#[tauri::command]
async fn send_discord_notification(webhook_url: String, message: String) -> Result<(), String> {
    notifications::send_discord(&webhook_url, &message).await
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "typhoon_terminal=info".into()),
        )
        .init();

    let state: SharedState = Arc::new(Mutex::new(AppState {
        broker: None,
        risk_config: RiskConfig::default(),
        martingale: MartingaleState::new(MartingaleConfig::default()),
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            connect,
            get_account,
            get_positions,
            get_bars,
            place_order,
            close_position,
            close_all,
            get_asset,
            send_discord_notification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TyphooN Terminal");
}
