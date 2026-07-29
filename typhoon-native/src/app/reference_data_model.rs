//! Pure view model for the Dataset Inspector's reference-data panel (ADR-135
//! §6.7–§6.8, milestone M2).
//!
//! Plain data and arithmetic only — no egui, no filesystem, no store. Every
//! read, digest, verify and bind happens on
//! [`ReferenceDataWorker`](typhoon_engine::core::strategy_reference_data_worker::ReferenceDataWorker);
//! the window folds its events into this struct and draws from nothing else.
//!
//! The panel's job is to refuse to lie. A snapshot that cannot materialize is
//! shown with the exact refusal and its promote button stays disabled; a sealed
//! artifact from a keyless feed is labelled unverified-public no matter how
//! successfully it sealed. Nothing here substitutes a rule-derived calendar for
//! a source that was not there.

use typhoon_engine::core::strategy_ir::ExecutionSettings;
use typhoon_engine::core::strategy_reference_data_worker::{
    MAX_LISTED_ARTIFACTS, ReferenceArtifactSummary, ReferenceDataWorkerEvent,
    ReferenceSourceSummary, ReferenceSubmitError,
};

/// Artifact summaries the panel will hold, bounded independently of the store.
pub(crate) const REFERENCE_LIST_LIMIT: usize = 128;

/// Which artifact a picker row is offering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReferenceSelectionSlot {
    Calendar,
    CorporateActions,
}

/// The result of a completed selection, kept so the window can show what a run
/// would actually be prepared with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReferenceSelection {
    pub(crate) config_id: String,
    pub(crate) calendar_artifact_id: String,
    pub(crate) corporate_action_artifact_id: String,
    /// Whether *both* bound artifacts clear exchange/vendor authority.
    pub(crate) authoritative: bool,
}

/// Everything the reference-data panel draws from.
#[derive(Debug, Default)]
pub(crate) struct ReferenceDataState {
    /// Operator-entered path of a persisted raw source snapshot.
    pub(crate) snapshot_path: String,
    /// What the worker said about the last inspected snapshot.
    pub(crate) snapshot: Option<ReferenceSourceSummary>,
    /// Operator acknowledgement that this snapshot's source is below exchange/
    /// vendor authority and is being sealed anyway. Always starts false and is
    /// cleared by every new inspection, so it can never carry silently from one
    /// snapshot to the next.
    pub(crate) accept_non_authoritative: bool,
    /// Sealed artifacts in the store, capped at [`REFERENCE_LIST_LIMIT`].
    pub(crate) artifacts: Vec<ReferenceArtifactSummary>,
    /// Artifacts the store holds beyond what is listed. Shown, never hidden.
    pub(crate) artifacts_omitted: usize,
    pub(crate) selected_calendar: Option<String>,
    pub(crate) selected_corporate_actions: Option<String>,
    /// Instrument the artifacts will be bound to.
    pub(crate) symbol: String,
    pub(crate) currency: String,
    /// The settings a completed selection produced, ready for run preparation.
    pub(crate) prepared_settings: Option<Box<ExecutionSettings>>,
    pub(crate) selection: Option<ReferenceSelection>,
    /// The one in-flight request. A reply for any other id is stale.
    pub(crate) pending: Option<u64>,
    pub(crate) status: String,
    next_request_id: u64,
}

impl ReferenceDataState {
    pub(crate) fn new() -> Self {
        Self {
            // Stated rather than assumed: the panel shows the currency it will
            // bind, and the operator can change it before selecting.
            currency: "USD".to_string(),
            ..Self::default()
        }
    }

    /// Allocate the next request id and mark it in flight. Any earlier
    /// request's reply becomes stale from this point.
    pub(crate) fn begin_request(&mut self) -> u64 {
        self.next_request_id = self.next_request_id.wrapping_add(1);
        self.pending = Some(self.next_request_id);
        self.next_request_id
    }

    /// Record that the worker refused a submission, freeing the pending slot so
    /// the next frame can retry — a stranded slot would wedge the panel.
    pub(crate) fn note_submit_failure(&mut self, error: ReferenceSubmitError) {
        self.pending = None;
        self.status = match error {
            ReferenceSubmitError::QueueFull => {
                "Reference-data worker busy — try again in a moment.".to_string()
            }
            ReferenceSubmitError::WorkerStopped => {
                "Reference-data worker is not running; reopen the window.".to_string()
            }
        };
    }

    /// Whether the inspected snapshot may be promoted to a sealed artifact.
    ///
    /// False whenever the worker's dry run found any refusal at all, and false
    /// for a source below exchange/vendor authority until the operator has
    /// explicitly acknowledged that. Sealing is durable, so promoting a
    /// rule-derived or keyless source is a decision someone makes rather than a
    /// default the panel takes on their behalf.
    pub(crate) fn can_materialize(&self) -> bool {
        self.pending.is_none() && self.materialize_blocker().is_none()
    }

    /// Why promotion is unavailable, or `None` when it is available.
    ///
    /// Never a generic "not ready": the operator gets the worker's own refusal,
    /// because a blocked source is a fact about the data, not a UI state.
    pub(crate) fn materialize_blocker(&self) -> Option<String> {
        let Some(summary) = self.snapshot.as_ref() else {
            return Some("Inspect a snapshot first.".to_string());
        };
        if let Some(reason) = summary.blocked.as_ref() {
            return Some(format!("Cannot materialize: {reason}"));
        }
        if !summary.authoritative && !self.accept_non_authoritative {
            return Some(format!(
                "This source is `{}`, not exchange-official or contracted-vendor. \
                 Acknowledge that below to seal it anyway.",
                summary.authority
            ));
        }
        None
    }

