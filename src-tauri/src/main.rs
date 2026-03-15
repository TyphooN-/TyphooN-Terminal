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

// ── Broker Commands ─────────────────────────────────────────────────

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

/// Fetch bars from multiple timeframes for a symbol (for MultiKAMA, ATR_Projection, PreviousCandleLevels).
/// Returns JSON: { "1Hour": [...bars], "4Hour": [...bars], "1Day": [...bars], ... }
#[tauri::command]
async fn get_multi_tf_bars(
    state: State<'_, SharedState>,
    symbol: String,
    timeframes: Vec<String>,
    limit: u32,
) -> Result<String, String> {
    let s = state.lock().await;
    let broker = s.broker.as_ref().ok_or("Not connected")?;
    let mut result = serde_json::Map::new();
    for tf in &timeframes {
        match broker.get_bars(&symbol, tf, limit).await {
            Ok(bars) => { result.insert(tf.clone(), serde_json::to_value(&bars).unwrap()); }
            Err(ref e) if e.contains("429") => {
                tracing::warn!("Rate limited on {symbol} @ {tf}, waiting 2s...");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                // Retry once
                match broker.get_bars(&symbol, tf, limit).await {
                    Ok(bars) => { result.insert(tf.clone(), serde_json::to_value(&bars).unwrap()); }
                    Err(e2) => { tracing::warn!("MTF bars retry failed for {symbol} @ {tf}: {e2}"); }
                }
            }
            Err(e) => { tracing::warn!("MTF bars failed for {symbol} @ {tf}: {e}"); }
        }
        // Rate limit between TF requests
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
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

    let (lots, count) = risk::calculate_lots(
        &s.risk_config,
        &spec,
        balance,
        equity,
        sl_distance,
        false, // TODO: break-even detection from positions
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
    let config: RiskConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Invalid config: {e}"))?;
    let mut s = state.lock().await;
    s.risk_config = config;
    Ok(())
}

// ── SL/TP Tracking Commands ─────────────────────────────────────────

#[tauri::command]
async fn set_sl_level(state: State<'_, SharedState>, symbol: String, price: f64) -> Result<(), String> {
    let mut s = state.lock().await;
    s.sl_levels.insert(symbol, price);
    Ok(())
}

#[tauri::command]
async fn set_tp_level(state: State<'_, SharedState>, symbol: String, price: f64) -> Result<(), String> {
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
    let config: MartingaleConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Invalid MG config: {e}"))?;
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
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            // Broker
            connect,
            get_account,
            get_positions,
            get_bars,
            place_order,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running TyphooN Terminal");
}
