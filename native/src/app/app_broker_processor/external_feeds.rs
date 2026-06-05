use super::*;

pub(super) async fn handle_external_feed_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    match cmd {
        BrokerCmd::FredFetch { api_key } => {
            use typhoon_engine::core::fred;
            let client = reqwest::Client::new();
            let mut series_data = Vec::new();
            for (id, _name) in fred::KEY_SERIES.iter() {
                if let Ok(s) = fred::fetch_series(&client, &api_key, id, 60).await {
                    series_data.push(s);
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            let yield_curve = fred::fetch_yield_curve(&client, &api_key)
                .await
                .unwrap_or_default();
            let _ = broker_msg_tx_clone.send(BrokerMsg::FredData(series_data, yield_curve));
        }
        BrokerCmd::FetchEconCalendar { finnhub_key } => {
            // Strategy: if Finnhub key present, use Finnhub (richer — includes "actual" values).
            // Otherwise fall back to ForexFactory weekly XML (free, no key, ForexFactory-parity data).
            let client = reqwest::Client::new();
            if !finnhub_key.is_empty() {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let next_week = (chrono::Utc::now() + chrono::Duration::days(7))
                    .format("%Y-%m-%d")
                    .to_string();
                let url = format!(
                    "https://finnhub.io/api/v1/calendar/economic?from={}&to={}&token={}",
                    today, next_week, finnhub_key
                );
                match client.get(&url).send().await {
                    Ok(resp) => {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            let mut events = Vec::new();
                            if let Some(arr) = body["economicCalendar"].as_array() {
                                for e in arr {
                                    let date = e["time"].as_str().unwrap_or("").to_string();
                                    let country = e["country"].as_str().unwrap_or("").to_string();
                                    let event_name = e["event"].as_str().unwrap_or("").to_string();
                                    let impact = e["impact"].as_str().unwrap_or("low").to_string();
                                    let actual = e["actual"]
                                        .as_f64()
                                        .map(|v| format!("{:.2}", v))
                                        .unwrap_or("\u{2014}".into());
                                    events.push((date, country, event_name, impact, actual));
                                }
                            }
                            let _ = broker_msg_tx_clone.send(BrokerMsg::EconCalendarData(events));
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = broker_msg_tx_clone
                            .send(BrokerMsg::Error(format!("Finnhub econ calendar: {}", e)));
                    }
                }
            }
            // ForexFactory fallback (keyless)
            match typhoon_engine::core::econ_calendar::fetch_forexfactory_week(&client).await {
                Ok(ff_events) => {
                    let events: Vec<(String, String, String, String, String)> = ff_events
                        .into_iter()
                        .map(|e| {
                            // Flatten MM-DD-YYYY + time into ISO-ish "YYYY-MM-DD HH:MM"
                            let date_str = if let Ok(d) =
                                chrono::NaiveDate::parse_from_str(&e.date, "%m-%d-%Y")
                            {
                                format!("{} {}", d.format("%Y-%m-%d"), e.time)
                            } else {
                                format!("{} {}", e.date, e.time)
                            };
                            let prev = if e.previous.is_empty() {
                                "\u{2014}".to_string()
                            } else {
                                e.previous.clone()
                            };
                            let actual = if e.forecast.is_empty() {
                                prev
                            } else {
                                format!(
                                    "fc:{} (prev:{})",
                                    e.forecast,
                                    if e.previous.is_empty() {
                                        "-"
                                    } else {
                                        &e.previous
                                    }
                                )
                            };
                            (
                                date_str,
                                e.country,
                                e.title,
                                e.impact.label().to_lowercase(),
                                actual,
                            )
                        })
                        .collect();
                    let _ = broker_msg_tx_clone.send(BrokerMsg::EconCalendarData(events));
                }
                Err(e) => {
                    let _ = broker_msg_tx_clone
                        .send(BrokerMsg::Error(format!("ForexFactory fallback: {}", e)));
                }
            }
        }
        BrokerCmd::FetchCongressTrades => {
            let client = reqwest::Client::builder()
                .user_agent("TyphooN-Terminal/1.0")
                .build()
                .unwrap_or_default();
            match client.get("https://house-stock-watcher-data.s3-us-west-2.amazonaws.com/data/all_transactions.json")
                        .timeout(std::time::Duration::from_secs(30))
                        .send().await {
                        Ok(resp) => {
                            if let Ok(body) = resp.json::<serde_json::Value>().await {
                                let mut trades = Vec::new();
                                if let Some(arr) = body.as_array() {
                                    // Take last 200 (most recent)
                                    for t in arr.iter().rev().take(200) {
                                        let date = t["transaction_date"].as_str().unwrap_or("").to_string();
                                        let rep = t["representative"].as_str().unwrap_or("").to_string();
                                        let ticker = t["ticker"].as_str().unwrap_or("").to_string();
                                        let tx_type = t["type"].as_str().unwrap_or("").to_string();
                                        let amount = t["amount"].as_str().unwrap_or("").to_string();
                                        let party = t["party"].as_str().unwrap_or("").to_string();
                                        if !ticker.is_empty() && ticker != "--" {
                                            trades.push((date, rep, ticker, tx_type, amount, party));
                                        }
                                    }
                                }
                                let _ = broker_msg_tx_clone.send(BrokerMsg::CongressData(trades));
                            }
                        }
                        Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Congress trades: {}", e))); }
                    }
        }
        BrokerCmd::SendNotification {
            discord_webhook,
            pushover_token,
            pushover_user,
            ntfy_topic,
            message,
        } => {
            use typhoon_engine::notifications;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let mut sent = false;
                if !discord_webhook.is_empty() {
                    if let Err(e) = notifications::send_discord(&discord_webhook, &message).await {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Discord: {}", e)));
                    } else {
                        sent = true;
                    }
                }
                if !pushover_token.is_empty() && !pushover_user.is_empty() {
                    if let Err(e) =
                        notifications::send_pushover(&pushover_token, &pushover_user, &message)
                            .await
                    {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Pushover: {}", e)));
                    } else {
                        sent = true;
                    }
                }
                if !ntfy_topic.is_empty() {
                    if let Err(e) = notifications::send_ntfy(&ntfy_topic, &message).await {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("ntfy: {}", e)));
                    } else {
                        sent = true;
                    }
                }
                if sent {
                    let _ = msg_tx.send(BrokerMsg::OrderResult(format!(
                        "Notification sent: {}",
                        &message[..message.len().min(60)]
                    )));
                }
            });
        }
        _ => unreachable!("non-external-feed command routed to external feed handler"),
    }
}
