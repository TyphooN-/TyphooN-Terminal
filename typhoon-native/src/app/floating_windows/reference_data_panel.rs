//! Reference Data — the calendar/corporate-action half of the Dataset
//! Inspector (ADR-135 §6.7–§6.8, milestone M2).
//!
//! A pure view over
//! [`ReferenceDataState`](crate::app::reference_data_model::ReferenceDataState).
//! Every button becomes a `ReferenceDataJob` on a bounded worker queue; this
//! frame callback never opens a snapshot, hashes a record, verifies an
//! artifact, or builds a config (ADR-098, ADR-134). A full queue is reported to
//! the user, not waited on.
//!
//! ## What the panel refuses to do
//!
//! - It never promotes a snapshot the worker's dry run said would fail. The
//!   button is disabled and the refusal itself is shown.
//! - It never presents a sealed artifact as authoritative because it sealed.
//!   Authority is the source's class, and the row says which class that is.
//! - It never fills a slot for the operator. No default calendar, no assumed
//!   symbol, no "latest" artifact.

use super::*;
use crate::app::reference_data_model::{ReferenceDataState, ReferenceSelectionSlot, short_id};
use typhoon_engine::core::strategy_reference_data_worker::{
    ReferenceArtifactSummary, ReferenceDataJob, ReferenceDataWorker,
};

/// Colour for an authority label: authoritative reads normal, everything below
/// it reads as a warning, because that is what it is.
fn authority_color(authoritative: bool) -> egui::Color32 {
    if authoritative {
        egui::Color32::from_rgb(120, 200, 140)
    } else {
        egui::Color32::from_rgb(226, 176, 76)
    }
}

impl TyphooNApp {
    /// Submit `job`, recording backpressure instead of blocking on it.
    fn submit_reference_job(&mut self, job: ReferenceDataJob) {
        let Some(worker) = self.reference_data_worker.as_ref() else {
            self.reference_data.status =
                "Reference-data worker is not running; reopen the window.".to_string();
            self.reference_data.pending = None;
            return;
        };
        if let Err(error) = worker.submit(job) {
            self.reference_data.note_submit_failure(error);
        }
    }

