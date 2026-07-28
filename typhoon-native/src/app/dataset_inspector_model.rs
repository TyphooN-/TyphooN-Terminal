//! Pure paging/projection model for the Dataset Inspector window (ADR-135
//! §11.2).
//!
//! Everything here is plain data and arithmetic — no egui, no filesystem, no
//! database. The window renders from [`DatasetInspectorState`] and nothing
//! else, which is what keeps the render path O(page) and free of the
//! store/database walks ADR-098 and ADR-134 forbid on the frame thread.
//!
//! The state holds exactly three bounded collections: at most
//! [`DATASET_LIST_LIMIT`] record summaries, at most [`MAX_PAGE_BARS`] rows, and
//! one optional QA summary. The bars themselves live on disk; the inspector
//! never has more than one window of them in memory.

use typhoon_engine::broker::alpaca::Bar as EngineBar;
use typhoon_engine::core::strategy_dataset::{
    DatasetManifestInput, DatasetQaIssue, DatasetQaSeverity,
};
use typhoon_engine::core::strategy_dataset_store::{
    DatasetPage, DatasetRecordSummary, MAX_LISTED_RECORDS, MAX_PAGE_BARS,
};
use typhoon_engine::core::strategy_dataset_worker::{
    DatasetBarChunks, DatasetQaSummary, DatasetSubmitError, DatasetWorkerEvent,
};

/// Page sizes offered in the UI. Every entry must be a legal engine page
/// request, which the tests assert rather than trust.
pub(crate) const DATASET_INSPECTOR_PAGE_SIZES: [usize; 4] = [50, 100, 250, 500];

/// Rows per page before the user changes it.
pub(crate) const DEFAULT_DATASET_PAGE_SIZE: usize = 100;

/// Record summaries the window will hold. Bounded independently of what the
/// store returns, so a large store cannot grow the window's memory.
pub(crate) const DATASET_LIST_LIMIT: usize = 256;

/// Maximum chart bars converted during one egui frame. Snapshot production is
/// incremental; hashing, QA, and persistence begin only after the bounded
/// chunks have produced one stable worker-bound snapshot.
pub(crate) const MATERIALIZE_BARS_PER_FRAME: usize = 256;

/// O(1) identity captured when chart materialization starts. Edge timestamps
/// catch same-length reloads without scanning the chart on every pump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MaterializationIdentity {
    pub(crate) chart_index: usize,
    pub(crate) symbol: String,
    pub(crate) timeframe: String,
    pub(crate) source: String,
    pub(crate) bars_generation: u64,
    pub(crate) len: usize,
    pub(crate) first_ts_ms: Option<i64>,
    pub(crate) last_ts_ms: Option<i64>,
}

#[derive(Debug)]
pub(crate) struct MaterializationDraft {
    identity: MaterializationIdentity,
    input: Option<DatasetManifestInput>,
    cursor: usize,
    bars: DatasetBarChunks,
}

#[derive(Debug)]
pub(crate) enum MaterializationPump {
    Pending,
    Changed,
    Complete {
        input: DatasetManifestInput,
        bars: DatasetBarChunks,
    },
}

