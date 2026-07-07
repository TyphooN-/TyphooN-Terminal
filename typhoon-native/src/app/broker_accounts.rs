//! Multi-account broker plumbing (ADR-130).
//!
//! Alpaca's free tier allows 1 live + 3 paper accounts, each with its own
//! market-data rate limit; connecting all of them multiplies historical
//! bar-sync throughput. Slot 1 is the legacy single-account credential pair
//! (`broker_api_key`/`broker_secret`, keyring `alpaca_api_key`); slots 2–4
//! store credentials under per-slot keyring keys and metadata in the session.
//! Kraken slots are trading identities only (its market data is public).

use super::*;

/// Default number of account slots a fresh install starts with (slot 1 + 3
/// extras) — Alpaca's free tier of 1 live + 3 paper.
pub(crate) const MAX_BROKER_ACCOUNT_SLOTS: usize = 4;

/// Hard cap on total account slots per broker (slot 1 + extras). Users with more
/// paid/paper accounts can grow past the free-tier default via Settings; this
/// only bounds the UI so the slot Vec can't run away. Keyring keys are per-slot
/// (`{key}_{slot}`), so any slot number up to the cap is addressable.
pub(crate) const BROKER_ACCOUNT_SLOT_CAP: usize = 16;

/// Whether another account slot can be added without exceeding the cap (total
/// slots = slot 1 + `extra_len` extras).
pub(crate) fn can_add_account_slot(extra_len: usize) -> bool {
    extra_len + 1 < BROKER_ACCOUNT_SLOT_CAP
}

/// The primary-account id after removing extra slot `removed_slot` (2-based),
/// or `None` when the current primary is unaffected. Slots at/above the removed
/// one shift down, so a primary pointing there no longer names the same account
/// — fall back to slot 1. A primary on slot 1 (or a lower-numbered slot) is
/// stable. `prefix` is `"alpaca"` or `"kraken"`.
pub(crate) fn primary_after_slot_removal(
    primary_id: &str,
    prefix: &str,
    removed_slot: usize,
) -> Option<String> {
    let n = primary_id.strip_prefix(prefix)?.parse::<usize>().ok()?;
    (n >= removed_slot).then(|| format!("{prefix}1"))
}

pub(crate) fn default_alpaca_extra_accounts() -> Vec<ExtraAccountConfig> {
    (2..=MAX_BROKER_ACCOUNT_SLOTS)
        .map(|_| ExtraAccountConfig {
            api_key: String::new(),
            secret: String::new(),
            paper: true,
        })
        .collect()
}

pub(crate) fn default_kraken_extra_accounts() -> Vec<ExtraAccountConfig> {
    (2..=MAX_BROKER_ACCOUNT_SLOTS)
        .map(|_| ExtraAccountConfig {
            api_key: String::new(),
            secret: String::new(),
            paper: false,
        })
        .collect()
}

/// Keyring key names for an Alpaca account slot (1-based). Slot 1 maps to the
/// legacy single-account keys so existing installs keep working unchanged.
pub(crate) fn alpaca_slot_keyring_keys(slot: usize) -> (String, String) {
    if slot <= 1 {
        (
            keyring::keys::ALPACA_API_KEY.to_string(),
            keyring::keys::ALPACA_SECRET.to_string(),
        )
    } else {
        (
            format!("{}_{slot}", keyring::keys::ALPACA_API_KEY),
            format!("{}_{slot}", keyring::keys::ALPACA_SECRET),
        )
    }
}

pub(crate) fn kraken_slot_keyring_keys(slot: usize) -> (String, String) {
    if slot <= 1 {
        (
            keyring::keys::KRAKEN_API_KEY.to_string(),
            keyring::keys::KRAKEN_API_SECRET.to_string(),
        )
    } else {
        (
            format!("{}_{slot}", keyring::keys::KRAKEN_API_KEY),
            format!("{}_{slot}", keyring::keys::KRAKEN_API_SECRET),
        )
    }
}

