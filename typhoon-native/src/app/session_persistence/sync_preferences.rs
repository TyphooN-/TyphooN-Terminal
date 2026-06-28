use super::*;

pub(super) fn persisted_bar_zstd_level(value: &serde_json::Value, current: i32) -> i32 {
    value["bar_zstd_level"]
        .as_i64()
        .map(|level| level as i32)
        .unwrap_or(current)
        .clamp(
            typhoon_engine::core::cache::MIN_ZSTD_LEVEL,
            typhoon_engine::core::cache::MAX_ZSTD_LEVEL,
        )
}

impl TyphooNApp {
    pub(in crate::app) fn build_sync_preferences_value(&self) -> serde_json::Value {
        serde_json::json!({
            "kraken_scrape_schema": 3,
            "alpaca_enabled": self.alpaca_enabled,
            "alpaca_full_bar_sync_enabled": self.alpaca_full_bar_sync_enabled,
            "kraken_enabled": self.kraken_enabled,
            "kraken_full_bar_sync_enabled": self.kraken_full_bar_sync_enabled,
            "primary_broker": self.primary_broker.as_persist_str(),
            "kraken_scrape_xstocks": self.kraken_scrape_xstocks,
            "kraken_scrape_usd_crypto": self.kraken_scrape_usd_crypto,
            "kraken_scrape_fiat_crypto": self.kraken_scrape_fiat_crypto,
            "kraken_scrape_crypto_crosses": self.kraken_scrape_crypto_crosses,
            "kraken_scrape_futures": self.kraken_scrape_futures,
            "backfill_alpaca_kraken_equities_enabled": self.backfill_alpaca_kraken_equities_enabled,
            "backfill_yahoo_chart_enabled": self.backfill_yahoo_chart_enabled,
            "kraken_ws_ohlc_enabled": self.kraken_ws_ohlc_enabled,
            "crypto_fiat_quote_usd": self.crypto_fiat_quote_usd,
            "crypto_fiat_quote_usdt": self.crypto_fiat_quote_usdt,
            "crypto_fiat_quote_usdc": self.crypto_fiat_quote_usdc,
            "crypto_fiat_quote_usdg": self.crypto_fiat_quote_usdg,
            "crypto_fiat_quote_eur": self.crypto_fiat_quote_eur,
            "crypto_fiat_quote_gbp": self.crypto_fiat_quote_gbp,
            "crypto_fiat_quote_cad": self.crypto_fiat_quote_cad,
            "crypto_fiat_quote_aud": self.crypto_fiat_quote_aud,
            "crypto_fiat_quote_jpy": self.crypto_fiat_quote_jpy,
            "crypto_fiat_quote_chf": self.crypto_fiat_quote_chf,
            "fund_source_alpaca": self.fund_source_alpaca,
            "fund_source_kraken": self.fund_source_kraken,
            "enabled_sync_timeframes": STANDARD_SYNC_TIMEFRAMES.iter()
                .filter_map(|(_, tf)| self.enabled_sync_timeframes.contains(*tf).then(|| serde_json::json!(tf)))
                .collect::<Vec<_>>(),
            "alpaca_historical_rpm_hint": self.alpaca_historical_rpm_hint,
            "bar_zstd_level": self.bar_zstd_level,
            "auto_compact_enabled": self.auto_compact_enabled,
            "auto_compact_last_run_ms": self.auto_compact_last_run_ms,
            "auto_compact_cadence_days": self.auto_compact_schedule.cadence_days,
            "auto_compact_window_weekday": self.auto_compact_schedule.window_weekday,
            "auto_compact_window_hour_start": self.auto_compact_schedule.window_hour_start,
            "auto_compact_window_hour_end": self.auto_compact_schedule.window_hour_end,
            "auto_compact_uncompacted_threshold": self.auto_compact_schedule.uncompacted_threshold,
        })
    }

