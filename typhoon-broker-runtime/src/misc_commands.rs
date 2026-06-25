use typhoon_engine::broker::alpaca::AlpacaBroker;
use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg};

pub async fn handle_misc_command(
    cmd: BrokerCmd,
    broker: Option<&AlpacaBroker>,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    match cmd {
        BrokerCmd::MarkUnresolvable {
            broker,
            symbol,
            timeframe,
            reason,
        } => {
            let _ = broker_msg_tx.send(BrokerMsg::Unresolvable {
                broker,
                symbol,
                timeframe,
                reason,
            });
        }
        BrokerCmd::GetQuote { symbol } => {
            if let Some(b) = broker {
                match b.get_latest_quote(&symbol).await {
                    Ok(q) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Quote(
                            symbol,
                            q.bid,
                            q.ask,
                            (q.bid + q.ask) / 2.0,
                        ));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(e));
                    }
                }
            }
        }
        BrokerCmd::GetMarketClock => {
            // US-equity/xStock session status is sourced from Alpaca's market clock.
            // Kraken crypto pairs are shown separately as 24/7 in the toolbar.
            if let Some(b) = broker {
                match b.get_market_clock().await {
                    Ok(v) => {
                        let is_open = v["is_open"].as_bool().unwrap_or(false);
                        let next_open = v["next_open"].as_str().unwrap_or("—");
                        let next_close = v["next_close"].as_str().unwrap_or("—");

                        let next_open_utc = chrono::DateTime::parse_from_rfc3339(next_open)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc));
                        let next_close_utc = chrono::DateTime::parse_from_rfc3339(next_close)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc));

                        let msg =
                            typhoon_engine::core::market_session::us_equities_session_status_at(
                                chrono::Utc::now(),
                                is_open,
                                next_open_utc,
                                next_close_utc,
                            );
                        let _ = broker_msg_tx.send(BrokerMsg::MarketClock(msg));
                    }
                    Err(e) => {
                        let _ = broker_msg_tx.send(BrokerMsg::Error(e));
                    }
                }
            }
        }
        _ => unreachable!("non-misc command routed to misc handler"),
    }
}
