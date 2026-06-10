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
                        // Read bars directly from cache and broadcast. Try live
                        // sources in priority order and serve the first hit.
                        if let Some(ref cache) = self.cache {
                            let mut data = None;
                            for source in ["kraken", "kraken-futures", "kraken-equities", "alpaca", "default"] {
                                for key in crate::app::chart::chart_source_cache_keys(source, &symbol, &timeframe) {
                                    if let Ok(Some(rows)) = cache.get_bars_raw(&key) {
                                        data = Some(rows);
                                        break;
                                    }
                                }
                                if data.is_some() {
                                    break;
                                }
                            }
                            if let Some(data) = data {
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
                }
            }
            if web_cmds_drained >= WEB_CMD_DRAIN_MAX {
                ctx.request_repaint();
            }
        }
    }
}
