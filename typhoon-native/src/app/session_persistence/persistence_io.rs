use super::*;

impl TyphooNApp {
    pub(in crate::app) fn session_json_path() -> PathBuf {
        let mut path = dirs_home();
        path.push("session.json");
        path
    }

    pub(in crate::app) fn write_session_json(json: &str) {
        let path = Self::session_json_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, json);
    }

    /// Persist a session snapshot (session.json + the `app:sync_preferences` KV
    /// row), newest-wins. `seq` is the snapshot's monotonic write sequence; the
    /// shared `gate` holds the highest sequence already written, so a stale
    /// (lower-seq) write that lost a race against a newer one is dropped instead
    /// of clobbering it. Safe to call from a blocking worker or the UI thread —
    /// the `put_kv` here is what contends with bulk bar-sync writers, which is
    /// exactly why the per-frame autosave runs this off the render thread.
    fn persist_session_to_disk(
        gate: &std::sync::Mutex<u64>,
        seq: u64,
        session_json: &str,
        pref_json: &str,
        cache: Option<&Arc<SqliteCache>>,
    ) {
        let mut persisted = gate.lock().unwrap_or_else(|p| p.into_inner());
        if seq <= *persisted {
            return;
        }
        Self::write_session_json(session_json);
        if let Some(cache) = cache {
            let _ = cache.put_kv("app:sync_preferences", pref_json);
        }
        *persisted = seq;
    }

    pub(in crate::app) fn mark_session_snapshot_clean(&mut self) {
        self.session_last_saved_json = self.build_session_json();
        self.session_dirty_since = None;
        self.session_last_scan_at = std::time::Instant::now();
        self.session_state_ready = true;
    }

    pub(in crate::app) fn hydrate_loaded_charts(&mut self) {
        let Some(ref cache) = self.cache else {
            return;
        };
        if self.charts.is_empty() {
            return;
        }
        self.active_tab = self.active_tab.min(self.charts.len().saturating_sub(1));
        // Eagerly load ONLY the focused tab (O(1)) so the chart the user is looking
        // at is ready on the first frame. Every other empty chart — the rest of an
        // MTF grid — is collected by the deferred-load queue build that follows in
        // `tick_cache_startup` and loaded off the render thread
        // (`spawn_deferred_chart_load`). The old MTF branch synchronously `try_load`'d
        // *every* chart here in a loop, which was the first-frame startup freeze.
        if let Some(chart) = self.charts.get_mut(self.active_tab) {
            if chart.bars.is_empty() {
                let mut gpu = self.gpu_indicators.take();
                let loaded = chart.try_load(cache, &mut self.log, gpu.as_mut());
                self.gpu_indicators = gpu;
                if !loaded {
                    self.queue_chart_reload(self.active_tab);
                }
            }
        }
    }

    pub(in crate::app) fn maybe_incremental_session_save(&mut self, ctx: &egui::Context) {
        self.flush_alpaca_retry_queue(false);
        self.flush_alpaca_no_data_marks(false);
        self.flush_unresolvable_marks(false);
        self.flush_alpaca_backfill_complete_marks(false);
        self.flush_kraken_backfill_complete_marks(false);
        if self.heavy_sync_in_progress {
            // build_session_json() walks a large amount of UI/session state and
            // write_session_json()/sync_preferences_save() hit disk/SQLite. During
            // startup/full-catalog sync those background states churn constantly,
            // turning autosave into periodic render-thread stalls. Forced saves on
            // exit still persist the latest state; keep the frame loop responsive.
            return;
        }
        if !self.session_state_ready {
            return;
        }
        let now = std::time::Instant::now();
        // Adaptive scan cadence: 500ms while the session is actively changing,
        // backing off toward 2s after sustained no-change so an idle terminal
        // isn't rebuilding+diffing the session JSON twice a second for nothing.
        // Any detected change resets to the fast cadence; the save debounce and
        // forced/exit saves are unaffected.
        let scan_interval =
            std::time::Duration::from_millis(500 + u64::from(self.session_idle_scans.min(6)) * 250);
        let save_debounce = std::time::Duration::from_millis(1200);
        let since_last_scan = now.saturating_duration_since(self.session_last_scan_at);
        if since_last_scan < scan_interval {
            return;
        }
        self.session_last_scan_at = now;
        let json = self.build_session_json();
        if json == self.session_last_saved_json {
            self.session_idle_scans = self.session_idle_scans.saturating_add(1);
            self.session_dirty_since = None;
            return;
        }
        self.session_idle_scans = 0;
        let dirty_since = self.session_dirty_since.get_or_insert(now);
        let dirty_for = now.saturating_duration_since(*dirty_since);
        if dirty_for < save_debounce {
            ctx.request_repaint_after(save_debounce - dirty_for);
            return;
        }
        // A prior off-thread autosave is still writing. Don't pile up a second
        // worker or block the render thread — leave the dirty flag set and retry
        // next scan so the newest state is what eventually lands on disk.
        if self
            .session_save_in_flight
            .load(std::sync::atomic::Ordering::Acquire)
        {
            ctx.request_repaint_after(scan_interval);
            return;
        }
        // Build the small (~6 KB) preference blob on the UI thread (cheap), then
        // hand the session.json write + SQLite put_kv to a blocking worker. The
        // render thread no longer waits on the shared cache write mutex — held
        // for seconds by bulk bar-sync writers — which was the dominant source
        // of the multi-second frame stalls.
        self.session_save_seq += 1;
        let seq = self.session_save_seq;
        let pref_json =
            serde_json::to_string(&self.build_sync_preferences_value()).unwrap_or_default();
        let gate = self.session_write_gate.clone();
        let in_flight = self.session_save_in_flight.clone();
        let cache = self.cache.clone();
        let json_for_disk = json.clone();
        in_flight.store(true, std::sync::atomic::Ordering::Release);
        self.rt_handle.spawn_blocking(move || {
            Self::persist_session_to_disk(&gate, seq, &json_for_disk, &pref_json, cache.as_ref());
            in_flight.store(false, std::sync::atomic::Ordering::Release);
        });
        self.session_last_saved_json = json;
        self.session_dirty_since = None;
    }

    pub(in crate::app) fn save_session(&mut self) {
        self.flush_alpaca_retry_queue(true);
        self.flush_alpaca_no_data_marks(true);
        self.flush_unresolvable_marks(true);
        self.flush_alpaca_backfill_complete_marks(true);
        self.flush_kraken_backfill_complete_marks(true);
        // Persist credentials to keyring + SQLite fallback — on background thread to avoid UI freeze
        // (each keyring::store can take 50-200ms on Linux due to DBUS roundtrip × 11 keys = 1-2s freeze)
        let cred_pairs: Vec<(String, String)> = vec![
            (
                keyring::keys::ALPACA_API_KEY.into(),
                self.broker_api_key.clone(),
            ),
            (
                keyring::keys::ALPACA_SECRET.into(),
                self.broker_secret.clone(),
            ),
            (keyring::keys::FINNHUB_KEY.into(), self.finnhub_key.clone()),
            (keyring::keys::FRED_KEY.into(), self.fred_key.clone()),
            (
                keyring::keys::DISCORD_WEBHOOK.into(),
                self.discord_webhook.clone(),
            ),
            (
                keyring::keys::PUSHOVER_TOKEN.into(),
                self.pushover_token.clone(),
            ),
            (
                keyring::keys::PUSHOVER_USER.into(),
                self.pushover_user.clone(),
            ),
            (keyring::keys::NTFY_TOPIC.into(), self.ntfy_topic.clone()),
            (
                keyring::keys::ANTHROPIC_KEY.into(),
                self.anthropic_key.clone(),
            ),
            (keyring::keys::OPENAI_KEY.into(), self.openai_key.clone()),
            (
                keyring::keys::KRAKEN_API_KEY.into(),
                self.kraken_api_key.clone(),
            ),
            (
                keyring::keys::KRAKEN_API_SECRET.into(),
                self.kraken_api_secret.clone(),
            ),
            (
                keyring::keys::KRAKEN_WS_API_KEY.into(),
                self.kraken_ws_api_key.clone(),
            ),
            (
                keyring::keys::KRAKEN_WS_API_SECRET.into(),
                self.kraken_ws_api_secret.clone(),
            ),
            (
                keyring::keys::CRYPTOPANIC_KEY.into(),
                self.cryptopanic_key.clone(),
            ),
        ];
        // Extra broker account slots (ADR-130) — only non-empty pairs, so a
        // slot cleared in Settings isn't resurrected by autosave.
        let mut cred_pairs = cred_pairs;
        for (idx, acct) in self.alpaca_extra_accounts.iter().enumerate() {
            if acct.api_key.trim().is_empty() || acct.secret.trim().is_empty() {
                continue;
            }
            let (key_name, secret_name) = super::super::broker_accounts::alpaca_slot_keyring_keys(idx + 2);
            cred_pairs.push((key_name, acct.api_key.clone()));
            cred_pairs.push((secret_name, acct.secret.clone()));
        }
        for (idx, acct) in self.kraken_extra_accounts.iter().enumerate() {
            if acct.api_key.trim().is_empty() || acct.secret.trim().is_empty() {
                continue;
            }
            let (key_name, secret_name) = super::super::broker_accounts::kraken_slot_keyring_keys(idx + 2);
            cred_pairs.push((key_name, acct.api_key.clone()));
            cred_pairs.push((secret_name, acct.secret.clone()));
        }
        let cred_pairs = cred_pairs;
        let cache_clone = self.cache.clone();
        let rt_handle = self.rt_handle.clone();
        rt_handle.spawn_blocking(move || {
            for (key, val) in &cred_pairs {
                let _ = keyring::store(key, val);
                if let Some(ref cache) = cache_clone {
                    let _ = cache.put_kv(&format!("cred:{}", key), val);
                }
            }
        });
        // Explicit save stays synchronous (atomic on the UI thread), but routes
        // through the write gate with a fresh, highest sequence so it always
        // wins over an in-flight background autosave that may finish afterward.
        let json = self.build_session_json();
        self.session_save_seq += 1;
        let seq = self.session_save_seq;
        let pref_json =
            serde_json::to_string(&self.build_sync_preferences_value()).unwrap_or_default();
        // Route the session.json write + SQLite put_kv off the UI thread (mirrors
        // the per-frame autosave). The monotonic `seq` + write gate guarantee this
        // explicit save still wins over any in-flight autosave, while keeping the
        // blocking put_kv — held for seconds by bulk bar-sync writers — off the
        // render thread (it was a 4 s+ frame stall during heavy sync).
        let gate = self.session_write_gate.clone();
        let cache = self.cache.clone();
        let json_for_disk = json.clone();
        self.rt_handle.spawn_blocking(move || {
            Self::persist_session_to_disk(&gate, seq, &json_for_disk, &pref_json, cache.as_ref());
        });
        self.session_last_saved_json = json;
        self.session_dirty_since = None;
        self.session_last_scan_at = std::time::Instant::now();
        self.session_state_ready = true;
    }
}
