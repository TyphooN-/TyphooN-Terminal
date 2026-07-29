//! Bounded native authoring and prepared-page databank presentation (ADR-135 M3).
//!
//! The renderers mutate transient builder drafts and draw immutable pages only.
//! Canonicalization stays in `typhoon-engine`; every SQLite operation is submitted
//! to the bounded `DatabankWorker` and completed pages are installed by request id.

use std::collections::BTreeSet;
use std::sync::Arc;

use typhoon_engine::core::strategy_builder::{
    DirectionConstraint, GeneralStrategyBuilder, IndicatorDraft, NnfxBuilderConfig, NnfxEntryMode,
    NnfxProfile,
};
use typhoon_engine::core::strategy_databank::{
    DatabankJob, DatabankPage, DatabankQuery, DatabankSort, DatabankSubmitError, DatabankWorker,
    DatabankWorkerEvent, MAX_COMPARE_RUNS, PutStrategyOutcome, StoredRun,
};
use typhoon_engine::core::strategy_ir::{IndicatorKind, StrategyIr};

use super::*;

pub(crate) struct StrategyResearchState {
    pub(crate) general: GeneralStrategyBuilder,
    pub(crate) nnfx: NnfxBuilderConfig,
    pub(crate) canonical_text: String,
    pub(crate) status: String,
    pub(crate) databank: DatabankBrowserState,
    pub(crate) palette_id: String,
    pub(crate) palette_period: u32,
    pub(crate) palette_kind: usize,
    pub(crate) baseline_id: String,
    pub(crate) strategy_load_id: String,
    artifact_request: Option<ArtifactRequest>,
    next_artifact_request_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArtifactRequest {
    Save {
        request_id: u64,
        strategy_id: String,
    },
    Load {
        request_id: u64,
        strategy_id: String,
    },
}

impl ArtifactRequest {
    fn request_id(&self) -> u64 {
        match self {
            Self::Save { request_id, .. } | Self::Load { request_id, .. } => *request_id,
        }
    }
}

impl StrategyResearchState {
    pub(crate) fn new() -> Self {
        Self {
            general: GeneralStrategyBuilder::new("Native strategy", "TyphooN operator"),
            nnfx: NnfxBuilderConfig::default(),
            canonical_text: String::new(),
            status: String::new(),
            databank: DatabankBrowserState::default(),
            palette_id: "baseline".into(),
            palette_period: 20,
            palette_kind: 2,
            baseline_id: "baseline".into(),
            strategy_load_id: String::new(),
            artifact_request: None,
            // Keep artifact ids disjoint from the browser's low-half sequence.
            next_artifact_request_id: 1_u64 << 63,
        }
    }

    pub(crate) fn seal_general(&mut self) -> Result<StrategyIr, String> {
        let ir = self.general.seal().map_err(|error| error.to_string())?;
        self.canonical_text = serde_json::to_string_pretty(&ir).map_err(|e| e.to_string())?;
        self.status = format!("Sealed canonical strategy {}", ir.strategy_id());
        Ok(ir)
    }

    pub(crate) fn seal_guided(&mut self) -> Result<StrategyIr, String> {
        let ir = self.nnfx.to_ir().map_err(|error| error.to_string())?;
        self.canonical_text = serde_json::to_string_pretty(&ir).map_err(|e| e.to_string())?;
        self.status = format!("Sealed guided strategy {}", ir.strategy_id());
        Ok(ir)
    }

    pub(crate) fn open_guided_in_general(&mut self) -> Result<(), String> {
        let ir = self.seal_guided()?;
        self.general = GeneralStrategyBuilder::from_definition(ir.to_input());
        let general = self.general.seal().map_err(|error| error.to_string())?;
        if general != ir {
            return Err("guided/general canonical identity mismatch".into());
        }
        self.status = format!(
            "Opened identical canonical strategy {} in general builder",
            ir.strategy_id()
        );
        Ok(())
    }

    pub(crate) fn clear_general(&mut self) {
        self.general = GeneralStrategyBuilder::new("Native strategy", "TyphooN operator");
        self.status = "Cleared transient general-builder draft".into();
    }

    pub(crate) fn load_canonical_text(&mut self) -> Result<(), String> {
        let mut loaded = GeneralStrategyBuilder::from_canonical_text(&self.canonical_text)
            .map_err(|error| error.to_string())?;
        let ir = loaded.seal().map_err(|error| error.to_string())?;
        self.general = loaded;
        self.canonical_text = serde_json::to_string_pretty(&ir).map_err(|e| e.to_string())?;
        self.status = format!("Reloaded verified canonical strategy {}", ir.strategy_id());
        Ok(())
    }

    fn next_artifact_request(&mut self) -> Result<u64, String> {
        if self.artifact_request.is_some() {
            return Err("a strategy save/load request is already active".into());
        }
        let request_id = self.next_artifact_request_id;
        self.next_artifact_request_id = self
            .next_artifact_request_id
            .checked_add(1)
            .filter(|id| *id != 0)
            .unwrap_or(1_u64 << 63);
        Ok(request_id)
    }

    pub(crate) fn submit_save_general(&mut self, worker: &DatabankWorker) -> Result<(), String> {
        let strategy = self.seal_general()?;
        self.submit_save(worker, strategy)
    }