impl TyphooNApp {
    /// Persist one credential field to the keyring + SQLite `cred:` fallback as
    /// soon as it is edited in Settings (a non-empty value stores, an emptied
    /// field deletes). Runs off the render thread; previously slot creds were
    /// only written by the Connect click / quit sweep, so keys typed while
    /// already connected were silently lost on an unclean exit.
    pub(crate) fn persist_credential_async(&self, key_name: String, value: String) {
        let cache_clone = self.cache.clone();
        self.rt_handle.spawn_blocking(move || {
            if value.trim().is_empty() {
                let _ = keyring::delete(&key_name);
                if let Some(ref cache) = cache_clone {
                    // Loaders treat an empty value as absent, so an empty
                    // put_kv tombstones the SQLite fallback copy too.
                    let _ = cache.put_kv(&format!("cred:{}", key_name), "");
                }
            } else {
                let _ = keyring::store(&key_name, &value);
                if let Some(ref cache) = cache_clone {
                    let _ = cache.put_kv(&format!("cred:{}", key_name), &value);
                }
            }
        });
    }

    /// Append an empty Alpaca account slot (up to [`BROKER_ACCOUNT_SLOT_CAP`]).
    /// Metadata (Paper/Live) persists with the session; credentials persist
    /// per-slot as the user types them. Applies to the roster on the next
    /// Connect, like credential edits.
    pub(crate) fn add_alpaca_account(&mut self) {
        if can_add_account_slot(self.alpaca_extra_accounts.len()) {
            self.alpaca_extra_accounts.push(ExtraAccountConfig {
                paper: true,
                ..Default::default()
            });
        }
    }

    /// Append an empty Kraken account slot (up to [`BROKER_ACCOUNT_SLOT_CAP`]).
    /// Kraken extras are trading identities only (its market data is public), so
    /// `paper` is unused.
    pub(crate) fn add_kraken_account(&mut self) {
        if can_add_account_slot(self.kraken_extra_accounts.len()) {
            self.kraken_extra_accounts
                .push(ExtraAccountConfig::default());
        }
    }

    /// Remove Alpaca extra slot `idx` (0-based into extras; slot = `idx + 2`).
    /// The slots above it shift down, so their in-memory credentials are
    /// re-written to their new per-slot keyring keys and the vacated top slot is
    /// tombstoned. A primary pointing at a removed/shifted slot resets to slot 1.
    pub(crate) fn remove_alpaca_account(&mut self, idx: usize) {
        if idx >= self.alpaca_extra_accounts.len() {
            return;
        }
        let removed_slot = idx + 2;
        let old_len = self.alpaca_extra_accounts.len();
        self.alpaca_extra_accounts.remove(idx);
        for i in 0..self.alpaca_extra_accounts.len() {
            let (ak, sk) = alpaca_slot_keyring_keys(i + 2);
            self.persist_credential_async(ak, self.alpaca_extra_accounts[i].api_key.clone());
            self.persist_credential_async(sk, self.alpaca_extra_accounts[i].secret.clone());
        }
        // Tombstone the now-unused highest slot (old_len extras occupied slots
        // 2..=old_len+1; the top one no longer exists after the shift down).
        let (ak, sk) = alpaca_slot_keyring_keys(old_len + 1);
        self.persist_credential_async(ak, String::new());
        self.persist_credential_async(sk, String::new());
        if let Some(id) =
            primary_after_slot_removal(&self.alpaca_primary_account_id, "alpaca", removed_slot)
        {
            self.alpaca_primary_account_id = id;
        }
    }