    pub(in crate::app) fn apply_sync_preferences_value(&mut self, value: &serde_json::Value) {
        let kraken_scrape_schema = value["kraken_scrape_schema"].as_u64().unwrap_or(1);
        if let Some(enabled) = value["alpaca_enabled"].as_bool() {
            self.alpaca_enabled = enabled;
        }
        if let Some(enabled) = value["alpaca_full_bar_sync_enabled"].as_bool() {
            self.alpaca_full_bar_sync_enabled = enabled;
        }
        if let Some(enabled) = value["kraken_full_bar_sync_enabled"].as_bool() {
            self.kraken_full_bar_sync_enabled = enabled;
        }
        if let Some(enabled) = value["kraken_enabled"].as_bool() {
            self.kraken_enabled = enabled;
        }
        if let Some(primary) = value["primary_broker"]
            .as_str()
            .and_then(OrderBroker::from_persist_str)
        {
            self.primary_broker = primary;
            // Routing default follows the persisted primary; resolve_order_broker
            // re-points only if the primary is unavailable once brokers connect.
            self.order_broker = primary;
            // Mirror into the merge's process-wide selection (ADR-126).
            set_chart_merge_primary_broker(primary);
        }
        if let Some(enabled) = value["kraken_scrape_xstocks"].as_bool() {
            self.kraken_scrape_xstocks = enabled;
        }
        if let Some(enabled) = value["kraken_scrape_usd_crypto"].as_bool() {
            self.kraken_scrape_usd_crypto = enabled;
        }
        if let Some(enabled) = value["kraken_scrape_fiat_crypto"].as_bool() {
            self.kraken_scrape_fiat_crypto = enabled;
        }
        if let Some(enabled) = value["kraken_scrape_crypto_crosses"].as_bool() {
            self.kraken_scrape_crypto_crosses = enabled;
        }
        if kraken_scrape_schema < 2 {
            self.kraken_scrape_fiat_crypto = true;
            self.kraken_scrape_crypto_crosses = true;
        }
        if kraken_scrape_schema < 3 {
            self.crypto_fiat_quote_usd = self.kraken_scrape_usd_crypto;
            self.crypto_fiat_quote_usdt = self.kraken_scrape_usd_crypto;
            self.crypto_fiat_quote_usdc = self.kraken_scrape_usd_crypto;
            self.crypto_fiat_quote_usdg = self.kraken_scrape_usd_crypto;
            self.crypto_fiat_quote_eur = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_gbp = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_cad = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_aud = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_jpy = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_chf = self.kraken_scrape_fiat_crypto;
        } else {
            if let Some(enabled) = value["crypto_fiat_quote_usd"].as_bool() {
                self.crypto_fiat_quote_usd = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_usdt"].as_bool() {
                self.crypto_fiat_quote_usdt = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_usdc"].as_bool() {
                self.crypto_fiat_quote_usdc = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_usdg"].as_bool() {
                self.crypto_fiat_quote_usdg = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_eur"].as_bool() {
                self.crypto_fiat_quote_eur = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_gbp"].as_bool() {
                self.crypto_fiat_quote_gbp = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_cad"].as_bool() {
                self.crypto_fiat_quote_cad = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_aud"].as_bool() {
                self.crypto_fiat_quote_aud = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_jpy"].as_bool() {
                self.crypto_fiat_quote_jpy = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_chf"].as_bool() {
                self.crypto_fiat_quote_chf = enabled;
            }
        }
        self.kraken_scrape_usd_crypto = self.crypto_fiat_quote_usd
            || self.crypto_fiat_quote_usdt
            || self.crypto_fiat_quote_usdc
            || self.crypto_fiat_quote_usdg;
        self.kraken_scrape_fiat_crypto = self.crypto_fiat_quote_eur
            || self.crypto_fiat_quote_gbp
            || self.crypto_fiat_quote_cad
            || self.crypto_fiat_quote_aud
            || self.crypto_fiat_quote_jpy
            || self.crypto_fiat_quote_chf;
        if let Some(enabled) = value["kraken_scrape_futures"].as_bool() {
            self.kraken_scrape_futures = enabled;
        }
        if let Some(enabled) = value["backfill_alpaca_kraken_equities_enabled"].as_bool() {
            self.backfill_alpaca_kraken_equities_enabled = enabled;
        }
        if let Some(enabled) = value["backfill_yahoo_chart_enabled"].as_bool() {
            self.backfill_yahoo_chart_enabled = enabled;
        }

        if let Some(enabled) = value["kraken_ws_ohlc_enabled"].as_bool() {
            self.kraken_ws_ohlc_enabled = enabled;
        }
        if let Some(arr) = value["enabled_sync_timeframes"].as_array() {
            self.enabled_sync_timeframes = arr
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(normalize_sync_timeframe_key)
                .map(str::to_string)
                .collect();
        }
        if let Some(rpm_hint) = value["alpaca_historical_rpm_hint"].as_u64() {
            self.alpaca_historical_rpm_hint = (rpm_hint as u32).min(100_000);
        }
        self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(
            persisted_bar_zstd_level(value, self.bar_zstd_level),
        );
        if let Some(b) = value["auto_compact_enabled"].as_bool() {
            self.auto_compact_enabled = b;
        }
        if let Some(ms) = value["auto_compact_last_run_ms"].as_i64() {
            self.auto_compact_last_run_ms = ms;
        }
        let mut schedule = self.auto_compact_schedule;
        if let Some(days) = value["auto_compact_cadence_days"].as_i64() {
            schedule.cadence_days = days;
        }
        if let Some(weekday) = value["auto_compact_window_weekday"].as_u64() {
            schedule.window_weekday = weekday as u32;
        }
        if let Some(hour) = value["auto_compact_window_hour_start"].as_u64() {
            schedule.window_hour_start = hour as u32;
        }
        if let Some(hour) = value["auto_compact_window_hour_end"].as_u64() {
            schedule.window_hour_end = hour as u32;
        }
        if let Some(threshold) = value["auto_compact_uncompacted_threshold"].as_i64() {
            schedule.uncompacted_threshold = threshold;
        }
        self.auto_compact_schedule = schedule.sanitized();
    }

