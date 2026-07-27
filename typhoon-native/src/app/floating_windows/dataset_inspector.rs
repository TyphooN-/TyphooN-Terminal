//! Dataset Inspector — the tabular half of "chart and table inspection"
//! (ADR-135 §11.2, milestone M0).
//!
//! The window is a pure view over
//! [`DatasetInspectorState`](crate::app::dataset_inspector_model::DatasetInspectorState):
//! it draws the bounded page the background worker last delivered and never
//! opens the store, walks a database, or aggregates anything itself
//! (ADR-098, ADR-134). Every button turns into a `DatasetJob` on the bounded
//! worker queue; a full queue is reported to the user, not waited on.
//!
//! Row drawing is virtualized through `ScrollArea::show_rows`, so the per-frame
//! cost is the number of *visible* rows rather than the page size.

use super::*;
use crate::app::dataset_inspector_model::{
    DATASET_INSPECTOR_PAGE_SIZES, DatasetInspectorRow, DatasetInspectorState, clamp_page_size,
    last_page_offset, next_page_offset, previous_page_offset,
};
use typhoon_engine::core::strategy_dataset::{
    AdjustmentPolicy, CalendarPolicy, DatasetManifestInput, DatasetProvenance, DatasetQaPolicy,
    DatasetQaSeverity,
};
use typhoon_engine::core::strategy_dataset_worker::{DatasetJob, DatasetWorker};

/// Row height for the virtualized bar table.
const DATASET_ROW_HEIGHT: f32 = 18.0;

/// Which dataset-side calendar policy a chart's bars should be judged under.
///
/// Deliberately coarse and derived only from what the chart already knows.
/// Getting this wrong produces noisy QA warnings, not wrong data — and the
/// choice is recorded in the manifest, so a reader can see what was assumed.
fn calendar_for_chart(symbol: &str, primary_source: &str) -> CalendarPolicy {
    if primary_source == "kraken-futures" || typhoon_engine::core::news::is_crypto_symbol(symbol) {
        CalendarPolicy::Continuous24x7
    } else if primary_source == "kraken-equities" {
        CalendarPolicy::XStock24x5
    } else {
        CalendarPolicy::UsEquityRegular
    }
}

fn severity_color(severity: Option<DatasetQaSeverity>) -> Option<egui::Color32> {
    match severity {
        Some(DatasetQaSeverity::Error) => Some(egui::Color32::from_rgb(232, 90, 90)),
        Some(DatasetQaSeverity::Warning) => Some(egui::Color32::from_rgb(226, 176, 76)),
        Some(DatasetQaSeverity::Info) => Some(egui::Color32::from_rgb(120, 170, 220)),
        None => None,
    }
}

impl TyphooNApp {
    /// Submit `job`, recording backpressure instead of blocking on it.
    fn submit_dataset_job(&mut self, job: DatasetJob) {
        let Some(worker) = self.dataset_worker.as_ref() else {
            self.dataset_inspector.status =
                "Dataset worker is not running; reopen the window.".to_string();
            self.dataset_inspector.pending = None;
            return;
        };
        if let Err(error) = worker.submit(job) {
            self.dataset_inspector.note_submit_failure(error);
        }
    }

    fn request_dataset_list(&mut self) {
        let request_id = self.dataset_inspector.begin_request();
        self.submit_dataset_job(DatasetJob::List {
            request_id,
            limit: DatasetInspectorState::list_limit(),
        });
    }

    fn request_dataset_page(&mut self, dataset_id: String, offset: u64) {
        let limit = self.dataset_inspector.effective_page_size();
        let request_id = self.dataset_inspector.begin_request();
        self.submit_dataset_job(DatasetJob::ReadPage {
            request_id,
            dataset_id,
            offset,
            limit,
        });
    }

