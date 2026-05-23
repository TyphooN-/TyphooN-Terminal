//! tastytrade Broker Integration — REST API client for tastytrade/tastyworks.
//!
//! Implements session authentication, account info, positions, orders,
//! and options chain data via tastytrade's REST API.
//!
//! API docs: https://developer.tastytrade.com/

use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://api.tastyworks.com";
const SANDBOX_BASE: &str = "https://api.cert.tastyworks.com";

fn merge_market_data_universe_sources(
    watchlists: Result<Vec<String>, String>,
    futures: Result<Vec<String>, String>,
) -> Result<Vec<String>, String> {
    let mut symbols = std::collections::BTreeSet::new();
    let mut errors = Vec::new();

    match watchlists {
        Ok(items) => {
            for symbol in items {
                let symbol = symbol.trim().to_ascii_uppercase();
                if !symbol.is_empty() {
                    symbols.insert(symbol);
                }
            }
        }
        Err(e) => errors.push(format!("public watchlists: {e}")),
    }

    match futures {
        Ok(items) => {
            for symbol in items {
                let symbol = symbol.trim().to_ascii_uppercase();
                if !symbol.is_empty() {
                    symbols.insert(symbol);
                }
            }
        }
        Err(e) => errors.push(format!("active futures: {e}")),
    }

    if !symbols.is_empty() {
        Ok(symbols.into_iter().collect())
    } else if errors.is_empty() {
        Ok(Vec::new())
    } else {
        Err(errors.join(" | "))
    }
}

fn format_equity_order_price(price: f64) -> String {
    if price >= 1.0 {
        format!("{:.2}", price)
    } else if price >= 0.01 {
        format!("{:.4}", price)
    } else {
        format!("{:.6}", price)
    }
}