    pub(crate) fn submit_save_guided(&mut self, worker: &DatabankWorker) -> Result<(), String> {
        let strategy = self.seal_guided()?;
        self.submit_save(worker, strategy)
    }

    fn submit_save(&mut self, worker: &DatabankWorker, strategy: StrategyIr) -> Result<(), String> {
        let request_id = self.next_artifact_request()?;
        let strategy_id = strategy.strategy_id().to_owned();
        worker
            .submit(DatabankJob::PutStrategy {
                request_id,
                strategy: Box::new(strategy),
            })
            .map_err(submit_error)?;
        self.artifact_request = Some(ArtifactRequest::Save {
            request_id,
            strategy_id: strategy_id.clone(),
        });
        self.status = format!("Saving canonical strategy {strategy_id} off the render thread");
        Ok(())
    }

    pub(crate) fn submit_load_strategy(&mut self, worker: &DatabankWorker) -> Result<(), String> {
        let strategy_id = self.strategy_load_id.trim().to_owned();
        if strategy_id.is_empty() {
            return Err("enter an exact strategy id to reload".into());
        }
        let request_id = self.next_artifact_request()?;
        worker
            .submit(DatabankJob::LoadStrategy {
                request_id,
                strategy_id: strategy_id.clone(),
            })
            .map_err(submit_error)?;
        self.artifact_request = Some(ArtifactRequest::Load {
            request_id,
            strategy_id: strategy_id.clone(),
        });
        self.status = format!("Loading verified canonical strategy {strategy_id}");
        Ok(())
    }

    pub(crate) fn cancel_artifact_request(&mut self, worker: &DatabankWorker) -> bool {
        let Some(request) = self.artifact_request.take() else {
            return false;
        };
        worker.cancel(request.request_id());
        self.status = "Strategy save/load cancelled; any completion is stale".into();
        true
    }

    pub(crate) fn artifact_request_active(&self) -> bool {
        self.artifact_request.is_some()
    }

    pub(crate) fn accept_artifact_event(&mut self, event: DatabankWorkerEvent) -> bool {
        let Some(active) = self.artifact_request.clone() else {
            return false;
        };
        if event.request_id() != active.request_id() {
            return false;
        }
        match (active, event) {
            (
                ArtifactRequest::Save { strategy_id, .. },
                DatabankWorkerEvent::StrategyPut { outcome, .. },
            ) => {
                self.artifact_request = None;
                self.strategy_load_id = strategy_id.clone();
                let verb = match outcome {
                    PutStrategyOutcome::Inserted => "Saved",
                    PutStrategyOutcome::AlreadyPresent => "Verified existing",
                };
                self.status = format!("{verb} canonical strategy {strategy_id}");
                true
            }
            (
                ArtifactRequest::Load { strategy_id, .. },
                DatabankWorkerEvent::StrategyLoaded { strategy, .. },
            ) => {
                self.artifact_request = None;
                if strategy.strategy_id() != strategy_id {
                    self.status = format!(
                        "Error: loaded strategy id {} did not match requested {strategy_id}",
                        strategy.strategy_id()
                    );
                    return true;
                }
                let mut general = GeneralStrategyBuilder::from_definition(strategy.to_input());
                match general.seal() {
                    Ok(resealed) if resealed == *strategy => {
                        self.canonical_text = match serde_json::to_string_pretty(&resealed) {
                            Ok(text) => text,
                            Err(error) => {
                                self.status =
                                    format!("Error: cannot encode loaded strategy: {error}");
                                return true;
                            }
                        };
                        self.general = general;
                        self.status =
                            format!("Reloaded exact verified canonical strategy {strategy_id}");
                    }
                    Ok(_) => {
                        self.status =
                            "Error: loaded strategy changed identity when opened in editor".into();
                    }
                    Err(error) => {
                        self.status = format!("Error: loaded strategy cannot open: {error}");
                    }
                }
                true
            }
            (_, DatabankWorkerEvent::Started { .. }) => {
                self.status = match self.artifact_request.as_ref() {
                    Some(ArtifactRequest::Save { strategy_id, .. }) => {
                        format!("Saving canonical strategy {strategy_id}")
                    }
                    Some(ArtifactRequest::Load { strategy_id, .. }) => {
                        format!("Loading verified canonical strategy {strategy_id}")
                    }
                    None => unreachable!(),
                };
                true
            }
            (_, DatabankWorkerEvent::Failed { message, .. }) => {
                self.artifact_request = None;
                self.status = format!("Error: {message}");
                true
            }
            (_, DatabankWorkerEvent::Cancelled { .. }) => {
                self.artifact_request = None;
                self.status = "Strategy save/load cancelled".into();
                true
            }
            _ => false,
        }
    }