    /// Materialize the active chart's in-memory bars as an immutable dataset.
    ///
    /// The bars are already in RAM for rendering, so this copies them and hands
    /// them to the worker — it does not read the cache on the render thread.
    fn materialize_active_chart_dataset(&mut self) {
        let Some(chart) = self.charts.get(self.active_tab) else {
            self.dataset_inspector.status = "No active chart.".to_string();
            return;
        };
        if chart.bars.is_empty() {
            self.dataset_inspector.status = "Active chart has no bars to materialize.".to_string();
            return;
        }

        let symbol = chart.symbol.trim().to_string();
        if symbol.is_empty() {
            self.dataset_inspector.status = "Active chart has no symbol.".to_string();
            return;
        }
        let source = if chart.primary_source.is_empty() {
            "unknown"
        } else {
            chart.primary_source
        };
        let bars: Vec<EngineBar> = chart
            .bars
            .iter()
            .map(|b| EngineBar {
                timestamp: chrono::DateTime::from_timestamp_millis(b.ts_ms)
                    .unwrap_or_default()
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
            })
            .collect();

        let input = DatasetManifestInput {
            symbol: symbol.clone(),
            timeframe: chart.timeframe.cache_suffix().to_string(),
            provenance: DatasetProvenance {
                source: source.to_string(),
                venue: source.to_string(),
                pipeline: "chart-window/v1".to_string(),
            },
            // The chart draws whatever the merge layer produced; the honest
            // label for it is "as reported", not a claim of split adjustment.
            adjustment: AdjustmentPolicy::Raw,
            calendar: calendar_for_chart(&symbol, source),
            qa_policy: DatasetQaPolicy::default(),
        };

        let request_id = self.dataset_inspector.begin_request();
        self.dataset_inspector.status = format!("Materializing {} bar(s)…", bars.len());
        self.submit_dataset_job(DatasetJob::Build {
            request_id,
            input,
            bars,
        });
    }

    /// Drain the worker's bounded event batch. Safe to call every frame: it is
    /// a non-blocking `try_recv` loop with a hard per-call ceiling.
    fn pump_dataset_worker(&mut self) {
        let Some(worker) = self.dataset_worker.as_ref() else {
            return;
        };
        for event in worker.poll() {
            self.dataset_inspector.apply_event(event);
        }
    }