    /// Pick a persisted snapshot with the platform dialog. The dialog returns a
    /// path and nothing else — the file itself is opened, bounded and decoded on
    /// the worker, exactly as a typed path would be.
    fn browse_for_reference_snapshot(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Open a persisted reference-data source snapshot")
            .add_filter("Source snapshot JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        self.reference_data.snapshot_path = path.display().to_string();
        // A new file is a new subject: the previous inspection described a
        // different snapshot and must not stay on screen beside this path.
        self.reference_data.snapshot = None;
        self.request_reference_inspect();
    }

    fn request_reference_inspect(&mut self) {
        let path = self.reference_data.snapshot_path.trim().to_string();
        if path.is_empty() {
            self.reference_data.status =
                "Enter the path of a persisted source snapshot.".to_string();
            return;
        }
        let request_id = self.reference_data.begin_request();
        self.submit_reference_job(ReferenceDataJob::InspectSnapshot {
            request_id,
            path: path.into(),
        });
    }

    fn request_reference_materialize(&mut self) {
        let path = self.reference_data.snapshot_path.trim().to_string();
        if path.is_empty() {
            self.reference_data.status =
                "Enter the path of a persisted source snapshot.".to_string();
            return;
        }
        let request_id = self.reference_data.begin_request();
        self.submit_reference_job(ReferenceDataJob::MaterializeSnapshot {
            request_id,
            path: path.into(),
        });
    }

    fn request_reference_list(&mut self) {
        let request_id = self.reference_data.begin_request();
        self.submit_reference_job(ReferenceDataJob::ListArtifacts {
            request_id,
            limit: ReferenceDataState::list_limit(),
        });
    }

    /// Bind the two chosen artifacts into execution settings and seal a config.
    ///
    /// The worker re-verifies both artifacts against their ids before binding,
    /// so a hand-edited file cannot be selected into a run.
    fn request_reference_selection(&mut self) {
        let Some(calendar_artifact_id) = self.reference_data.selected_calendar.clone() else {
            return;
        };
        let Some(corporate_action_artifact_id) =
            self.reference_data.selected_corporate_actions.clone()
        else {
            return;
        };
        let symbol = self.reference_data.symbol.trim().to_string();
        let currency = self.reference_data.currency.trim().to_string();
        // Start from whatever the last preparation produced, so binding a
        // second instrument extends the config rather than resetting it.
        let settings = self
            .reference_data
            .prepared_settings
            .clone()
            .unwrap_or_else(|| {
                Box::new(
                    typhoon_engine::core::strategy_ir::ExecutionSettings::conservative_defaults(),
                )
            });
        let request_id = self.reference_data.begin_request();
        self.submit_reference_job(ReferenceDataJob::SelectIntoConfig {
            request_id,
            settings,
            symbol,
            currency,
            calendar_artifact_id,
            corporate_action_artifact_id,
        });
    }

    /// Drain the worker's bounded event batch. Safe every frame: a non-blocking
    /// `try_recv` loop with a hard per-call ceiling.
    pub(super) fn pump_reference_data_worker(&mut self) {
        let Some(worker) = self.reference_data_worker.as_ref() else {
            return;
        };
        for event in worker.poll() {
            self.reference_data.apply_event(event);
        }
    }

    /// Spawn the reference-data worker over the on-disk store, once. Store
    /// creation and every filesystem operation happen on that worker.
    pub(super) fn start_reference_data_worker(&mut self) {
        let root = crate::app::platform::strategy_reference_data_dir();
        match ReferenceDataWorker::spawn_at(root.clone()) {
            Ok(worker) => {
                self.reference_data_worker = Some(worker);
                self.reference_data.status = format!("Reference-data store: {}", root.display());
                self.request_reference_list();
            }
            Err(error) => {
                self.reference_data.status = format!(
                    "Cannot open reference-data store at {}: {error}",
                    root.display()
                );
            }
        }
    }

    /// Draw the panel. The action the user asked for is collected during the
    /// closure and applied after it ends, so no submit happens mid-borrow.
    pub(super) fn draw_reference_data_panel(&mut self, ui: &mut egui::Ui) {
        let mut action = ReferencePanelAction::None;
        let busy = self.reference_data.pending.is_some();

        ui.collapsing("Reference data — calendars & corporate actions", |ui| {
            ui.label(
                egui::RichText::new(
                    "Materialize persisted raw source snapshots into verified, content-addressed \
                     artifacts, then select them into an execution config. Rule-derived calendars \
                     and the keyless Yahoo feed are not authoritative and are refused when a \
                     snapshot requires authority.",
                )
                .weak(),
            );
            ui.add_space(4.0);

            // ── Snapshot ────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.label("Snapshot path:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.reference_data.snapshot_path)
                        .desired_width(420.0)
                        .hint_text("/path/to/persisted-source-snapshot.json"),
                );
                if ui
                    .add_enabled(!busy, egui::Button::new("Browse…"))
                    .on_hover_text("Pick a persisted source snapshot and inspect it.")
                    .clicked()
                {
                    action = ReferencePanelAction::Browse;
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Inspect"))
                    .on_hover_text(
                        "Read and describe the snapshot off the render thread. Read-only: \
                         nothing is promoted.",
                    )
                    .clicked()
                {
                    action = ReferencePanelAction::Inspect;
                }
            });

            if let Some(snapshot) = self.reference_data.snapshot.as_ref() {
                ui.add_space(2.0);
                egui::Grid::new("reference_snapshot_summary")
                    .num_columns(4)
                    .spacing([18.0, 3.0])
                    .show(ui, |ui| {
                        ui.label("Kind");
                        ui.label(egui::RichText::new(snapshot.kind).strong());
                        ui.label("Scope");
                        ui.label(egui::RichText::new(&snapshot.scope).strong());
                        ui.end_row();

                        ui.label("Source");
                        ui.label(&snapshot.source_system);
                        ui.label("Authority");
                        ui.label(
                            egui::RichText::new(snapshot.authority)
                                .color(authority_color(snapshot.authoritative)),
                        );
                        ui.end_row();

                        ui.label("Covered");
                        ui.label(&snapshot.covered_range);
                        ui.label("Requested");
                        ui.label(&snapshot.requested_range);
                        ui.end_row();

                        ui.label("Complete");
                        ui.label(if snapshot.complete { "yes" } else { "no" });
                        ui.label("Records");
                        ui.label(snapshot.record_count.to_string());
                        ui.end_row();

                        ui.label("As-of (ns)");
                        ui.label(snapshot.as_of_ns.to_string());
                        ui.label("Retrieved (ns)");
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · audit only, not hashed",
                                snapshot.retrieved_at_ns
                            ))
                            .weak(),
                        );
                        ui.end_row();

                        ui.label("Requires authority");
                        ui.label(if snapshot.require_authoritative {
                            "yes"
                        } else {
                            "no"
                        });
                        ui.label("");
                        ui.label("");
                        ui.end_row();
                    });
            }

            // Sealing is durable, so promoting a rule-derived or keyless source
            // is a decision the operator states, not one the panel makes for
            // them. The box only appears when it is the thing in the way.
            if self.reference_data.needs_authority_acknowledgement() {
                ui.checkbox(
                    &mut self.reference_data.accept_non_authoritative,
                    egui::RichText::new(
                        "Seal this source anyway — I accept that it is not exchange-official or \
                         contracted-vendor data",
                    )
                    .color(authority_color(false)),
                )
                .on_hover_text(
                    "The artifact records the source's real authority either way. Nothing here \
                     upgrades it.",
                );
            }

            ui.horizontal(|ui| {
                let can_materialize = self.reference_data.can_materialize();
                let button = ui.add_enabled(
                    can_materialize && !busy,
                    egui::Button::new("Materialize into store"),
                );
                // The disabled reason is the worker's own refusal, so the
                // operator learns what is wrong with the data, not the button.
                let button = match self.reference_data.materialize_blocker() {
                    Some(reason) => button.on_disabled_hover_text(reason),
                    None => button.on_hover_text(
                        "Seal a verified, content-addressed artifact from this snapshot.",
                    ),
                };
                if button.clicked() {
                    action = ReferencePanelAction::Materialize;
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Refresh artifacts"))
                    .clicked()
                {
                    action = ReferencePanelAction::List;
                }
                if busy {
                    ui.spinner();
                    ui.label(egui::RichText::new("working…").weak());
                }
            });

            if !self.reference_data.status.is_empty() {
                ui.label(egui::RichText::new(&self.reference_data.status).weak());
            }
            ui.separator();

            // ── Artifacts ───────────────────────────────────────────
            ui.label(egui::RichText::new("Verified artifacts").strong());
            if self.reference_data.artifacts_omitted > 0 {
                ui.label(
                    egui::RichText::new(format!(
                        "{} further artifact(s) in the store are not listed.",
                        self.reference_data.artifacts_omitted
                    ))
                    .weak(),
                );
            }
            egui::ScrollArea::vertical()
                .id_salt("reference_data_artifacts")
                .max_height(150.0)
                .show(ui, |ui| {
                    if self.reference_data.artifacts.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "None yet — inspect and materialize a source snapshot.",
                            )
                            .weak(),
                        );
                    }
                    for artifact in &self.reference_data.artifacts {
                        let slot = if artifact.symbol.is_some() {
                            ReferenceSelectionSlot::CorporateActions
                        } else {
                            ReferenceSelectionSlot::Calendar
                        };
                        let selected = match slot {
                            ReferenceSelectionSlot::Calendar => {
                                self.reference_data.selected_calendar.as_deref()
                            }
                            ReferenceSelectionSlot::CorporateActions => {
                                self.reference_data.selected_corporate_actions.as_deref()
                            }
                        } == Some(artifact.artifact_id.as_str());
                        if ui
                            .selectable_label(selected, artifact_label(artifact))
                            .on_hover_text(artifact_detail(artifact))
                            .clicked()
                        {
                            action = ReferencePanelAction::Select {
                                slot,
                                artifact_id: artifact.artifact_id.clone(),
                            };
                        }
                    }
                });
            ui.separator();

            // ── Selection into a config ─────────────────────────────
            ui.horizontal(|ui| {
                ui.label("Bind to symbol:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.reference_data.symbol)
                        .desired_width(90.0)
                        .hint_text("AAPL"),
                );
                ui.label("currency:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.reference_data.currency)
                        .desired_width(60.0),
                );
                let can_select = self.reference_data.can_select();
                let button = ui.add_enabled(
                    can_select && !busy,
                    egui::Button::new("Select into execution config"),
                );
                let button = match self.reference_data.select_blocker() {
                    Some(reason) => button.on_disabled_hover_text(reason),
                    None => button.on_hover_text(
                        "Re-verify both artifacts, bind them into execution settings, and seal \
                         the resulting config id for run preparation.",
                    ),
                };
                if button.clicked() {
                    action = ReferencePanelAction::SelectIntoConfig;
                }
            });

            if let Some(selection) = self.reference_data.selection.as_ref() {
                ui.add_space(2.0);
                egui::Grid::new("reference_selection_summary")
                    .num_columns(2)
                    .spacing([18.0, 3.0])
                    .show(ui, |ui| {
                        ui.label("Prepared config");
                        ui.label(
                            egui::RichText::new(&selection.config_id)
                                .monospace()
                                .strong(),
                        );
                        ui.end_row();
                        ui.label("Calendar artifact");
                        ui.label(egui::RichText::new(&selection.calendar_artifact_id).monospace());
                        ui.end_row();
                        // One artifact per symbol, so the whole bound set is
                        // listed. Showing only the last selection would read as
                        // "this run carries one symbol's actions" when a
                        // multi-symbol config carries several.
                        ui.label(format!(
                            "Corporate actions ({})",
                            selection.bound_corporate_action_artifact_ids.len()
                        ));
                        ui.vertical(|ui| {
                            for id in &selection.bound_corporate_action_artifact_ids {
                                let added = *id == selection.corporate_action_artifact_id;
                                let text = egui::RichText::new(id).monospace();
                                ui.label(if added { text.strong() } else { text });
                            }
                        });
                        ui.end_row();
                        ui.label("Authority");
                        ui.label(
                            egui::RichText::new(if selection.authoritative {
                                "exchange/vendor authoritative"
                            } else {
                                "NOT authoritative — at least one source is rule-derived or \
                                 unverified-public"
                            })
                            .color(authority_color(selection.authoritative)),
                        );
                        ui.end_row();
                    });
            }
        });

        match action {
            ReferencePanelAction::None => {}
            ReferencePanelAction::Browse => self.browse_for_reference_snapshot(),
            ReferencePanelAction::Inspect => self.request_reference_inspect(),
            ReferencePanelAction::Materialize => self.request_reference_materialize(),
            ReferencePanelAction::List => self.request_reference_list(),
            ReferencePanelAction::Select { slot, artifact_id } => {
                self.reference_data.select(slot, &artifact_id);
            }
            ReferencePanelAction::SelectIntoConfig => self.request_reference_selection(),
        }
    }
}

