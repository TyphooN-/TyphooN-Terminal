use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg};

pub fn handle_research_fetch_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    match cmd {
        // Company events, sentiment, transcripts, commodities, and tape research handlers
        BrokerCmd::FetchCompanyProfile {
            symbol,
            finnhub_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_finnhub_profile(&client, &symbol, &finnhub_key).await {
                    Ok(p) => {
                        let _ = msg_tx.send(BrokerMsg::CompanyProfile(p));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("DES profile: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchStockPeers {
            symbol,
            finnhub_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_finnhub_peers(&client, &symbol, &finnhub_key).await {
                    Ok(peers) => {
                        let _ = msg_tx.send(BrokerMsg::StockPeers(symbol, peers));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("PEERS: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchEarningsHistory {
            symbol,
            finnhub_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_finnhub_earnings(&client, &symbol, &finnhub_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::EarningsHistory(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("EARNINGS: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchIpoCalendar {
            finnhub_key,
            days_ahead,
            days_back,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                let today = chrono::Utc::now();
                let from = (today - chrono::Duration::days(days_back.max(0)))
                    .format("%Y-%m-%d")
                    .to_string();
                let to = (today + chrono::Duration::days(days_ahead.max(0)))
                    .format("%Y-%m-%d")
                    .to_string();
                match research::fetch_finnhub_ipo_calendar(&client, &finnhub_key, &from, &to).await
                {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::IpoCalendar(rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("IPO: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchPressReleases {
            symbol,
            finnhub_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_finnhub_press(&client, &symbol, &finnhub_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::PressReleases(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("PRESS: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchSocialSentiment {
            symbol,
            finnhub_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_finnhub_social(&client, &symbol, &finnhub_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::SocialSentiment(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("SENTIMENT: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchTranscriptList { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_transcript_list(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::TranscriptList(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("TRANSCRIPTS list: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchTranscriptBody {
            symbol,
            quarter,
            year,
            fmp_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_transcript(&client, &symbol, quarter, year, &fmp_key)
                    .await
                {
                    Ok(t) => {
                        let _ = msg_tx.send(BrokerMsg::TranscriptBody(t));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("TRANSCRIPTS body: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchCommoditiesQuotes => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .unwrap_or_default();
                let symbols: Vec<&str> = research::COMMODITIES_UNIVERSE
                    .iter()
                    .map(|(s, _, _)| *s)
                    .collect();
                match research::fetch_yahoo_quotes(&client, &symbols).await {
                    Ok(quotes) => {
                        let quotes_by_symbol: std::collections::HashMap<&str, &_> =
                            quotes.iter().map(|q| (q.0.as_str(), q)).collect();
                        let out: Vec<research::CommodityQuote> = research::COMMODITIES_UNIVERSE
                            .iter()
                            .map(|(sym, display, _)| {
                                if let Some(q) = quotes_by_symbol.get(*sym).copied() {
                                    research::CommodityQuote {
                                        symbol: sym.to_string(),
                                        display: display.to_string(),
                                        price: q.1,
                                        change: q.2,
                                        change_pct: q.3,
                                    }
                                } else {
                                    research::CommodityQuote {
                                        symbol: sym.to_string(),
                                        display: display.to_string(),
                                        ..Default::default()
                                    }
                                }
                            })
                            .collect();
                        let _ = msg_tx.send(BrokerMsg::CommoditiesQuotes(out));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("GLCO: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchDividendHistory { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_dividend_history(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::DividendHistory(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("DVD: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchEarningsEstimates { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_earnings_estimates(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::EarningsEstimates(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("EEB: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchRatingChanges { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_rating_changes(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::RatingChanges(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("UPDG: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchTreasuryYields => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .unwrap_or_default();
                match research::fetch_treasury_yields(&client).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::TreasuryYields(rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("GY: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchFinancialStatements { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_financial_bundle(&client, &symbol, &fmp_key).await {
                    Ok(bundle) => {
                        let _ = msg_tx.send(BrokerMsg::FinancialStatementsMsg(symbol, bundle));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("FA: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchExecutives {
            symbol,
            finnhub_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_finnhub_executives(&client, &symbol, &finnhub_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::Executives(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("MGMT: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchCotReports => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .unwrap_or_default();
                match research::fetch_cftc_cot(&client).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::CotReports(rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("COT: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchStockSplits { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_stock_splits(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::StockSplitsMsg(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("SPLT: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchEtfHoldings { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_etf_holdings(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::EtfHoldingsMsg(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("ETF: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchAnalystRecs {
            symbol,
            finnhub_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_finnhub_recommendations(&client, &symbol, &finnhub_key).await
                {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::AnalystRecsMsg(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("ANR: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchPriceTarget {
            symbol,
            finnhub_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_finnhub_price_target(&client, &symbol, &finnhub_key).await {
                    Ok(pt) => {
                        let _ = msg_tx.send(BrokerMsg::PriceTargetMsg(symbol, pt));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("PT: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchEsgScores { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_esg(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::EsgScoresMsg(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("ESG: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchIndexMembers {
            index_code,
            fmp_key,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_index_members(&client, &index_code, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::IndexMembersMsg(index_code, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("MEMB: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchInsiderTrades { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_insider_trades(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::InsiderTradesMsg(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("INS: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchInstitutionalHolders { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_institutional_holders(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::InstitutionalHoldersMsg(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("HDS: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchSharesFloat { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_shares_float(&client, &symbol, &fmp_key).await {
                    Ok(snap) => {
                        let _ = msg_tx.send(BrokerMsg::SharesFloatMsg(symbol, snap));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("FLOAT: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchHistoricalPrice {
            symbol,
            fmp_key,
            limit,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(25))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_historical_price(&client, &symbol, &fmp_key, limit).await
                {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::HistoricalPriceMsg(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("HP: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchEarningsSurprises { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_earnings_surprises(&client, &symbol, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::EarningsSurpriseMsg(symbol, rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("EPS: {}", e)));
                    }
                }
            });
        }
        // ── handlers ──
        BrokerCmd::FetchWorldIndices => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .unwrap_or_default();
                match research::fetch_world_indices(&client).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::WorldIndicesMsg(rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("WEI: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchMarketMovers { fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(25))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_market_movers(&client, &fmp_key).await {
                    Ok(mov) => {
                        let _ = msg_tx.send(BrokerMsg::MarketMoversMsg(mov));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("MOV: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchSectorPerformance { fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_fmp_sector_performance(&client, &fmp_key).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::SectorPerformanceMsg(rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("INDU: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::FetchWaccSnapshot {
            symbol,
            fmp_key,
            risk_free_pct,
        } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(25))
                    .build()
                    .unwrap_or_default();
                // Fetch profile (beta + market cap)
                let profile_url = format!(
                    "https://financialmodelingprep.com/api/v3/profile/{}?apikey={}",
                    symbol, fmp_key
                );
                let profile_resp = match client.get(&profile_url).send().await {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("WACC profile: {e}")));
                        return;
                    }
                };
                let profile_arr: Vec<serde_json::Value> = match profile_resp.json().await {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("WACC profile parse: {e}")));
                        return;
                    }
                };
                let profile = profile_arr.first().cloned().unwrap_or_default();
                let beta = profile["beta"].as_f64().unwrap_or(1.0);
                let market_cap = profile["mktCap"].as_f64().unwrap_or(0.0);
                // Fetch key metrics TTM (effective tax rate fallback)
                let km_url = format!(
                    "https://financialmodelingprep.com/api/v3/key-metrics-ttm/{}?apikey={}",
                    symbol, fmp_key
                );
                let km_arr: Vec<serde_json::Value> = match client.get(&km_url).send().await {
                    Ok(r) => r.json().await.unwrap_or_default(),
                    Err(_) => Vec::new(),
                };
                let km = km_arr.first().cloned().unwrap_or_default();
                // Fetch income statement for interest expense + tax
                let is_url = format!(
                    "https://financialmodelingprep.com/api/v3/income-statement/{}?period=annual&limit=1&apikey={}",
                    symbol, fmp_key
                );
                let is_arr: Vec<serde_json::Value> = match client.get(&is_url).send().await {
                    Ok(r) => r.json().await.unwrap_or_default(),
                    Err(_) => Vec::new(),
                };
                let is_row = is_arr.first().cloned().unwrap_or_default();
                let interest_expense = is_row["interestExpense"].as_f64().unwrap_or(0.0);
                let income_before_tax = is_row["incomeBeforeTax"].as_f64().unwrap_or(0.0);
                let income_tax = is_row["incomeTaxExpense"].as_f64().unwrap_or(0.0);
                let effective_tax_rate_pct = if income_before_tax.abs() > 1e-6 {
                    (income_tax / income_before_tax) * 100.0
                } else {
                    km["effectiveTaxRateTTM"].as_f64().unwrap_or(0.21) * 100.0
                };
                // Fetch balance sheet for total debt
                let bs_url = format!(
                    "https://financialmodelingprep.com/api/v3/balance-sheet-statement/{}?period=annual&limit=1&apikey={}",
                    symbol, fmp_key
                );
                let bs_arr: Vec<serde_json::Value> = match client.get(&bs_url).send().await {
                    Ok(r) => r.json().await.unwrap_or_default(),
                    Err(_) => Vec::new(),
                };
                let bs_row = bs_arr.first().cloned().unwrap_or_default();
                let total_debt = bs_row["totalDebt"].as_f64().unwrap_or_else(|| {
                    let lt = bs_row["longTermDebt"].as_f64().unwrap_or(0.0);
                    let st = bs_row["shortTermDebt"].as_f64().unwrap_or(0.0);
                    lt + st
                });
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let snap = research::compute_wacc_snapshot(
                    &symbol,
                    &today,
                    beta,
                    market_cap,
                    risk_free_pct,
                    total_debt,
                    interest_expense,
                    effective_tax_rate_pct,
                );
                let _ = msg_tx.send(BrokerMsg::WaccSnapshotMsg(symbol, snap));
            });
        }
        // ── handlers ──
        BrokerCmd::FetchCurrencyRates => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                match research::fetch_currency_rates(&client).await {
                    Ok(rows) => {
                        let _ = msg_tx.send(BrokerMsg::CurrencyRatesMsg(rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("WCR: {e}")));
                    }
                }
            });
        }
        BrokerCmd::FetchBetaSnapshot { symbol, fmp_key } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("TyphooN-Terminal/1.0")
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .unwrap_or_default();
                // Fetch 5 years of bars for both the symbol and SPY.
                let sym_bars =
                    match research::fetch_fmp_historical_price(&client, &symbol, &fmp_key, 1300)
                        .await
                    {
                        Ok(rows) => rows,
                        Err(e) => {
                            let _ =
                                msg_tx.send(BrokerMsg::Error(format!("BETA {symbol} bars: {e}")));
                            return;
                        }
                    };
                let mkt_bars = match research::fetch_fmp_historical_price(
                    &client, "SPY", &fmp_key, 1300,
                )
                .await
                {
                    Ok(rows) => rows,
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("BETA SPY bars: {e}")));
                        return;
                    }
                };
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let snap =
                    research::compute_beta_snapshot(&symbol, "SPY", &today, &sym_bars, &mkt_bars);
                let _ = msg_tx.send(BrokerMsg::BetaSnapshotMsg(symbol, snap));
            });
        }
        _ => unreachable!("non-research-fetch command routed to research fetch handler"),
    }
}