/// tastytrade broker client.
#[derive(Clone)]
pub struct TastytradeBroker {
    client: reqwest::Client,
    base_url: String,
    session_token: Option<String>,
    account_number: Option<String>,
    // Last error from the post-login get_accounts() call, if any. login() returns
    // Ok with the session token even when accounts can't be fetched (auth worked,
    // but the customer record may have no trading accounts attached). The caller
    // inspects this to build a useful error message instead of a generic "no
    // customer account was returned".
    last_accounts_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastySession {
    pub session_token: String,
    pub remember_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyAccount {
    pub account_number: String,
    pub account_type: String,
    pub nickname: Option<String>,
    pub margin_or_cash: String,
    pub is_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyPosition {
    pub symbol: String,
    pub instrument_type: String, // "Equity", "Equity Option", "Future", etc.
    pub quantity: f64,
    pub quantity_direction: String, // "Long" or "Short"
    pub close_price: f64,
    pub average_open_price: f64,
    pub mark_price: Option<f64>,
    pub unrealized_pnl: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyOrder {
    pub id: String,
    pub order_type: String,
    pub time_in_force: String,
    pub status: String,
    pub legs: Vec<TastyOrderLeg>,
    pub price: Option<f64>,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyOrderLeg {
    pub instrument_type: String,
    pub symbol: String,
    pub action: String, // "Buy to Open", "Sell to Close", etc.
    pub quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyOptionChain {
    pub underlying_symbol: String,
    pub expirations: Vec<TastyExpiration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyExpiration {
    pub expiration_date: String,
    pub strikes: Vec<TastyStrike>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyStrike {
    pub strike_price: f64,
    pub call_symbol: String,
    pub put_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyGreeks {
    pub delta: f64,
    pub gamma: f64,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
    pub implied_volatility: f64,
}

impl TastytradeBroker {
    /// Create a new tastytrade broker client.
    pub fn new(sandbox: bool) -> Self {
        let base_url = if sandbox { SANDBOX_BASE } else { API_BASE };
        Self {
            client: reqwest::Client::builder()
                .user_agent("TyphooN-Terminal/1.0")
                .build()
                .unwrap_or_default(),
            base_url: base_url.to_string(),
            session_token: None,
            account_number: None,
            last_accounts_error: None,
        }
    }

    /// Authenticate with username and password.
    pub async fn login(&mut self, username: &str, password: &str) -> Result<TastySession, String> {
        let url = format!("{}/sessions", self.base_url);
        let body = serde_json::json!({
            "login": username,
            "password": password,
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("tastytrade login failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            // Strip HTML from error responses (nginx 502/503 pages)
            let clean = if text.contains('<') {
                text.lines()
                    .filter(|l| !l.trim().starts_with('<'))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string()
            } else {
                text
            };
            let msg = if clean.is_empty() {
                status.to_string()
            } else {
                clean
            };
            return Err(format!("tastytrade login returned {} — {}", status, msg));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("tastytrade parse failed: {e}"))?;

        let session_token = data["data"]["session-token"]
            .as_str()
            .ok_or_else(|| "No session token in response".to_string())?
            .to_string();

        self.session_token = Some(session_token.clone());

        // Get accounts. Stash the error (if any) so the caller can include it in
        // a user-facing message rather than just reporting "no customer account
        // was returned" — which hid 401/404/empty-array distinctions and made
        // sandbox setup debugging painful.
        self.last_accounts_error = None;
        match self.get_accounts().await {
            Ok(accounts) => {
                if let Some(first) = accounts.first() {
                    self.account_number = Some(first.account_number.clone());
                } else {
                    self.last_accounts_error =
                        Some("API returned 200 with an empty accounts list".to_string());
                }
            }
            Err(e) => {
                self.last_accounts_error = Some(e);
            }
        }

        Ok(TastySession {
            session_token,
            remember_token: data["data"]["remember-token"]
                .as_str()
                .map(|s| s.to_string()),
        })
    }

    /// Helper: add auth header to request.
    fn auth_header(&self) -> Option<String> {
        self.session_token.clone()
    }

    /// Get all accounts for the authenticated user.
    pub async fn get_accounts(&self) -> Result<Vec<TastyAccount>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let url = format!("{}/customers/me/accounts", self.base_url);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("Get accounts failed: {e}"))?;

        // Check HTTP status before parsing — silently parsing a 401/404 response
        // hid the real reason a sandbox login appeared to succeed but produced
        // no account_number.
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("Read accounts body failed: {e}"))?;
        if !status.is_success() {
            return Err(format!(
                "tastytrade /customers/me/accounts returned {} — {}",
                status,
                body.chars().take(300).collect::<String>()
            ));
        }

        let data: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Parse accounts failed: {e}"))?;

        let items = data["data"]["items"]
            .as_array()
            .ok_or_else(|| "tastytrade accounts response missing data.items array".to_string())?;

        let mut accounts = Vec::new();
        for item in items {
            let acct = &item["account"];
            accounts.push(TastyAccount {
                account_number: acct["account-number"].as_str().unwrap_or("").to_string(),
                account_type: acct["account-type-name"].as_str().unwrap_or("").to_string(),
                nickname: acct["nickname"].as_str().map(|s| s.to_string()),
                margin_or_cash: acct["margin-or-cash"]
                    .as_str()
                    .unwrap_or("Cash")
                    .to_string(),
                is_closed: acct["is-closed"].as_bool().unwrap_or(false),
            });
        }
        Ok(accounts)
    }

    /// Get positions for the primary account.
    pub async fn get_positions(&self) -> Result<Vec<TastyPosition>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let acct = self.account_number.as_ref().ok_or("No account selected")?;
        let url = format!("{}/accounts/{}/positions", self.base_url, acct);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("Get positions failed: {e}"))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse positions failed: {e}"))?;

        let items = data["data"]["items"]
            .as_array()
            .ok_or_else(|| "No positions array".to_string())?;

        let mut positions = Vec::new();
        for item in items {
            positions.push(TastyPosition {
                symbol: item["symbol"].as_str().unwrap_or("").to_string(),
                instrument_type: item["instrument-type"].as_str().unwrap_or("").to_string(),
                quantity: item["quantity"].as_f64().unwrap_or(0.0),
                quantity_direction: item["quantity-direction"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                close_price: item["close-price"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                average_open_price: item["average-open-price"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                mark_price: item["mark-price"].as_str().and_then(|s| s.parse().ok()),
                unrealized_pnl: None, // Computed client-side
            });
        }
        Ok(positions)
    }

    /// Get orders for the primary account.
    pub async fn get_orders(&self, status: &str) -> Result<Vec<TastyOrder>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let acct = self.account_number.as_ref().ok_or("No account selected")?;
        let url = format!(
            "{}/accounts/{}/orders?status={}",
            self.base_url, acct, status
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("Get orders failed: {e}"))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse orders failed: {e}"))?;

        let items = data["data"]["items"]
            .as_array()
            .ok_or_else(|| "No orders array".to_string())?;

        let mut orders = Vec::new();
        for item in items {
            let mut legs = Vec::new();
            if let Some(leg_arr) = item["legs"].as_array() {
                for leg in leg_arr {
                    legs.push(TastyOrderLeg {
                        instrument_type: leg["instrument-type"].as_str().unwrap_or("").to_string(),
                        symbol: leg["symbol"].as_str().unwrap_or("").to_string(),
                        action: leg["action"].as_str().unwrap_or("").to_string(),
                        quantity: leg["quantity"].as_i64().unwrap_or(0),
                    });
                }
            }
            orders.push(TastyOrder {
                id: item["id"]
                    .as_str()
                    .or(item["id"].as_i64().map(|_| ""))
                    .unwrap_or("")
                    .to_string(),
                order_type: item["order-type"].as_str().unwrap_or("").to_string(),
                time_in_force: item["time-in-force"].as_str().unwrap_or("").to_string(),
                status: item["status"].as_str().unwrap_or("").to_string(),
                legs,
                price: item["price"].as_str().and_then(|s| s.parse().ok()),
                size: item["size"].as_i64().unwrap_or(0),
            });
        }
        Ok(orders)
    }

    fn order_status_is_live(status: &str) -> bool {
        !matches!(
            status.to_ascii_lowercase().as_str(),
            "filled" | "cancelled" | "canceled" | "rejected" | "expired" | "removed"
        )
    }

    fn collect_live_exit_order_ids_for_symbol(orders: &[TastyOrder], symbol: &str) -> Vec<String> {
        let mut ids = Vec::new();
        for order in orders {
            if !Self::order_status_is_live(&order.status) {
                continue;
            }
            if order.legs.iter().any(|leg| {
                leg.symbol.eq_ignore_ascii_case(symbol)
                    && leg.action.to_ascii_lowercase().contains("close")
            }) && !ids.contains(&order.id)
            {
                ids.push(order.id.clone());
            }
        }
        ids
    }

    /// Place an equity order.
    pub async fn place_equity_order(
        &self,
        symbol: &str,
        qty: i64,
        action: &str,
        order_type: &str,
        price: Option<f64>,
        tif: &str,
    ) -> Result<String, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let acct = self.account_number.as_ref().ok_or("No account selected")?;
        let url = format!("{}/accounts/{}/orders", self.base_url, acct);

        let mut order = serde_json::json!({
            "time-in-force": tif,
            "order-type": order_type,
            "legs": [{
                "instrument-type": "Equity",
                "symbol": symbol,
                "action": action,
                "quantity": qty,
            }],
        });
        if let Some(p) = price {
            order["price"] = serde_json::json!(format_equity_order_price(p));
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", &token)
            .header("Content-Type", "application/json")
            .json(&order)
            .send()
            .await
            .map_err(|e| format!("Place order failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Order rejected: {}", text));
        }

        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        Ok(data["data"]["order"]["id"]
            .as_str()
            .unwrap_or("ok")
            .to_string())
    }

    pub async fn cancel_live_exit_orders_for_symbol(&self, symbol: &str) -> Result<usize, String> {
        let orders = self.get_orders("Live").await?;
        let ids = Self::collect_live_exit_order_ids_for_symbol(&orders, symbol);
        for order_id in &ids {
            self.cancel_order(order_id).await.map_err(|e| {
                format!("Cancel live exit order {order_id} for {symbol} failed: {e}")
            })?;
        }
        Ok(ids.len())
    }

    pub async fn sync_equity_position_exits(
        &self,
        symbol: &str,
        sl_price: Option<f64>,
        tp_price: Option<f64>,
    ) -> Result<String, String> {
        let positions = self.get_positions().await?;
        let pos = positions
            .iter()
            .find(|p| p.symbol.eq_ignore_ascii_case(symbol))
            .ok_or_else(|| format!("No open tastytrade position for {symbol}"))?;
        let qty_abs = pos.quantity.abs().round() as i64;
        if qty_abs <= 0 {
            return Err(format!("Position {symbol} has zero quantity"));
        }
        let exit_action = if pos.quantity_direction.eq_ignore_ascii_case("Long") {
            "Sell to Close"
        } else {
            "Buy to Close"
        };

        let cancelled = self.cancel_live_exit_orders_for_symbol(symbol).await?;
        let mut placements = Vec::new();
        if let Some(sl) = sl_price {
            self.place_equity_order(symbol, qty_abs, exit_action, "Stop", Some(sl), "GTC")
                .await?;
            placements.push(format!("SL {}", format_equity_order_price(sl)));
        }
        if let Some(tp) = tp_price {
            self.place_equity_order(symbol, qty_abs, exit_action, "Limit", Some(tp), "GTC")
                .await?;
            placements.push(format!("TP {}", format_equity_order_price(tp)));
        }
        if placements.is_empty() {
            placements.push("cleared exits".to_string());
        } else if sl_price.is_some() && tp_price.is_some() {
            placements
                .push("broker-native link unavailable; exits placed independently".to_string());
        }

        Ok(format!(
            "{} for {} {} {} (cancelled {} existing exit order(s))",
            placements.join(", "),
            exit_action,
            qty_abs,
            symbol,
            cancelled
        ))
    }

    /// Cancel an open order by order ID.
    pub async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let acct = self.account_number.as_ref().ok_or("No account selected")?;
        let url = format!("{}/accounts/{}/orders/{}", self.base_url, acct, order_id);
        let resp = self
            .client
            .delete(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("Cancel order failed: {e}"))?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Cancel rejected: {text}"));
        }
        Ok(())
    }

    /// Close an open equity position at market.
    /// Looks up the current position quantity and submits a market order
    /// in the opposite direction for the full size.
    ///
    /// tastytrade does not expose a dedicated "close position" endpoint —
    /// closure is a regular order in the opposite direction. We wrap the
    /// lookup + order submission here so callers don't have to replicate it.
    pub async fn close_equity_position(&self, symbol: &str) -> Result<String, String> {
        let positions = self.get_positions().await?;
        let pos = positions
            .iter()
            .find(|p| p.symbol.eq_ignore_ascii_case(symbol))
            .ok_or_else(|| format!("No open tastytrade position for {symbol}"))?;

        // Determine closing direction from the position's quantity_direction field.
        // Long  → Sell to Close.  Short → Buy to Close.
        let qty_abs = pos.quantity.abs() as i64;
        if qty_abs == 0 {
            return Err(format!("Position {symbol} has zero quantity"));
        }
        self.close_equity_position_qty(symbol, qty_abs).await
    }

    pub async fn close_equity_position_qty(
        &self,
        symbol: &str,
        qty: i64,
    ) -> Result<String, String> {
        let positions = self.get_positions().await?;
        let pos = positions
            .iter()
            .find(|p| p.symbol.eq_ignore_ascii_case(symbol))
            .ok_or_else(|| format!("No open tastytrade position for {symbol}"))?;

        let qty_abs = pos.quantity.abs().round() as i64;
        if qty_abs == 0 {
            return Err(format!("Position {symbol} has zero quantity"));
        }
        let close_qty = qty.max(0).min(qty_abs);
        if close_qty == 0 {
            return Err(format!("Requested close size for {symbol} is zero"));
        }
        let action = if pos.quantity_direction.eq_ignore_ascii_case("Long") {
            "Sell to Close"
        } else {
            "Buy to Close"
        };
        let _ = self.cancel_live_exit_orders_for_symbol(symbol).await;
        self.place_equity_order(symbol, close_qty, action, "Market", None, "Day")
            .await
    }

    pub async fn close_all_equity_positions(&self) -> Result<usize, String> {
        let positions = self.get_positions().await?;
        let mut closed = 0usize;
        for pos in positions.iter().filter(|p| p.quantity.abs() > 0.0) {
            self.close_equity_position(&pos.symbol).await?;
            closed += 1;
        }
        Ok(closed)
    }

    /// Search symbols by query string.
    pub async fn search_symbols(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let url = format!("{}/symbols/search/{}", self.base_url, query);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("tastytrade search failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("tastytrade search: HTTP {}", resp.status()));
        }
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("tastytrade search parse: {e}"))?;
        Ok(json["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// Fetch symbols from tastytrade's public watchlists.
    pub async fn get_public_watchlist_symbols(&self) -> Result<Vec<String>, String> {
        let url = format!("{}/public-watchlists", self.base_url);
        let mut req = self.client.get(&url);
        if let Some(token) = self.auth_header() {
            req = req.header("Authorization", token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("tastytrade public watchlists failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!(
                "tastytrade public watchlists: HTTP {}",
                resp.status()
            ));
        }
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("tastytrade public watchlists parse: {e}"))?;
        let mut symbols = std::collections::BTreeSet::new();
        for watchlist in json["data"]["items"].as_array().into_iter().flatten() {
            for entry in watchlist["watchlist-entries"]
                .as_array()
                .into_iter()
                .flatten()
            {
                if let Some(symbol) = entry["symbol"].as_str() {
                    let symbol = symbol.trim().to_ascii_uppercase();
                    if !symbol.is_empty() {
                        symbols.insert(symbol);
                    }
                }
            }
        }
        Ok(symbols.into_iter().collect())
    }

    /// Fetch all active futures symbols tastytrade currently exposes.
    pub async fn get_active_futures_symbols(&self) -> Result<Vec<String>, String> {
        let token = self
            .auth_header()
            .ok_or_else(|| "tastytrade futures instruments: not authenticated".to_string())?;
        let url = format!("{}/instruments/futures", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("tastytrade futures instruments failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!(
                "tastytrade futures instruments: HTTP {}",
                resp.status()
            ));
        }
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("tastytrade futures instruments parse: {e}"))?;
        let mut symbols = std::collections::BTreeSet::new();
        for item in json["data"]["items"].as_array().into_iter().flatten() {
            let active = item["active"].as_bool().unwrap_or(false);
            if !active {
                continue;
            }
            if let Some(symbol) = item["symbol"].as_str() {
                let symbol = symbol.trim().to_ascii_uppercase();
                if !symbol.is_empty() {
                    symbols.insert(symbol);
                }
            }
        }
        Ok(symbols.into_iter().collect())
    }

    /// Best-effort tastytrade market-data universe currently available from
    /// the public API: public watchlists plus all active futures contracts.
    pub async fn get_market_data_universe_symbols(&self) -> Result<Vec<String>, String> {
        merge_market_data_universe_sources(
            self.get_public_watchlist_symbols().await,
            self.get_active_futures_symbols().await,
        )
    }

    /// Get option chain for a symbol.
    pub async fn get_option_chain(&self, symbol: &str) -> Result<Vec<TastyExpiration>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let url = format!("{}/option-chains/{}/nested", self.base_url, symbol);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("Get option chain failed: {e}"))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse chain failed: {e}"))?;

        let items = data["data"]["items"]
            .as_array()
            .ok_or_else(|| "No chain data".to_string())?;

        let mut expirations = Vec::new();
        for item in items {
            if let Some(exp_arr) = item["expirations"].as_array() {
                for exp in exp_arr {
                    let mut strikes = Vec::new();
                    if let Some(strike_arr) = exp["strikes"].as_array() {
                        for s in strike_arr {
                            strikes.push(TastyStrike {
                                strike_price: s["strike-price"]
                                    .as_str()
                                    .and_then(|p| p.parse().ok())
                                    .unwrap_or(0.0),
                                call_symbol: s["call"].as_str().unwrap_or("").to_string(),
                                put_symbol: s["put"].as_str().unwrap_or("").to_string(),
                            });
                        }
                    }
                    expirations.push(TastyExpiration {
                        expiration_date: exp["expiration-date"].as_str().unwrap_or("").to_string(),
                        strikes,
                    });
                }
            }
        }
        Ok(expirations)
    }

    /// Get quote snapshots for equity symbols (bid/ask/last/volume/daily OHLC).
    /// Uses the /api/quote/equities endpoint (REST, no WebSocket needed).
    /// Returns up to 100 symbols per request.
    pub async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<TastyQuote>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let sym_param = symbols.join(",");
        let url = format!("{}/market-data?symbols={}", self.base_url, sym_param);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("Get quotes failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Quotes request failed: {text}"));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse quotes failed: {e}"))?;

        let empty = vec![];
        let items = data["data"]["items"].as_array().unwrap_or(&empty);

        let mut quotes = Vec::new();
        for item in items {
            quotes.push(TastyQuote {
                symbol: item["symbol"].as_str().unwrap_or("").to_string(),
                bid: parse_num(&item["bid"]),
                ask: parse_num(&item["ask"]),
                last: parse_num(&item["last"]),
                open: parse_num(&item["open"]),
                high: parse_num(&item["high"]),
                low: parse_num(&item["low"]),
                close: parse_num(&item["close"]),
                prev_close: parse_num(&item["prev-close"]),
                volume: item["volume"].as_i64().unwrap_or(0),
                bid_size: item["bid-size"].as_i64().unwrap_or(0),
                ask_size: item["ask-size"].as_i64().unwrap_or(0),
            });
        }
        Ok(quotes)
    }