impl MaterializationDraft {
    pub(crate) fn start(identity: MaterializationIdentity, input: DatasetManifestInput) -> Self {
        Self {
            identity,
            input: Some(input),
            cursor: 0,
            // Do not reserve the chart length here: clicking the button must
            // remain O(1) in both work and allocation.
            bars: DatasetBarChunks::default(),
        }
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    #[cfg(test)]
    pub(crate) fn produced_len(&self) -> usize {
        self.bars.len()
    }

    #[cfg(test)]
    pub(crate) fn max_chunk_len(&self) -> usize {
        self.bars.max_chunk_len()
    }

    #[cfg(test)]
    pub(crate) fn allocation_high_water(&self) -> usize {
        self.bars.allocation_high_water()
    }

    pub(crate) fn total_len(&self) -> usize {
        self.identity.len
    }

    pub(crate) fn pump(
        &mut self,
        current: &MaterializationIdentity,
        mut bar_at: impl FnMut(usize) -> EngineBar,
    ) -> MaterializationPump {
        if current != &self.identity {
            return MaterializationPump::Changed;
        }
        let end = self
            .cursor
            .saturating_add(MATERIALIZE_BARS_PER_FRAME)
            .min(self.identity.len);
        let mut chunk = Vec::with_capacity(end - self.cursor);
        while self.cursor < end {
            chunk.push(bar_at(self.cursor));
            self.cursor += 1;
        }
        if !chunk.is_empty() {
            self.bars.push_chunk(chunk);
        }
        if self.cursor < self.identity.len {
            MaterializationPump::Pending
        } else {
            MaterializationPump::Complete {
                input: self.input.take().expect("completed draft owns its input"),
                bars: std::mem::take(&mut self.bars),
            }
        }
    }
}

/// Clamp a requested page size into the engine's accepted range.
pub(crate) fn clamp_page_size(requested: usize) -> usize {
    requested.clamp(1, MAX_PAGE_BARS)
}

/// Offset of the page before `offset`, or `None` at the start.
pub(crate) fn previous_page_offset(offset: u64, page_size: usize) -> Option<u64> {
    if offset == 0 {
        return None;
    }
    Some(offset.saturating_sub(page_size.max(1) as u64))
}

/// Offset of the page after `offset`, or `None` when it would be empty.
pub(crate) fn next_page_offset(offset: u64, page_size: usize, total_bars: u64) -> Option<u64> {
    let next = offset.checked_add(page_size.max(1) as u64)?;
    (next < total_bars).then_some(next)
}

/// Offset of the last non-empty page.
pub(crate) fn last_page_offset(page_size: usize, total_bars: u64) -> u64 {
    let page_size = page_size.max(1) as u64;
    if total_bars == 0 {
        return 0;
    }
    ((total_bars - 1) / page_size) * page_size
}

/// A short label naming what a finding is about, for the row's flag cell.
pub(crate) fn issue_flag_label(issue: &DatasetQaIssue) -> &'static str {
    match issue {
        DatasetQaIssue::EmptyDataset => "empty",
        DatasetQaIssue::UnparsableTimestamp { .. } => "bad timestamp",
        DatasetQaIssue::DuplicateTimestamp { .. } => "duplicate",
        DatasetQaIssue::TimestampOutOfOrder { .. } => "out of order",
        DatasetQaIssue::NonFiniteValue { .. } => "non-finite",
        DatasetQaIssue::NonPositivePrice { .. } => "price ≤ 0",
        DatasetQaIssue::NegativeVolume { .. } => "negative volume",
        DatasetQaIssue::OhlcViolation { .. } => "OHLC",
        DatasetQaIssue::UnexpectedWeekendBar { .. } => "weekend",
        DatasetQaIssue::UnexpectedHolidayBar { .. } => "holiday",
        DatasetQaIssue::UnexpectedSessionBar { .. } => "out of session",
        DatasetQaIssue::PriceSpike { .. } => "spike",
        DatasetQaIssue::SuspiciousLevelShift { .. } => "level shift",
        DatasetQaIssue::CarryForwardBar { .. } => "carry bar",
        DatasetQaIssue::MissingBars { .. } => "gap",
    }
}

/// One table row: an absolute bar index, the bar, and whatever QA said about
/// it. Flags are pre-joined so the render pass does no string building.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DatasetInspectorRow {
    pub(crate) index: u64,
    pub(crate) timestamp: String,
    pub(crate) open: f64,
    pub(crate) high: f64,
    pub(crate) low: f64,
    pub(crate) close: f64,
    pub(crate) volume: f64,
    pub(crate) severity: Option<DatasetQaSeverity>,
    pub(crate) flags: String,
}

/// Project a page into rows, attaching each finding to its own bar.
///
/// Findings whose index falls outside the window are dropped rather than
/// wrapped onto a row that did not produce them — a mislabelled row is worse
/// than a missing label.
pub(crate) fn build_rows(page: &DatasetPage) -> Vec<DatasetInspectorRow> {
    let end = page.offset.saturating_add(page.bars.len() as u64);
    let mut rows: Vec<DatasetInspectorRow> = page
        .bars
        .iter()
        .enumerate()
        .map(|(slot, bar)| DatasetInspectorRow {
            index: page.offset + slot as u64,
            timestamp: bar.timestamp.clone(),
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            severity: None,
            flags: String::new(),
        })
        .collect();

    for finding in &page.findings {
        let Some(bar_index) = finding.bar_index else {
            continue;
        };
        let bar_index = bar_index as u64;
        if bar_index < page.offset || bar_index >= end {
            continue;
        }
        let row = &mut rows[(bar_index - page.offset) as usize];
        row.severity = Some(match row.severity {
            Some(existing) => existing.max(finding.severity),
            None => finding.severity,
        });
        let label = issue_flag_label(&finding.issue);
        if !row.flags.split(", ").any(|existing| existing == label) {
            if !row.flags.is_empty() {
                row.flags.push_str(", ");
            }
            row.flags.push_str(label);
        }
    }
    rows
}

