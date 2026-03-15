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
#![allow(dead_code)]

mod broker;
mod core;
mod notifications;
mod strategies;

use broker::alpaca::AlpacaBroker;
use core::risk::{self, OrderMode, RiskConfig, SymbolSpec};
use core::var;
use core::margin;
use strategies::martingale::{MartingaleConfig, MartingaleMode, MartingaleState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

/// Shared application state.
struct AppState {
    broker: Option<AlpacaBroker>,
    risk_config: RiskConfig,
    martingale: MartingaleState,
    /// Per-symbol SL/TP tracked locally (Alpaca can't modify after placement).
    sl_levels: std::collections::HashMap<String, f64>,
    tp_levels: std::collections::HashMap<String, f64>,
    /// Cached symbol list for autocomplete.
    symbols: Vec<(String, String)>, // (symbol, name)
}

type SharedState = Arc<Mutex<AppState>>;

// ── Input Validation ────────────────────────────────────────────────

/// Strict symbol validation: 1-10 alphanumeric chars, optional single "/" for crypto pairs.
fn is_valid_symbol(symbol: &str) -> bool {
    if symbol.is_empty() || symbol.len() > 10 {
        return false;
    }
    let slash_count = symbol.chars().filter(|&c| c == '/').count();
    if slash_count > 1 {
        return false;
    }
    symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '.')
}

/// Validate timeframe input against known Alpaca timeframes.
fn is_valid_timeframe(tf: &str) -> bool {
    matches!(tf, "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" | "1Day" | "1Week" | "1Month")
}

// ── Broker Commands ─────────────────────────────────────────────────

#[tauri::command]
async fn connect(
    state: State<'_, SharedState>,
    api_key: String,
    secret_key: String,
    paper: bool,
) -> Result<String, String> {
    // Validate API key format (Alpaca keys are 20 alphanumeric chars)
    if api_key.is_empty() || api_key.len() > 100 || !api_key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid API key format".to_string());
    }
    if secret_key.is_empty() || secret_key.len() > 100 || !secret_key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid secret key format".to_string());
    }
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
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !is_valid_timeframe(&timeframe) { return Err("Invalid timeframe".into()); }
    let limit = limit.min(50_000);
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let bars = broker.get_bars(&symbol, &timeframe, limit).await?;
    Ok(serde_json::to_string(&bars).unwrap())
}

/// Fetch bars from multiple timeframes for a symbol (for MultiKAMA, ATR_Projection, PreviousCandleLevels).
/// Returns JSON: { "1Hour": [...bars], "4Hour": [...bars], "1Day": [...bars], ... }
#[tauri::command]
async fn get_multi_tf_bars(
    state: State<'_, SharedState>,
    symbol: String,
    timeframes: Vec<String>,
    limit: u32,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if timeframes.len() > 10 { return Err("Too many timeframes".into()); }
    for tf in &timeframes {
        if !is_valid_timeframe(tf) { return Err(format!("Invalid timeframe: {tf}")); }
    }
    let limit = limit.min(50_000);
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let mut result = serde_json::Map::new();
    for tf in &timeframes {
        // Rate limiting handled by broker.rate_limiter inside get_bars
        match broker.get_bars(&symbol, tf, limit).await {
            Ok(bars) => { result.insert(tf.clone(), serde_json::to_value(&bars).unwrap()); }
            Err(e) => { tracing::warn!("MTF bars {symbol} @ {tf}: {e}"); }
        }
    }
    Ok(serde_json::Value::Object(result).to_string())
}