/// What the user clicked. Collected during the closure and applied after, so no
/// worker submit happens while `self.reference_data` is borrowed for drawing.
enum ReferencePanelAction {
    None,
    Browse,
    Inspect,
    Materialize,
    List,
    Select {
        slot: ReferenceSelectionSlot,
        artifact_id: String,
    },
    SelectIntoConfig,
}

fn artifact_label(artifact: &ReferenceArtifactSummary) -> String {
    format!(
        "{}  ·  {}  ·  {} event(s)  ·  {}  ·  {}",
        artifact.kind,
        artifact.scope,
        artifact.event_count,
        artifact.authority,
        short_id(&artifact.artifact_id)
    )
}

fn artifact_detail(artifact: &ReferenceArtifactSummary) -> String {
    let mut detail = format!(
        "{}\nsource: {}\ncovered: {}\nas-of (ns): {}\nsource records: {}",
        artifact.artifact_id,
        artifact.source_system,
        artifact.covered_range,
        artifact.as_of_ns,
        artifact.record_count,
    );
    if let Some(adjustment) = artifact.adjustment {
        detail.push_str(&format!("\ndataset adjustment: {adjustment}"));
    }
    if !artifact.authoritative {
        detail.push_str(
            "\n\nNOT exchange/vendor authoritative — this source carries no correction feed.",
        );
    }
    detail
}