    pub(crate) fn accept_databank_event(&mut self, event: DatabankWorkerEvent) -> bool {
        if self
            .artifact_request
            .as_ref()
            .is_some_and(|request| request.request_id() == event.request_id())
        {
            self.accept_artifact_event(event)
        } else {
            self.databank.accept_event(event)
        }
    }
}

pub(crate) struct DatabankBrowserState {
    pub(crate) strategy_filter: String,
    pub(crate) dataset_filter: String,
    pub(crate) tag_filter: String,
    pub(crate) min_profit: String,
    pub(crate) max_drawdown: String,
    pub(crate) sort: DatabankSort,
    pub(crate) page_size: usize,
    pub(crate) offset: usize,
    pub(crate) page: Option<Arc<DatabankPage>>,
    pub(crate) compare_selection: BTreeSet<String>,
    pub(crate) comparison: Option<Arc<[StoredRun]>>,
    pub(crate) status: String,
    next_request_id: u64,
    active_query: Option<u64>,
    active_compare: Option<u64>,
}

impl Default for DatabankBrowserState {
    fn default() -> Self {
        Self {
            strategy_filter: String::new(),
            dataset_filter: String::new(),
            tag_filter: String::new(),
            min_profit: String::new(),
            max_drawdown: String::new(),
            sort: DatabankSort::CreatedDesc,
            page_size: 50,
            offset: 0,
            page: None,
            compare_selection: BTreeSet::new(),
            comparison: None,
            status: String::new(),
            next_request_id: 0,
            active_query: None,
            active_compare: None,
        }
    }
}

impl DatabankBrowserState {
    fn next_request(&mut self) -> u64 {
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        self.next_request_id
    }

    pub(crate) fn begin_query(&mut self) -> u64 {
        let request = self.next_request();
        self.active_query = Some(request);
        request
    }

    pub(crate) fn query(&self) -> Result<DatabankQuery, String> {
        let optional = |text: &str| (!text.trim().is_empty()).then(|| text.trim().to_owned());
        let number = |text: &str, label: &str| -> Result<Option<f64>, String> {
            if text.trim().is_empty() {
                return Ok(None);
            }
            let value = text
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("{label} must be a number"))?;
            if !value.is_finite() {
                return Err(format!("{label} must be finite"));
            }
            Ok(Some(value))
        };
        Ok(DatabankQuery {
            strategy_id: optional(&self.strategy_filter),
            dataset_id: optional(&self.dataset_filter),
            tag: optional(&self.tag_filter),
            min_net_profit: number(&self.min_profit, "minimum profit")?,
            max_drawdown_percent: number(&self.max_drawdown, "maximum drawdown")?,
            sort: self.sort,
            offset: self.offset,
            limit: self.page_size,
        })
    }

    pub(crate) fn submit_query(&mut self, worker: &DatabankWorker) -> Result<(), String> {
        let query = self.query()?;
        let request_id = self.begin_query();
        worker
            .submit(DatabankJob::Query { request_id, query })
            .map_err(submit_error)?;
        self.status = format!("Loading prepared page at offset {}", self.offset);
        Ok(())
    }

    pub(crate) fn submit_compare(&mut self, worker: &DatabankWorker) -> Result<(), String> {
        if !(2..=MAX_COMPARE_RUNS).contains(&self.compare_selection.len()) {
            return Err(format!("select 2..={MAX_COMPARE_RUNS} runs"));
        }
        let request_id = self.next_request();
        self.active_compare = Some(request_id);
        worker
            .submit(DatabankJob::Compare {
                request_id,
                run_ids: self.compare_selection.iter().cloned().collect(),
            })
            .map_err(submit_error)?;
        self.status = "Loading bounded comparison".into();
        Ok(())
    }

    pub(crate) fn toggle_compare(&mut self, run_id: String) -> bool {
        if self.compare_selection.remove(&run_id) {
            return true;
        }
        if self.compare_selection.len() >= MAX_COMPARE_RUNS {
            self.status = format!("Comparison is limited to {MAX_COMPARE_RUNS} runs");
            return false;
        }
        self.compare_selection.insert(run_id);
        true
    }

    pub(crate) fn accept_event(&mut self, event: DatabankWorkerEvent) -> bool {
        match event {
            DatabankWorkerEvent::Page { request_id, page }
                if self.active_query == Some(request_id) =>
            {
                self.active_query = None;
                self.status = format!("Prepared {} rows off the render thread", page.rows.len());
                self.page = Some(Arc::new(page));
                true
            }
            DatabankWorkerEvent::Comparison { request_id, runs }
                if self.active_compare == Some(request_id) =>
            {
                self.active_compare = None;
                self.status = format!("Prepared comparison for {} runs", runs.len());
                self.comparison = Some(Arc::from(runs));
                true
            }
            DatabankWorkerEvent::Failed {
                request_id,
                message,
            } if self.active_query == Some(request_id)
                || self.active_compare == Some(request_id) =>
            {
                if self.active_query == Some(request_id) {
                    self.active_query = None;
                }
                if self.active_compare == Some(request_id) {
                    self.active_compare = None;
                }
                self.status = format!("Error: {message}");
                true
            }
            DatabankWorkerEvent::Cancelled { request_id }
                if self.active_query == Some(request_id)
                    || self.active_compare == Some(request_id) =>
            {
                if self.active_query == Some(request_id) {
                    self.active_query = None;
                }
                if self.active_compare == Some(request_id) {
                    self.active_compare = None;
                }
                self.status = "Databank request cancelled".into();
                true
            }
            _ => false,
        }
    }
}

fn submit_error(error: DatabankSubmitError) -> String {
    match error {
        DatabankSubmitError::QueueFull => "databank worker queue is full".into(),
        DatabankSubmitError::Stopped => "databank worker stopped".into(),
    }
}