#[tauri::command]
async fn place_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
) -> Result<String, String> {
    // Input validation — strict symbol whitelist
    if !is_valid_symbol(&symbol) {
        return Err("Invalid symbol".to_string());
    }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() {
        return Err(format!("Invalid quantity: {qty}. Must be 0 < qty <= 1,000,000"));
    }
    if side != "buy" && side != "sell" {
        return Err(format!("Invalid side: {side}. Must be 'buy' or 'sell'"));
    }
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
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if let Some(q) = qty {
        if q <= 0.0 || !q.is_finite() { return Err("Invalid quantity".into()); }
    }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let result = broker.close_position(&symbol, qty).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn place_limit_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    limit_price: f64,
    tif: String,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if !limit_price.is_finite() || limit_price <= 0.0 { return Err("Invalid limit price".into()); }
    let tif = if matches!(tif.as_str(), "day" | "gtc" | "ioc" | "fok") { tif } else { "gtc".to_string() };
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let result = broker.limit_order(&symbol, qty, &side, limit_price, &tif).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn place_stop_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    stop_price: f64,
    tif: String,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if !stop_price.is_finite() || stop_price <= 0.0 { return Err("Invalid stop price".into()); }
    let tif = if matches!(tif.as_str(), "day" | "gtc" | "ioc" | "fok") { tif } else { "gtc".to_string() };
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let result = broker.stop_order(&symbol, qty, &side, stop_price, &tif).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn place_stop_limit_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    stop_price: f64,
    limit_price: f64,
    tif: String,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if !stop_price.is_finite() || stop_price <= 0.0 || !limit_price.is_finite() || limit_price <= 0.0 {
        return Err("Invalid price".into());
    }
    let tif = if matches!(tif.as_str(), "day" | "gtc" | "ioc" | "fok") { tif } else { "gtc".to_string() };
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let result = broker.stop_limit_order(&symbol, qty, &side, stop_price, limit_price, &tif).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn place_trailing_stop(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    trail_price: Option<f64>,
    trail_percent: Option<f64>,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if trail_price.is_none() && trail_percent.is_none() { return Err("Must specify trail_price or trail_percent".into()); }
    if let Some(tp) = trail_price { if !tp.is_finite() || tp <= 0.0 { return Err("Invalid trail price".into()); } }
    if let Some(tp) = trail_percent { if !tp.is_finite() || tp <= 0.0 || tp > 50.0 { return Err("Invalid trail percent".into()); } }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let result = broker.trailing_stop_order(&symbol, qty, &side, trail_price, trail_percent, "gtc").await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn place_bracket_order(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    tp_price: f64,
    sl_price: f64,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if qty <= 0.0 || qty > 1_000_000.0 || !qty.is_finite() { return Err("Invalid quantity".into()); }
    if side != "buy" && side != "sell" { return Err("Invalid side".into()); }
    if !tp_price.is_finite() || tp_price <= 0.0 || !sl_price.is_finite() || sl_price <= 0.0 {
        return Err("Invalid TP/SL price".into());
    }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let result = broker.bracket_order(&symbol, qty, &side, tp_price, sl_price).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn get_open_orders(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let orders = broker.get_orders("open", 100).await?;
    Ok(serde_json::to_string(&orders).unwrap())
}

#[tauri::command]
async fn get_order_history(state: State<'_, SharedState>, limit: u32) -> Result<String, String> {
    let limit = limit.min(500);
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let orders = broker.get_orders("closed", limit).await?;
    Ok(serde_json::to_string(&orders).unwrap())
}

#[tauri::command]
async fn modify_order(
    state: State<'_, SharedState>,
    order_id: String,
    qty: Option<f64>,
    limit_price: Option<f64>,
    stop_price: Option<f64>,
    trail: Option<f64>,
) -> Result<String, String> {
    if order_id.is_empty() || order_id.len() > 100 { return Err("Invalid order ID".into()); }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let result = broker.modify_order(&order_id, qty, limit_price, stop_price, trail).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn cancel_order(state: State<'_, SharedState>, order_id: String) -> Result<(), String> {
    if order_id.is_empty() || order_id.len() > 100 { return Err("Invalid order ID".into()); }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    broker.cancel_order(&order_id).await
}

#[tauri::command]
async fn close_all(state: State<'_, SharedState>) -> Result<(), String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    broker.close_all_positions().await
}

#[tauri::command]
async fn load_symbols(state: State<'_, SharedState>) -> Result<String, String> {
    let mut s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let assets = broker.get_all_assets().await?;
    let symbols: Vec<(String, String)> = assets
        .iter()
        .map(|a| (a.symbol.clone(), a.name.clone()))
        .collect();
    let count = symbols.len();
    s.symbols = symbols;
    Ok(format!("{count}"))
}

#[tauri::command]
async fn search_symbols(state: State<'_, SharedState>, query: String) -> Result<String, String> {
    if query.len() > 50 { return Err("Query too long".into()); }
    let s = state.lock().await;
    let q = query.to_uppercase();
    let matches: Vec<&(String, String)> = s.symbols
        .iter()
        .filter(|(sym, name)| {
            sym.starts_with(&q) || name.to_uppercase().contains(&q)
        })
        .take(20)
        .collect();
    Ok(serde_json::to_string(&matches).unwrap())
}

#[tauri::command]
async fn get_asset(state: State<'_, SharedState>, symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let asset = broker.get_asset(&symbol).await?;
    Ok(serde_json::to_string(&asset).unwrap())
}

#[tauri::command]
async fn send_discord_notification(webhook_url: String, message: String) -> Result<(), String> {
    notifications::send_discord(&webhook_url, &message).await
}

// ── Risk & Lot Calculation Commands ─────────────────────────────────

#[tauri::command]
async fn calculate_lots(
    state: State<'_, SharedState>,
    symbol: String,
    sl_price: f64,
    tp_price: f64,
    current_price: f64,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !sl_price.is_finite() || !tp_price.is_finite() || !current_price.is_finite() {
        return Err("Invalid price value".into());
    }
    if sl_price <= 0.0 || tp_price <= 0.0 || current_price <= 0.0 {
        return Err("Prices must be positive".into());
    }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;

    let account = broker.get_account().await?;
    let balance = account.balance;
    let equity = account.equity;

    // Determine direction from TP/SL
    let is_buy = tp_price > sl_price;
    let sl_distance = if is_buy {
        current_price - sl_price
    } else {
        sl_price - current_price
    };

    if sl_distance <= 0.0 {
        return Err("SL must be on the opposite side of entry price".to_string());
    }

    // Get asset specs
    let asset = broker.get_asset(&symbol).await?;
    let spec = SymbolSpec {
        symbol: symbol.clone(),
        tick_size: asset.price_increment.unwrap_or(0.01),
        tick_value: asset.price_increment.unwrap_or(0.01), // 1:1 for stocks
        volume_min: asset.min_order_size.unwrap_or(1.0),
        volume_max: 1_000_000.0,
        volume_step: asset.min_trade_increment.unwrap_or(1.0),
        contract_size: 1.0,
        margin_rate: 1.0,
    };

    // VaR per lot (if VaR mode)
    let var_per_lot = if s.risk_config.order_mode == OrderMode::VaR {
        let bars = broker.get_bars(&symbol, &s.risk_config.var_timeframe, s.risk_config.var_periods + 1).await?;
        let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
        var::calculate_var(&closes, 1.0, spec.tick_value, spec.tick_size, current_price, s.risk_config.var_confidence)
            .map(|r| r.var_dollars)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    // Break-even detection: check if any existing position on this symbol has SL ≈ entry
    let has_break_even = {
        let positions = broker.get_positions().await.unwrap_or_default();
        let tick = spec.tick_size;
        let symbol_no_slash = symbol.replace("/", "");
        positions.iter().any(|p| {
            (p.symbol == symbol || p.symbol == symbol_no_slash) && {
                // Check if SL is tracked in backend state
                if let Some(&sl) = s.sl_levels.get(&symbol) {
                    (sl - p.avg_entry_price).abs() < tick * 0.5
                } else {
                    false
                }
            }
        })
    };

    let (lots, count) = risk::calculate_lots(
        &s.risk_config,
        &spec,
        balance,
        equity,
        sl_distance,
        has_break_even,
        var_per_lot,
    );

    let side = if is_buy { "buy" } else { "sell" };

    Ok(serde_json::to_string(&serde_json::json!({
        "lots": lots,
        "count": count,
        "side": side,
        "sl_distance": sl_distance,
        "mode": format!("{:?}", s.risk_config.order_mode),
        "risk_money": if s.risk_config.order_mode == OrderMode::Standard {
            balance * (s.risk_config.risk_pct / 100.0)
        } else { 0.0 },
    })).unwrap())
}

#[tauri::command]
async fn calculate_position_var(
    state: State<'_, SharedState>,
    symbol: String,
    position_size: f64,
    current_price: f64,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !position_size.is_finite() || !current_price.is_finite() || current_price <= 0.0 {
        return Err("Invalid price or position size".into());
    }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;

    let bars = broker.get_bars(&symbol, &s.risk_config.var_timeframe, s.risk_config.var_periods + 1).await?;
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();

    let asset = broker.get_asset(&symbol).await?;
    let tick_size = asset.price_increment.unwrap_or(0.01);
    let tick_value = tick_size; // 1:1 for stocks

    match var::calculate_var(&closes, position_size, tick_value, tick_size, current_price, s.risk_config.var_confidence) {
        Some(result) => Ok(serde_json::to_string(&result).unwrap()),
        None => Err("VaR calculation failed — insufficient price data".to_string()),
    }
}

// ── Risk Config Commands ────────────────────────────────────────────

#[tauri::command]
async fn get_risk_config(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    Ok(serde_json::to_string(&s.risk_config).unwrap())
}

#[tauri::command]
async fn set_order_mode(state: State<'_, SharedState>, mode: String) -> Result<(), String> {
    let mut s = state.lock().await;
    s.risk_config.order_mode = match mode.as_str() {
        "Standard" => OrderMode::Standard,
        "Fixed" => OrderMode::Fixed,
        "Dynamic" => OrderMode::Dynamic,
        "VaR" => OrderMode::VaR,
        _ => return Err(format!("Unknown order mode: {mode}")),
    };
    Ok(())
}

#[tauri::command]
async fn set_risk_config(state: State<'_, SharedState>, config_json: String) -> Result<(), String> {
    if config_json.len() > 4096 { return Err("Config too large".into()); }
    let config: RiskConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Invalid config: {e}"))?;
    // Bounds validation on all financial parameters
    if config.risk_pct < 0.0 || config.risk_pct > 100.0 { return Err("risk_pct must be 0-100".into()); }
    if config.max_risk_pct < 0.0 || config.max_risk_pct > 100.0 { return Err("max_risk_pct must be 0-100".into()); }
    if config.var_confidence < 0.0 || config.var_confidence > 1.0 { return Err("var_confidence must be 0-1".into()); }
    if config.fixed_lots < 0.0 || config.fixed_lots > 1_000_000.0 { return Err("fixed_lots out of range".into()); }
    if config.fixed_orders > 100 { return Err("fixed_orders too large".into()); }
    if config.var_risk_pct < 0.0 || config.var_risk_pct > 100.0 { return Err("var_risk_pct must be 0-100".into()); }
    if config.var_notional < 0.0 || config.var_notional > 1e9 { return Err("var_notional out of range".into()); }
    if config.var_periods > 10_000 { return Err("var_periods too large".into()); }
    if config.margin_buffer_pct < 0.0 || config.margin_buffer_pct > 100.0 { return Err("margin_buffer_pct must be 0-100".into()); }
    if config.min_balance < 0.0 { return Err("min_balance must be non-negative".into()); }
    if config.additional_risk_ratio < 0.0 || config.additional_risk_ratio > 10.0 { return Err("additional_risk_ratio out of range".into()); }
    if !is_valid_timeframe(&config.var_timeframe) { return Err("Invalid var_timeframe".into()); }
    let mut s = state.lock().await;
    s.risk_config = config;
    Ok(())
}

// ── SL/TP Tracking Commands ─────────────────────────────────────────

#[tauri::command]
async fn set_sl_level(state: State<'_, SharedState>, symbol: String, price: f64) -> Result<(), String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !price.is_finite() || price <= 0.0 { return Err("Invalid price".into()); }
    let mut s = state.lock().await;
    s.sl_levels.insert(symbol, price);
    Ok(())
}

#[tauri::command]
async fn set_tp_level(state: State<'_, SharedState>, symbol: String, price: f64) -> Result<(), String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !price.is_finite() || price <= 0.0 { return Err("Invalid price".into()); }
    let mut s = state.lock().await;
    s.tp_levels.insert(symbol, price);
    Ok(())
}

#[tauri::command]
async fn get_sl_tp_pl(
    state: State<'_, SharedState>,
    symbol: String,
    qty: f64,
    side: String,
    entry_price: f64,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    if !qty.is_finite() || !entry_price.is_finite() { return Err("Invalid numeric value".into()); }
    if side != "long" && side != "short" { return Err("Invalid side".into()); }
    let s = state.lock().await;
    let sl = s.sl_levels.get(&symbol).copied();
    let tp = s.tp_levels.get(&symbol).copied();

    let sl_pl = sl.map(|sl_price| {
        if side == "long" { (sl_price - entry_price) * qty }
        else { (entry_price - sl_price) * qty }
    });
    let tp_pl = tp.map(|tp_price| {
        if side == "long" { (tp_price - entry_price) * qty }
        else { (entry_price - tp_price) * qty }
    });
    let rr = match (sl_pl, tp_pl) {
        (Some(s), Some(t)) if s.abs() > 1e-10 => Some(t / s.abs()),
        _ => None,
    };

    Ok(serde_json::to_string(&serde_json::json!({
        "sl_pl": sl_pl,
        "tp_pl": tp_pl,
        "rr": rr,
        "sl_price": sl,
        "tp_price": tp,
    })).unwrap())
}

// ── Martingale Commands ─────────────────────────────────────────────

#[tauri::command]
async fn get_martingale_state(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    Ok(serde_json::to_string(&serde_json::json!({
        "mode": format!("{:?}", s.martingale.mode),
        "label": s.martingale.mode.label(),
        "enabled": s.martingale.config.enabled,
        "trim_pct": s.martingale.config.trim_pct,
        "protect_pct": s.martingale.config.protect_pct,
        "hedge_closes": s.martingale.hedge_closes,
        "bias_closes": s.martingale.bias_closes,
        "protect_fires": s.martingale.protect_fire_count,
    })).unwrap())
}

#[tauri::command]
async fn set_martingale_mode(state: State<'_, SharedState>, mode: String) -> Result<String, String> {
    let mut s = state.lock().await;
    s.martingale.mode = match mode.as_str() {
        "Off" => MartingaleMode::Off,
        "Long" => MartingaleMode::Long,
        "Short" => MartingaleMode::Short,
        "Unwind" => MartingaleMode::Unwind,
        _ => return Err(format!("Unknown MG mode: {mode}")),
    };
    s.martingale.config.enabled = s.martingale.mode != MartingaleMode::Off;
    Ok(s.martingale.mode.label().to_string())
}

#[tauri::command]
async fn toggle_martingale(state: State<'_, SharedState>) -> Result<String, String> {
    let mut s = state.lock().await;
    s.martingale.mode = s.martingale.mode.next();
    s.martingale.config.enabled = s.martingale.mode != MartingaleMode::Off;
    Ok(serde_json::to_string(&serde_json::json!({
        "mode": format!("{:?}", s.martingale.mode),
        "label": s.martingale.mode.label(),
    })).unwrap())
}

#[tauri::command]
async fn set_martingale_config(state: State<'_, SharedState>, config_json: String) -> Result<(), String> {
    if config_json.len() > 4096 { return Err("Config too large".into()); }
    let config: MartingaleConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Invalid MG config: {e}"))?;
    // Bounds: percentages must be 0-10000 (allow high margin levels), spread tolerance non-negative
    if config.trim_pct < 0.0 || config.protect_pct < 0.0 || config.hard_floor_pct < 0.0 {
        return Err("Margin thresholds must be non-negative".into());
    }
    if config.spread_tolerance < 0.0 { return Err("Spread tolerance must be non-negative".into()); }
    let mut s = state.lock().await;
    s.martingale.config = config;
    Ok(())
}

#[tauri::command]
async fn calc_open_mg_size(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let account = broker.get_account().await?;

    let (per_side, safe_gross) = s.martingale.calc_open_mg_size(account.equity);

    Ok(serde_json::to_string(&serde_json::json!({
        "per_side": per_side,
        "safe_gross": safe_gross,
        "equity": account.equity,
        "spread_tolerance": s.martingale.config.spread_tolerance,
    })).unwrap())
}

#[tauri::command]
async fn open_martingale_hedge(
    state: State<'_, SharedState>,
    symbol: String,
    direction: String,
) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let account = broker.get_account().await?;

    let (per_side, safe_gross) = s.martingale.calc_open_mg_size(account.equity);
    if per_side <= 0.0 {
        return Err("Insufficient equity for MG position".to_string());
    }

    let bias_side = match direction.as_str() {
        "Long" | "long" => "buy",
        "Short" | "short" => "sell",
        _ => return Err(format!("Invalid direction: {direction}")),
    };
    let hedge_side = if bias_side == "buy" { "sell" } else { "buy" };

    // Place hedge first (safer — neutral until bias placed)
    let hedge_result = broker.market_order(&symbol, per_side, hedge_side).await?;
    let bias_result = broker.market_order(&symbol, per_side, bias_side).await?;

    Ok(serde_json::to_string(&serde_json::json!({
        "hedge_order": hedge_result,
        "bias_order": bias_result,
        "per_side": per_side,
        "safe_gross": safe_gross,
        "direction": direction,
    })).unwrap())
}

// ── Margin Calculation Command ──────────────────────────────────────

#[tauri::command]
async fn get_margin_info(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let account = broker.get_account().await?;

    let ml = margin::margin_level_pct(account.equity, account.initial_margin);
    let usable = margin::usable_margin(
        account.balance,
        account.initial_margin,
        s.risk_config.margin_buffer_pct,
    );
    let positions = broker.get_positions().await?;
    let gross: f64 = positions.iter().map(|p| p.qty.abs()).sum();
    let spread_tol = margin::spread_tolerance(account.equity, gross);

    // Determine MG zone — only show zone if positions exist and MG is active
    let zone = if gross <= 0.0 || !s.martingale.config.enabled {
        ""
    } else if ml <= s.martingale.config.hard_floor_pct {
        "HARD FLOOR"
    } else if ml < s.martingale.config.protect_pct {
        "PROTECT"
    } else if ml <= s.martingale.config.trim_pct {
        "DEAD ZONE"
    } else {
        "TRIM"
    };

    Ok(serde_json::to_string(&serde_json::json!({
        "margin_level_pct": ml,
        "usable_margin": usable,
        "spread_tolerance": spread_tol,
        "gross_lots": gross,
        "zone": zone,
        "equity": account.equity,
        "balance": account.balance,
        "margin_used": account.initial_margin,
    })).unwrap())
}

// ── News & Events ───────────────────────────────────────────────

#[tauri::command]
async fn get_news(state: State<'_, SharedState>, symbol: String, limit: u32) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let limit = limit.min(50);
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let news = broker.get_news(&symbol, limit).await?;
    Ok(serde_json::to_string(&news).unwrap())
}

#[tauri::command]
async fn get_corporate_actions(state: State<'_, SharedState>, symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let actions = broker.get_corporate_actions(&symbol, "dividend").await?;
    Ok(serde_json::to_string(&actions).unwrap())
}

#[tauri::command]
async fn get_sec_filings(symbol: String, filing_type: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let result = broker::alpaca::AlpacaBroker::get_sec_filings(&symbol, &filing_type, 20).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
async fn get_company_fundamentals(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let result = broker::alpaca::AlpacaBroker::get_sec_company_facts(&symbol).await?;
    Ok(serde_json::to_string(&result).unwrap())
}

/// Fetch article content from URL, return as text. For in-app reading.
/// Hardened: HTTPS only, 10s timeout, 2MB max response.
#[tauri::command]
async fn fetch_article(url: String) -> Result<String, String> {
    if !url.starts_with("https://") {
        return Err("Only HTTPS URLs allowed".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;
    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible)")
        .send()
        .await
        .map_err(|_| "Article fetch failed".to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    // Limit response body to 2MB
    let bytes = resp.bytes().await.map_err(|_| "Read failed".to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Response too large".to_string());
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| "Invalid UTF-8".to_string())
}

/// Clear all cached data for a specific symbol from cold storage.
/// Hardened: validates symbol, ensures deletions stay within cache directory.
#[tauri::command]
async fn clear_symbol_cache(symbol: String) -> Result<String, String> {
    if !is_valid_symbol(&symbol) { return Err("Invalid symbol".into()); }
    let dir = get_cache_dir();
    let canonical_dir = std::fs::canonicalize(&dir).map_err(|e| format!("Cache dir error: {e}"))?;
    let prefix = symbol.replace('/', "_");
    let mut removed = 0;
    if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            // Ensure file is actually inside cache directory (prevent symlink traversal)
            let entry_path = entry.path();
            if let Ok(canonical_entry) = std::fs::canonicalize(&entry_path) {
                if !canonical_entry.starts_with(&canonical_dir) { continue; }
            } else {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                if !name.ends_with(".zst") { continue; }
                if name.starts_with(&prefix) || name.contains(&format!("_{prefix}")) {
                    tokio::fs::remove_file(entry_path).await.ok();
                    removed += 1;
                }
            }
        }
    }
    tracing::info!("Cleared {removed} cache files for {symbol}");
    Ok(format!("Cleared {removed} files"))
}

// ── Cold Cache (zstd-compressed files on disk) ──────────────────

fn get_cache_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = std::path::PathBuf::from(home).join(".config").join("typhoon-terminal").join("cache");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn cache_key_to_filename(key: &str) -> String {
    key.replace('/', "_").replace(':', "_") + ".zst"
}

#[tauri::command]
async fn save_cold_cache(key: String, data: String) -> Result<(), String> {
    // Validate cache key doesn't contain path traversal
    let filename = cache_key_to_filename(&key);
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Invalid cache key".to_string());
    }
    // Limit uncompressed data to 50MB (prevents disk exhaustion)
    if data.len() > 50 * 1024 * 1024 {
        return Err("Cache data too large".to_string());
    }
    let dir = get_cache_dir();
    let path = dir.join(&filename);
    let compressed = zstd::encode_all(data.as_bytes(), 3)
        .map_err(|e| format!("zstd compress failed: {e}"))?;
    let raw_size = data.len();
    let compressed_size = compressed.len();
    tokio::fs::write(&path, compressed).await
        .map_err(|e| format!("Cache write failed: {e}"))?;
    tracing::debug!(
        "Cold cache: {} → {} bytes ({:.1}x compression)",
        raw_size, compressed_size, raw_size as f64 / compressed_size.max(1) as f64
    );
    Ok(())
}

#[tauri::command]
async fn load_cold_cache(key: String) -> Result<String, String> {
    let filename = cache_key_to_filename(&key);
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Invalid cache key".to_string());
    }
    let dir = get_cache_dir();
    let path = dir.join(&filename);
    if !path.exists() {
        return Err("Not in cold cache".to_string());
    }
    let compressed = tokio::fs::read(&path).await
        .map_err(|e| format!("Cache read failed: {e}"))?;
    // Reject suspiciously large compressed files (>10MB compressed → could be a zstd bomb)
    if compressed.len() > 10 * 1024 * 1024 {
        return Err("Compressed cache file too large".to_string());
    }
    let decompressed = zstd::decode_all(compressed.as_slice())
        .map_err(|e| format!("zstd decompress failed: {e}"))?;
    // Cap decompressed size at 50MB
    if decompressed.len() > 50 * 1024 * 1024 {
        return Err("Decompressed data too large".to_string());
    }
    String::from_utf8(decompressed)
        .map_err(|e| format!("UTF-8 decode failed: {e}"))
}

#[tauri::command]
async fn list_cold_cache() -> Result<String, String> {
    let dir = get_cache_dir();
    let mut entries = Vec::new();
    if let Ok(mut read_dir) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".zst") {
                    let size = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
                    entries.push(serde_json::json!({
                        "key": name.trim_end_matches(".zst").replace('_', ":"),
                        "size": size,
                    }));
                    if entries.len() >= 10_000 { break; } // cap listing
                }
            }
        }
    }
    Ok(serde_json::to_string(&entries).unwrap())
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
        sl_levels: std::collections::HashMap::new(),
        tp_levels: std::collections::HashMap::new(),
        symbols: Vec::new(),
    }));

    tauri::Builder::default()
        // tauri-plugin-shell removed — not used, reduces attack surface
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            // Broker
            connect,
            get_account,
            get_positions,
            get_bars,
            place_order,
            place_limit_order,
            place_stop_order,
            place_stop_limit_order,
            place_trailing_stop,
            place_bracket_order,
            get_open_orders,
            get_order_history,
            modify_order,
            cancel_order,
            close_position,
            close_all,
            get_asset,
            get_multi_tf_bars,
            load_symbols,
            search_symbols,
            // Risk
            calculate_lots,
            calculate_position_var,
            get_risk_config,
            set_order_mode,
            set_risk_config,
            // SL/TP
            set_sl_level,
            set_tp_level,
            get_sl_tp_pl,
            // Martingale
            get_martingale_state,
            set_martingale_mode,
            toggle_martingale,
            set_martingale_config,
            calc_open_mg_size,
            open_martingale_hedge,
            // Margin
            get_margin_info,
            // Notifications
            send_discord_notification,
            // News, Events & Fundamentals
            get_news,
            get_corporate_actions,
            get_sec_filings,
            get_company_fundamentals,
            // Articles & cache management
            fetch_article,
            clear_symbol_cache,
            // Cold cache (zstd)
            save_cold_cache,
            load_cold_cache,
            list_cold_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TyphooN Terminal");
}