    pub(in crate::app) fn sync_preferences_save(&self) {
        if let Some(ref cache) = self.cache {
            let json =
                serde_json::to_string(&self.build_sync_preferences_value()).unwrap_or_default();
            let _ = cache.put_kv("app:sync_preferences", &json);
        }
    }

    pub(in crate::app) fn sync_preferences_load(&mut self) {
        if let Some(ref cache) = self.cache {
            if let Ok(Some(json)) = cache.get_kv("app:sync_preferences") {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
                    self.apply_sync_preferences_value(&value);
                }
            }
        }
    }

    /// Auto-compact scheduler tick. Cheap on the steady-state path: returns
    /// immediately if the next-check throttle hasn't elapsed. ADR-089.
    pub(in crate::app) fn tick_auto_compact(&mut self) {
        let now = std::time::Instant::now();
        if now < self.auto_compact_next_check_at {
            return;
        }
        // Re-evaluate at most once per minute regardless of outcome.
        self.auto_compact_next_check_at = now + std::time::Duration::from_secs(60);

        let now_ms = chrono::Utc::now().timestamp_millis();
        // Stale-flag guard: if a compact has been "in progress" for longer than
        // any sane run (8h), assume the completion log was lost and reset so the
        // gate can recover on its own.
        if self.auto_compact_in_progress {
            let stale_after_ms: i64 = 8 * 60 * 60 * 1000;
            if self.auto_compact_started_ms <= 0
                || (now_ms - self.auto_compact_started_ms) > stale_after_ms
            {
                self.auto_compact_in_progress = false;
                self.auto_compact_started_ms = 0;
                self.auto_compact_last_skip =
                    Some("previous compact run timed out after 8h".to_string());
            }
        }

        let cache = match self.cache.clone() {
            Some(c) => c,
            None => return,
        };
        let uncompacted = cache
            .count_uncompacted_bars(auto_compact::TARGET_LEVEL)
            .unwrap_or(0);

        let (weekday, hour) = auto_compact::local_weekday_hour_now();
        let idle_for = now
            .saturating_duration_since(self.auto_compact_last_input_at)
            .as_secs();
        let inputs = auto_compact::GateInputs {
            enabled: self.auto_compact_enabled,
            schedule: self.auto_compact_schedule,
            last_run_ms: self.auto_compact_last_run_ms,
            now_ms,
            local_weekday: weekday,
            local_hour: hour,
            idle_for_secs: idle_for,
            on_ac: auto_compact::on_ac_power(),
            uncompacted_count: uncompacted,
            in_progress: self.auto_compact_in_progress,
            heavy_sync: self.heavy_sync_in_progress,
        };
        let decision = auto_compact::evaluate_gate(&inputs);
        if !decision.run {
            self.auto_compact_last_skip = Some(decision.reason);
            return;
        }

        // Gate passed — dispatch the same BrokerCmd the manual button uses, so
        // the existing importing_flag coordination and progress logging apply.
        let db_path = cache_db_path();
        let _ = self.broker_tx.send(BrokerCmd::CompactStorage {
            db_path,
            level: auto_compact::TARGET_LEVEL,
        });
        self.auto_compact_in_progress = true;
        self.auto_compact_started_ms = now_ms;
        self.auto_compact_last_skip = None;
        self.log.push_back(LogEntry::info(format!(
            "Auto-compact (zstd-{}): {} entries pending — running in background",
            auto_compact::TARGET_LEVEL,
            uncompacted
        )));
    }
}