/// Everything the Dataset Inspector window draws from.
#[derive(Debug, Default)]
pub(crate) struct DatasetInspectorState {
    /// Stored datasets, capped at [`DATASET_LIST_LIMIT`].
    pub(crate) records: Vec<DatasetRecordSummary>,
    /// Dataset id the user is looking at.
    pub(crate) selected: Option<String>,
    /// Manifest/provenance header for `selected`.
    pub(crate) summary: Option<DatasetRecordSummary>,
    /// Report-level QA context for `selected`.
    pub(crate) qa: Option<DatasetQaSummary>,
    /// The current window, capped at [`MAX_PAGE_BARS`].
    pub(crate) rows: Vec<DatasetInspectorRow>,
    pub(crate) page_offset: u64,
    pub(crate) page_size: usize,
    pub(crate) total_bars: u64,
    /// The one in-flight request. A reply for any other id is stale.
    pub(crate) pending: Option<u64>,
    pub(crate) status: String,
    /// Incrementally produced chart snapshot. At most one bounded chunk is
    /// allocated per frame; the chunks move into one worker job without a
    /// render-thread flatten/copy.
    pub(crate) materialization: Option<MaterializationDraft>,
    next_request_id: u64,
}

impl DatasetInspectorState {
    pub(crate) fn new() -> Self {
        Self {
            page_size: DEFAULT_DATASET_PAGE_SIZE,
            ..Self::default()
        }
    }

    /// The page size to use, whatever the stored value is.
    pub(crate) fn effective_page_size(&self) -> usize {
        clamp_page_size(if self.page_size == 0 {
            DEFAULT_DATASET_PAGE_SIZE
        } else {
            self.page_size
        })
    }

    /// Allocate the next request id and mark it in flight. Any earlier
    /// request's reply becomes stale from this point.
    pub(crate) fn begin_request(&mut self) -> u64 {
        self.next_request_id = self.next_request_id.wrapping_add(1);
        let request_id = self.next_request_id;
        self.pending = Some(request_id);
        request_id
    }

    /// Record that the worker refused a submission. The pending slot is freed
    /// so the next frame can retry — a stranded slot would wedge the window.
    pub(crate) fn note_submit_failure(&mut self, error: DatasetSubmitError) {
        self.pending = None;
        self.status = match error {
            DatasetSubmitError::QueueFull => {
                "Dataset worker busy — try again in a moment.".to_string()
            }
            DatasetSubmitError::WorkerStopped => {
                "Dataset worker is not running; reopen the window.".to_string()
            }
        };
    }

    /// Fold one worker event into the window. Replies for superseded requests
    /// are dropped.
    pub(crate) fn apply_event(&mut self, event: DatasetWorkerEvent) {
        match event {
            // Advisory only: the request is still in flight.
            DatasetWorkerEvent::Started { .. } => {}
            DatasetWorkerEvent::Listed {
                request_id,
                mut records,
            } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                records.truncate(DATASET_LIST_LIMIT);
                self.status = format!("{} dataset(s) stored.", records.len());
                self.records = records;
            }
            DatasetWorkerEvent::Built {
                request_id,
                summary,
                outcome,
            } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                self.status = format!(
                    "{} · {} — {:?} ({} bars, {} error(s), {} warning(s)).",
                    summary.symbol,
                    summary.timeframe,
                    outcome,
                    summary.bar_count,
                    summary.qa_error_count,
                    summary.qa_warning_count
                );
            }
            DatasetWorkerEvent::Page {
                request_id,
                summary,
                page,
                qa_summary,
            } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                self.page_offset = page.offset;
                self.total_bars = page.total_bars;
                self.rows = build_rows(&page);
                self.selected = Some(summary.dataset_id.clone());
                self.summary = Some(summary);
                self.qa = Some(qa_summary);
                self.status = if self.rows.is_empty() {
                    format!(
                        "No bars in this window (dataset holds {}).",
                        self.total_bars
                    )
                } else {
                    format!(
                        "Bars {}–{} of {}.",
                        self.page_offset.saturating_add(1),
                        self.page_offset.saturating_add(self.rows.len() as u64),
                        self.total_bars
                    )
                };
            }
            DatasetWorkerEvent::Failed {
                request_id,
                message,
            } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                self.status = format!("Failed: {message}");
            }
            DatasetWorkerEvent::Cancelled { request_id } => {
                if !self.is_current(request_id) {
                    return;
                }
                self.pending = None;
                self.status = "Request cancelled.".to_string();
            }
        }
    }

    fn is_current(&self, request_id: u64) -> bool {
        self.pending == Some(request_id)
    }

    /// How many records to ask the store for — bounded on both ends.
    pub(crate) fn list_limit() -> usize {
        DATASET_LIST_LIMIT.min(MAX_LISTED_RECORDS)
    }
}

#[cfg(test)]
mod tests;