    pub(super) fn render_dataset_inspector_window(&mut self, ctx: &egui::Context) {
        // Drained unconditionally: a job submitted just before the window was
        // closed still needs its reply consumed, or the worker parks on a full
        // event queue.
        self.pump_dataset_worker();
        if !self.show_dataset_inspector {
            return;
        }
        if self.dataset_worker.is_none() {
            self.start_dataset_worker();
        }

        let mut open = self.show_dataset_inspector;
        let mut pending_list = false;
        let mut pending_build = false;
        let mut pending_page: Option<(String, u64)> = None;

        egui::Window::new("Dataset Inspector")
            .open(&mut open)
            .resizable(true)
            .default_size([980.0, 620.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button("Refresh")
                        .on_hover_text(
                            "List stored datasets (bounded query, off the render thread)",
                        )
                        .clicked()
                    {
                        pending_list = true;
                    }
                    if ui
                        .button("Materialize active chart")
                        .on_hover_text(
                            "Content-address the active chart's bars, run dataset QA, and store \
                             the result immutably (ADR-135 §5.1/§11.1).",
                        )
                        .clicked()
                    {
                        pending_build = true;
                    }
                    if self.dataset_inspector.pending.is_some() {
                        ui.spinner();
                        ui.label(egui::RichText::new("working…").weak());
                    }
                });
                if !self.dataset_inspector.status.is_empty() {
                    ui.label(egui::RichText::new(&self.dataset_inspector.status).weak());
                }
                ui.separator();

                ui.label(egui::RichText::new("Stored datasets").strong());
                egui::ScrollArea::vertical()
                    .id_salt("dataset_inspector_records")
                    .max_height(120.0)
                    .show(ui, |ui| {
                        if self.dataset_inspector.records.is_empty() {
                            ui.label(
                                egui::RichText::new(
                                    "None yet — press Refresh, or materialize the active chart.",
                                )
                                .weak(),
                            );
                        }
                        for record in &self.dataset_inspector.records {
                            let selected = self.dataset_inspector.selected.as_deref()
                                == Some(&record.dataset_id);
                            let label = format!(
                                "{}  ·  {} bars  ·  {} err / {} warn  ·  {}",
                                record.title(),
                                record.bar_count,
                                record.qa_error_count,
                                record.qa_warning_count,
                                &record.dataset_id[..12]
                            );
                            if ui.selectable_label(selected, label).clicked() {
                                pending_page = Some((record.dataset_id.clone(), 0));
                            }
                        }
                    });
                ui.separator();

                let Some(summary) = self.dataset_inspector.summary.clone() else {
                    ui.label(
                        egui::RichText::new("Select a dataset to inspect its bars and QA flags.")
                            .weak(),
                    );
                    return;
                };

                egui::Grid::new("dataset_inspector_manifest")
                    .num_columns(4)
                    .spacing([18.0, 3.0])
                    .show(ui, |ui| {
                        ui.label("Symbol");
                        ui.label(egui::RichText::new(&summary.symbol).strong());
                        ui.label("Timeframe");
                        ui.label(egui::RichText::new(&summary.timeframe).strong());
                        ui.end_row();

                        ui.label("Source");
                        ui.label(&summary.source);
                        ui.label("Venue");
                        ui.label(&summary.venue);
                        ui.end_row();

                        ui.label("Pipeline");
                        ui.label(&summary.pipeline);
                        ui.label("Adjustment");
                        ui.label(summary.adjustment.wire_id());
                        ui.end_row();

                        ui.label("Calendar");
                        ui.label(&summary.calendar_policy_id);
                        ui.label("QA policy");
                        ui.label(&summary.qa_policy_id);
                        ui.end_row();

                        ui.label("Range");
                        ui.label(format!(
                            "{} → {}",
                            summary.first_timestamp.as_deref().unwrap_or("—"),
                            summary.last_timestamp.as_deref().unwrap_or("—")
                        ));
                        ui.label("Bars");
                        ui.label(summary.bar_count.to_string());
                        ui.end_row();

                        ui.label("Dataset id");
                        ui.label(egui::RichText::new(&summary.dataset_id).monospace().small());
                        ui.label("Manifest seal");
                        ui.label(
                            egui::RichText::new(&summary.manifest_id)
                                .monospace()
                                .small(),
                        );
                        ui.end_row();
                    });

                if let Some(qa) = &self.dataset_inspector.qa {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("QA:");
                        ui.colored_label(
                            severity_color(Some(DatasetQaSeverity::Error))
                                .unwrap_or(egui::Color32::GRAY),
                            format!("{} error(s)", qa.error_count),
                        );
                        ui.colored_label(
                            severity_color(Some(DatasetQaSeverity::Warning))
                                .unwrap_or(egui::Color32::GRAY),
                            format!("{} warning(s)", qa.warning_count),
                        );
                        ui.label(format!("gaps: {}", qa.gap_detection));
                        ui.label(format!("spikes: {}", qa.spike_detection));
                        if qa.findings_truncated {
                            ui.colored_label(
                                egui::Color32::from_rgb(232, 90, 90),
                                format!("findings capped — {} omitted", qa.findings_omitted),
                            );
                        }
                    });
                }
                ui.separator();

                let page_size = self.dataset_inspector.effective_page_size();
                let offset = self.dataset_inspector.page_offset;
                let total = self.dataset_inspector.total_bars;
                ui.horizontal(|ui| {
                    let dataset_id = summary.dataset_id.clone();
                    if ui
                        .add_enabled(offset > 0, egui::Button::new("⏮ First"))
                        .clicked()
                    {
                        pending_page = Some((dataset_id.clone(), 0));
                    }
                    if ui
                        .add_enabled(offset > 0, egui::Button::new("◀ Prev"))
                        .clicked()
                    {
                        if let Some(target) = previous_page_offset(offset, page_size) {
                            pending_page = Some((dataset_id.clone(), target));
                        }
                    }
                    let has_next = next_page_offset(offset, page_size, total).is_some();
                    if ui
                        .add_enabled(has_next, egui::Button::new("Next ▶"))
                        .clicked()
                    {
                        if let Some(target) = next_page_offset(offset, page_size, total) {
                            pending_page = Some((dataset_id.clone(), target));
                        }
                    }
                    if ui
                        .add_enabled(has_next, egui::Button::new("Last ⏭"))
                        .clicked()
                    {
                        pending_page =
                            Some((dataset_id.clone(), last_page_offset(page_size, total)));
                    }

                    ui.separator();
                    ui.label("Rows/page:");
                    let mut chosen = page_size;
                    egui::ComboBox::from_id_salt("dataset_inspector_page_size")
                        .selected_text(page_size.to_string())
                        .show_ui(ui, |ui| {
                            for size in DATASET_INSPECTOR_PAGE_SIZES {
                                ui.selectable_value(&mut chosen, size, size.to_string());
                            }
                        });
                    if chosen != page_size {
                        self.dataset_inspector.page_size = clamp_page_size(chosen);
                        pending_page = Some((dataset_id, offset));
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("#").monospace().strong());
                    ui.add_space(46.0);
                    ui.label(egui::RichText::new("timestamp").monospace().strong());
                    ui.add_space(96.0);
                    ui.label(
                        egui::RichText::new("open  high  low  close  volume  ·  QA flags")
                            .monospace()
                            .strong(),
                    );
                });

                let rows = &self.dataset_inspector.rows;
                egui::ScrollArea::vertical()
                    .id_salt("dataset_inspector_rows")
                    .auto_shrink([false, false])
                    .show_rows(ui, DATASET_ROW_HEIGHT, rows.len(), |ui, range| {
                        for row in &rows[range] {
                            draw_dataset_row(ui, row);
                        }
                    });
            });

        self.show_dataset_inspector = open;
        if pending_list {
            self.request_dataset_list();
        }
        if pending_build {
            self.materialize_active_chart_dataset();
        }
        if let Some((dataset_id, offset)) = pending_page {
            self.request_dataset_page(dataset_id, offset);
        }
    }

    /// Spawn the dataset worker over the on-disk store, once. Store creation
    /// and every filesystem operation happen on that worker, never in this
    /// frame callback.
    fn start_dataset_worker(&mut self) {
        let root = crate::app::platform::strategy_dataset_dir();
        let worker = DatasetWorker::spawn_at(root.clone()).map_err(|error| error.to_string());
        match worker {
            Ok(worker) => {
                self.dataset_worker = Some(worker);
                self.dataset_inspector.status = format!("Dataset store: {}", root.display());
                self.request_dataset_list();
            }
            Err(error) => {
                self.dataset_inspector.status =
                    format!("Cannot open dataset store at {}: {error}", root.display());
            }
        }
    }
}

/// One virtualized table row. Pure painting — no allocation beyond the
/// formatted numbers, and no lookups.
fn draw_dataset_row(ui: &mut egui::Ui, row: &DatasetInspectorRow) {
    ui.horizontal(|ui| {
        let tint = severity_color(row.severity);
        let mut text = egui::RichText::new(format!(
            "{:>8}  {:<26}  {:>12.6}  {:>12.6}  {:>12.6}  {:>12.6}  {:>14.2}",
            row.index, row.timestamp, row.open, row.high, row.low, row.close, row.volume
        ))
        .monospace()
        .small();
        if let Some(color) = tint {
            text = text.color(color);
        }
        ui.label(text);
        if !row.flags.is_empty() {
            ui.colored_label(
                tint.unwrap_or(egui::Color32::GRAY),
                egui::RichText::new(format!("· {}", row.flags)).small(),
            );
        }
    });
}