const PALETTE: [(&str, IndicatorKind); 9] = [
    ("ATR", IndicatorKind::Atr),
    ("SMA", IndicatorKind::Sma),
    ("EMA", IndicatorKind::Ema),
    ("KAMA", IndicatorKind::Kama),
    ("RSI", IndicatorKind::Rsi),
    ("Fisher", IndicatorKind::FisherTransform),
    ("MACD", IndicatorKind::Macd),
    ("ADX", IndicatorKind::Adx),
    ("StdDev", IndicatorKind::StdDev),
];

impl TyphooNApp {
    fn ensure_databank_worker(&mut self) {
        if self.strategy_databank_worker.is_some() {
            return;
        }
        let path = platform::strategy_databank_path();
        let result = path
            .parent()
            .ok_or("databank path has no parent".to_string())
            .and_then(|parent| std::fs::create_dir_all(parent).map_err(|e| e.to_string()))
            .and_then(|()| DatabankWorker::spawn(path).map_err(|e| e.to_string()));
        match result {
            Ok(worker) => self.strategy_databank_worker = Some(worker),
            Err(error) => self.strategy_research.databank.status = format!("Error: {error}"),
        }
    }

    pub(super) fn render_strategy_research_windows(&mut self, ctx: &egui::Context) {
        if self.show_strategy_builder || self.show_nnfx_builder || self.show_strategy_databank {
            self.ensure_databank_worker();
        }
        let events = self
            .strategy_databank_worker
            .as_ref()
            .map(DatabankWorker::poll)
            .unwrap_or_default();
        for event in events {
            let _ = self.strategy_research.accept_databank_event(event);
        }
        self.render_general_builder(ctx);
        self.render_nnfx_builder(ctx);
        self.render_databank_browser(ctx);
    }

