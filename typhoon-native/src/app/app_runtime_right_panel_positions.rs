use super::*;

fn alpaca_positions_should_render(
    _broker_connected: bool,
    show_alpaca_positions: bool,
    positions: &[PositionInfo],
) -> bool {
    show_alpaca_positions && !positions.is_empty()
}

pub(super) fn position_unrealized_pl_from_price(
    pos: &PositionInfo,
    current_price: Option<f64>,
) -> f64 {
    current_price
        .filter(|price| price.is_finite() && *price > 0.0)
        .filter(|_| pos.avg_entry_price.is_finite() && pos.avg_entry_price > 0.0)
        .map(|price| {
            let dir = if pos.side.eq_ignore_ascii_case("short") {
                -1.0
            } else {
                1.0
            };
            (price - pos.avg_entry_price) * pos.qty * dir
        })
        .unwrap_or(pos.unrealized_pl)
}

pub(super) fn position_cost_basis(pos: &PositionInfo) -> f64 {
    if pos.avg_entry_price.is_finite() && pos.avg_entry_price > 0.0 {
        pos.avg_entry_price.abs() * pos.qty.abs()
    } else {
        (pos.market_value - pos.unrealized_pl).abs()
    }
}

fn position_unrealized_pl_pct(pos: &PositionInfo, display_pl: f64) -> f64 {
    let cost_basis = position_cost_basis(pos);
    if cost_basis > f64::EPSILON {
        display_pl / cost_basis * 100.0
    } else {
        0.0
    }
}

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_positions_section(&mut self, ui: &mut egui::Ui) {
        // ── Positions Section ─────────────────────────────────
        let alpaca_positions_available = self.alpaca_enabled;
        let kr_positions_available = self.kraken_enabled;
        let show_alpaca_positions = alpaca_positions_available && self.show_alpaca_positions;
        let show_kr_positions = kr_positions_available && self.show_kr_positions;
        let position_source_count = [alpaca_positions_available, kr_positions_available]
            .into_iter()
            .filter(|visible| *visible)
            .count();
        let alpaca_count = if show_alpaca_positions {
            self.live_positions.len()
        } else {
            0
        };
        let kr_count = if show_kr_positions {
            self.kr_positions.len()
        } else {
            0
        };
        let pos_count = alpaca_count + kr_count;
        let (pos_stale_lbl, pos_stale_col) = self.staleness_badge(self.positions_last_update_ts);
        let pos_header = format!("☰ Positions ({})  •  {}", pos_count, pos_stale_lbl);
        let positions_section = egui::CollapsingHeader::new(
            egui::RichText::new(pos_header)
                .strong()
                .small()
                .color(pos_stale_col),
        )
        .id_salt("positions_section")
        .default_open(self.right_positions_open)
        .show(ui, |ui| {
            // Visibility toggles only matter when more than one eligible
            // position source exists. Do not show disabled brokers.
            if position_source_count > 1 {
                ui.horizontal(|ui| {
                    if alpaca_positions_available {
                        ui.checkbox(
                            &mut self.show_alpaca_positions,
                            egui::RichText::new("Alpaca").small(),
                        );
                    }
                    if kr_positions_available {
                        ui.checkbox(
                            &mut self.show_kr_positions,
                            egui::RichText::new("Kraken").small(),
                        );
                    }
                });
                ui.add_space(4.0);
            }
            let mut has_positions = false;
            // Live broker positions from Alpaca.
            if alpaca_positions_should_render(
                self.broker_connected,
                show_alpaca_positions,
                &self.live_positions,
            ) {
                has_positions = true;
                let mut close_sym: Option<String> = None;
                let mut lp_action = SymbolAction::None;
                for pos in &self.live_positions {
                    let side_c = if pos.side == "long" { UP } else { DOWN };
                    let side_label = if pos.side == "long" { "Long" } else { "Short" };
                    let current_price = self
                        .live_quote_mid_for_symbol(&pos.symbol)
                        .or_else(|| self.latest_cached_equity_price_for_symbol(&pos.symbol))
                        .or_else(|| {
                            if pos.qty.abs() > f64::EPSILON {
                                Some(pos.market_value.abs() / pos.qty.abs())
                            } else {
                                None
                            }
                        });
                    ui.horizontal_wrapped(|ui| {
                        let (_, act) = symbol_label_with_menu(
                            ui,
                            &pos.symbol,
                            egui::RichText::new(&pos.symbol).small().strong(),
                        );
                        if !matches!(act, SymbolAction::None) {
                            lp_action = act;
                        }
                        ui.label(
                            egui::RichText::new(side_label).color(side_c).small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}", pos.qty)).small(),
                        );
                        let display_pl = position_unrealized_pl_from_price(pos, current_price);
                        let pl_c = if display_pl >= 0.0 { UP } else { DOWN };
                        let pl_pct = position_unrealized_pl_pct(pos, display_pl);
                        ui.label(
                            egui::RichText::new(format!(
                                "${:.2} ({:+.1}%)",
                                display_pl, pl_pct
                            ))
                            .color(pl_c)
                            .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "entry {}  cur {}",
                                format_price(pos.avg_entry_price),
                                current_price
                                    .map(format_price)
                                    .unwrap_or_else(|| "—".to_string())
                            ))
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if self.broker_connected {
                            if ui
                                .small_button(egui::RichText::new("x").color(DOWN))
                                .on_hover_text("Close position")
                                .clicked()
                            {
                                close_sym = Some(pos.symbol.clone());
                            }
                        }
                    });
                    ui.separator();
                }
                if let Some(sym) = close_sym {
                    let _ = self
                        .broker_tx
                        .send(BrokerCmd::ClosePosition { symbol: sym, qty: None });
                }
                if !matches!(lp_action, SymbolAction::None) {
                    self.deferred_symbol_action = lp_action;
                }
            }
            if show_kr_positions && !self.kr_positions.is_empty() {
                let mut close_sym: Option<String> = None;
                let mut kr_action = SymbolAction::None;
                for pos in &self.kr_positions {
                    // Kraken's OpenPositions endpoint also reports spot/funded
                    // trades (leverage=1) whose underlying sits in the wallet.
                    // When the matching balance covers the full position qty,
                    // the spot row in the balances section is the real exposure
                    // — rendering this row too double-counts it (and the Close
                    // button would try to close a non-existent margin position).
                    if let Some((_, bal_qty)) =
                        self.kraken_spot_balance_for_pair(&pos.symbol)
                    {
                        let tol = (pos.qty.abs() * 0.01).max(1e-9);
                        if (bal_qty - pos.qty).abs() <= tol {
                            continue;
                        }
                    }
                    has_positions = true;
                    let side_c = if pos.side == "long" { UP } else { DOWN };
                    let side_label = if pos.side == "long" { "Long" } else { "Short" };
                    let avg_entry = if pos.avg_entry_price > 0.0 {
                        Some(pos.avg_entry_price)
                    } else {
                        self.kraken_position_avg_price(&pos.symbol)
                    };
                    let current_price = self.live_quote_mid_for_symbol(&pos.symbol).or_else(|| {
                        if pos.asset_id.starts_with("equity_balance:")
                            || pos.asset_class.eq_ignore_ascii_case("stock")
                        {
                            self.latest_cached_equity_price_for_symbol(&pos.symbol)
                        } else {
                            self.latest_cached_price_for_symbol(&pos.symbol)
                        }
                    });
                    let quote_meta = self.kraken_equity_quote_meta_for_symbol(&pos.symbol);
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let (quote_label, quote_color) = if let Some(meta) = quote_meta {
                        let age_secs = ((now_ms - meta.received_at_ms).max(0) / 1000) as i64;
                        let delayed = if meta.delayed { " delayed" } else { "" };
                        let label = if age_secs < 60 {
                            format!("q {}s{}", age_secs, delayed)
                        } else {
                            format!("STALE q {}m{}", age_secs / 60, delayed)
                        };
                        let color = if age_secs <= 30 {
                            UP
                        } else if age_secs <= 60 {
                            egui::Color32::from_rgb(255, 200, 50)
                        } else {
                            DOWN
                        };
                        (label, color)
                    } else {
                        ("NO QUOTE".to_string(), DOWN)
                    };
                    let derived_unrealized_pl = avg_entry.zip(current_price).map(|(avg, cur)| {
                        let dir = if pos.side == "short" { -1.0 } else { 1.0 };
                        (cur - avg) * pos.qty * dir
                    });
                    let display_pl = derived_unrealized_pl.unwrap_or(pos.unrealized_pl);
                    ui.horizontal_wrapped(|ui| {
                        let (_, act) = symbol_label_with_menu(
                            ui,
                            &pos.symbol,
                            egui::RichText::new(&pos.symbol).small().strong(),
                        );
                        if !matches!(act, SymbolAction::None) {
                            kr_action = act;
                        }
                        ui.label(
                            egui::RichText::new(format!("[Kraken] {}", side_label))
                                .color(side_c)
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.6}", pos.qty)).small(),
                        );
                        let pl_c = if display_pl >= 0.0 { UP } else { DOWN };
                        ui.label(
                            egui::RichText::new(format!("${:.2}", display_pl))
                                .color(pl_c)
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "entry {}  cur {}",
                                avg_entry
                                    .map(format_price)
                                    .unwrap_or_else(|| "—".to_string()),
                                current_price
                                    .map(format_price)
                                    .unwrap_or_else(|| "—".to_string())
                            ))
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        ui.label(
                            egui::RichText::new(&quote_label)
                                .color(quote_color)
                                .small(),
                        )
                        .on_hover_text(if let Some(meta) = quote_meta {
                            let quote_ts = chrono::DateTime::from_timestamp_millis(
                                meta.quote_time_ms,
                            )
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_else(|| "unknown".to_string());
                            format!(
                                "Kraken equity quote overlay: last={} quote_ts={} received_ts={}{}",
                                format_price(meta.price),
                                quote_ts,
                                chrono::DateTime::from_timestamp_millis(
                                    meta.received_at_ms,
                                )
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| "unknown".to_string()),
                                if meta.delayed { " delayed" } else { "" }
                            )
                        } else {
                            "No Kraken equity quote has landed for this open position yet".to_string()
                        });
                        if ui
                            .small_button("Close")
                            .on_hover_text(format!(
                                "Close active Kraken position {} at market",
                                pos.symbol
                            ))
                            .clicked()
                        {
                            close_sym = Some(pos.symbol.clone());
                        }
                    });
                    ui.separator();
                }
                if let Some(sym) = close_sym {
                    let _ = self.broker_tx.send(BrokerCmd::KrakenClosePosition {
                        pair: sym.clone(),
                        volume: None,
                    });
                    self.log.push_back(LogEntry::info(format!(
                        "Kraken: closing active position {sym} at market"
                    )));
                }
                if !matches!(kr_action, SymbolAction::None) {
                    self.deferred_symbol_action = kr_action;
                }
            }
            let kraken_sellable_balances: Vec<(String, f64)> = self
                .kraken_balances
                .iter()
                .filter(|(asset, qty)| {
                    qty.is_finite()
                        && *qty > 0.0
                        && !Self::kraken_is_cash_balance_asset(asset)
                })
                .cloned()
                .collect();
            if show_kr_positions && !kraken_sellable_balances.is_empty() {
                has_positions = true;
                let mut sell_balance: Option<(String, f64)> = None;
                for (asset, qty) in kraken_sellable_balances {
                    let display_asset = Self::kraken_display_asset(&asset);
                    let display_holding = display_asset
                        .strip_suffix(".EQ")
                        .unwrap_or(display_asset.as_str())
                        .to_string();
                    let pair = Self::kraken_spot_pair_for_balance_asset(&asset);
                    let avg_price = self.kraken_balance_avg_price(&asset);
                    let current_price = if Self::kraken_display_asset(&asset).ends_with(".EQ") {
                        self.latest_cached_equity_price_for_symbol(&pair)
                    } else {
                        self.latest_cached_price_for_symbol(&pair)
                    };
                    let pl = avg_price
                        .zip(current_price)
                        .map(|(avg, cur)| ((cur - avg) * qty, (cur - avg) / avg * 100.0));
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Long")
                                .color(UP)
                                .small()
                                .strong(),
                        );
                        let qty_text = if qty.fract().abs() < 1e-9 {
                            format!("{qty:.0}")
                        } else {
                            format!("{qty:.8}")
                                .trim_end_matches('0')
                                .trim_end_matches('.')
                                .to_string()
                        };
                        ui.label(
                            egui::RichText::new(format!("{qty_text} {display_holding}"))
                                .small()
                                .monospace(),
                        );
                        let fmt_bal_price = |p: f64| -> String {
                            if p.abs() >= 1.0 {
                                format!("{:.2}", p)
                            } else {
                                format!("{:.4}", p)
                            }
                        };
                        if let Some(avg) = avg_price {
                            ui.label(
                                egui::RichText::new(format!("avg {}", fmt_bal_price(avg)))
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if let Some(cur) = current_price {
                            ui.label(
                                egui::RichText::new(format!("cur {}", fmt_bal_price(cur)))
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if let Some((pl_value, pl_pct)) = pl {
                            let c = if pl_value >= 0.0 { UP } else { DOWN };
                            ui.label(
                                egui::RichText::new(format!(
                                    "P/L ${:+.2} ({:+.2}%)",
                                    pl_value, pl_pct
                                ))
                                .color(c)
                                .small(),
                            );
                        }
                        if ui
                            .small_button("Sell…")
                            .on_hover_text(format!(
                                "Open Kraken sell ticket for {display_asset}; choose lots with a slider"
                            ))
                            .clicked()
                        {
                            sell_balance = Some((asset.clone(), qty));
                        }
                    });
                }
                if let Some((asset, qty)) = sell_balance {
                    self.open_kraken_spot_sell_dialog(asset, qty);
                }
                ui.separator();
            }
            if !has_positions {
                ui.label(
                    egui::RichText::new("No open positions.")
                        .color(AXIS_TEXT)
                        .small(),
                );
            }
        });
        self.right_positions_open = positions_section.fully_open();
        self.handle_right_panel_section_drag(
            ui,
            RightPanelSectionId::Positions,
            &positions_section.header_response,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(symbol: &str) -> PositionInfo {
        PositionInfo {
            symbol: symbol.to_string(),
            qty: 1.0,
            side: "long".to_string(),
            avg_entry_price: 10.0,
            market_value: 10.0,
            unrealized_pl: 0.0,
            asset_class: "us_equity".to_string(),
            asset_id: format!("asset:{symbol}"),
        }
    }

    #[test]
    fn cached_alpaca_positions_render_even_when_broker_is_temporarily_disconnected() {
        let positions = vec![position("CC"), position("WEN")];

        assert!(alpaca_positions_should_render(false, true, &positions));
    }

    #[test]
    fn alpaca_position_pl_uses_current_price_when_snapshot_pl_is_stale() {
        let mut pos = position("CC");
        pos.qty = 2.0;
        pos.avg_entry_price = 10.0;
        pos.market_value = 20.0;
        pos.unrealized_pl = 0.0;

        let display_pl = position_unrealized_pl_from_price(&pos, Some(12.5));

        assert_eq!(display_pl, 5.0);
        assert_eq!(position_unrealized_pl_pct(&pos, display_pl), 25.0);
    }
}
