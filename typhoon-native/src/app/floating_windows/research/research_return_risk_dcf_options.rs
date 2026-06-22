use super::*;

impl TyphooNApp {
    pub(super) fn render_research_return_risk_dcf_options_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hra_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hra_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hra_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hra(&conn, &sym_u)
                                    {
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
                            let rf = self
                                .treasury_yields
                                .iter()
                                .find(|y| y.tenor.contains("10"))
                                .map(|y| y.yield_pct)
                                .unwrap_or(4.0);
                            let _ = self.broker_tx.send(BrokerCmd::FetchHraSnapshot {
                                symbol: sym,
                                risk_free_pct: rf,
                            });
                        }
                        if self.hra_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_hra_snapshot(ui, &self.hra_snapshot);
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
                    super::render::render_dcf_snapshot(ui, &self.dcf_snapshot);
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
                    super::render::render_svm_snapshot(ui, &self.svm_snapshot);
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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.omon_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.omon_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.omon_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_options_chain(
                                            &conn, &sym_u,
                                        )
                                    {
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
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::FetchOptionsChain { symbol: sym });
                        }
                        if self.omon_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_omon_snapshot(ui, &self.omon_snapshot);
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
                    super::render::render_ivol_snapshot(ui, &self.ivol_snapshot);
                });
            self.show_ivol = open;
        }
    }
}
