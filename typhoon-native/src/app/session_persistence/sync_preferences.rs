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
            // Multi-account metadata (ADR-130). Credentials stay in the
            // keyring; only labels/flags/primary selection persist here.
            "alpaca_primary_account_id": self.alpaca_primary_account_id,
            "kraken_primary_account_id": self.kraken_primary_account_id,
            "alpaca_extra_accounts": self.alpaca_extra_accounts,
            "kraken_extra_accounts": self.kraken_extra_accounts,
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
            self.primary_cycle_dirty = true;
        }
        if let Some(enabled) = value["alpaca_full_bar_sync_enabled"].as_bool() {
            self.alpaca_full_bar_sync_enabled = enabled;
        }
        if let Some(enabled) = value["kraken_full_bar_sync_enabled"].as_bool() {
            self.kraken_full_bar_sync_enabled = enabled;
        }
        if let Some(enabled) = value["kraken_enabled"].as_bool() {
            self.kraken_enabled = enabled;
            self.primary_cycle_dirty = true;
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
        if let Some(id) = value["alpaca_primary_account_id"].as_str() {
            if !id.is_empty() {
                self.alpaca_primary_account_id = id.to_string();
            }
        }
        if let Some(id) = value["kraken_primary_account_id"].as_str() {
            if !id.is_empty() {
                self.kraken_primary_account_id = id.to_string();
            }
        }
        // Account-slot metadata: rebuild the slot Vec to the *persisted* count
        // (dynamic accounts — the user may have added or removed slots), keeping
        // any creds already in memory for slots that still exist. Creds for new
        // slots arrive right after from the keyring; `#[serde(skip)]` keeps them
        // out of this JSON. Bounded by the cap so a corrupt value can't run away.
        let rebuild_slots = |persisted: Vec<ExtraAccountConfig>,
                             existing: &[ExtraAccountConfig]|
         -> Vec<ExtraAccountConfig> {
            let mut rebuilt = persisted;
            rebuilt.truncate(super::super::broker_accounts::BROKER_ACCOUNT_SLOT_CAP - 1);
            for (idx, slot) in rebuilt.iter_mut().enumerate() {
                if let Some(have) = existing.get(idx) {
                    slot.api_key = have.api_key.clone();
                    slot.secret = have.secret.clone();
                }
            }
            rebuilt
        };
        if let Ok(extra) = serde_json::from_value::<Vec<ExtraAccountConfig>>(
            value["alpaca_extra_accounts"].clone(),
        ) {
            let rebuilt = rebuild_slots(extra, &self.alpaca_extra_accounts);
            self.alpaca_extra_accounts = rebuilt;
        }
        if let Ok(extra) = serde_json::from_value::<Vec<ExtraAccountConfig>>(
            value["kraken_extra_accounts"].clone(),
        ) {
            let rebuilt = rebuild_slots(extra, &self.kraken_extra_accounts);
            self.kraken_extra_accounts = rebuilt;
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

    pub(in crate::app) fn sync_preferences_save(&mut self) {
        let session_json = self.build_session_json();
        let pref_json =
            serde_json::to_string(&self.build_sync_preferences_value()).unwrap_or_default();
        // Preference-only writes cannot safely race full-session snapshots: an
        // older explicit save could otherwise overwrite this value after it
        // lands. Persist a complete, newer session snapshot through the same
        // sequence gate instead. The contended SQLite write remains off egui.
        self.session_save_seq = self.session_save_seq.wrapping_add(1);
        let seq = self.session_save_seq;
        let gate = self.session_write_gate.clone();
        let cache = self.cache.clone();
        self.rt_handle.spawn_blocking(move || {
            Self::persist_session_to_disk(&gate, seq, &session_json, &pref_json, cache.as_ref());
        });
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

        let (weekday, hour) = auto_compact::local_weekday_hour_now();
        let idle_for = now
            .saturating_duration_since(self.auto_compact_last_input_at)
            .as_secs();
        let schedule = self.auto_compact_schedule.sanitized();

        // `count_uncompacted_bars` scans storage and showed up as 200–370ms
        // pre-broker stalls while heavy sync was active. Evaluate the cheap gates
        // first; only count rows once a run is otherwise eligible.
        let cheap_inputs = auto_compact::GateInputs {
            enabled: self.auto_compact_enabled,
            schedule,
            last_run_ms: self.auto_compact_last_run_ms,
            now_ms,
            local_weekday: weekday,
            local_hour: hour,
            idle_for_secs: idle_for,
            on_ac: auto_compact::on_ac_power(),
            uncompacted_count: i64::MAX,
            in_progress: self.auto_compact_in_progress,
            heavy_sync: self.heavy_sync_in_progress,
        };
        let cheap_decision = auto_compact::evaluate_gate(&cheap_inputs);
        if !cheap_decision.run && !cheap_decision.reason.starts_with("only ") {
            self.auto_compact_last_skip = Some(cheap_decision.reason);
            return;
        }

        let cache = match self.cache.clone() {
            Some(c) => c,
            None => return,
        };
        let uncompacted = cache
            .count_uncompacted_bars(auto_compact::TARGET_LEVEL)
            .unwrap_or(0);

        let inputs = auto_compact::GateInputs {
            enabled: self.auto_compact_enabled,
            schedule,
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