    /// Remove Kraken extra slot `idx` (0-based; slot = `idx + 2`), renumbering
    /// the keyring the same way as [`Self::remove_alpaca_account`].
    pub(crate) fn remove_kraken_account(&mut self, idx: usize) {
        if idx >= self.kraken_extra_accounts.len() {
            return;
        }
        let removed_slot = idx + 2;
        let old_len = self.kraken_extra_accounts.len();
        self.kraken_extra_accounts.remove(idx);
        for i in 0..self.kraken_extra_accounts.len() {
            let (kk, ks) = kraken_slot_keyring_keys(i + 2);
            self.persist_credential_async(kk, self.kraken_extra_accounts[i].api_key.clone());
            self.persist_credential_async(ks, self.kraken_extra_accounts[i].secret.clone());
        }
        let (kk, ks) = kraken_slot_keyring_keys(old_len + 1);
        self.persist_credential_async(kk, String::new());
        self.persist_credential_async(ks, String::new());
        if let Some(id) =
            primary_after_slot_removal(&self.kraken_primary_account_id, "kraken", removed_slot)
        {
            self.kraken_primary_account_id = id;
        }
    }

    /// Every configured Alpaca account (slot 1 + populated extras) as connect
    /// specs. Empty-credential slots are skipped.
    pub(crate) fn alpaca_account_specs(&self) -> Vec<BrokerAccountSpec> {
        let mut specs = Vec::new();
        if !self.broker_api_key.is_empty() && !self.broker_secret.is_empty() {
            specs.push(BrokerAccountSpec {
                id: "alpaca1".to_string(),
                label: if self.broker_paper {
                    "Alpaca 1 (Paper)".to_string()
                } else {
                    "Alpaca 1 (Live)".to_string()
                },
                api_key: self.broker_api_key.clone(),
                secret: self.broker_secret.clone(),
                paper: self.broker_paper,
                trade_enabled: true,
                data_sync_enabled: true,
            });
        }
        for (idx, acct) in self.alpaca_extra_accounts.iter().enumerate() {
            if acct.api_key.trim().is_empty() || acct.secret.trim().is_empty() {
                continue;
            }
            let slot = idx + 2;
            specs.push(BrokerAccountSpec {
                id: format!("alpaca{slot}"),
                label: if acct.paper {
                    format!("Alpaca {slot} (Paper)")
                } else {
                    format!("Alpaca {slot} (Live)")
                },
                api_key: acct.api_key.clone(),
                secret: acct.secret.clone(),
                paper: acct.paper,
                // Every configured slot syncs data and can trade — slots are
                // uniform; TradeCopy target selection happens in its own window.
                trade_enabled: true,
                data_sync_enabled: true,
            });
        }
        specs
    }

    pub(crate) fn kraken_extra_account_specs(&self) -> Vec<BrokerAccountSpec> {
        let mut specs = Vec::new();
        for (idx, acct) in self.kraken_extra_accounts.iter().enumerate() {
            if acct.api_key.trim().is_empty() || acct.secret.trim().is_empty() {
                continue;
            }
            let slot = idx + 2;
            specs.push(BrokerAccountSpec {
                id: format!("kraken{slot}"),
                label: format!("Kraken {slot}"),
                api_key: acct.api_key.clone(),
                secret: acct.secret.clone(),
                paper: false,
                trade_enabled: true,
                data_sync_enabled: false,
            });
        }
        specs
    }

    /// Alpaca accounts in the bar-sync fan-out rotation. Prefers the live
    /// roster; falls back to configured specs before the first connect. Always
    /// ≥ 1 so capacity math never zeroes out.
    pub(crate) fn alpaca_data_account_count(&self) -> usize {
        let connected = self
            .alpaca_account_roster
            .iter()
            .filter(|a| a.connected && a.data_sync_enabled)
            .count();
        if connected > 0 {
            return connected;
        }
        self.alpaca_account_specs()
            .iter()
            .filter(|a| a.data_sync_enabled)
            .count()
            .max(1)
    }

    /// True when the current primary Alpaca account is a paper account.
    /// Falls back to the slot-1 flag before the roster arrives.
    pub(crate) fn alpaca_primary_is_paper(&self) -> bool {
        self.alpaca_roster_by_id.get(&self.alpaca_primary_account_id)
            .or_else(|| self.alpaca_roster_by_id.get(&self.kraken_primary_account_id))
            .map(|a| a.paper)
            .unwrap_or(self.broker_paper)
    }