    /// Get market metrics (IV rank, IV percentile, liquidity) for symbols.
    pub async fn get_market_metrics(
        &self,
        symbols: &[String],
    ) -> Result<Vec<TastyMarketMetric>, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let sym_param = symbols.join(",");
        let url = format!("{}/market-metrics?symbols={}", self.base_url, sym_param);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("Get metrics failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Metrics request failed: {text}"));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse metrics failed: {e}"))?;

        let empty = vec![];
        let items = data["data"]["items"].as_array().unwrap_or(&empty);

        let mut metrics = Vec::new();
        for item in items {
            metrics.push(TastyMarketMetric {
                symbol: item["symbol"].as_str().unwrap_or("").to_string(),
                iv_index: parse_num(&item["implied-volatility-index"]),
                iv_rank: parse_num(&item["implied-volatility-index-rank"]),
                iv_percentile: parse_num(&item["implied-volatility-percentile"]),
                liquidity_rating: item["liquidity-rating"].as_i64().unwrap_or(0) as i32,
                liquidity_rank: parse_num(&item["liquidity-rank"]),
                beta: parse_num(&item["beta"]),
                earnings_date: item["earnings"]["expected-report-date"]
                    .as_str()
                    .map(|s| s.to_string()),
            });
        }
        Ok(metrics)
    }

    /// Get account balances.
    pub async fn get_balances(&self) -> Result<TastyBalances, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        let acct = self.account_number.as_ref().ok_or("No account selected")?;
        let url = format!("{}/accounts/{}/balances", self.base_url, acct);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| format!("Get balances failed: {e}"))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse balances failed: {e}"))?;

        let b = &data["data"];
        Ok(TastyBalances {
            cash_balance: parse_num(&b["cash-balance"]),
            net_liquidating_value: parse_num(&b["net-liquidating-value"]),
            equity_buying_power: parse_num(&b["equity-buying-power"]),
            maintenance_requirement: parse_num(&b["maintenance-requirement"]),
            pending_cash: parse_num(&b["pending-cash"]),
        })
    }

    /// Get the DXLink streaming token for WebSocket market data.
    pub async fn get_streaming_token(&self) -> Result<super::dxlink::DxLinkToken, String> {
        let token = self.auth_header().ok_or("Not authenticated")?;
        super::dxlink::get_streaming_token(&self.base_url, &token).await
    }

    pub fn is_authenticated(&self) -> bool {
        self.session_token.is_some()
    }
    pub fn account_number(&self) -> Option<&str> {
        self.account_number.as_deref()
    }

    /// Diagnostic from the post-login `get_accounts()` call. Populated when login
    /// succeeded (session token issued) but no `account_number` could be picked,
    /// so the caller can build an error that points at the real cause.
    pub fn last_accounts_error(&self) -> Option<&str> {
        self.last_accounts_error.as_deref()
    }
}

