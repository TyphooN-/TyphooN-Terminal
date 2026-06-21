use super::*;

impl TyphooNApp {
    pub(super) fn render_research_return_risk_dcf_options_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research: String = self
            .charts
            .get(self.active_tab)
            .map(|c| {
                c.symbol
                    .split(':')
                    .rev()
                    .nth(1)
                    .or_else(|| c.symbol.split(':').last())
                    .unwrap_or("AAPL")
                    .to_string()
            })
            .unwrap_or_else(|| "AAPL".to_string());

        // ── Research section ──

        // HRA — historical return / risk analysis
        if self.show_hra {
            if self.hra_symbol.is_empty() {
                self.hra_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hra;
            egui::Window::new("HRA — Historical Return / Risk")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 460.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.hra_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.hra_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hra_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_hra(&conn, &sym_u) {
                                        self.hra_snapshot = snap;
                                        self.hra_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hra_symbol.to_uppercase();
                            self.hra_loading = true;
                            self.hra_symbol = sym.clone();
                            let rf = self.treasury_yields.iter()
                                .find(|y| y.tenor.contains("10"))
                                .map(|y| y.yield_pct)
                                .unwrap_or(4.0);
                            let _ = self.broker_tx.send(BrokerCmd::FetchHraSnapshot { symbol: sym, risk_free_pct: rf });
                        }
                        if self.hra_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.hra_snapshot;
                    if snap.symbol.is_empty() || snap.windows.is_empty() {
                        ui.label(egui::RichText::new("No data — run HP for this symbol to populate history, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        ui.label(egui::RichText::new(format!("{} — last close ${:.2} — as of {}",
                            snap.symbol, snap.last_close, snap.as_of)).strong().color(AXIS_TEXT));
                        ui.separator();
                        egui::Grid::new("hra_ratios_grid").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            let row = |ui: &mut egui::Ui, k: &str, v: String| {
                                ui.label(egui::RichText::new(k).color(AXIS_TEXT).small());
                                ui.label(egui::RichText::new(v).small().monospace().strong());
                                ui.end_row();
                            };
                            row(ui, "Annualized volatility", format!("{:.2}%", snap.volatility_annual_pct));
                            row(ui, "Sharpe ratio",          format!("{:.3}", snap.sharpe_ratio));
                            row(ui, "Sortino ratio",         format!("{:.3}", snap.sortino_ratio));
                            row(ui, "Calmar ratio",          format!("{:.3}", snap.calmar_ratio));
                            row(ui, "Max drawdown",          format!("{:.2}%", snap.max_drawdown_pct));
                            row(ui, "DD peak",               snap.drawdown_peak_date.clone());
                            row(ui, "DD trough",             snap.drawdown_trough_date.clone());
                            row(ui, "Risk-free rate",        format!("{:.2}%", snap.risk_free_pct));
                        });
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("hra_windows_grid").striped(true).num_columns(4).min_col_width(80.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Window").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Return").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("CAGR").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("N").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for w in &snap.windows {
                                    let c = if w.return_pct >= 0.0 { UP } else { DOWN };
                                    ui.label(egui::RichText::new(&w.label).small().monospace().strong());
                                    ui.label(egui::RichText::new(format!("{:+.2}%", w.return_pct)).color(c).small().monospace());
                                    let cagr = if w.cagr_pct == 0.0 { "—".to_string() } else { format!("{:+.2}%", w.cagr_pct) };
                                    ui.label(egui::RichText::new(cagr).small().monospace());
                                    ui.label(egui::RichText::new(format!("{}", w.n_observations)).small().monospace());
                                    ui.end_row();
                                }
                            });
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).color(AXIS_TEXT).small().italics());
                        }
                    }
                });
            self.show_hra = open;
        }

        // DCF — Discounted Cash Flow fair value
        if self.show_dcf {
            if self.dcf_symbol.is_empty() {
                self.dcf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dcf;
            egui::Window::new("DCF — Discounted Cash Flow (FCFF)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 520.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.dcf_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.dcf_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dcf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_dcf(&conn, &sym_u) {
                                        self.dcf_snapshot = snap;
                                        self.dcf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Growth %").color(AXIS_TEXT).small());
                        ui.add(egui::DragValue::new(&mut self.dcf_growth_pct).speed(0.1).range(-20.0..=40.0));
                        ui.label(egui::RichText::new("Terminal g %").color(AXIS_TEXT).small());
                        ui.add(egui::DragValue::new(&mut self.dcf_terminal_growth_pct).speed(0.1).range(0.0..=5.0));
                        ui.label(egui::RichText::new("Years").color(AXIS_TEXT).small());
                        ui.add(egui::DragValue::new(&mut self.dcf_projection_years).range(3..=15));
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dcf_symbol.to_uppercase();
                            self.dcf_loading = true;
                            self.dcf_symbol = sym.clone();
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    // Gather inputs from cached fundamentals + trailing 4Q financials.
                                    let fund = typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym)
                                        .unwrap_or(None).unwrap_or_default();
                                    let quarters = typhoon_engine::core::fundamentals::get_quarterly_financials(&conn, &sym)
                                        .unwrap_or_default();
                                    let base_revenue = quarters.iter().take(4).filter_map(|q| q.total_revenue).sum::<f64>();
                                    let base_fcff = quarters.iter().take(4).filter_map(|q| q.free_cash_flow).sum::<f64>();
                                    let wacc_pct = if self.wacc_snapshot.symbol.eq_ignore_ascii_case(&sym) && self.wacc_snapshot.wacc_pct > 0.0 {
                                        self.wacc_snapshot.wacc_pct
                                    } else { 10.0 };
                                    let _ = self.broker_tx.send(BrokerCmd::ComputeDcfSnapshot {
                                        symbol: sym,
                                        base_revenue,
                                        base_fcff,
                                        growth_pct: self.dcf_growth_pct,
                                        terminal_growth_pct: self.dcf_terminal_growth_pct,
                                        wacc_pct,
                                        tax_rate_pct: 21.0,
                                        projection_years: self.dcf_projection_years,
                                        total_debt: fund.total_debt.unwrap_or(0.0),
                                        cash_and_equivalents: fund.cash_and_equivalents.unwrap_or(0.0),
                                        shares_outstanding: fund.shares_outstanding.unwrap_or(0.0),
                                    });
                                }
                            }
                        }
                        if self.dcf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.dcf_snapshot;
                    if snap.symbol.is_empty() {
                        ui.label(egui::RichText::new("No data — run DES + FA/IS/CF for this symbol first, then click Compute.")
                            .color(AXIS_TEXT).small());
                        ui.label(egui::RichText::new("Tip: run WACC for this symbol to use it as the discount rate.").color(AXIS_TEXT).small());
                    } else {
                        let color = if snap.implied_price > 0.0 { UP } else { DOWN };
                        ui.label(egui::RichText::new(format!("{} — implied price ${:.2}", snap.symbol, snap.implied_price))
                            .strong().size(16.0).color(color));
                        ui.label(egui::RichText::new(format!("{} — as of {} — WACC {:.2}%", snap.method, snap.as_of, snap.wacc_pct))
                            .color(AXIS_TEXT).small());
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("dcf_sum_grid").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                                let row = |ui: &mut egui::Ui, k: &str, v: String| {
                                    ui.label(egui::RichText::new(k).color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(v).small().monospace().strong());
                                    ui.end_row();
                                };
                                row(ui, "Base revenue (TTM)", format!("${:.0}M", snap.base_revenue / 1e6));
                                row(ui, "Base FCFF (TTM)",    format!("${:.0}M", snap.base_fcff / 1e6));
                                row(ui, "FCFF margin",         format!("{:.2}%", snap.fcff_margin_pct));
                                row(ui, "Revenue growth",      format!("{:.2}%", snap.growth_pct));
                                row(ui, "Terminal growth",     format!("{:.2}%", snap.terminal_growth_pct));
                                row(ui, "Enterprise value",    format!("${:.0}M", snap.enterprise_value / 1e6));
                                row(ui, "(+) Cash",            format!("${:.0}M", snap.cash_and_equivalents / 1e6));
                                row(ui, "(-) Debt",            format!("${:.0}M", snap.total_debt / 1e6));
                                row(ui, "Equity value",        format!("${:.0}M", snap.equity_value / 1e6));
                                row(ui, "Shares outstanding",  format!("{:.0}M", snap.shares_outstanding / 1e6));
                                row(ui, "Implied price",       format!("${:.2}", snap.implied_price));
                            });
                            ui.separator();
                            egui::Grid::new("dcf_years_grid").striped(true).num_columns(6).min_col_width(80.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Year").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Revenue").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("EBIT").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("NOPAT").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("FCFF").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("PV FCFF").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for y in &snap.years {
                                    ui.label(egui::RichText::new(format!("{}", y.year)).small().monospace().strong());
                                    ui.label(egui::RichText::new(format!("${:.0}M", y.revenue / 1e6)).small().monospace());
                                    ui.label(egui::RichText::new(format!("${:.0}M", y.ebit / 1e6)).small().monospace());
                                    ui.label(egui::RichText::new(format!("${:.0}M", y.nopat / 1e6)).small().monospace());
                                    ui.label(egui::RichText::new(format!("${:.0}M", y.fcff / 1e6)).small().monospace());
                                    ui.label(egui::RichText::new(format!("${:.0}M", y.pv_fcff / 1e6)).small().monospace());
                                    ui.end_row();
                                }
                            });
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small().italics());
                        }
                    }
                });
            self.show_dcf = open;
        }

        // SVM — Stock Valuation Model synthesis
        if self.show_svm {
            if self.svm_symbol.is_empty() {
                self.svm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_svm;
            egui::Window::new("SVM — Stock Valuation Model")
                .open(&mut open)
                .resizable(true)
                .default_size([680.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.svm_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.svm_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.svm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_svm(&conn, &sym_u) {
                                        self.svm_snapshot = snap;
                                        self.svm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.svm_symbol.to_uppercase();
                            self.svm_loading = true;
                            self.svm_symbol = sym.clone();
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let self_fund = typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym)
                                        .unwrap_or(None).unwrap_or_default();
                                    let current_price = self_fund.stock_price.unwrap_or(0.0);
                                    let ddm = typhoon_engine::core::research::get_ddm(&conn, &sym).unwrap_or(None);
                                    let dcf = typhoon_engine::core::research::get_dcf(&conn, &sym).unwrap_or(None);
                                    let quarters = typhoon_engine::core::fundamentals::get_quarterly_financials(&conn, &sym)
                                        .unwrap_or_default();
                                    let ttm_eps: Option<f64> = {
                                        let s: f64 = quarters.iter().take(4).filter_map(|q| q.eps).sum();
                                        if s > 0.0 { Some(s) } else { None }
                                    };
                                    let ttm_ebitda: Option<f64> = {
                                        let s: f64 = quarters.iter().take(4).filter_map(|q| q.ebitda).sum();
                                        if s > 0.0 { Some(s) } else { None }
                                    };
                                    let peer_syms = typhoon_engine::core::research::get_peers(&conn, &sym)
                                        .unwrap_or(None).unwrap_or_default();
                                    let peers: Vec<typhoon_engine::core::fundamentals::Fundamentals> = peer_syms.iter()
                                        .filter(|p| !p.eq_ignore_ascii_case(&sym))
                                        .filter_map(|p| typhoon_engine::core::fundamentals::get_fundamentals(&conn, p).unwrap_or(None))
                                        .collect();
                                    fn median(mut v: Vec<f64>) -> Option<f64> {
                                        if v.is_empty() { return None; }
                                        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                                        let m = v.len() / 2;
                                        Some(if v.len() % 2 == 0 { (v[m-1] + v[m]) / 2.0 } else { v[m] })
                                    }
                                    let peer_pe = median(peers.iter().filter_map(|p| p.pe_ratio).collect())
                                        .and_then(|m| ttm_eps.map(|e| (m, e)));
                                    let peer_ev = median(peers.iter().filter_map(|p| p.ev_to_ebitda).collect())
                                        .and_then(|m| {
                                            let ebitda = ttm_ebitda?;
                                            let shares = self_fund.shares_outstanding?;
                                            Some((
                                                m, ebitda,
                                                self_fund.total_debt.unwrap_or(0.0),
                                                self_fund.cash_and_equivalents.unwrap_or(0.0),
                                                shares,
                                            ))
                                        });
                                    // BVPS: approximate book value / shares from market_cap / p/b and shares.
                                    let bvps: Option<f64> = (|| {
                                        let mc = self_fund.market_cap?;
                                        let shares = self_fund.shares_outstanding?;
                                        let pb = self_fund.price_to_book?;
                                        if pb > 0.0 && shares > 0.0 {
                                            Some((mc / pb) / shares)
                                        } else { None }
                                    })();
                                    let peer_pb = median(peers.iter().filter_map(|p| p.price_to_book).collect())
                                        .and_then(|m| bvps.map(|bv| (m, bv)));

                                    let ddm_json = serde_json::to_string(&ddm).unwrap_or_else(|_| "null".to_string());
                                    let dcf_json = serde_json::to_string(&dcf).unwrap_or_else(|_| "null".to_string());
                                    let peer_pe_tuple_json = serde_json::to_string(&peer_pe).unwrap_or_else(|_| "null".to_string());
                                    let peer_ev_tuple_json = serde_json::to_string(&peer_ev).unwrap_or_else(|_| "null".to_string());
                                    let peer_pb_tuple_json = serde_json::to_string(&peer_pb).unwrap_or_else(|_| "null".to_string());

                                    let _ = self.broker_tx.send(BrokerCmd::ComputeSvmSnapshot {
                                        symbol: sym, current_price,
                                        ddm_json, dcf_json,
                                        peer_pe_tuple_json, peer_ev_tuple_json, peer_pb_tuple_json,
                                    });
                                }
                            }
                        }
                        if self.svm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.svm_snapshot;
                    if snap.symbol.is_empty() || snap.rows.is_empty() {
                        ui.label(egui::RichText::new("No data — run DDM/DCF/PEERS for this symbol first, then click Compute.")
                            .color(AXIS_TEXT).small());
                    } else {
                        let color = if snap.upside_mid_pct >= 0.0 { UP } else { DOWN };
                        ui.label(egui::RichText::new(format!("{} — current ${:.2} — fair mid ${:.2} ({:+.2}%)",
                            snap.symbol, snap.current_price, snap.fair_mid, snap.upside_mid_pct))
                            .strong().size(16.0).color(color));
                        ui.label(egui::RichText::new(format!("Fair range ${:.2} – ${:.2} — as of {}",
                            snap.fair_low, snap.fair_high, snap.as_of)).color(AXIS_TEXT).small());
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("svm_grid").striped(true).num_columns(5).min_col_width(110.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Model").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Implied").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Upside").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Confidence").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Source").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for r in &snap.rows {
                                    let rc = if r.upside_pct >= 0.0 { UP } else { DOWN };
                                    ui.label(egui::RichText::new(&r.model).small().monospace().strong());
                                    ui.label(egui::RichText::new(format!("${:.2}", r.implied_price)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:+.2}%", r.upside_pct)).color(rc).small().monospace());
                                    ui.label(egui::RichText::new(&r.confidence).small().monospace());
                                    ui.label(egui::RichText::new(&r.source).small().monospace());
                                    ui.end_row();
                                }
                            });
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).color(AXIS_TEXT).small().italics());
                        }
                    }
                });
            self.show_svm = open;
        }

        // OMON — Options chain monitor
        if self.show_omon {
            if self.omon_symbol.is_empty() {
                self.omon_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_omon;
            egui::Window::new("OMON — Options Chain Monitor")
                .open(&mut open)
                .resizable(true)
                .default_size([780.0, 560.0])
                .max_size([780.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.omon_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.omon_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.omon_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_options_chain(&conn, &sym_u) {
                                        self.omon_snapshot = snap;
                                        self.omon_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Fetch").fill(BTN_MG)).clicked() {
                            let sym = self.omon_symbol.to_uppercase();
                            self.omon_loading = true;
                            self.omon_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchOptionsChain { symbol: sym });
                        }
                        if self.omon_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.omon_snapshot;
                    if snap.symbol.is_empty() || snap.expirations.is_empty() {
                        ui.label(egui::RichText::new("No data — click Fetch to pull the nearest expiration from Yahoo (no key).")
                            .color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!("{} — underlying ${:.2} — {} expiry — as of {}",
                            snap.symbol, snap.underlying_price, snap.expirations.len(), snap.as_of))
                            .strong().color(AXIS_TEXT));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for exp in &snap.expirations {
                                ui.label(egui::RichText::new(format!("Expiry {} ({} days) — {} calls / {} puts",
                                    exp.expiration, exp.days_to_expiry, exp.calls.len(), exp.puts.len()))
                                    .strong().color(AXIS_TEXT));
                                egui::Grid::new(format!("omon_calls_{}", exp.expiration)).striped(true).num_columns(7).min_col_width(70.0).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Strike").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("C Last").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("C IV").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("C Vol").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("P Last").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("P IV").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("P Vol").color(AXIS_TEXT).small().strong());
                                    ui.end_row();
                                    let mut strikes: Vec<f64> = exp.calls.iter().map(|c| c.strike).collect();
                                    for p in &exp.puts {
                                        if !strikes.iter().any(|s| (s - p.strike).abs() < 1e-6) {
                                            strikes.push(p.strike);
                                        }
                                    }
                                    strikes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                                    for k in strikes.iter().take(40) {
                                        let call = exp.calls.iter().find(|c| (c.strike - k).abs() < 1e-6);
                                        let put = exp.puts.iter().find(|p| (p.strike - k).abs() < 1e-6);
                                        ui.label(egui::RichText::new(format!("{:.2}", k)).small().monospace().strong());
                                        if let Some(c) = call {
                                            ui.label(egui::RichText::new(format!("{:.2}", c.last_price)).small().monospace());
                                            ui.label(egui::RichText::new(format!("{:.1}%", c.implied_volatility * 100.0)).small().monospace());
                                            ui.label(egui::RichText::new(format!("{:.0}", c.volume)).small().monospace());
                                        } else {
                                            ui.label(""); ui.label(""); ui.label("");
                                        }
                                        if let Some(p) = put {
                                            ui.label(egui::RichText::new(format!("{:.2}", p.last_price)).small().monospace());
                                            ui.label(egui::RichText::new(format!("{:.1}%", p.implied_volatility * 100.0)).small().monospace());
                                            ui.label(egui::RichText::new(format!("{:.0}", p.volume)).small().monospace());
                                        } else {
                                            ui.label(""); ui.label(""); ui.label("");
                                        }
                                        ui.end_row();
                                    }
                                });
                                ui.separator();
                            }
                        });
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(AXIS_TEXT).small().italics());
                        }
                    }
                });
            self.show_omon = open;
        }

        // IVOL — Implied volatility rank / percentile
        if self.show_ivol {
            if self.ivol_symbol.is_empty() {
                self.ivol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ivol;
            egui::Window::new("IVOL — Implied Vol Rank / Percentile")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ivol_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ivol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ivol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ivol(&conn, &sym_u)
                                    {
                                        self.ivol_snapshot = snap;
                                        self.ivol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ivol_symbol.to_uppercase();
                            self.ivol_loading = true;
                            self.ivol_symbol = sym.clone();
                            // Derive current ATM IV from cached OMON nearest-expiry nearest-to-money option.
                            let (current_iv, history_json) = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let chain = typhoon_engine::core::research::get_options_chain(
                                        &conn, &sym,
                                    )
                                    .unwrap_or(None);
                                    let iv = chain
                                        .as_ref()
                                        .and_then(|c| {
                                            let exp = c.expirations.first()?;
                                            let spot = c.underlying_price;
                                            let mut all = Vec::with_capacity(
                                                exp.calls.len() + exp.puts.len(),
                                            );
                                            all.extend(exp.calls.iter().cloned());
                                            all.extend(exp.puts.iter().cloned());
                                            all.sort_by(|a, b| {
                                                (a.strike - spot)
                                                    .abs()
                                                    .partial_cmp(&(b.strike - spot).abs())
                                                    .unwrap_or(std::cmp::Ordering::Equal)
                                            });
                                            all.first().map(|c| c.implied_volatility * 100.0)
                                        })
                                        .unwrap_or(0.0);
                                    // History = prior IvolSnapshot.history entries rolled forward
                                    let prior =
                                        typhoon_engine::core::research::get_ivol(&conn, &sym)
                                            .unwrap_or(None);
                                    let mut hist: Vec<
                                        typhoon_engine::core::research::IvolObservation,
                                    > = prior.map(|p| p.history).unwrap_or_default();
                                    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                                    if iv > 0.0 {
                                        hist.retain(|h| h.date != today);
                                        hist.push(
                                            typhoon_engine::core::research::IvolObservation {
                                                date: today,
                                                atm_iv_pct: iv,
                                            },
                                        );
                                    }
                                    (
                                        iv,
                                        serde_json::to_string(&hist)
                                            .unwrap_or_else(|_| "[]".to_string()),
                                    )
                                } else {
                                    (0.0, "[]".to_string())
                                }
                            } else {
                                (0.0, "[]".to_string())
                            };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeIvolSnapshot {
                                symbol: sym,
                                current_atm_iv_pct: current_iv,
                                history_json,
                            });
                        }
                        if self.ivol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ivol_snapshot;
                    if snap.symbol.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No data — run OMON to pull today's ATM IV, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — ATM IV {:.1}% — as of {}",
                                snap.symbol, snap.current_atm_iv_pct, snap.as_of
                            ))
                            .strong()
                            .color(AXIS_TEXT),
                        );
                        ui.separator();
                        egui::Grid::new("ivol_grid")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                let row = |ui: &mut egui::Ui, k: &str, v: String| {
                                    ui.label(egui::RichText::new(k).color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(v).small().monospace().strong());
                                    ui.end_row();
                                };
                                row(
                                    ui,
                                    "Current ATM IV",
                                    format!("{:.2}%", snap.current_atm_iv_pct),
                                );
                                row(ui, "52w low", format!("{:.2}%", snap.iv_52w_low_pct));
                                row(ui, "52w high", format!("{:.2}%", snap.iv_52w_high_pct));
                                row(ui, "IV rank", format!("{:.1}", snap.iv_rank));
                                row(ui, "IV percentile", format!("{:.1}", snap.iv_percentile));
                                row(ui, "Observations", format!("{}", snap.observation_count));
                            });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(
                                egui::RichText::new(&snap.note)
                                    .color(AXIS_TEXT)
                                    .small()
                                    .italics(),
                            );
                        }
                    }
                });
            self.show_ivol = open;
        }
    }
}
