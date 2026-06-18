use super::*;

mod coverage_relative_event;
mod dividend_sentiment_ranks;
mod factor_rank_core;
mod fundamental_risk;
mod growth_flow_regime;
mod valuation_quality_risk;
mod insider_dividend_momentum;
mod market_liquidity_credit;
mod price_rank_risk_overlays;
mod return_distribution_stats;
mod volatility_stat_tests;
mod performance_runs_tests;
mod significance_stationarity;
mod tail_risk_diagnostics;
mod entropy_dependence;
mod upside_drawdown_risk;
mod entropy_stationarity;
mod robust_quantile_volatility;
mod normality_liquidity_tail;
mod fractal_rank_dynamics;
mod jump_trend_diagnostics;
mod solvency_quality;

pub(super) fn handle_risk_compute(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        // Leverage, accruals, realized-volatility, cash-flow, and short-interest research
        cmd @ (BrokerCmd::ComputeLeverageSnapshot { .. }
            | BrokerCmd::ComputeAccrualsSnapshot { .. }
            | BrokerCmd::ComputeRealizedVolSnapshot { .. }
            | BrokerCmd::ComputeFcfYieldSnapshot { .. }
            | BrokerCmd::ComputeShortInterestSnapshot { .. }) => {
            fundamental_risk::handle_fundamental_risk_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Solvency, quality, volatility-estimator, EPS-beat, and price-target research
        cmd @ (BrokerCmd::ComputeAltmanZSnapshot { .. }
            | BrokerCmd::ComputePiotroskiSnapshot { .. }
            | BrokerCmd::ComputeOhlcVolSnapshot { .. }
            | BrokerCmd::ComputeEpsBeatSnapshot { .. }
            | BrokerCmd::ComputePriceTargetDispersionSnapshot { .. }) => {
            solvency_quality::handle_solvency_quality_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Insider, dividend-growth, earnings-revision, sector-rotation, and upgrade/downgrade research
        cmd @ (BrokerCmd::ComputeInsiderActivitySnapshot { .. }
            | BrokerCmd::ComputeDivgSnapshot { .. }
            | BrokerCmd::ComputeEarmSnapshot { .. }
            | BrokerCmd::ComputeSectorRotationSnapshot { .. }
            | BrokerCmd::ComputeUpdmSnapshot { .. }) => {
            insider_dividend_momentum::handle_insider_dividend_momentum_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Momentum, liquidity, breakout, cash-cycle, and credit research
        cmd @ (BrokerCmd::ComputeMomentumSnapshot { .. }
            | BrokerCmd::ComputeLiquiditySnapshot { .. }
            | BrokerCmd::ComputeBreakoutSnapshot { .. }
            | BrokerCmd::ComputeCashCycleSnapshot { .. }
            | BrokerCmd::ComputeCreditSnapshot { .. }) => {
            market_liquidity_credit::handle_market_liquidity_credit_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Growth-momentum, ownership-flow, and market-regime research
        cmd @ (BrokerCmd::ComputeGrowmSnapshot { .. }
            | BrokerCmd::ComputeFlowSnapshot { .. }
            | BrokerCmd::ComputeRegimeSnapshot { .. }) => {
            growth_flow_regime::handle_growth_flow_regime_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Relative-volume, margin, valuation, quality, and composite-risk research
        cmd @ (BrokerCmd::ComputeRelvolSnapshot { .. }
            | BrokerCmd::ComputeMarginsSnapshot { .. }
            | BrokerCmd::ComputeValSnapshot { .. }
            | BrokerCmd::ComputeQualSnapshot { .. }
            | BrokerCmd::ComputeRiskSnapshot { .. }) => {
            valuation_quality_risk::handle_valuation_quality_risk_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Insider-streak, analyst-coverage, relative rank, earnings-growth, and PEAD research
        cmd @ (BrokerCmd::ComputeInsstrkSnapshot { .. }
            | BrokerCmd::ComputeCovgSnapshot { .. }
            | BrokerCmd::ComputeVrkSnapshot { .. }
            | BrokerCmd::ComputeQrkSnapshot { .. }
            | BrokerCmd::ComputeRrkSnapshot { .. }
            | BrokerCmd::ComputeRelepsgrSnapshot { .. }
            | BrokerCmd::ComputePeadSnapshot { .. }) => {
            coverage_relative_event::handle_coverage_relative_event_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Size, momentum, PEAD, quality, reversal, leverage, operating, and liquidity rank research
        cmd @ (BrokerCmd::ComputeSizefSnapshot { .. }
            | BrokerCmd::ComputeMomfSnapshot { .. }
            | BrokerCmd::ComputePeadrankSnapshot { .. }
            | BrokerCmd::ComputeFqmSnapshot { .. }
            | BrokerCmd::ComputeRevrankSnapshot { .. }
            | BrokerCmd::ComputeLevrankSnapshot { .. }
            | BrokerCmd::ComputeOperankSnapshot { .. }
            | BrokerCmd::ComputeFqmrankSnapshot { .. }
            | BrokerCmd::ComputeLiqrankSnapshot { .. }) => {
            factor_rank_core::handle_factor_rank_core_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Surprise streak, dividend/earnings/upgrade ranks, yield/short ranks, and insider concentration research
        cmd @ (BrokerCmd::ComputeSurpstkSnapshot { .. }
            | BrokerCmd::ComputeDvdrankSnapshot { .. }
            | BrokerCmd::ComputeEarmrankSnapshot { .. }
            | BrokerCmd::ComputeUpdgrankSnapshot { .. }
            | BrokerCmd::ComputeGySnapshot { .. }
            | BrokerCmd::ComputeDesSnapshot { .. }
            | BrokerCmd::ComputeDvdyieldrankSnapshot { .. }
            | BrokerCmd::ComputeShrankSnapshot { .. }
            | BrokerCmd::ComputeShortrankDeltaSnapshot { .. }
            | BrokerCmd::ComputeInsiderconcSnapshot { .. }) => {
            dividend_sentiment_ranks::handle_dividend_sentiment_rank_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Price-performance, beta/PEG/range ranks, correlation, accrual, and volatility-risk-premium research
        cmd @ (BrokerCmd::ComputeAtrannSnapshot { .. }
            | BrokerCmd::ComputeDdhistSnapshot { .. }
            | BrokerCmd::ComputePriceperfSnapshot { .. }
            | BrokerCmd::ComputeMomrankMultiSnapshot { .. }
            | BrokerCmd::ComputeBetarankSnapshot { .. }
            | BrokerCmd::ComputePegrankSnapshot { .. }
            | BrokerCmd::ComputeFhighlowSnapshot { .. }
            | BrokerCmd::ComputeRvconeSnapshot { .. }
            | BrokerCmd::ComputeCalpbSnapshot { .. }
            | BrokerCmd::ComputeCorrstkSnapshot { .. }
            | BrokerCmd::ComputeTlrankSnapshot { .. }
            | BrokerCmd::ComputeCorrrankSnapshot { .. }
            | BrokerCmd::ComputeOperankDeltaSnapshot { .. }
            | BrokerCmd::ComputeDivaccSnapshot { .. }
            | BrokerCmd::ComputeEpsaccSnapshot { .. }
            | BrokerCmd::ComputeVrpSnapshot { .. }) => {
            price_rank_risk_overlays::handle_price_rank_risk_overlay_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Return-distribution, autocorrelation, drawdown, downside, and seasonality research
        cmd @ (BrokerCmd::ComputeRetskewSnapshot { .. }
            | BrokerCmd::ComputeRetkurtSnapshot { .. }
            | BrokerCmd::ComputeTailrSnapshot { .. }
            | BrokerCmd::ComputeRunlenSnapshot { .. }
            | BrokerCmd::ComputeDayrangeSnapshot { .. }
            | BrokerCmd::ComputeAutocorSnapshot { .. }
            | BrokerCmd::ComputeHurstSnapshot { .. }
            | BrokerCmd::ComputeHitrateSnapshot { .. }
            | BrokerCmd::ComputeGlasymSnapshot { .. }
            | BrokerCmd::ComputeVolratioSnapshot { .. }
            | BrokerCmd::ComputeDrawupSnapshot { .. }
            | BrokerCmd::ComputeGapstatsSnapshot { .. }
            | BrokerCmd::ComputeVolclusterSnapshot { .. }
            | BrokerCmd::ComputeCloseplcSnapshot { .. }
            | BrokerCmd::ComputeMrhlSnapshot { .. }
            | BrokerCmd::ComputeDownvolSnapshot { .. }
            | BrokerCmd::ComputeSharprSnapshot { .. }
            | BrokerCmd::ComputeEffratioSnapshot { .. }
            | BrokerCmd::ComputeWickbiasSnapshot { .. }
            | BrokerCmd::ComputeVolofvolSnapshot { .. }
            | BrokerCmd::ComputeCalmarSnapshot { .. }
            | BrokerCmd::ComputeUlcerSnapshot { .. }
            | BrokerCmd::ComputeVarratioSnapshot { .. }
            | BrokerCmd::ComputeAmihudSnapshot { .. }
            | BrokerCmd::ComputeJbnormSnapshot { .. }
            | BrokerCmd::ComputeOmegaSnapshot { .. }
            | BrokerCmd::ComputeDfaSnapshot { .. }
            | BrokerCmd::ComputeBurkeSnapshot { .. }
            | BrokerCmd::ComputeMonthseasSnapshot { .. }
            | BrokerCmd::ComputeRollsprdSnapshot { .. }) => {
            return_distribution_stats::handle_return_distribution_stat_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Parkinson, Garman-Klass, Rogers-Satchell, CVaR, and day-of-week research
        cmd @ (BrokerCmd::ComputeParkinsonSnapshot { .. }
            | BrokerCmd::ComputeGkvolSnapshot { .. }
            | BrokerCmd::ComputeRsvolSnapshot { .. }
            | BrokerCmd::ComputeCvarSnapshot { .. }
            | BrokerCmd::ComputeDoweffectSnapshot { .. }) => {
            volatility_stat_tests::handle_volatility_stat_test_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Sterling, Kelly, Ljung-Box, runs-test, and zero-return research
        cmd @ (BrokerCmd::ComputeSterlingSnapshot { .. }
            | BrokerCmd::ComputeKellyfSnapshot { .. }
            | BrokerCmd::ComputeLjungbSnapshot { .. }
            | BrokerCmd::ComputeRunstestSnapshot { .. }
            | BrokerCmd::ComputeZeroretSnapshot { .. }) => {
            performance_runs_tests::handle_performance_run_test_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // PSR, ADF, Mann-Kendall, bipower, and drawdown-duration research
        cmd @ (BrokerCmd::ComputePsrSnapshot { .. }
            | BrokerCmd::ComputeAdfSnapshot { .. }
            | BrokerCmd::ComputeMnkendallSnapshot { .. }
            | BrokerCmd::ComputeBipowerSnapshot { .. }
            | BrokerCmd::ComputeDddurSnapshot { .. }) => {
            significance_stationarity::handle_significance_stationarity_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Hill-tail, ARCH-LM, pain-ratio, CUSUM, and Cornish-Fisher VaR research
        cmd @ (BrokerCmd::ComputeHilltailSnapshot { .. }
            | BrokerCmd::ComputeArchlmSnapshot { .. }
            | BrokerCmd::ComputePainratioSnapshot { .. }
            | BrokerCmd::ComputeCusumSnapshot { .. }
            | BrokerCmd::ComputeCfvarSnapshot { .. }) => {
            tail_risk_diagnostics::handle_tail_risk_diagnostic_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Entropy, Rachev, gain-pain, PACF, and approximate-entropy research
        cmd @ (BrokerCmd::ComputeEntropySnapshot { .. }
            | BrokerCmd::ComputeRachevSnapshot { .. }
            | BrokerCmd::ComputeGprSnapshot { .. }
            | BrokerCmd::ComputePacfSnapshot { .. }
            | BrokerCmd::ComputeApenSnapshot { .. }) => {
            entropy_dependence::handle_entropy_dependence_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Upside-potential, leverage-effect, drawdown-at-risk, VaR-half-life, and Gini research
        cmd @ (BrokerCmd::ComputeUprSnapshot { .. }
            | BrokerCmd::ComputeLevereffSnapshot { .. }
            | BrokerCmd::ComputeDrawdarSnapshot { .. }
            | BrokerCmd::ComputeVarhalfSnapshot { .. }
            | BrokerCmd::ComputeGiniSnapshot { .. }) => {
            upside_drawdown_risk::handle_upside_drawdown_risk_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Sample-entropy, permutation-entropy, recurrence-factor, KPSS, and spectral-entropy research
        cmd @ (BrokerCmd::ComputeSampenSnapshot { .. }
            | BrokerCmd::ComputePermenSnapshot { .. }
            | BrokerCmd::ComputeRecfactSnapshot { .. }
            | BrokerCmd::ComputeKpssSnapshot { .. }
            | BrokerCmd::ComputeSpecentSnapshot { .. }) => {
            entropy_stationarity::handle_entropy_stationarity_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Robust-volatility, Renyi-entropy, return-quantile, market-sentiment, and EWMA-volatility research
        cmd @ (BrokerCmd::ComputeRobvolSnapshot { .. }
            | BrokerCmd::ComputeRenyientSnapshot { .. }
            | BrokerCmd::ComputeRetquantSnapshot { .. }
            | BrokerCmd::ComputeMsentSnapshot { .. }
            | BrokerCmd::ComputeEwmavolSnapshot { .. }) => {
            robust_quantile_volatility::handle_robust_quantile_volatility_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // KS-normality, Anderson-Darling, L-moment, Kyle-lambda, and peak-over-threshold research
        cmd @ (BrokerCmd::ComputeKsnormSnapshot { .. }
            | BrokerCmd::ComputeAdtestSnapshot { .. }
            | BrokerCmd::ComputeLmomSnapshot { .. }
            | BrokerCmd::ComputeKylelamSnapshot { .. }
            | BrokerCmd::ComputePeakoverSnapshot { .. }) => {
            normality_liquidity_tail::handle_normality_liquidity_tail_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Higuchi, Pickands, kappa, Lyapunov, and rank-autocorrelation research
        cmd @ (BrokerCmd::ComputeHiguchiSnapshot { .. }
            | BrokerCmd::ComputePickandsSnapshot { .. }
            | BrokerCmd::ComputeKappa3Snapshot { .. }
            | BrokerCmd::ComputeLyapunovSnapshot { .. }
            | BrokerCmd::ComputeRankacSnapshot { .. }) => {
            fractal_rank_dynamics::handle_fractal_rank_dynamics_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // Jump-test, Phillips-Perron, MF-DFA, Hill-KS, and trend-strength research
        cmd @ (BrokerCmd::ComputeBnsjumpSnapshot { .. }
            | BrokerCmd::ComputePprootSnapshot { .. }
            | BrokerCmd::ComputeMfdfaSnapshot { .. }
            | BrokerCmd::ComputeHillksSnapshot { .. }
            | BrokerCmd::ComputeTsiSnapshot { .. }) => {
            jump_trend_diagnostics::handle_jump_trend_diagnostic_compute(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        // GARCH, SADF, correlation-dimension, spectral-skew, and automutual-information research
        BrokerCmd::ComputeGarch11Snapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_garch11_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::Garch11SnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSadfSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_sadf_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::SadfSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeCordimSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_cordim_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::CordimSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeSkspecSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_skspec_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::SkspecSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeAutomiSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_automi_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::AutomiSnapshotMsg(symbol, snap));
            });
        }
        // RPDE, Hurst-cycle, IAAFT, bid-ask bounce, break-even volatility, and related quant research
        BrokerCmd::ComputeDurbinWatsonSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_durbinwatson_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::DurbinWatsonSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBdsTestSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_bdstest_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BdsTestSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBreuschPaganSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_breuschpagan_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BreuschPaganSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeTurnPtsSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_turnpts_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::TurnPtsSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputePeriodogramSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_periodogram_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::PeriodogramSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeMcLeodLiSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_mcleodli_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::McLeodLiSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeOuFitSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_oufit_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::OuFitSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeGphSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_gph_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::GphSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBurgSpecSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_burgspec_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::BurgSpecSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKendallTauSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_kendalltau_snapshot(&symbol, &today, &bars);
                let _ = msg_tx.send(BrokerMsg::KendallTauSnapshotMsg(symbol, snap));
            });
        }
        // Structural-break, ICSS, Hurst-rolling, tail-dependence, volatility-spillover, and correlation-stability research
        cmd @ (BrokerCmd::ComputeSqueezeSnapshot { .. }
        | BrokerCmd::ComputeSqueezeRankSnapshot { .. }
        | BrokerCmd::RefreshSqueezeWatchlist
        | BrokerCmd::ComputeBbsqueezeSnapshot { .. }
        | BrokerCmd::ComputeDonchianSnapshot { .. }
        | BrokerCmd::ComputeKamaSnapshot { .. }
        | BrokerCmd::ComputeIchimokuSnapshot { .. }
        | BrokerCmd::ComputeSupertrendSnapshot { .. }
        | BrokerCmd::ComputeKeltnerSnapshot { .. }
        | BrokerCmd::ComputeFisherSnapshot { .. }
        | BrokerCmd::ComputeAroonSnapshot { .. }
        | BrokerCmd::ComputeAdxSnapshot { .. }
        | BrokerCmd::ComputeCciSnapshot { .. }
        | BrokerCmd::ComputeCmfSnapshot { .. }
        | BrokerCmd::ComputeMfiSnapshot { .. }
        | BrokerCmd::ComputePsarSnapshot { .. }
        | BrokerCmd::ComputeVortexSnapshot { .. }
        | BrokerCmd::ComputeChopSnapshot { .. }
        | BrokerCmd::ComputeObvSnapshot { .. }
        | BrokerCmd::ComputeTrixSnapshot { .. }
        | BrokerCmd::ComputeHmaSnapshot { .. }
        | BrokerCmd::ComputePpoSnapshot { .. }
        | BrokerCmd::ComputeDpoSnapshot { .. }
        | BrokerCmd::ComputeKstSnapshot { .. }
        | BrokerCmd::ComputeUltoscSnapshot { .. }
        | BrokerCmd::ComputeWillrSnapshot { .. }
        | BrokerCmd::ComputeMassSnapshot { .. }
        | BrokerCmd::ComputeChaikoscSnapshot { .. }
        | BrokerCmd::ComputeKlingerSnapshot { .. }
        | BrokerCmd::ComputeStochRsiSnapshot { .. }
        | BrokerCmd::ComputeAwesomeSnapshot { .. }
        | BrokerCmd::ComputeEfiSnapshot { .. }
        | BrokerCmd::ComputeEmvSnapshot { .. }
        | BrokerCmd::ComputeNviSnapshot { .. }
        | BrokerCmd::ComputePviSnapshot { .. }
        | BrokerCmd::ComputeCoppockSnapshot { .. }
        | BrokerCmd::ComputeCmoSnapshot { .. }
        | BrokerCmd::ComputeQstickSnapshot { .. }
        | BrokerCmd::ComputeDisparitySnapshot { .. }
        | BrokerCmd::ComputeBopSnapshot { .. }
        | BrokerCmd::ComputeSchaffSnapshot { .. }
        | BrokerCmd::ComputeStochSnapshot { .. }
        | BrokerCmd::ComputeMacdSnapshot { .. }
        | BrokerCmd::ComputeVwapSnapshot { .. }
        | BrokerCmd::ComputeMcgdSnapshot { .. }
        | BrokerCmd::ComputeRwiSnapshot { .. }
        | BrokerCmd::ComputeDemaSnapshot { .. }
        | BrokerCmd::ComputeTemaSnapshot { .. }
        | BrokerCmd::ComputeLinregSnapshot { .. }
        | BrokerCmd::ComputePivotsSnapshot { .. }
        | BrokerCmd::ComputeHeikinSnapshot { .. }
        | BrokerCmd::ComputeAlmaSnapshot { .. }
        | BrokerCmd::ComputeZlemaSnapshot { .. }
        | BrokerCmd::ComputeElderRaySnapshot { .. }
        | BrokerCmd::ComputeTsfSnapshot { .. }
        | BrokerCmd::ComputeRviSnapshot { .. }
        | BrokerCmd::ComputeTrimaSnapshot { .. }
        | BrokerCmd::ComputeT3Snapshot { .. }
        | BrokerCmd::ComputeVidyaSnapshot { .. }
        | BrokerCmd::ComputeSmiSnapshot { .. }
        | BrokerCmd::ComputePvtSnapshot { .. }
        | BrokerCmd::ComputeAcSnapshot { .. }
        | BrokerCmd::ComputeChvolSnapshot { .. }
        | BrokerCmd::ComputeBbwidthSnapshot { .. }
        | BrokerCmd::ComputeElderImpSnapshot { .. }
        | BrokerCmd::ComputeRmiSnapshot { .. }
        | BrokerCmd::ComputeSymbolExpirations { .. }
        | BrokerCmd::ComputeSmmaSnapshot { .. }
        | BrokerCmd::ComputeAlligatorSnapshot { .. }
        | BrokerCmd::ComputeCrsiSnapshot { .. }
        | BrokerCmd::ComputeSebSnapshot { .. }
        | BrokerCmd::ComputeImiSnapshot { .. }
        | BrokerCmd::ComputeGmmaSnapshot { .. }
        | BrokerCmd::ComputeMaenvSnapshot { .. }
        | BrokerCmd::ComputeAdlSnapshot { .. }
        | BrokerCmd::ComputeVhfSnapshot { .. }
        | BrokerCmd::ComputeVrocSnapshot { .. }
        | BrokerCmd::ComputeKdjSnapshot { .. }
        | BrokerCmd::ComputeQqeSnapshot { .. }
        | BrokerCmd::ComputePmoSnapshot { .. }
        | BrokerCmd::ComputeCfoSnapshot { .. }
        | BrokerCmd::ComputeTmfSnapshot { .. }
        | BrokerCmd::ComputeFractalsSnapshot { .. }
        | BrokerCmd::ComputeIftRsiSnapshot { .. }
        | BrokerCmd::ComputeMamaSnapshot { .. }
        | BrokerCmd::ComputeCogSnapshot { .. }
        | BrokerCmd::ComputeDidiSnapshot { .. }
        | BrokerCmd::ComputeDemarkerSnapshot { .. }
        | BrokerCmd::ComputeGatorSnapshot { .. }
        | BrokerCmd::ComputeBwMfiSnapshot { .. }
        | BrokerCmd::ComputeVwmaSnapshot { .. }
        | BrokerCmd::ComputeStddevSnapshot { .. }
        | BrokerCmd::ComputeWmaSnapshot { .. }
        | BrokerCmd::ComputeRainbowSnapshot { .. }
        | BrokerCmd::ComputeMesaSineSnapshot { .. }
        | BrokerCmd::ComputeFramaSnapshot { .. }
        | BrokerCmd::ComputeIbsSnapshot { .. }
        | BrokerCmd::ComputeLaguerreRsiSnapshot { .. }
        | BrokerCmd::ComputeZigzagSnapshot { .. }
        | BrokerCmd::ComputePgoSnapshot { .. }
        | BrokerCmd::ComputeHtTrendlineSnapshot { .. }
        | BrokerCmd::ComputeMidpointSnapshot { .. }
        | BrokerCmd::ComputeMassIndexSnapshot { .. }
        | BrokerCmd::ComputeNatrSnapshot { .. }
        | BrokerCmd::ComputeTtmSqueezeSnapshot { .. }
        | BrokerCmd::ComputeForceIndexSnapshot { .. }
        | BrokerCmd::ComputeTrangeSnapshot { .. }
        | BrokerCmd::ComputeLinearregSlopeSnapshot { .. }
        | BrokerCmd::ComputeHtDcperiodSnapshot { .. }
        | BrokerCmd::ComputeHtTrendmodeSnapshot { .. }
        | BrokerCmd::ComputeAccbandsSnapshot { .. }
        | BrokerCmd::ComputeStochfSnapshot { .. }
        | BrokerCmd::ComputeLinearregSnapshot { .. }
        | BrokerCmd::ComputeLinearregAngleSnapshot { .. }
        | BrokerCmd::ComputeHtDcphaseSnapshot { .. }
        | BrokerCmd::ComputeHtSineSnapshot { .. }
        | BrokerCmd::ComputeHtPhasorSnapshot { .. }
        | BrokerCmd::ComputeMidpriceSnapshot { .. }
        | BrokerCmd::ComputeApoSnapshot { .. }
        | BrokerCmd::ComputeMomSnapshot { .. }
        | BrokerCmd::ComputeSarextSnapshot { .. }
        | BrokerCmd::ComputeAdxrSnapshot { .. }
        | BrokerCmd::ComputeAvgpriceSnapshot { .. }
        | BrokerCmd::ComputeMedpriceSnapshot { .. }
        | BrokerCmd::ComputeTypPriceSnapshot { .. }
        | BrokerCmd::ComputeWclPriceSnapshot { .. }
        | BrokerCmd::ComputeVarianceSnapshot { .. }
        | BrokerCmd::ComputePlusDiSnapshot { .. }
        | BrokerCmd::ComputeMinusDiSnapshot { .. }
        | BrokerCmd::ComputePlusDmSnapshot { .. }
        | BrokerCmd::ComputeMinusDmSnapshot { .. }
        | BrokerCmd::ComputeDxSnapshot { .. }
        | BrokerCmd::ComputeRocSnapshot { .. }
        | BrokerCmd::ComputeRocpSnapshot { .. }
        | BrokerCmd::ComputeRocrSnapshot { .. }
        | BrokerCmd::ComputeRocr100Snapshot { .. }
        | BrokerCmd::ComputeCorrelSnapshot { .. }
        | BrokerCmd::ComputeMinSnapshot { .. }
        | BrokerCmd::ComputeMaxSnapshot { .. }
        | BrokerCmd::ComputeMinMaxSnapshot { .. }
        | BrokerCmd::ComputeMinIndexSnapshot { .. }
        | BrokerCmd::ComputeMaxIndexSnapshot { .. }
        | BrokerCmd::ComputeBbandsSnapshot { .. }
        | BrokerCmd::ComputeAdSnapshot { .. }
        | BrokerCmd::ComputeAdoscSnapshot { .. }
        | BrokerCmd::ComputeSumSnapshot { .. }
        | BrokerCmd::ComputeLinearRegInterceptSnapshot { .. }
        | BrokerCmd::ComputeAroonoscSnapshot { .. }
        | BrokerCmd::ComputeMinMaxIndexSnapshot { .. }
        | BrokerCmd::ComputeMacdextSnapshot { .. }
        | BrokerCmd::ComputeMacdfixSnapshot { .. }
        | BrokerCmd::ComputeMavpSnapshot { .. }
        | BrokerCmd::ComputeCdlDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlHammerSnapshot { .. }
        | BrokerCmd::ComputeCdlShootingStarSnapshot { .. }
        | BrokerCmd::ComputeCdlEngulfingSnapshot { .. }
        | BrokerCmd::ComputeCdlHaramiSnapshot { .. }
        | BrokerCmd::ComputeCdlMorningStarSnapshot { .. }
        | BrokerCmd::ComputeCdlEveningStarSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeBlackCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeWhiteSoldiersSnapshot { .. }
        | BrokerCmd::ComputeCdlDarkCloudCoverSnapshot { .. }
        | BrokerCmd::ComputeCdlPiercingSnapshot { .. }
        | BrokerCmd::ComputeCdlDragonflyDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlGravestoneDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlHangingManSnapshot { .. }
        | BrokerCmd::ComputeCdlInvertedHammerSnapshot { .. }
        | BrokerCmd::ComputeCdlHaramiCrossSnapshot { .. }
        | BrokerCmd::ComputeCdlLongLeggedDojiSnapshot { .. }
        | BrokerCmd::ComputeCdlMarubozuSnapshot { .. }
        | BrokerCmd::ComputeCdlSpinningTopSnapshot { .. }
        | BrokerCmd::ComputeCdlTristarSnapshot { .. }
        | BrokerCmd::ComputeCdlDojiStarSnapshot { .. }
        | BrokerCmd::ComputeCdlMorningDojiStarSnapshot { .. }
        | BrokerCmd::ComputeCdlEveningDojiStarSnapshot { .. }
        | BrokerCmd::ComputeCdlAbandonedBabySnapshot { .. }
        | BrokerCmd::ComputeCdlThreeInsideSnapshot { .. }
        | BrokerCmd::ComputeCdlBeltHoldSnapshot { .. }
        | BrokerCmd::ComputeCdlClosingMarubozuSnapshot { .. }
        | BrokerCmd::ComputeCdlHighWaveSnapshot { .. }
        | BrokerCmd::ComputeCdlLongLineSnapshot { .. }
        | BrokerCmd::ComputeCdlShortLineSnapshot { .. }
        | BrokerCmd::ComputeCdlCounterattackSnapshot { .. }
        | BrokerCmd::ComputeCdlHomingPigeonSnapshot { .. }
        | BrokerCmd::ComputeCdlInNeckSnapshot { .. }
        | BrokerCmd::ComputeCdlOnNeckSnapshot { .. }
        | BrokerCmd::ComputeCdlThrustingSnapshot { .. }
        | BrokerCmd::ComputeCdlTwoCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeLineStrikeSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeOutsideSnapshot { .. }
        | BrokerCmd::ComputeCdlMatchingLowSnapshot { .. }
        | BrokerCmd::ComputeCdlSeparatingLinesSnapshot { .. }
        | BrokerCmd::ComputeCdlStickSandwichSnapshot { .. }
        | BrokerCmd::ComputeCdlRickshawManSnapshot { .. }
        | BrokerCmd::ComputeCdlTakuriSnapshot { .. }
        | BrokerCmd::ComputeCdlThreeStarsInSouthSnapshot { .. }
        | BrokerCmd::ComputeCdlIdenticalThreeCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlKickingSnapshot { .. }
        | BrokerCmd::ComputeCdlKickingByLengthSnapshot { .. }
        | BrokerCmd::ComputeCdlLadderBottomSnapshot { .. }
        | BrokerCmd::ComputeCdlUniqueThreeRiverSnapshot { .. }
        | BrokerCmd::ComputeCdlAdvanceBlockSnapshot { .. }
        | BrokerCmd::ComputeCdlBreakawaySnapshot { .. }
        | BrokerCmd::ComputeCdlGapSideSideWhiteSnapshot { .. }
        | BrokerCmd::ComputeCdlUpsideGapTwoCrowsSnapshot { .. }
        | BrokerCmd::ComputeCdlXSideGapThreeMethodsSnapshot { .. }
        | BrokerCmd::ComputeCdlConcealBabySwallowSnapshot { .. }
        | BrokerCmd::ComputeCdlHikkakeSnapshot { .. }
        | BrokerCmd::ComputeCdlHikkakeModSnapshot { .. }
        | BrokerCmd::ComputeCdlMatHoldSnapshot { .. }
        | BrokerCmd::ComputeCdlRiseFallThreeMethodsSnapshot { .. }
        | BrokerCmd::ComputeCdlStalledPatternSnapshot { .. }
        | BrokerCmd::ComputeCdlTasukiGapSnapshot { .. }) => {
            technical_indicators::handle_technical_indicator_command(
                cmd,
                broker_msg_tx_clone.clone(),
                shared_cache_broker.clone(),
            );
        }
        BrokerCmd::ComputeModSharpeSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_modsharpe_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_modsharpe(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ModSharpeSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHsiehTestSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_hsieh_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_hsiehtest(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HsiehTestSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeChowBreakSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_chowbreak_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_chowbreak(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::ChowBreakSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDriftBurstSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_driftburst_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_driftburst(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DriftBurstSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeHlvClustSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_hlvclust_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_hlvclust(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::HlvClustSnapshotMsg(symbol, snap));
            });
        }
        // Quant-statistics validation, break-test, and coverage-test research
        BrokerCmd::ComputeYangZhangSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_yangzhang_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_yangzhang(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::YangZhangSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKuiperSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_kuiper_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_kuiper(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::KuiperSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeDagostinoSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_dagostino_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_dagostino(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::DagostinoSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeBaiPerronSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_baiperron_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_baiperron(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::BaiPerronSnapshotMsg(symbol, snap));
            });
        }
        BrokerCmd::ComputeKupiecPofSnapshot { symbol } => {
            use typhoon_engine::core::research;
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let bars =
                    if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                        if let Ok(conn) = cache.connection() {
                            research::get_historical_price(&conn, &symbol)
                                .ok()
                                .flatten()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                let snap = research::compute_kupiecpof_snapshot(&symbol, &today, &bars);
                if let Some(cache) = shared_cache_broker.read().ok().and_then(|g| g.clone()) {
                    if let Ok(conn) = cache.connection() {
                        let _ = research::upsert_kupiecpof(&conn, &symbol, &snap);
                    }
                }
                let _ = msg_tx.send(BrokerMsg::KupiecPofSnapshotMsg(symbol, snap));
            });
        }
        _ => { /* not risk */ }
    }
}