/// Quote snapshot from tastytrade REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyQuote {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub prev_close: f64,
    pub volume: i64,
    pub bid_size: i64,
    pub ask_size: i64,
}

/// Market metrics (IV, liquidity) from tastytrade REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyMarketMetric {
    pub symbol: String,
    pub iv_index: f64,
    pub iv_rank: f64,
    pub iv_percentile: f64,
    pub liquidity_rating: i32,
    pub liquidity_rank: f64,
    pub beta: f64,
    pub earnings_date: Option<String>,
}

/// Account balances from tastytrade REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyBalances {
    pub cash_balance: f64,
    pub net_liquidating_value: f64,
    pub equity_buying_power: f64,
    pub maintenance_requirement: f64,
    pub pending_cash: f64,
}

/// Helper: parse number from tastytrade JSON (may be string or number).
fn parse_num(v: &serde_json::Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasty_session_construction() {
        let session = TastySession {
            session_token: "tok_abc123".to_string(),
            remember_token: Some("rem_xyz".to_string()),
        };
        assert_eq!(session.session_token, "tok_abc123");
        assert_eq!(session.remember_token.as_deref(), Some("rem_xyz"));
    }

    #[test]
    fn tasty_session_no_remember_token() {
        let session = TastySession {
            session_token: "tok_only".to_string(),
            remember_token: None,
        };
        assert_eq!(session.session_token, "tok_only");
        assert!(session.remember_token.is_none());
    }

    #[test]
    fn tasty_account_construction() {
        let acct = TastyAccount {
            account_number: "5YZ12345".to_string(),
            account_type: "Individual".to_string(),
            nickname: Some("Main".to_string()),
            margin_or_cash: "Margin".to_string(),
            is_closed: false,
        };
        assert_eq!(acct.account_number, "5YZ12345");
        assert_eq!(acct.account_type, "Individual");
        assert_eq!(acct.nickname.as_deref(), Some("Main"));
        assert_eq!(acct.margin_or_cash, "Margin");
        assert!(!acct.is_closed);
    }

    #[test]
    fn tasty_account_closed() {
        let acct = TastyAccount {
            account_number: "CLOSED1".to_string(),
            account_type: "Entity".to_string(),
            nickname: None,
            margin_or_cash: "Cash".to_string(),
            is_closed: true,
        };
        assert!(acct.is_closed);
        assert!(acct.nickname.is_none());
    }

    #[test]
    fn tasty_position_long() {
        let pos = TastyPosition {
            symbol: "AAPL".to_string(),
            instrument_type: "Equity".to_string(),
            quantity: 100.0,
            quantity_direction: "Long".to_string(),
            close_price: 175.50,
            average_open_price: 150.00,
            mark_price: Some(176.00),
            unrealized_pnl: Some(2600.0),
        };
        assert_eq!(pos.symbol, "AAPL");
        assert_eq!(pos.instrument_type, "Equity");
        assert!((pos.quantity - 100.0).abs() < f64::EPSILON);
        assert_eq!(pos.quantity_direction, "Long");
        assert!((pos.close_price - 175.50).abs() < f64::EPSILON);
        assert!((pos.average_open_price - 150.0).abs() < f64::EPSILON);
        assert_eq!(pos.mark_price, Some(176.00));
        assert_eq!(pos.unrealized_pnl, Some(2600.0));
    }

    #[test]
    fn tasty_position_short_no_mark() {
        let pos = TastyPosition {
            symbol: "SPY".to_string(),
            instrument_type: "Equity Option".to_string(),
            quantity: 5.0,
            quantity_direction: "Short".to_string(),
            close_price: 3.20,
            average_open_price: 4.50,
            mark_price: None,
            unrealized_pnl: None,
        };
        assert_eq!(pos.quantity_direction, "Short");
        assert!(pos.mark_price.is_none());
        assert!(pos.unrealized_pnl.is_none());
    }

    #[test]
    fn format_equity_order_price_scales_for_penny_names() {
        assert_eq!(format_equity_order_price(12.3456), "12.35");
        assert_eq!(format_equity_order_price(0.123456), "0.1235");
        assert_eq!(format_equity_order_price(0.000321), "0.000321");
    }

    #[test]
    fn sandbox_host_matches_official_docs() {
        let broker = TastytradeBroker::new(true);
        assert_eq!(broker.base_url, "https://api.cert.tastyworks.com");
    }

    #[test]
    fn production_host_matches_official_sdk() {
        let broker = TastytradeBroker::new(false);
        assert_eq!(broker.base_url, "https://api.tastyworks.com");
    }

    #[test]
    fn merge_market_data_universe_sources_keeps_partial_success() {
        let merged = merge_market_data_universe_sources(
            Err("HTTP 502".into()),
            Ok(vec!["/ESM6".into(), "/NQM6".into(), "/esm6".into()]),
        )
        .unwrap();
        assert_eq!(merged, vec!["/ESM6".to_string(), "/NQM6".to_string()]);
    }

    #[test]
    fn merge_market_data_universe_sources_reports_total_failure() {
        let err =
            merge_market_data_universe_sources(Err("HTTP 502".into()), Err("HTTP 503".into()))
                .unwrap_err();
        assert!(err.contains("public watchlists"));
        assert!(err.contains("active futures"));
    }

    #[test]
    fn collect_live_exit_order_ids_for_symbol_keeps_close_orders_only() {
        let orders = vec![
            TastyOrder {
                id: "exit-1".to_string(),
                order_type: "Limit".to_string(),
                time_in_force: "GTC".to_string(),
                status: "Live".to_string(),
                legs: vec![TastyOrderLeg {
                    instrument_type: "Equity".to_string(),
                    symbol: "AAPL".to_string(),
                    action: "Sell to Close".to_string(),
                    quantity: 10,
                }],
                price: Some(220.0),
                size: 10,
            },
            TastyOrder {
                id: "entry-1".to_string(),
                order_type: "Limit".to_string(),
                time_in_force: "GTC".to_string(),
                status: "Live".to_string(),
                legs: vec![TastyOrderLeg {
                    instrument_type: "Equity".to_string(),
                    symbol: "AAPL".to_string(),
                    action: "Buy to Open".to_string(),
                    quantity: 10,
                }],
                price: Some(200.0),
                size: 10,
            },
        ];

        let ids = TastytradeBroker::collect_live_exit_order_ids_for_symbol(&orders, "AAPL");
        assert_eq!(ids, vec!["exit-1".to_string()]);
    }

    #[test]
    fn tasty_order_with_legs() {
        let leg = TastyOrderLeg {
            instrument_type: "Equity".to_string(),
            symbol: "MSFT".to_string(),
            action: "Buy to Open".to_string(),
            quantity: 10,
        };
        let order = TastyOrder {
            id: "ORD-12345".to_string(),
            order_type: "Limit".to_string(),
            time_in_force: "Day".to_string(),
            status: "Received".to_string(),
            legs: vec![leg],
            price: Some(350.00),
            size: 10,
        };
        assert_eq!(order.id, "ORD-12345");
        assert_eq!(order.order_type, "Limit");
        assert_eq!(order.legs.len(), 1);
        assert_eq!(order.legs[0].symbol, "MSFT");
        assert_eq!(order.legs[0].action, "Buy to Open");
        assert_eq!(order.legs[0].quantity, 10);
        assert_eq!(order.price, Some(350.00));
    }

    #[test]
    fn tasty_order_no_price() {
        let order = TastyOrder {
            id: "ORD-99".to_string(),
            order_type: "Market".to_string(),
            time_in_force: "GTC".to_string(),
            status: "Filled".to_string(),
            legs: vec![],
            price: None,
            size: 50,
        };
        assert!(order.price.is_none());
        assert!(order.legs.is_empty());
        assert_eq!(order.size, 50);
    }

    #[test]
    fn tasty_greeks_construction() {
        let greeks = TastyGreeks {
            delta: 0.45,
            gamma: 0.03,
            theta: -0.12,
            vega: 0.25,
            rho: 0.01,
            implied_volatility: 0.32,
        };
        assert!((greeks.delta - 0.45).abs() < f64::EPSILON);
        assert!((greeks.gamma - 0.03).abs() < f64::EPSILON);
        assert!((greeks.theta - (-0.12)).abs() < f64::EPSILON);
        assert!((greeks.vega - 0.25).abs() < f64::EPSILON);
        assert!((greeks.rho - 0.01).abs() < f64::EPSILON);
        assert!((greeks.implied_volatility - 0.32).abs() < f64::EPSILON);
    }

    #[test]
    fn tasty_greeks_zero_values() {
        let greeks = TastyGreeks {
            delta: 0.0,
            gamma: 0.0,
            theta: 0.0,
            vega: 0.0,
            rho: 0.0,
            implied_volatility: 0.0,
        };
        assert!((greeks.delta).abs() < f64::EPSILON);
        assert!((greeks.gamma).abs() < f64::EPSILON);
        assert!((greeks.theta).abs() < f64::EPSILON);
        assert!((greeks.vega).abs() < f64::EPSILON);
        assert!((greeks.rho).abs() < f64::EPSILON);
        assert!((greeks.implied_volatility).abs() < f64::EPSILON);
    }

    #[test]
    fn tasty_expiration_empty_strikes() {
        let exp = TastyExpiration {
            expiration_date: "2026-04-18".to_string(),
            strikes: vec![],
        };
        assert_eq!(exp.expiration_date, "2026-04-18");
        assert!(exp.strikes.is_empty());
    }

    #[test]
    fn tasty_expiration_with_strikes() {
        let strike = TastyStrike {
            strike_price: 150.0,
            call_symbol: "AAPL  260418C00150000".to_string(),
            put_symbol: "AAPL  260418P00150000".to_string(),
        };
        let exp = TastyExpiration {
            expiration_date: "2026-04-18".to_string(),
            strikes: vec![strike],
        };
        assert_eq!(exp.strikes.len(), 1);
        assert!((exp.strikes[0].strike_price - 150.0).abs() < f64::EPSILON);
        assert!(exp.strikes[0].call_symbol.contains('C'));
        assert!(exp.strikes[0].put_symbol.contains('P'));
    }

    #[test]
    fn tasty_option_chain_construction() {
        let chain = TastyOptionChain {
            underlying_symbol: "AAPL".to_string(),
            expirations: vec![TastyExpiration {
                expiration_date: "2026-04-18".to_string(),
                strikes: vec![],
            }],
        };
        assert_eq!(chain.underlying_symbol, "AAPL");
        assert_eq!(chain.expirations.len(), 1);
    }

    #[test]
    fn tasty_quote_construction() {
        let quote = TastyQuote {
            symbol: "NVDA".to_string(),
            bid: 880.0,
            ask: 881.0,
            last: 880.50,
            open: 875.0,
            high: 890.0,
            low: 870.0,
            close: 880.50,
            prev_close: 872.0,
            volume: 50_000_000,
            bid_size: 100,
            ask_size: 200,
        };
        assert_eq!(quote.symbol, "NVDA");
        assert!((quote.bid - 880.0).abs() < f64::EPSILON);
        assert_eq!(quote.volume, 50_000_000);
    }

    #[test]
    fn tasty_balances_construction() {
        let bal = TastyBalances {
            cash_balance: 10_000.0,
            net_liquidating_value: 50_000.0,
            equity_buying_power: 100_000.0,
            maintenance_requirement: 25_000.0,
            pending_cash: 0.0,
        };
        assert!((bal.cash_balance - 10_000.0).abs() < f64::EPSILON);
        assert!((bal.net_liquidating_value - 50_000.0).abs() < f64::EPSILON);
        assert!((bal.equity_buying_power - 100_000.0).abs() < f64::EPSILON);
        assert!((bal.maintenance_requirement - 25_000.0).abs() < f64::EPSILON);
        assert!((bal.pending_cash).abs() < f64::EPSILON);
    }

    #[test]
    fn tasty_market_metric_construction() {
        let metric = TastyMarketMetric {
            symbol: "SPY".to_string(),
            iv_index: 0.18,
            iv_rank: 0.35,
            iv_percentile: 0.42,
            liquidity_rating: 5,
            liquidity_rank: 0.99,
            beta: 1.0,
            earnings_date: None,
        };
        assert_eq!(metric.symbol, "SPY");
        assert_eq!(metric.liquidity_rating, 5);
        assert!(metric.earnings_date.is_none());
    }

    #[test]
    fn tastytrade_broker_new_production() {
        let broker = TastytradeBroker::new(false);
        assert!(!broker.is_authenticated());
        assert!(broker.account_number().is_none());
    }

    #[test]
    fn tastytrade_broker_new_sandbox() {
        let broker = TastytradeBroker::new(true);
        assert!(!broker.is_authenticated());
        assert!(broker.account_number().is_none());
    }

    #[test]
    fn parse_num_from_number() {
        let v = serde_json::json!(42.5);
        assert!((parse_num(&v) - 42.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_num_from_string() {
        let v = serde_json::json!("99.9");
        assert!((parse_num(&v) - 99.9).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_num_from_null() {
        let v = serde_json::json!(null);
        assert!((parse_num(&v)).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_num_from_invalid_string() {
        let v = serde_json::json!("not_a_number");
        assert!((parse_num(&v)).abs() < f64::EPSILON);
    }
}