    /// Dispatch the pooled connect for all configured Alpaca accounts.
    pub(crate) fn send_alpaca_connect(&mut self) -> bool {
        let specs = self.alpaca_account_specs();
        if specs.is_empty() {
            return false;
        }
        let n_data = specs.iter().filter(|s| s.data_sync_enabled).count().max(1);
        let n_total = specs.len();
        let capacity = self.alpaca_sync_capacity();
        let aggregate_rpm = self.alpaca_effective_historical_rpm() as usize * n_data;
        let primary_id = if specs.iter().any(|s| s.id == self.alpaca_primary_account_id) {
            self.alpaca_primary_account_id.clone()
        } else {
            specs[0].id.clone()
        }; // TODO: precompute spec ids set for O(1) if grows
        let _ = self.broker_tx.send(BrokerCmd::Connect {
            accounts: specs,
            primary_id,
            bar_requests_per_minute: self.alpaca_effective_historical_rpm(),
            fetch_permits: capacity.fetch_permits,
        });
        self.log.push_back(LogEntry::info(format!(
            "Alpaca connecting {} account(s) — {} in data-sync rotation (~{} req/min aggregate), {} workers, queue {}, batch {}",
            n_total,
            n_data,
            aggregate_rpm,
            capacity.fetch_permits,
            capacity.queue_window,
            capacity.batch_size
        )));
        true
    }

    pub(crate) fn handle_account_roster(
        &mut self,
        broker: OrderBroker,
        accounts: Vec<AccountRosterEntry>,
    ) {
        let connected = accounts.iter().filter(|a| a.connected).count();
        let summary: Vec<String> = accounts
            .iter()
            .map(|a| {
                format!(
                    "{}{}{}",
                    a.label,
                    if a.is_primary { " [primary]" } else { "" },
                    if a.connected { "" } else { " (offline)" }
                )
            })
            .collect();
        self.log.push_back(LogEntry::info(format!(
            "{} accounts ({} connected): {}",
            broker.label(),
            connected,
            summary.join(", ")
        )));
        match broker {
            OrderBroker::Alpaca => {
                if let Some(primary) = self.alpaca_roster_by_id.get(&self.alpaca_primary_account_id).or_else(|| accounts.iter().find(|a| (a.id == self.alpaca_primary_account_id || a.is_primary) && a.connected)) {
                    self.alpaca_primary_account_id = primary.id.clone();
                } else if let Some(primary) = accounts.iter().find(|a| (a.id == self.alpaca_primary_account_id || a.is_primary) && a.connected) {
                    self.alpaca_primary_account_id = primary.id.clone();
                }
                self.alpaca_account_roster = accounts.clone();
                self.alpaca_roster_by_id = accounts.into_iter().map(|a| (a.id.clone(), a)).collect();
            }
            OrderBroker::Kraken => {
                if let Some(primary) = self.kraken_roster_by_id.get(&self.kraken_primary_account_id).or_else(|| accounts.iter().find(|a| (a.id == self.kraken_primary_account_id || a.is_primary) && a.connected)) {
                    self.kraken_primary_account_id = primary.id.clone();
                } else if let Some(primary) = accounts.iter().find(|a| (a.id == self.kraken_primary_account_id || a.is_primary) && a.connected) {
                    self.kraken_primary_account_id = primary.id.clone();
                }
                self.kraken_account_roster = accounts.clone();
                self.kraken_roster_by_id = accounts.into_iter().map(|a| (a.id.clone(), a)).collect();
            }
        }
    }