    fn render_general_builder(&mut self, ctx: &egui::Context) {
        if !self.show_strategy_builder {
            return;
        }
        let mut open = self.show_strategy_builder;
        egui::Window::new("Strategy Builder · Canonical IR").open(&mut open)
            .resizable(true).default_size([760.0, 620.0]).show(ctx, |ui| {
            ui.small("Bounded transient typed graph; only validated canonical Strategy IR is sealed.");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("strategy_palette").selected_text(PALETTE[self.strategy_research.palette_kind].0).show_ui(ui, |ui| {
                    for (index, (label, _)) in PALETTE.iter().enumerate() { ui.selectable_value(&mut self.strategy_research.palette_kind, index, *label); }
                });
                ui.add(egui::TextEdit::singleline(&mut self.strategy_research.palette_id).char_limit(64).hint_text("node id"));
                ui.add(egui::DragValue::new(&mut self.strategy_research.palette_period).range(1..=100_000));
                if ui.button("Add typed node").clicked() {
                    let (_, kind) = &PALETTE[self.strategy_research.palette_kind];
                    self.strategy_research.general.add_indicator(IndicatorDraft::new(self.strategy_research.palette_id.trim(), kind.clone(), self.strategy_research.palette_period));
                }
            });
            egui::ScrollArea::vertical().max_height(130.0).show(ui, |ui| for node in &self.strategy_research.general.definition().indicators {
                ui.monospace(format!("{} · {:?} · {} inputs", node.id, node.kind, node.inputs.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Baseline"); ui.add(egui::TextEdit::singleline(&mut self.strategy_research.baseline_id).char_limit(64));
                if ui.button("Wire crossover entry + exit").clicked() {
                    let id = self.strategy_research.baseline_id.trim().to_owned(); self.strategy_research.general.set_baseline_crossover(&id);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Validate + seal").clicked() && let Err(e) = self.strategy_research.seal_general() { self.strategy_research.status = format!("Error: {e}"); }
                if ui.add_enabled(!self.strategy_research.artifact_request_active(), egui::Button::new("Seal + save to databank")).clicked()
                    && let Some(worker) = self.strategy_databank_worker.as_ref()
                    && let Err(e) = self.strategy_research.submit_save_general(worker)
                { self.strategy_research.status = format!("Error: {e}"); }
                if ui.button("Clear draft").clicked() { self.strategy_research.clear_general(); }
                if ui.button("Reload canonical text").clicked() && let Err(e) = self.strategy_research.load_canonical_text() { self.strategy_research.status = format!("Error: {e}"); }
                if ui.button("Databank").clicked() { self.show_strategy_databank = true; }
            });
            ui.horizontal(|ui| {
                ui.label("Stored strategy id");
                ui.add(egui::TextEdit::singleline(&mut self.strategy_research.strategy_load_id).char_limit(128).desired_width(420.0));
                if ui.add_enabled(!self.strategy_research.artifact_request_active(), egui::Button::new("Reload exact stored artifact")).clicked()
                    && let Some(worker) = self.strategy_databank_worker.as_ref()
                    && let Err(e) = self.strategy_research.submit_load_strategy(worker)
                { self.strategy_research.status = format!("Error: {e}"); }
                if ui.add_enabled(self.strategy_research.artifact_request_active(), egui::Button::new("Cancel save/load")).clicked()
                    && let Some(worker) = self.strategy_databank_worker.as_ref()
                { let _ = self.strategy_research.cancel_artifact_request(worker); }
            });
            ui.label(&self.strategy_research.status);
            ui.add(egui::TextEdit::multiline(&mut self.strategy_research.canonical_text).code_editor().desired_rows(15).desired_width(f32::INFINITY).char_limit(1_048_576));
        });
        self.show_strategy_builder = open;
    }

    fn render_nnfx_builder(&mut self, ctx: &egui::Context) {
        if !self.show_nnfx_builder {
            return;
        }
        let mut open = self.show_nnfx_builder;
        egui::Window::new("NNFX Guided Builder")
            .open(&mut open)
            .resizable(true)
            .default_size([620.0, 540.0])
            .show(ctx, |ui| {
                let config = &mut self.strategy_research.nnfx;
                egui::ComboBox::from_label("Profile")
                    .selected_text(format!("{:?}", config.profile))
                    .show_ui(ui, |ui| {
                        for value in NnfxProfile::ALL {
                            ui.selectable_value(&mut config.profile, value, format!("{value:?}"));
                        }
                    });
                egui::ComboBox::from_label("Entry mode")
                    .selected_text(format!("{:?}", config.entry_mode))
                    .show_ui(ui, |ui| {
                        for value in NnfxEntryMode::ALL {
                            ui.selectable_value(
                                &mut config.entry_mode,
                                value,
                                format!("{value:?}"),
                            );
                        }
                    });
                egui::ComboBox::from_label("Direction")
                    .selected_text(format!("{:?}", config.direction))
                    .show_ui(ui, |ui| {
                        for value in DirectionConstraint::ALL {
                            ui.selectable_value(&mut config.direction, value, format!("{value:?}"));
                        }
                    });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut config.one_candle_rule, "One Candle");
                    ui.checkbox(&mut config.bridge_too_far_rule, "A Bridge Too Far");
                    ui.checkbox(&mut config.news_filter, "News");
                    ui.checkbox(&mut config.market_filter, "External market");
                });
                egui::Grid::new("nnfx_roles").striped(true).show(ui, |ui| {
                    ui.strong("Role");
                    ui.strong("Indicator");
                    ui.strong("Period");
                    ui.end_row();
                    for (label, slot) in [
                        ("ATR", &mut config.slots.atr),
                        ("Baseline", &mut config.slots.baseline),
                        ("C1", &mut config.slots.confirmation_1),
                        ("C2", &mut config.slots.confirmation_2),
                        ("Volume", &mut config.slots.volume),
                        ("Exit", &mut config.slots.exit),
                        ("Continuation", &mut config.slots.continuation),
                    ] {
                        ui.label(label);
                        egui::ComboBox::from_id_salt(("nnfx", label))
                            .selected_text(format!("{:?}", slot.kind))
                            .show_ui(ui, |ui| {
                                for (name, kind) in PALETTE.iter() {
                                    ui.selectable_value(&mut slot.kind, kind.clone(), *name);
                                }
                            });
                        ui.add(egui::DragValue::new(&mut slot.period).range(1..=100_000));
                        ui.end_row();
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Seal canonical IR").clicked()
                        && let Err(e) = self.strategy_research.seal_guided()
                    {
                        self.strategy_research.status = format!("Error: {e}");
                    }
                    if ui.button("Open identical IR in general builder").clicked() {
                        match self.strategy_research.open_guided_in_general() {
                            Ok(()) => self.show_strategy_builder = true,
                            Err(e) => self.strategy_research.status = format!("Error: {e}"),
                        }
                    }
                    if ui
                        .add_enabled(
                            !self.strategy_research.artifact_request_active(),
                            egui::Button::new("Seal + save to databank"),
                        )
                        .clicked()
                        && let Some(worker) = self.strategy_databank_worker.as_ref()
                        && let Err(e) = self.strategy_research.submit_save_guided(worker)
                    {
                        self.strategy_research.status = format!("Error: {e}");
                    }
                });
                ui.label(&self.strategy_research.status);
            });
        self.show_nnfx_builder = open;
    }

    fn render_databank_browser(&mut self, ctx: &egui::Context) {
        if !self.show_strategy_databank {
            return;
        }
        let mut open = self.show_strategy_databank;
        egui::Window::new("Strategy Databank").open(&mut open).resizable(true).default_size([920.0, 620.0]).show(ctx, |ui| {
            let browser = &mut self.strategy_research.databank;
            ui.small("Immutable prepared pages only; SQLite and comparisons run on the bounded worker.");
            ui.horizontal_wrapped(|ui| { ui.label("Strategy"); ui.add(egui::TextEdit::singleline(&mut browser.strategy_filter).char_limit(128)); ui.label("Dataset"); ui.add(egui::TextEdit::singleline(&mut browser.dataset_filter).char_limit(128)); ui.label("Tag"); ui.add(egui::TextEdit::singleline(&mut browser.tag_filter).char_limit(128)); });
            ui.horizontal(|ui| { ui.label("Min net"); ui.add(egui::TextEdit::singleline(&mut browser.min_profit).char_limit(32)); ui.label("Max DD %"); ui.add(egui::TextEdit::singleline(&mut browser.max_drawdown).char_limit(32)); egui::ComboBox::from_label("Sort").selected_text(format!("{:?}", browser.sort)).show_ui(ui, |ui| for sort in [DatabankSort::CreatedDesc, DatabankSort::NetProfitDesc, DatabankSort::DrawdownAsc, DatabankSort::SharpeDesc] { ui.selectable_value(&mut browser.sort, sort, format!("{sort:?}")); }); });
            let mut action = None;
            ui.horizontal(|ui| {
                if ui.button("Query").clicked() { browser.offset = 0; action = Some(false); }
                if ui.add_enabled(browser.offset > 0, egui::Button::new("Previous")).clicked() { browser.offset = browser.offset.saturating_sub(browser.page_size); action = Some(false); }
                if ui.add_enabled(browser.page.as_ref().is_some_and(|p| p.has_more), egui::Button::new("Next")).clicked() { browser.offset = browser.offset.saturating_add(browser.page_size); action = Some(false); }
                if ui.add_enabled((2..=MAX_COMPARE_RUNS).contains(&browser.compare_selection.len()), egui::Button::new("Compare")).clicked() { action = Some(true); }
                ui.label(format!("offset {} · compare {}/{}", browser.offset, browser.compare_selection.len(), MAX_COMPARE_RUNS));
            });
            if let (Some(compare), Some(worker)) = (action, self.strategy_databank_worker.as_ref()) { let result = if compare { browser.submit_compare(worker) } else { browser.submit_query(worker) }; if let Err(e) = result { browser.status = format!("Error: {e}"); } }
            ui.label(&browser.status);
            let page = browser.page.clone(); let mut toggles = Vec::new();
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| egui::Grid::new("databank_rows").striped(true).show(ui, |ui| {
                for h in ["✓", "Run", "Strategy", "Dataset", "Net", "DD %", "Sharpe"] { ui.strong(h); } ui.end_row();
                if let Some(page) = page.as_ref() { for row in &page.rows { let mut selected = browser.compare_selection.contains(&row.run_id); if ui.checkbox(&mut selected, "").changed() { toggles.push(row.run_id.clone()); } ui.monospace(&row.run_id); ui.monospace(&row.strategy_id); ui.monospace(&row.dataset_id); ui.label(format!("{:.3}", row.net_profit)); ui.label(format!("{:.3}", row.max_drawdown_percent)); ui.label(format!("{:.3}", row.sharpe_ratio)); ui.end_row(); } }
            }));
            for id in toggles { let _ = browser.toggle_compare(id); }
            if let Some(runs) = &browser.comparison { ui.heading("Prepared exact metric vectors"); for run in runs.iter() { ui.label(format!("{} · {} · {} metrics", run.run_id, run.metrics_version, run.metrics.len())); } }
        });
        self.show_strategy_databank = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typhoon_engine::core::strategy_builder::IndicatorDraft;
    use typhoon_engine::core::strategy_databank::{
        DatabankRow, DatabankRunInput, MAX_COMPARE_RUNS,
    };
    use typhoon_engine::core::strategy_ir::IndicatorKind;

    use crate::app::strategy_sub_bar_run::{
        RunChartContext, RunRequestIdentity, StrategyRunJob, execute_strategy_run_job,
    };

    #[test]
    fn general_and_guided_editors_share_one_canonical_artifact() {
        let mut state = StrategyResearchState::new();
        state.open_guided_in_general().expect("guided graph");
        let guided = state.nnfx.to_ir().unwrap();
        let general = state.general.seal().unwrap();
        assert_eq!(guided, general);
        assert_eq!(
            state.canonical_text,
            serde_json::to_string_pretty(&guided).unwrap()
        );

        state.general = GeneralStrategyBuilder::new("native", "operator");
        state
            .general
            .add_indicator(IndicatorDraft::new("baseline", IndicatorKind::Ema, 20));
        state.general.set_baseline_crossover("baseline");
        let authored = state.seal_general().unwrap();
        state.clear_general();
        state.load_canonical_text().unwrap();
        assert_eq!(state.general.seal().unwrap(), authored);
    }

    fn drain_until_artifact_terminal(state: &mut StrategyResearchState, worker: &DatabankWorker) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while state.artifact_request_active() {
            for event in worker.poll() {
                let _ = state.accept_databank_event(event);
            }
            assert!(
                std::time::Instant::now() < deadline,
                "databank worker timeout"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn canonical_strategy_save_clear_and_exact_worker_reload_restores_editor() {
        let worker = DatabankWorker::spawn_in_memory().expect("worker");
        let mut state = StrategyResearchState::new();
        state.open_guided_in_general().expect("guided graph");
        let expected = state.general.seal().expect("canonical strategy");

        state.submit_save_general(&worker).expect("submit save");
        assert!(state.artifact_request_active());
        assert!(state.submit_load_strategy(&worker).is_err());
        drain_until_artifact_terminal(&mut state, &worker);
        assert_eq!(state.strategy_load_id, expected.strategy_id());

        state.clear_general();
        state.canonical_text.clear();
        state.submit_load_strategy(&worker).expect("submit load");
        drain_until_artifact_terminal(&mut state, &worker);

        assert_eq!(state.general.seal().unwrap(), expected);
        assert_eq!(
            state.canonical_text,
            serde_json::to_string_pretty(&expected).unwrap()
        );
        assert!(state.status.starts_with("Reloaded exact verified"));
    }

    #[test]
    fn strategy_artifact_cancel_failure_and_stale_completion_are_explicit() {
        let worker = DatabankWorker::spawn_in_memory().expect("worker");
        let strategy = NnfxBuilderConfig::default().to_ir().unwrap();
        let mut state = StrategyResearchState::new();
        state.strategy_load_id = strategy.strategy_id().into();
        state.submit_load_strategy(&worker).expect("submit load");
        let cancelled_request = state.artifact_request.as_ref().unwrap().request_id();
        assert!(state.cancel_artifact_request(&worker));
        assert!(!state.artifact_request_active());
        assert!(state.status.contains("cancelled"));
        assert!(
            !state.accept_artifact_event(DatabankWorkerEvent::StrategyLoaded {
                request_id: cancelled_request,
                strategy: Box::new(strategy.clone()),
            })
        );

        state
            .submit_load_strategy(&worker)
            .expect("submit failure case");
        let failed_request = state.artifact_request.as_ref().unwrap().request_id();
        assert!(state.accept_artifact_event(DatabankWorkerEvent::Failed {
            request_id: failed_request,
            message: "precise failure".into(),
        }));
        assert!(!state.artifact_request_active());
        assert_eq!(state.status, "Error: precise failure");
    }

    #[test]
    fn save_clear_reload_rerun_preserves_verified_report_metrics_exactly() {
        use chrono::{Duration, TimeZone, Utc};
        use typhoon_engine::broker::alpaca::Bar;
        use typhoon_engine::core::strategy_dataset::{
            AdjustmentPolicy, CalendarPolicy, DatasetManifestInput, DatasetProvenance,
            DatasetQaPolicy,
        };
        use typhoon_engine::core::strategy_dataset_store::FileDatasetStore;
        use typhoon_engine::core::strategy_ir::{
            DatasetBinding, ExecutionSettings, FidelityLevel, RunBinding, StrategyExecutionConfig,
            StrategyRunManifest, SubBarDatasetBinding,
        };
        use typhoon_engine::core::strategy_metrics::METRICS_SCHEMA_VERSION;
        use typhoon_engine::core::strategy_report::StrategyReportArtifact;

        fn bar(timestamp: chrono::DateTime<Utc>, open: f64, close: f64) -> Bar {
            Bar {
                timestamp: timestamp.to_rfc3339(),
                open,
                high: open.max(close) + 2.0,
                low: open.min(close) - 2.0,
                close,
                volume: 10_000.0,
            }
        }

        fn wait_for(
            worker: &DatabankWorker,
            request_id: u64,
            accept: impl Fn(&DatabankWorkerEvent) -> bool,
        ) {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                for event in worker.poll() {
                    if event.request_id() == request_id {
                        if let DatabankWorkerEvent::Failed { message, .. } = &event {
                            panic!("databank request failed: {message}");
                        }
                        if accept(&event) {
                            return;
                        }
                    }
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "databank worker timeout"
                );
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }

        let databank = DatabankWorker::spawn_in_memory().expect("databank");
        let mut state = StrategyResearchState::new();
        state.general = GeneralStrategyBuilder::new("literal M3 gate", "native test");
        state
            .general
            .add_indicator(IndicatorDraft::new("baseline", IndicatorKind::Ema, 2));
        state.general.set_baseline_crossover("baseline");
        let authored = state.general.seal().expect("authored strategy");
        state.submit_save_general(&databank).expect("save submit");
        drain_until_artifact_terminal(&mut state, &databank);
        state.clear_general();
        state.canonical_text.clear();
        state.submit_load_strategy(&databank).expect("load submit");
        drain_until_artifact_terminal(&mut state, &databank);
        let reloaded = state.general.seal().expect("reloaded strategy");
        assert_eq!(reloaded, authored);

        let root = std::env::temp_dir().join(format!(
            "typhoon-m3-literal-gate-{}-{}",
            std::process::id(),
            authored.strategy_id()
        ));
        let store = FileDatasetStore::open(&root).expect("dataset store");
        let input = |timeframe: &str| DatasetManifestInput {
            symbol: "M3TEST".into(),
            timeframe: timeframe.into(),
            provenance: DatasetProvenance {
                source: "native-m3-test".into(),
                venue: "test".into(),
                pipeline: "verified-run/v1".into(),
            },
            adjustment: AdjustmentPolicy::Raw,
            calendar: CalendarPolicy::Continuous24x7,
            qa_policy: DatasetQaPolicy::default(),
        };
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let closes = [100.0, 104.0, 98.0, 109.0, 101.0, 112.0, 99.0, 115.0];
        let parent_bars: Vec<_> = closes
            .iter()
            .enumerate()
            .map(|(index, close)| {
                let open = if index == 0 {
                    *close
                } else {
                    closes[index - 1]
                };
                bar(start + Duration::days(index as i64), open, *close)
            })
            .collect();
        let mut finer_bars = Vec::new();
        for (day, close) in closes.iter().enumerate() {
            let day_open = if day == 0 { *close } else { closes[day - 1] };
            for quarter in 0..4 {
                let fraction = f64::from(quarter + 1) / 4.0;
                let quarter_close = day_open + (*close - day_open) * fraction;
                let quarter_open = if quarter == 0 {
                    day_open
                } else {
                    day_open + (*close - day_open) * (f64::from(quarter) / 4.0)
                };
                finer_bars.push(bar(
                    start + Duration::days(day as i64) + Duration::hours(i64::from(quarter) * 6),
                    quarter_open,
                    quarter_close,
                ));
            }
        }
        let parent = store
            .build_and_put(&input("1Day"), &parent_bars)
            .expect("parent dataset");
        let finer = store
            .build_and_put(&input("6Hour"), &finer_bars)
            .expect("finer dataset");

        let mut settings = ExecutionSettings::conservative_defaults();
        settings.fidelity = FidelityLevel::SubBar {
            sub_bar_seconds: 21_600,
        };
        settings.initial_capital = 10_000.0;
        let config = StrategyExecutionConfig::build(&settings).expect("config");
        let manifest = StrategyRunManifest::build(&RunBinding {
            datasets: vec![DatasetBinding {
                input_id: "primary".into(),
                dataset_id: parent.manifest.dataset_id.clone(),
            }],
            sub_bar_datasets: vec![SubBarDatasetBinding {
                parent_input_id: "primary".into(),
                dataset_id: finer.manifest.dataset_id.clone(),
            }],
            strategy_id: reloaded.strategy_id().into(),
            config_id: config.config_id().into(),
            seed: 135,
            engine_version: "typhoon-native/m3-gate".into(),
            metrics_version: METRICS_SCHEMA_VERSION.into(),
            intervention_log_id: None,
            repaint_qa: vec![],
        })
        .expect("manifest");
        let run_job = StrategyRunJob {
            identity: RunRequestIdentity {
                request_id: 1,
                generation: 1,
            },
            strategy: reloaded,
            config: config.clone(),
            manifest: manifest.clone(),
            chart: RunChartContext {
                chart_index: 0,
                bars_generation: 1,
                symbol: "M3TEST".into(),
                bar_times_ms: Arc::from(
                    parent_bars
                        .iter()
                        .map(|bar| {
                            chrono::DateTime::parse_from_rfc3339(&bar.timestamp)
                                .unwrap()
                                .timestamp_millis()
                        })
                        .collect::<Vec<_>>(),
                ),
            },
        };
        let first = execute_strategy_run_job(&root, &run_job).expect("first verified run");
        let second = execute_strategy_run_job(&root, &run_job).expect("verified rerun");
        let first_artifact =
            StrategyReportArtifact::from_json_slice(&first.view.report_artifact_json)
                .expect("first artifact");
        let second_artifact =
            StrategyReportArtifact::from_json_slice(&second.view.report_artifact_json)
                .expect("second artifact");
        assert_eq!(first_artifact.report_id(), second_artifact.report_id());
        assert_eq!(
            first_artifact.analysis().metrics,
            second_artifact.analysis().metrics
        );

        let append_request = 77;
        databank
            .submit(DatabankJob::AppendRun {
                request_id: append_request,
                run: DatabankRunInput {
                    run_id: manifest.run_id().into(),
                    strategy_id: authored.strategy_id().into(),
                    dataset_id: parent.manifest.dataset_id.clone(),
                    config_id: config.config_id().into(),
                    metrics_version: first_artifact.metrics_version().into(),
                    seed: 135,
                    created_sequence: 1,
                    metrics: first_artifact.analysis().metrics.clone(),
                    tags: vec!["native-m3-literal-gate".into()],
                    parent_run_id: None,
                    retest_of_run_id: None,
                },
            })
            .expect("append submit");
        wait_for(&databank, append_request, |event| {
            matches!(event, DatabankWorkerEvent::RunAppended { .. })
        });
        let verify_request = 78;
        databank
            .submit(DatabankJob::VerifyRerun {
                request_id: verify_request,
                run_id: manifest.run_id().into(),
                metrics: second_artifact.analysis().metrics.clone(),
            })
            .expect("verify submit");
        wait_for(&databank, verify_request, |event| {
            matches!(event, DatabankWorkerEvent::RerunVerified { .. })
        });
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn browser_accepts_only_latest_prepared_page_and_caps_comparison_selection() {
        let mut browser = DatabankBrowserState::default();
        let old = browser.begin_query();
        let current = browser.begin_query();
        let page = DatabankPage {
            rows: vec![DatabankRow {
                run_id: "run".into(),
                strategy_id: "strategy".into(),
                dataset_id: "dataset".into(),
                created_sequence: 1,
                net_profit: 2.0,
                max_drawdown_percent: 3.0,
                sharpe_ratio: 4.0,
            }],
            has_more: false,
        };
        assert!(!browser.accept_event(DatabankWorkerEvent::Page {
            request_id: old,
            page: page.clone()
        }));
        assert!(browser.accept_event(DatabankWorkerEvent::Page {
            request_id: current,
            page
        }));
        assert_eq!(browser.page.as_ref().unwrap().rows.len(), 1);
        for index in 0..MAX_COMPARE_RUNS {
            assert!(browser.toggle_compare(format!("run-{index}")));
        }
        assert!(!browser.toggle_compare("overflow".into()));
        assert_eq!(browser.compare_selection.len(), MAX_COMPARE_RUNS);
    }
}