    /// Whether the inspected snapshot needs the non-authoritative
    /// acknowledgement before it can be sealed.
    pub(crate) fn needs_authority_acknowledgement(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|summary| !summary.authoritative && summary.blocked.is_none())
    }

    /// Whether both slots are filled and a selection may be submitted.
    pub(crate) fn can_select(&self) -> bool {
        self.pending.is_none()
            && self.selected_calendar.is_some()
            && self.selected_corporate_actions.is_some()
            && !self.symbol.trim().is_empty()
            && !self.currency.trim().is_empty()
    }

    /// Why selection is unavailable, or `None` when it is available.
    pub(crate) fn select_blocker(&self) -> Option<&'static str> {
        if self.selected_calendar.is_none() {
            Some("Choose a calendar-exception artifact.")
        } else if self.selected_corporate_actions.is_none() {
            Some("Choose a corporate-action artifact.")
        } else if self.symbol.trim().is_empty() {
            Some("Enter the instrument symbol to bind.")
        } else if self.currency.trim().is_empty() {
            Some("Enter the instrument currency to bind.")
        } else {
            None
        }
    }

    /// Pick an artifact into its slot. Selecting a different artifact discards
    /// any prepared settings: they were sealed against the previous choice and
    /// showing them beside a new one would misstate what a run would use.
    pub(crate) fn select(&mut self, slot: ReferenceSelectionSlot, artifact_id: &str) {
        let slot = match slot {
            ReferenceSelectionSlot::Calendar => &mut self.selected_calendar,
            ReferenceSelectionSlot::CorporateActions => &mut self.selected_corporate_actions,
        };
        let chosen = Some(artifact_id.to_string());
        if *slot == chosen {
            return;
        }
        *slot = chosen;
        self.prepared_settings = None;
        self.selection = None;
    }

    /// Fold one worker event in. Replies for superseded requests are dropped.
    pub(crate) fn apply_event(&mut self, event: ReferenceDataWorkerEvent) {
        match event {
            // Advisory only: the request is still in flight.
            ReferenceDataWorkerEvent::Started { .. } => {}
            ReferenceDataWorkerEvent::SnapshotInspected {
                request_id,
                summary,
            } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                self.status = match &summary.blocked {
                    Some(reason) => format!(
                        "{} · {} — cannot be promoted: {reason}",
                        summary.kind, summary.scope
                    ),
                    None => format!(
                        "{} · {} — {} record(s), {} authority, ready to materialize.",
                        summary.kind, summary.scope, summary.record_count, summary.authority
                    ),
                };
                // A new inspection is a new source: an acknowledgement given for
                // the previous one says nothing about this one.
                self.accept_non_authoritative = false;
                self.snapshot = Some(*summary);
            }
            ReferenceDataWorkerEvent::Materialized {
                request_id,
                summary,
            } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                self.status = format!(
                    "Sealed {} · {} as {} ({} authority).",
                    summary.kind,
                    summary.scope,
                    short_id(&summary.artifact_id),
                    summary.authority
                );
            }
            ReferenceDataWorkerEvent::ArtifactsListed {
                request_id,
                mut summaries,
                omitted,
            } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                let dropped = summaries.len().saturating_sub(REFERENCE_LIST_LIMIT);
                summaries.truncate(REFERENCE_LIST_LIMIT);
                self.artifacts_omitted = omitted.saturating_add(dropped);
                self.status = if self.artifacts_omitted == 0 {
                    format!("{} verified artifact(s) in the store.", summaries.len())
                } else {
                    format!(
                        "{} verified artifact(s) shown; {} more not listed.",
                        summaries.len(),
                        self.artifacts_omitted
                    )
                };
                // A selection whose artifact is no longer listed is dropped
                // rather than left pointing at something the panel cannot show.
                self.retain_selection(&summaries);
                self.artifacts = summaries;
            }
            ReferenceDataWorkerEvent::Selected {
                request_id,
                config_id,
                settings,
                calendar_artifact_id,
                corporate_action_artifact_id,
                authoritative,
            } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                self.status = format!(
                    "Prepared execution config {} from verified artifacts{}.",
                    short_id(&config_id),
                    if authoritative {
                        ""
                    } else {
                        " — NOT exchange/vendor authoritative"
                    }
                );
                self.prepared_settings = Some(settings);
                self.selection = Some(ReferenceSelection {
                    config_id,
                    calendar_artifact_id,
                    corporate_action_artifact_id,
                    authoritative,
                });
            }
            ReferenceDataWorkerEvent::Failed {
                request_id,
                message,
            } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                self.status = format!("Failed: {message}");
            }
        }
    }

    fn retain_selection(&mut self, summaries: &[ReferenceArtifactSummary]) {
        let known = |id: &Option<String>| {
            id.as_ref()
                .is_some_and(|id| summaries.iter().any(|summary| &summary.artifact_id == id))
        };
        if !known(&self.selected_calendar) {
            self.selected_calendar = None;
        }
        if !known(&self.selected_corporate_actions) {
            self.selected_corporate_actions = None;
        }
        if self.selected_calendar.is_none() || self.selected_corporate_actions.is_none() {
            self.prepared_settings = None;
            self.selection = None;
        }
    }

    fn is_current(&self, request_id: u64) -> bool {
        self.pending == Some(request_id)
    }

    /// How many artifacts to ask the worker for — bounded on both ends.
    pub(crate) fn list_limit() -> usize {
        REFERENCE_LIST_LIMIT.min(MAX_LISTED_ARTIFACTS)
    }
}

/// First 12 characters of a digest, for a label that stays one line.
pub(crate) fn short_id(id: &str) -> &str {
    &id[..id.len().min(12)]
}

#[cfg(test)]
mod tests;