    /// Cycle order for the top-bar Primary switch: every connected (or, before
    /// connect, configured) account of every enabled broker. Entries are
    /// (broker, account_id, chip label).
    pub(crate) fn primary_account_cycle(&self) -> Vec<(OrderBroker, String, String)> {
        let mut out: Vec<(OrderBroker, String, String)> = Vec::new();
        if self.alpaca_enabled {
            let connected: Vec<&AccountRosterEntry> = self
                .alpaca_account_roster
                .iter()
                .filter(|a| a.connected)
                .collect();
            if connected.is_empty() {
                out.push((
                    OrderBroker::Alpaca,
                    self.alpaca_primary_account_id.clone(),
                    "Alpaca".to_string(),
                ));
            } else {
                for a in connected {
                    out.push((OrderBroker::Alpaca, a.id.clone(), a.label.clone()));
                }
            }
        }
        if self.kraken_enabled {
            let connected: Vec<&AccountRosterEntry> = self
                .kraken_account_roster
                .iter()
                .filter(|a| a.connected)
                .collect();
            if connected.is_empty() {
                out.push((
                    OrderBroker::Kraken,
                    self.kraken_primary_account_id.clone(),
                    "Kraken".to_string(),
                ));
            } else {
                for a in connected {
                    out.push((OrderBroker::Kraken, a.id.clone(), a.label.clone()));
                }
            }
        }
        out
    }

    /// Currently-selected account id for a broker (the per-broker primary).
    pub(crate) fn primary_account_id_for(&self, broker: OrderBroker) -> String {
        match broker {
            OrderBroker::Alpaca => self.alpaca_primary_account_id.clone(),
            OrderBroker::Kraken => self.kraken_primary_account_id.clone(),
        }
    }

