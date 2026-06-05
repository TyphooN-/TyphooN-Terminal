use super::*;

impl TyphooNApp {
    pub(super) fn drain_web_client_commands(&mut self, ctx: &egui::Context) {
        // ── drain web client commands ────────────────────────────────────
        if let Some(ref mut rx) = self.web_cmd_rx {
            let mut web_cmds_drained = 0usize;
            const WEB_CMD_DRAIN_MAX: usize = 64;
            while web_cmds_drained < WEB_CMD_DRAIN_MAX {
                let Ok(cmd) = rx.try_recv() else {
                    break;
                };
                web_cmds_drained += 1;
                match cmd {
                    typhoon_web_protocol::WebCmd::GetAccount => {
                        if self.alpaca_enabled {
                            let _ = self.broker_tx.send(BrokerCmd::GetAccount);
                        }
                    }
                    typhoon_web_protocol::WebCmd::GetPositions => {
                        if self.alpaca_enabled {
                            let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                        }
                    }
                    typhoon_web_protocol::WebCmd::GetOrders => {
                        if self.alpaca_enabled {
                            let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                        }
                    }
                    typhoon_web_protocol::WebCmd::GetWatchlistQuotes { symbols } => {
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::GetWatchlistQuotes { symbols });
                    }
                    typhoon_web_protocol::WebCmd::GetMarketClock => {
                        if self.alpaca_enabled {
                            let _ = self.broker_tx.send(BrokerCmd::GetMarketClock);
                        }
                    }
                    typhoon_web_protocol::WebCmd::GetBars { symbol, timeframe } => {
                        // Read bars directly from cache and broadcast
                        if let Some(ref cache) = self.cache {
                            let key = format!("mt5:{}:{}", symbol, timeframe);
                            if let Ok(Some(data)) = cache.get_bars_raw(&key) {
                                let bars: Vec<typhoon_web_protocol::BarData> = data
                                    .iter()
                                    .map(|b| typhoon_web_protocol::BarData {
                                        timestamp: b.0,
                                        open: b.1,
                                        high: b.2,
                                        low: b.3,
                                        close: b.4,
                                        volume: b.5,
                                    })
                                    .collect();
                                if let Some(ref tx) = self.web_msg_tx {
                                    let _ = tx.send(typhoon_web_protocol::WebMsg::Bars {
                                        symbol,
                                        timeframe,
                                        bars,
                                    });
                                }
                            }
                        }
                    }
                    typhoon_web_protocol::WebCmd::Ping => {
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(typhoon_web_protocol::WebMsg::Pong);
                        }
                    }
                    typhoon_web_protocol::WebCmd::Auth { .. } => {
                        // Auth is handled by web-server before relay — ignore here
                    }
                    // ── Phase 2: order entry from phone ──
                    // Server-side validation already happened in web-server.
                    // We still confirm the broker selection matches a connected broker,
                    // translate to the native BrokerCmd, and reply via the broadcast channel.
                    typhoon_web_protocol::WebCmd::PlaceOrder {
                        symbol,
                        qty,
                        side,
                        order_type,
                        limit_price,
                        stop_price,
                        take_profit,
                        stop_loss,
                        broker,
                        ..
                    } => {
                        let lower_side = side.to_ascii_lowercase();
                        let lower_type = order_type.trim().replace('-', "_").to_ascii_lowercase();
                        let broker_key = broker.to_ascii_lowercase();
                        let reply = match broker_key.as_str() {
                            "alpaca" => {
                                if !self.alpaca_enabled {
                                    typhoon_web_protocol::WebMsg::OrderResult {
                                        ok: false,
                                        message: "Alpaca broker is disabled on host".into(),
                                    }
                                } else if !self.broker_connected {
                                    typhoon_web_protocol::WebMsg::OrderResult {
                                        ok: false,
                                        message: "Alpaca broker not connected on host".into(),
                                    }
                                } else {
                                    // Dispatch based on order_type
                                    match lower_type.as_str() {
                                        "market" => {
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::AlpacaMarketOrder {
                                                    symbol: symbol.clone(),
                                                    qty,
                                                    side: lower_side.clone(),
                                                });
                                        }
                                        "limit" => {
                                            let lp = limit_price.unwrap_or(0.0);
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::AlpacaLimitOrder {
                                                    symbol: symbol.clone(),
                                                    qty,
                                                    side: lower_side.clone(),
                                                    limit_price: lp,
                                                });
                                        }
                                        "stop" => {
                                            let sp = stop_price.unwrap_or(0.0);
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::AlpacaStopOrder {
                                                    symbol: symbol.clone(),
                                                    qty,
                                                    side: lower_side.clone(),
                                                    stop_price: sp,
                                                });
                                        }
                                        _ => {}
                                    }
                                    typhoon_web_protocol::WebMsg::OrderResult {
                                        ok: true,
                                        message: format!(
                                            "{} {} {} {} dispatched to Alpaca",
                                            lower_side, qty, symbol, lower_type
                                        ),
                                    }
                                }
                            }
                            "tastytrade" => {
                                let _ = self.broker_tx.send(BrokerCmd::TastytradeEquityOrder {
                                    symbol: symbol.clone(),
                                    qty: qty as i64,
                                    side: if lower_side == "buy" {
                                        "Buy to Open"
                                    } else {
                                        "Sell to Open"
                                    }
                                    .into(),
                                    order_type: if lower_type == "market" {
                                        "Market"
                                    } else {
                                        "Limit"
                                    }
                                    .into(),
                                    price: limit_price,
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!(
                                        "{} {} {} {} dispatched to Tastytrade",
                                        lower_side, qty, symbol, lower_type
                                    ),
                                }
                            }
                            "kraken" => {
                                if !self.kraken_enabled {
                                    typhoon_web_protocol::WebMsg::OrderResult {
                                        ok: false,
                                        message: "Kraken broker is disabled on host".into(),
                                    }
                                } else if !self.kraken_connected {
                                    typhoon_web_protocol::WebMsg::OrderResult {
                                        ok: false,
                                        message: "Kraken broker not connected on host".into(),
                                    }
                                } else {
                                    let kraken_type = match lower_type.as_str() {
                                        "market" => "market",
                                        "limit" => "limit",
                                        "stop" | "stoploss" | "stop_loss" => "stop-loss",
                                        "stoplimit" | "stop_limit" | "stoploss_limit"
                                        | "stop_loss_limit" => "stop-loss-limit",
                                        "takeprofit" | "take_profit" => "take-profit",
                                        "takeprofit_limit" | "take_profit_limit" => {
                                            "take-profit-limit"
                                        }
                                        "trailingstop" | "trailing_stop" => "trailing-stop",
                                        "trailingstop_limit" | "trailing_stop_limit" => {
                                            "trailing-stop-limit"
                                        }
                                        "iceberg" => "iceberg",
                                        "settle_position" => "settle-position",
                                        _ => lower_type.as_str(),
                                    };
                                    let mut order =
                                        typhoon_engine::broker::kraken::KrakenOrderRequest::basic(
                                            symbol.clone(),
                                            lower_side.clone(),
                                            kraken_type,
                                            qty,
                                        );
                                    let primary_price = match kraken_type {
                                        "limit" | "iceberg" => limit_price,
                                        "stop-loss" | "take-profit" | "trailing-stop" => {
                                            stop_price.or(limit_price)
                                        }
                                        "stop-loss-limit"
                                        | "take-profit-limit"
                                        | "trailing-stop-limit" => stop_price,
                                        _ => None,
                                    };
                                    if let Some(price) = primary_price {
                                        order.price = Some(price.to_string());
                                    }
                                    if matches!(
                                        kraken_type,
                                        "stop-loss-limit"
                                            | "take-profit-limit"
                                            | "trailing-stop-limit"
                                    ) && let Some(price2) = limit_price
                                    {
                                        order.price2 = Some(price2.to_string());
                                    }
                                    match order.validate() {
                                        Ok(()) => {
                                            let _ = self.broker_tx.send(
                                                BrokerCmd::KrakenPlaceOrderAdvanced { order },
                                            );
                                            if stop_loss.is_some() || take_profit.is_some() {
                                                let _ = self.broker_tx.send(
                                                    BrokerCmd::KrakenSyncExits {
                                                        pair: symbol.clone(),
                                                        sl_price: stop_loss,
                                                        tp_price: take_profit,
                                                        wait_for_position: true,
                                                        wait_for_qty_at_most: None,
                                                    },
                                                );
                                            }
                                            typhoon_web_protocol::WebMsg::OrderResult {
                                                ok: true,
                                                message: format!(
                                                    "{} {} {} {} dispatched to Kraken",
                                                    lower_side, qty, symbol, kraken_type
                                                ),
                                            }
                                        }
                                        Err(e) => typhoon_web_protocol::WebMsg::OrderResult {
                                            ok: false,
                                            message: format!("Kraken order rejected locally: {e}"),
                                        },
                                    }
                                }
                            }
                            other => typhoon_web_protocol::WebMsg::OrderResult {
                                ok: false,
                                message: format!("Unknown broker: {other}"),
                            },
                        };
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(reply);
                        }
                        // Mirror to local log so the host operator sees web-originated orders.
                        self.log.push_back(LogEntry::info(format!(
                            "Web order: {} {} {} {} via {}",
                            side, qty, symbol, order_type, broker
                        )));
                    }
                    typhoon_web_protocol::WebCmd::CancelOrder { order_id, broker } => {
                        let broker_key = broker.to_ascii_lowercase();
                        let reply = match broker_key.as_str() {
                            "alpaca" => {
                                let _ = self.broker_tx.send(BrokerCmd::AlpacaCancelOrder {
                                    order_id: order_id.clone(),
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Cancel {} dispatched to Alpaca", order_id),
                                }
                            }
                            "tastytrade" => {
                                let _ = self.broker_tx.send(BrokerCmd::TastytradeCancelOrder {
                                    order_id: order_id.clone(),
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!(
                                        "Cancel {} dispatched to tastytrade",
                                        order_id
                                    ),
                                }
                            }
                            "kraken" => {
                                let _ = self.broker_tx.send(BrokerCmd::KrakenCancelOrder {
                                    txid: order_id.clone(),
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Cancel {} dispatched to Kraken", order_id),
                                }
                            }
                            other => typhoon_web_protocol::WebMsg::OrderResult {
                                ok: false,
                                message: format!("Unknown broker: {other}"),
                            },
                        };
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(reply);
                        }
                        self.log.push_back(LogEntry::info(format!(
                            "Web cancel: {} via {}",
                            order_id, broker
                        )));
                    }
                    typhoon_web_protocol::WebCmd::ClosePosition { symbol, broker } => {
                        let broker_key = broker.to_ascii_lowercase();
                        let reply = match broker_key.as_str() {
                            "alpaca" => {
                                let _ = self.broker_tx.send(BrokerCmd::ClosePosition {
                                    symbol: symbol.clone(),
                                    qty: None,
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Close {} dispatched to Alpaca", symbol),
                                }
                            }
                            "tastytrade" => {
                                let _ =
                                    self.broker_tx.send(BrokerCmd::TastytradeClosePositionQty {
                                        symbol: symbol.clone(),
                                        qty: None,
                                    });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Close {} dispatched to Tastytrade", symbol),
                                }
                            }
                            "kraken" => {
                                let _ = self.broker_tx.send(BrokerCmd::KrakenClosePosition {
                                    pair: symbol.clone(),
                                    volume: None,
                                });
                                typhoon_web_protocol::WebMsg::OrderResult {
                                    ok: true,
                                    message: format!("Close {} dispatched to Kraken", symbol),
                                }
                            }
                            other => typhoon_web_protocol::WebMsg::OrderResult {
                                ok: false,
                                message: format!("Unknown broker: {other}"),
                            },
                        };
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(reply);
                        }
                        self.log.push_back(LogEntry::info(format!(
                            "Web close: {} via {}",
                            symbol, broker
                        )));
                    }
                    // ── ADR-092: new WebCmd handlers ──
                    typhoon_web_protocol::WebCmd::GetIndicators {
                        symbol,
                        timeframe,
                        indicators,
                    } => {
                        self.log.push_back(LogEntry::info(format!(
                            "Web indicator request: {symbol} {timeframe} {:?}",
                            indicators
                        )));
                        // Send indicator data from current chart cache
                        // Full GPU dispatch integration will be wired in GPU compute task
                        for name in &indicators {
                            if let Some(ref tx) = self.web_msg_tx {
                                let _ = tx.send(typhoon_web_protocol::WebMsg::IndicatorData {
                                    symbol: symbol.clone(),
                                    timeframe: timeframe.clone(),
                                    name: name.clone(),
                                    values: Vec::new(),
                                });
                            }
                        }
                    }
                    typhoon_web_protocol::WebCmd::CreateAlert {
                        symbol,
                        condition: _,
                        price,
                        message,
                    } => {
                        self.log
                            .push_back(LogEntry::info(format!("Web alert: {symbol} @ {price}")));
                        let label = if message.is_empty() { symbol } else { message };
                        self.alerts.push((price, label));
                    }
                    typhoon_web_protocol::WebCmd::DeleteAlert { alert_id } => {
                        // alert_id is index-based from web: "web-N"
                        if let Some(idx) = alert_id
                            .strip_prefix("web-")
                            .and_then(|s| s.parse::<usize>().ok())
                        {
                            if idx < self.alerts.len() {
                                self.alerts.remove(idx);
                            }
                        }
                    }
                    typhoon_web_protocol::WebCmd::ListAlerts => {
                        if let Some(ref tx) = self.web_msg_tx {
                            let _ = tx.send(typhoon_web_protocol::WebMsg::AlertList {
                                items: self
                                    .alerts
                                    .iter()
                                    .enumerate()
                                    .map(|(i, (price, label))| {
                                        typhoon_web_protocol::AlertSnapshot {
                                            id: format!("web-{i}"),
                                            symbol: label.clone(),
                                            condition: "reaches".into(),
                                            price: *price,
                                            message: label.clone(),
                                            active: true,
                                        }
                                    })
                                    .collect(),
                            });
                        }
                    }
                    typhoon_web_protocol::WebCmd::GetNews { symbol } => {
                        self.log
                            .push_back(LogEntry::info(format!("Web news: {:?}", symbol)));
                        if let Some(ref tx) = self.web_msg_tx {
                            let items: Vec<typhoon_web_protocol::NewsItem> = self
                                .news_articles
                                .iter()
                                .filter(|n| {
                                    symbol
                                        .as_ref()
                                        .map(|s| n.0.contains(s) || n.1.contains(s))
                                        .unwrap_or(true)
                                })
                                .take(typhoon_web_protocol::MAX_NEWS_ITEMS)
                                .map(|n| typhoon_web_protocol::NewsItem {
                                    headline: n.0.clone(),
                                    source: n.2.clone(),
                                    url: n.1.clone(),
                                    symbol: symbol.clone(),
                                    timestamp: 0,
                                    summary: String::new(),
                                })
                                .collect();
                            let _ = tx.send(typhoon_web_protocol::WebMsg::NewsFeed { items });
                        }
                    }
                    typhoon_web_protocol::WebCmd::Subscribe { symbol, timeframe } => {
                        self.log.push_back(LogEntry::info(format!(
                            "Web subscribe: {symbol}:{timeframe}"
                        )));
                    }
                    typhoon_web_protocol::WebCmd::Unsubscribe { .. } => {}
                    typhoon_web_protocol::WebCmd::GetDarwinWeb { ticker } => {
                        // Return cached DWX web data to web client
                        if let Some(ref update) = self.dwx_last_update {
                            let ticker_filter = ticker.as_ref().map(|t| t.to_uppercase());
                            let matches_ticker =
                                |t: &str| ticker_filter.as_ref().map_or(true, |f| t == f);

                            let snapshots: Vec<typhoon_web_protocol::DarwinWebSnapshot> = update
                                .snapshots
                                .iter()
                                .filter(|s| matches_ticker(&s.ticker))
                                .map(|s| typhoon_web_protocol::DarwinWebSnapshot {
                                    ticker: s.ticker.clone(),
                                    timestamp_ms: s.timestamp_ms,
                                    quote: s.quote,
                                    daily_return_pct: s.daily_return_pct,
                                    monthly_return_pct: s.monthly_return_pct,
                                    ytd_return_pct: s.ytd_return_pct,
                                    all_time_return_pct: s.all_time_return_pct,
                                    dscore: s.dscore,
                                    ds_experience: s.ds_experience,
                                    ds_risk_mgmt: s.ds_risk_mgmt,
                                    ds_risk_adjustment: s.ds_risk_adjustment,
                                    ds_performance: s.ds_performance,
                                    ds_scalability: s.ds_scalability,
                                    ds_market_correlation: s.ds_market_correlation,
                                    var_monthly: s.var_monthly,
                                    max_drawdown_pct: s.max_drawdown_pct,
                                    volatility_annual: s.volatility_annual,
                                    sharpe_ratio: s.sharpe_ratio,
                                    sortino_ratio: s.sortino_ratio,
                                    investors: s.investors,
                                    aum: s.aum,
                                    capacity_remaining_pct: s.capacity_remaining_pct,
                                    total_trades: s.total_trades,
                                    win_rate: s.win_rate,
                                    profit_factor: s.profit_factor,
                                    avg_holding_time_hours: s.avg_holding_time_hours,
                                    avg_trade_return_pct: s.avg_trade_return_pct,
                                    symbols_traded: s.symbols_traded,
                                    excluded: s.excluded,
                                    exclusion_reason: s.exclusion_reason.clone(),
                                    correlation_portfolio: s.correlation_portfolio,
                                })
                                .collect();
                            let correlations: Vec<typhoon_web_protocol::DarwinWebCorrelation> =
                                update
                                    .correlations
                                    .iter()
                                    .map(|c| typhoon_web_protocol::DarwinWebCorrelation {
                                        darwin_a: c.darwin_a.clone(),
                                        darwin_b: c.darwin_b.clone(),
                                        correlation: c.correlation,
                                    })
                                    .collect();
                            let alerts: Vec<typhoon_web_protocol::DarwinCorrelationAlert> = update
                                .correlation_alerts
                                .iter()
                                .map(|a| typhoon_web_protocol::DarwinCorrelationAlert {
                                    darwin_a: a.darwin_a.clone(),
                                    darwin_b: a.darwin_b.clone(),
                                    correlation: a.correlation,
                                    threshold: a.threshold,
                                    suggestion: a.suggestion.clone(),
                                })
                                .collect();
                            // Map expanded tab data
                            let monthly_returns: Vec<typhoon_web_protocol::DarwinMonthlyReturns> =
                                update
                                    .monthly_returns
                                    .iter()
                                    .filter(|mr| matches_ticker(&mr.ticker))
                                    .map(|mr| typhoon_web_protocol::DarwinMonthlyReturns {
                                        ticker: mr.ticker.clone(),
                                        rows: mr
                                            .rows
                                            .iter()
                                            .map(|r| typhoon_web_protocol::MonthlyReturnRow {
                                                year: r.year,
                                                months: r.months,
                                                year_total: r.year_total,
                                            })
                                            .collect(),
                                        cagr: mr.cagr,
                                        best_month_pct: mr.best_month_pct,
                                        worst_month_pct: mr.worst_month_pct,
                                        avg_month_pct: mr.avg_month_pct,
                                        positive_months: mr.positive_months,
                                        negative_months: mr.negative_months,
                                    })
                                    .collect();
                            let equity_curves: Vec<typhoon_web_protocol::DarwinEquityCurve> =
                                update
                                    .equity_curves
                                    .iter()
                                    .filter(|ec| matches_ticker(&ec.ticker))
                                    .map(|ec| typhoon_web_protocol::DarwinEquityCurve {
                                        ticker: ec.ticker.clone(),
                                        points: ec
                                            .points
                                            .iter()
                                            .map(|p| typhoon_web_protocol::EquityPoint {
                                                timestamp_ms: p.timestamp_ms,
                                                value: p.value,
                                            })
                                            .collect(),
                                    })
                                    .collect();
                            let var_histories: Vec<typhoon_web_protocol::DarwinVaRHistory> = update
                                .var_histories
                                .iter()
                                .filter(|vh| matches_ticker(&vh.ticker))
                                .map(|vh| typhoon_web_protocol::DarwinVaRHistory {
                                    ticker: vh.ticker.clone(),
                                    points: vh
                                        .points
                                        .iter()
                                        .map(|p| typhoon_web_protocol::VaRPoint {
                                            timestamp_ms: p.timestamp_ms,
                                            var_pct: p.var_pct,
                                        })
                                        .collect(),
                                    current_var: vh.current_var,
                                    avg_var: vh.avg_var,
                                    max_var: vh.max_var,
                                    min_var: vh.min_var,
                                    var_violations: vh.var_violations,
                                    drawdown_periods: vh
                                        .drawdown_periods
                                        .iter()
                                        .map(|dd| typhoon_web_protocol::DrawdownPeriod {
                                            start_ms: dd.start_ms,
                                            end_ms: dd.end_ms,
                                            depth_pct: dd.depth_pct,
                                            recovery_days: dd.recovery_days,
                                        })
                                        .collect(),
                                })
                                .collect();
                            let dscore_histories: Vec<typhoon_web_protocol::DarwinDScoreHistory> =
                                update
                                    .dscore_histories
                                    .iter()
                                    .filter(|dh| matches_ticker(&dh.ticker))
                                    .map(|dh| typhoon_web_protocol::DarwinDScoreHistory {
                                        ticker: dh.ticker.clone(),
                                        points: dh
                                            .points
                                            .iter()
                                            .map(|p| typhoon_web_protocol::DScorePoint {
                                                timestamp_ms: p.timestamp_ms,
                                                dscore: p.dscore,
                                                experience: p.experience,
                                                risk_stability: p.risk_stability,
                                                risk_adjustment: p.risk_adjustment,
                                                performance: p.performance,
                                                scalability: p.scalability,
                                                market_correlation: p.market_correlation,
                                            })
                                            .collect(),
                                    })
                                    .collect();
                            let investor_flows: Vec<typhoon_web_protocol::DarwinInvestorFlow> =
                                update
                                    .investor_flows
                                    .iter()
                                    .filter(|ifl| matches_ticker(&ifl.ticker))
                                    .map(|ifl| typhoon_web_protocol::DarwinInvestorFlow {
                                        ticker: ifl.ticker.clone(),
                                        points: ifl
                                            .points
                                            .iter()
                                            .map(|p| typhoon_web_protocol::InvestorFlowPoint {
                                                timestamp_ms: p.timestamp_ms,
                                                investor_count: p.investor_count,
                                                aum: p.aum,
                                            })
                                            .collect(),
                                        capital_in: ifl.capital_in,
                                        capital_out: ifl.capital_out,
                                        net_flow: ifl.net_flow,
                                        divergence_pct: ifl.divergence_pct,
                                    })
                                    .collect();
                            let portfolio_performance =
                                update.portfolio_performance.as_ref().map(|pp| {
                                    typhoon_web_protocol::PortfolioPerformance {
                                        total_return_pct: pp.total_return_pct,
                                        cagr: pp.cagr,
                                        best_month_pct: pp.best_month_pct,
                                        worst_month_pct: pp.worst_month_pct,
                                        monthly_returns: pp
                                            .monthly_returns
                                            .iter()
                                            .map(|r| typhoon_web_protocol::MonthlyReturnRow {
                                                year: r.year,
                                                months: r.months,
                                                year_total: r.year_total,
                                            })
                                            .collect(),
                                        equity_points: pp
                                            .equity_points
                                            .iter()
                                            .map(|p| typhoon_web_protocol::EquityPoint {
                                                timestamp_ms: p.timestamp_ms,
                                                value: p.value,
                                            })
                                            .collect(),
                                    }
                                });
                            let portfolio_risk = update.portfolio_risk.as_ref().map(|pr| {
                                typhoon_web_protocol::PortfolioRisk {
                                    current_var: pr.current_var,
                                    max_drawdown_pct: pr.max_drawdown_pct,
                                    diversification_benefit_pct: pr.diversification_benefit_pct,
                                    var_history: pr
                                        .var_history
                                        .iter()
                                        .map(|p| typhoon_web_protocol::VaRPoint {
                                            timestamp_ms: p.timestamp_ms,
                                            var_pct: p.var_pct,
                                        })
                                        .collect(),
                                }
                            });
                            let allocations: Vec<typhoon_web_protocol::DarwinAllocation> = update
                                .allocations
                                .iter()
                                .map(|a| typhoon_web_protocol::DarwinAllocation {
                                    ticker: a.ticker.clone(),
                                    weight_pct: a.weight_pct,
                                    invested: a.invested,
                                    pnl: a.pnl,
                                })
                                .collect();
                            if let Some(ref tx) = self.web_msg_tx {
                                let _ = tx.send(typhoon_web_protocol::WebMsg::DarwinWebUpdate {
                                    snapshots,
                                    correlations,
                                    correlation_alerts: alerts,
                                    monthly_returns,
                                    equity_curves,
                                    var_histories,
                                    dscore_histories,
                                    investor_flows,
                                    portfolio_performance,
                                    portfolio_risk,
                                    allocations,
                                });
                            }
                        }
                    }
                }
            }
            if web_cmds_drained >= WEB_CMD_DRAIN_MAX {
                ctx.request_repaint();
            }
        }
    }
}
