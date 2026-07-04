//! Multi-account broker plumbing (ADR-130).
//!
//! Alpaca's free tier allows 1 live + 3 paper accounts, each with its own
//! market-data rate limit; connecting all of them multiplies historical
//! bar-sync throughput. Slot 1 is the legacy single-account credential pair
//! (`broker_api_key`/`broker_secret`, keyring `alpaca_api_key`); slots 2–4
//! store credentials under per-slot keyring keys and metadata in the session.
//! Kraken slots are trading identities only (its market data is public).

use super::*;

pub(crate) const MAX_BROKER_ACCOUNT_SLOTS: usize = 4;

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
        self.alpaca_account_roster
            .iter()
            .find(|a| a.is_primary)
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
        };
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
                if let Some(primary) = accounts.iter().find(|a| a.is_primary && a.connected) {
                    self.alpaca_primary_account_id = primary.id.clone();
                }
                self.alpaca_account_roster = accounts;
            }
            OrderBroker::Kraken => {
                if let Some(primary) = accounts.iter().find(|a| a.is_primary && a.connected) {
                    self.kraken_primary_account_id = primary.id.clone();
                }
                self.kraken_account_roster = accounts;
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
                let multi = connected.len() > 1;
                for a in connected {
                    let label = if multi {
                        format!("Alpaca · {}", a.label)
                    } else {
                        "Alpaca".to_string()
                    };
                    out.push((OrderBroker::Alpaca, a.id.clone(), label));
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
                let multi = connected.len() > 1;
                for a in connected {
                    let label = if multi {
                        format!("Kraken · {}", a.label)
                    } else {
                        "Kraken".to_string()
                    };
                    out.push((OrderBroker::Kraken, a.id.clone(), label));
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
                    .alpaca_account_roster
                    .iter()
                    .find(|a| a.id == account_id)
                    .map(|a| a.label.clone())
                    .unwrap_or_else(|| account_id.to_string()),
                OrderBroker::Kraken => self
                    .kraken_account_roster
                    .iter()
                    .find(|a| a.id == account_id)
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
                if !sources.iter().any(|a| a.id == self.tradecopy_source_id) {
                    self.tradecopy_source_id = sources
                        .iter()
                        .find(|a| a.is_primary)
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
                        if live_locked { " — enable live targets below" } else { "" }
                    );
                    let resp = ui.add_enabled(
                        !live_locked,
                        egui::Checkbox::new(&mut checked, label),
                    );
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
                        egui::Button::new(
                            egui::RichText::new("Copy positions now").strong(),
                        ),
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
}