    /// Apply a Primary-cycle selection: broker-level effects (order routing,
    /// merge lane) when the broker changed, plus a runtime primary-account
    /// switch when the account within that broker changed.
    pub(crate) fn apply_primary_selection(&mut self, broker: OrderBroker, account_id: &str) {
        let broker_changed = broker != self.primary_broker;
        if broker_changed {
            self.primary_broker = broker;
            // Routing follows the primary immediately; the per-trade Broker
            // combo can still override.
            self.order_broker = broker;
            // Flip the equity data-merge trusted lane too (ADR-126).
            set_chart_merge_primary_broker(broker);
        }
        let account_changed = self.primary_account_id_for(broker) != account_id;
        match broker {
            OrderBroker::Alpaca => self.alpaca_primary_account_id = account_id.to_string(),
            OrderBroker::Kraken => self.kraken_primary_account_id = account_id.to_string(),
        }
        if account_changed {
            let _ = self.broker_tx.send(BrokerCmd::SetPrimaryAccount {
                broker,
                account_id: account_id.to_string(),
            });
        }
        if broker_changed || account_changed {
            let selected_label = match broker {
                OrderBroker::Alpaca => self
                    .alpaca_roster_by_id
                    .get(account_id)
                    .map(|a| a.label.clone())
                    .unwrap_or_else(|| account_id.to_string()),
                OrderBroker::Kraken => self
                    .kraken_roster_by_id
                    .get(account_id)
                    .map(|a| a.label.clone())
                    .unwrap_or_else(|| account_id.to_string()),
            };
            let assists = self.assist_brokers();
            let assist_str = if assists.is_empty() {
                "none".to_string()
            } else {
                assists
                    .iter()
                    .map(|broker| broker.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let message = format!(
                "Primary → {} ({}) [id: {}] (assist: {})",
                selected_label,
                broker.label(),
                account_id,
                assist_str
            );
            self.log.push_back(LogEntry::info(message.clone()));
            self.push_connection_toast(message, true);
            self.save_session();
        }
    }

    /// TradeCopy window: pick a source account, target accounts, and copy open
    /// positions across; optionally enable live order mirroring (ADR-130).
    /// Broker-aware: Alpaca copies net positions; Kraken copies spot xStock
    /// holdings (margin positions are skipped). Targets are always the same
    /// broker as the source.
    pub(crate) fn render_tradecopy_window(&mut self, ctx: &egui::Context) {
        if !self.show_tradecopy {
            return;
        }
        let mut show = self.show_tradecopy;
        let alpaca_roster = self.alpaca_account_roster.clone();
        let kraken_roster = self.kraken_account_roster.clone();
        let alpaca_connected: Vec<&AccountRosterEntry> =
            alpaca_roster.iter().filter(|a| a.connected).collect();
        let kraken_connected: Vec<&AccountRosterEntry> =
            kraken_roster.iter().filter(|a| a.connected).collect();
        let mut copy_request: Option<(String, Vec<String>, bool)> = None;
        let mut mirror_toggled: Option<bool> = None;
        let mut targets_changed = false;
        egui::Window::new("TradeCopy")
            .open(&mut show)
            .resizable(true)
            .default_size([420.0, 360.0])
            .show(ctx, |ui| {
                // A copy needs ≥2 connected accounts on the same broker.
                let alpaca_ok = alpaca_connected.len() >= 2;
                let kraken_ok = kraken_connected.len() >= 2;
                if !alpaca_ok && !kraken_ok {
                    ui.label(
                        egui::RichText::new(
                            "TradeCopy needs at least two connected accounts of the same \
                             broker. Add account credentials under Settings → API Keys \
                             (slots 2–4), then reconnect.",
                        )
                        .color(AXIS_TEXT),
                    );
                    return;
                }
                ui.label(
                    egui::RichText::new(
                        "Copies open positions from the source account to each selected target \
                         by submitting market orders for the per-symbol quantity delta. Results \
                         land in the Log. Kraken copies spot xStock holdings only (margin \
                         positions are skipped; every Kraken account is LIVE).",
                    )
                    .color(AXIS_TEXT)
                    .small(),
                );
                ui.separator();
                // Sources come from brokers that actually have a same-broker
                // target available.
                let sources: Vec<&AccountRosterEntry> = alpaca_connected
                    .iter()
                    .filter(|_| alpaca_ok)
                    .chain(kraken_connected.iter().filter(|_| kraken_ok))
                    .copied()
                    .collect();
                let source_ids: std::collections::HashSet<_> = sources.iter().map(|a| a.id.as_str()).collect();
                if !source_ids.contains(self.tradecopy_source_id.as_str()) {
                    self.tradecopy_source_id = sources
                        .iter()
                        .find(|a| a.id == self.alpaca_primary_account_id || a.id == self.kraken_primary_account_id)
                        .or(sources.first())
                        .map(|a| a.id.clone())
                        .unwrap_or_default();
                }
                let source_is_kraken = self.tradecopy_source_id.starts_with("kraken");
                // Targets are the source's broker only — cross-broker copy is
                // out of scope (symbols and settlement semantics differ).
                let connected: &Vec<&AccountRosterEntry> = if source_is_kraken {
                    &kraken_connected
                } else {
                    &alpaca_connected
                };
                let source_label = sources
                    .iter()
                    .find(|a| a.id == self.tradecopy_source_id)
                    .map(|a| a.label.clone())
                    .unwrap_or_else(|| self.tradecopy_source_id.clone());
                ui.horizontal(|ui| {
                    ui.label("Source account:");
                    egui::ComboBox::from_id_salt("tradecopy_source")
                        .selected_text(source_label)
                        .show_ui(ui, |ui| {
                            for a in &sources {
                                ui.selectable_value(
                                    &mut self.tradecopy_source_id,
                                    a.id.clone(),
                                    format!(
                                        "{} ({}, ${:.0})",
                                        a.label,
                                        if a.paper { "paper" } else { "LIVE" },
                                        a.equity
                                    ),
                                );
                            }
                        });
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Target accounts").strong().small());
                for a in connected.iter() {
                    if a.id == self.tradecopy_source_id {
                        continue;
                    }
                    let mut checked = self.tradecopy_target_ids.contains(&a.id);
                    let live_locked = !a.paper && !self.tradecopy_allow_live_targets;
                    let label = format!(
                        "{} ({}, ${:.0}){}",
                        a.label,
                        if a.paper { "paper" } else { "LIVE" },
                        a.equity,
                        if live_locked {
                            " — enable live targets below"
                        } else {
                            ""
                        }
                    );
                    let resp =
                        ui.add_enabled(!live_locked, egui::Checkbox::new(&mut checked, label));
                    if resp.changed() {
                        if checked {
                            self.tradecopy_target_ids.insert(a.id.clone());
                        } else {
                            self.tradecopy_target_ids.remove(&a.id);
                        }
                        targets_changed = true;
                    }
                    if live_locked && self.tradecopy_target_ids.remove(&a.id) {
                        targets_changed = true;
                    }
                }
                ui.add_space(4.0);
                ui.checkbox(
                    &mut self.tradecopy_flatten_extra,
                    "Flatten extra target positions (close symbols the source doesn't hold)",
                );
                ui.checkbox(
                    &mut self.tradecopy_allow_live_targets,
                    "Allow LIVE accounts as targets (danger: real orders)",
                );
                ui.separator();
                // Only same-broker targets count — a stale check from the
                // other broker (source switched since) must not ride along.
                let broker_prefix = if source_is_kraken { "kraken" } else { "alpaca" };
                let target_ids: Vec<String> = self
                    .tradecopy_target_ids
                    .iter()
                    .filter(|id| **id != self.tradecopy_source_id)
                    .filter(|id| id.starts_with(broker_prefix))
                    .cloned()
                    .collect();
                let can_copy = !target_ids.is_empty();
                if ui
                    .add_enabled(
                        can_copy,
                        egui::Button::new(egui::RichText::new("Copy positions now").strong()),
                    )
                    .on_hover_text(
                        "Fetches source + target positions and submits market orders for the \
                         quantity deltas on each target account.",
                    )
                    .clicked()
                {
                    copy_request = Some((
                        self.tradecopy_source_id.clone(),
                        target_ids,
                        self.tradecopy_flatten_extra,
                    ));
                }
                // Live order mirroring is Alpaca-only: Kraken's copy is the
                // one-shot spot replication above.
                if !source_is_kraken {
                    ui.add_space(6.0);
                    let mut mirror = self.tradecopy_mirror_orders;
                    // Opt-in only: the checkbox is disabled until at least one
                    // target is checked (it stays clickable while ON so mirroring
                    // can always be turned off). Never persisted across restarts.
                    if ui
                        .add_enabled(
                            can_copy || mirror,
                            egui::Checkbox::new(
                                &mut mirror,
                                "Live mirroring: replicate app-placed Alpaca orders to the checked \
                                 target accounts (opt-in, resets on restart)",
                            ),
                        )
                        .on_hover_text(
                            "While enabled, every order placed from this app on the primary \
                             account is also submitted to each checked target account \
                             (cancels/modifies excluded — order ids differ per account). \
                             Mirroring is always off at startup.",
                        )
                        .changed()
                    {
                        mirror_toggled = Some(mirror);
                    }
                }
            });
        self.show_tradecopy = show;
        if let Some((source, targets, flatten)) = copy_request {
            self.log.push_back(LogEntry::info(format!(
                "TradeCopy: copying positions {} → {} target(s)…",
                source,
                targets.len()
            )));
            let cmd = if source.starts_with("kraken") {
                BrokerCmd::KrakenTradeCopy {
                    source_id: source,
                    target_ids: targets,
                    flatten_extra: flatten,
                }
            } else {
                BrokerCmd::AlpacaTradeCopy {
                    source_id: source,
                    target_ids: targets,
                    flatten_extra: flatten,
                }
            };
            let _ = self.broker_tx.send(cmd);
        }
        // Sync the runtime whenever the toggle flips or the opted-in target
        // set changes while mirroring is on. Mirroring with an empty opt-in
        // set turns itself off — copying is opt-in, never opt-out. Mirroring
        // is an Alpaca-pool feature, so only Alpaca ids are valid targets.
        let mirror_targets: Vec<String> = self
            .tradecopy_target_ids
            .iter()
            .filter(|id| **id != self.tradecopy_source_id)
            .filter(|id| id.starts_with("alpaca"))
            .cloned()
            .collect();
        if let Some(enabled) = mirror_toggled {
            let effective = enabled && !mirror_targets.is_empty();
            if enabled && !effective {
                self.log.push_back(LogEntry::warn(
                    "TradeCopy mirroring stays OFF — check at least one target account first",
                ));
            }
            self.tradecopy_mirror_orders = effective;
            let _ = self.broker_tx.send(BrokerCmd::SetOrderMirroring {
                enabled: effective,
                target_ids: mirror_targets,
            });
            self.save_session();
        } else if targets_changed && self.tradecopy_mirror_orders {
            let effective = !mirror_targets.is_empty();
            if !effective {
                self.log.push_back(LogEntry::warn(
                    "TradeCopy mirroring disabled — no target accounts remain opted in",
                ));
            }
            self.tradecopy_mirror_orders = effective;
            let _ = self.broker_tx.send(BrokerCmd::SetOrderMirroring {
                enabled: effective,
                target_ids: mirror_targets,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_keyring_keys_keep_slot1_legacy_names() {
        assert_eq!(
            alpaca_slot_keyring_keys(1),
            ("alpaca_api_key".to_string(), "alpaca_secret".to_string())
        );
        assert_eq!(
            alpaca_slot_keyring_keys(3),
            (
                "alpaca_api_key_3".to_string(),
                "alpaca_secret_3".to_string()
            )
        );
        assert_eq!(
            kraken_slot_keyring_keys(1),
            (
                "kraken_api_key".to_string(),
                "kraken_api_secret".to_string()
            )
        );
        assert_eq!(
            kraken_slot_keyring_keys(4),
            (
                "kraken_api_key_4".to_string(),
                "kraken_api_secret_4".to_string()
            )
        );
    }

    #[test]
    fn default_extra_account_slots_cover_slots_2_to_4() {
        let alpaca = default_alpaca_extra_accounts();
        assert_eq!(alpaca.len(), 3);
        assert!(alpaca.iter().all(|a| a.paper));
        let kraken = default_kraken_extra_accounts();
        assert_eq!(kraken.len(), 3);
        assert!(kraken.iter().all(|a| !a.paper));
    }

    #[test]
    fn can_add_account_slot_stops_at_the_cap() {
        // Default 3 extras (4 total) is well under the cap.
        assert!(can_add_account_slot(3));
        // The last addable extra brings the total to the cap …
        assert!(can_add_account_slot(BROKER_ACCOUNT_SLOT_CAP - 2));
        // … and one more would exceed it (extras + slot 1 == cap already).
        assert!(!can_add_account_slot(BROKER_ACCOUNT_SLOT_CAP - 1));
    }

    #[test]
    fn primary_resets_only_when_at_or_above_the_removed_slot() {
        // Removing slot 3: a primary on slot 3 or 4 is now wrong (shifted/gone).
        assert_eq!(
            primary_after_slot_removal("alpaca4", "alpaca", 3),
            Some("alpaca1".to_string())
        );
        assert_eq!(
            primary_after_slot_removal("alpaca3", "alpaca", 3),
            Some("alpaca1".to_string())
        );
        // A lower-numbered primary (slot 1 or 2) is unaffected by removing slot 3.
        assert_eq!(primary_after_slot_removal("alpaca2", "alpaca", 3), None);
        assert_eq!(primary_after_slot_removal("alpaca1", "alpaca", 3), None);
        // Works for the Kraken prefix too.
        assert_eq!(
            primary_after_slot_removal("kraken5", "kraken", 2),
            Some("kraken1".to_string())
        );
        // Unparseable / foreign ids are left alone.
        assert_eq!(primary_after_slot_removal("", "alpaca", 2), None);
        assert_eq!(primary_after_slot_removal("alpacaX", "alpaca", 2), None);
    }
}
