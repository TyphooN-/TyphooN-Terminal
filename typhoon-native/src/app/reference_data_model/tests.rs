use super::*;

// ── Helpers ────────────────────────────────────────────────────────

fn digest(seed: char) -> String {
    std::iter::repeat_n(seed, 64).collect()
}

fn snapshot_summary(authoritative: bool, blocked: Option<&str>) -> ReferenceSourceSummary {
    ReferenceSourceSummary {
        path: "/tmp/snapshot.json".to_string(),
        kind: "calendar exceptions",
        scope: "XNYS · us_eastern".to_string(),
        source_system: if authoritative {
            "{\"system\":\"exchange_publication\",\"exchange\":\"NYSE\"}".to_string()
        } else {
            "{\"system\":\"rule_derived\",\"ruleset\":\"built-in NYSE rules\"}".to_string()
        },
        authority: if authoritative {
            "exchange_official"
        } else {
            "derived_rule"
        },
        authoritative,
        complete: true,
        covered_range: "2024-01-01 … 2025-12-31 (exchange dates)".to_string(),
        requested_range: "2024-01-01 … 2025-12-31 (exchange dates)".to_string(),
        as_of_ns: 1_799_000_000_000_000_000,
        retrieved_at_ns: 1_800_000_000_000_000_000,
        record_count: 3,
        // A snapshot below exchange/vendor authority only reaches an unblocked
        // summary when it did not demand authority — the worker refuses the
        // other combination outright, so the fixture never fakes one.
        require_authoritative: authoritative,
        blocked: blocked.map(str::to_string),
    }
}

fn calendar_artifact(id: &str) -> ReferenceArtifactSummary {
    ReferenceArtifactSummary {
        artifact_id: id.to_string(),
        kind: "calendar exceptions",
        scope: "XNYS · us_eastern".to_string(),
        symbol: None,
        currency: None,
        source_system: "{\"system\":\"exchange_publication\",\"exchange\":\"NYSE\"}".to_string(),
        authority: "exchange_official",
        authoritative: true,
        covered_range: "2024-01-01 … 2025-12-31 (exchange dates)".to_string(),
        as_of_ns: 1_799_000_000_000_000_000,
        record_count: 3,
        event_count: 3,
        adjustment: None,
    }
}

fn corporate_artifact(id: &str, authoritative: bool) -> ReferenceArtifactSummary {
    ReferenceArtifactSummary {
        artifact_id: id.to_string(),
        kind: "corporate actions",
        scope: "XNYS · AAA · USD".to_string(),
        symbol: Some("AAA".to_string()),
        currency: Some("USD".to_string()),
        source_system: "{\"system\":\"yahoo_chart_keyless\"}".to_string(),
        authority: if authoritative {
            "contracted_vendor"
        } else {
            "unverified_public"
        },
        authoritative,
        covered_range: "0 … 1 (UTC ns)".to_string(),
        as_of_ns: 1_799_000_000_000_000_000,
        record_count: 1,
        event_count: 1,
        adjustment: Some("raw"),
    }
}

fn listed(
    state: &mut ReferenceDataState,
    summaries: Vec<ReferenceArtifactSummary>,
    omitted: usize,
) {
    let request_id = state.begin_request();
    state.apply_event(ReferenceDataWorkerEvent::ArtifactsListed {
        request_id,
        summaries,
        omitted,
    });
}

// ── Tests ──────────────────────────────────────────────────────────

/// The panel offers nothing until a worker reply arrives. No default calendar,
/// no assumed symbol, no artifact chosen on the operator's behalf.
#[test]
fn a_fresh_panel_offers_no_defaults_and_no_promotion() {
    let state = ReferenceDataState::new();
    assert!(state.snapshot.is_none());
    assert!(state.artifacts.is_empty());
    assert!(state.selected_calendar.is_none());
    assert!(state.selected_corporate_actions.is_none());
    assert!(state.prepared_settings.is_none());
    assert!(!state.can_materialize());
    assert!(!state.can_select());
    assert_eq!(
        state.materialize_blocker().as_deref(),
        Some("Inspect a snapshot first.")
    );
    assert_eq!(
        state.select_blocker(),
        Some("Choose a calendar-exception artifact.")
    );
}

/// A snapshot the worker could not materialize keeps promotion unavailable, and
/// the panel repeats the worker's exact refusal rather than a generic message.
#[test]
fn a_blocked_snapshot_keeps_promotion_unavailable_and_states_why() {
    let mut state = ReferenceDataState::new();
    let request_id = state.begin_request();
    state.apply_event(ReferenceDataWorkerEvent::SnapshotInspected {
        request_id,
        summary: Box::new(snapshot_summary(
            false,
            Some("this run requires exchange-official or contracted-vendor reference data"),
        )),
    });

    assert!(!state.can_materialize());
    let blocker = state.materialize_blocker().expect("a stated reason");
    assert!(
        blocker.contains("exchange-official or contracted-vendor"),
        "the worker's own refusal must survive to the UI: {blocker}"
    );
    assert!(state.status.contains("cannot be promoted"));

    // The same snapshot with nothing blocking it does become promotable.
    let request_id = state.begin_request();
    state.apply_event(ReferenceDataWorkerEvent::SnapshotInspected {
        request_id,
        summary: Box::new(snapshot_summary(true, None)),
    });
    assert!(state.can_materialize());
    assert!(state.materialize_blocker().is_none());
}

/// A source that materializes but is below exchange/vendor authority needs the
/// operator to say so. Sealing is durable, and the panel must not make that
/// call on their behalf — nor carry one snapshot's acknowledgement to the next.
#[test]
fn a_non_authoritative_source_needs_an_explicit_acknowledgement() {
    let mut state = ReferenceDataState::new();
    let inspect = |state: &mut ReferenceDataState, authoritative: bool| {
        let request_id = state.begin_request();
        state.apply_event(ReferenceDataWorkerEvent::SnapshotInspected {
            request_id,
            summary: Box::new(snapshot_summary(authoritative, None)),
        });
    };

    inspect(&mut state, false);
    assert!(state.needs_authority_acknowledgement());
    assert!(!state.accept_non_authoritative, "never on by default");
    assert!(!state.can_materialize());
    let blocker = state.materialize_blocker().expect("a stated reason");
    assert!(
        blocker.contains("derived_rule"),
        "the blocker must name the real authority class: {blocker}"
    );

    state.accept_non_authoritative = true;
    assert!(state.can_materialize());
    assert!(state.materialize_blocker().is_none());

    // Inspecting the next snapshot clears the acknowledgement.
    inspect(&mut state, false);
    assert!(!state.accept_non_authoritative);
    assert!(!state.can_materialize());

    // An exchange/vendor source never asks for it in the first place.
    inspect(&mut state, true);
    assert!(!state.needs_authority_acknowledgement());
    assert!(state.can_materialize());
}

/// A reply for a superseded request is dropped: the panel must never show a
/// stale snapshot beside a newer one's status.
#[test]
fn superseded_replies_are_dropped() {
    let mut state = ReferenceDataState::new();
    let stale = state.begin_request();
    let current = state.begin_request();
    assert_ne!(stale, current);

    state.apply_event(ReferenceDataWorkerEvent::SnapshotInspected {
        request_id: stale,
        summary: Box::new(snapshot_summary(true, None)),
    });
    assert!(state.snapshot.is_none(), "a stale reply must not land");
    assert_eq!(state.pending, Some(current));

    state.apply_event(ReferenceDataWorkerEvent::SnapshotInspected {
        request_id: current,
        summary: Box::new(snapshot_summary(true, None)),
    });
    assert!(state.snapshot.is_some());
    assert!(state.pending.is_none());
}

/// Both slots must be filled by the operator before a selection is offered, and
/// each is filled only by the artifact's own kind.
#[test]
fn selection_requires_both_slots_a_symbol_and_a_currency() {
    let mut state = ReferenceDataState::new();
    listed(
        &mut state,
        vec![
            calendar_artifact(&digest('a')),
            corporate_artifact(&digest('b'), true),
        ],
        0,
    );
    assert_eq!(state.artifacts.len(), 2);
    assert!(!state.can_select());

    state.select(ReferenceSelectionSlot::Calendar, &digest('a'));
    assert_eq!(
        state.select_blocker(),
        Some("Choose a corporate-action artifact.")
    );

    state.select(ReferenceSelectionSlot::CorporateActions, &digest('b'));
    state.symbol = "  ".to_string();
    assert_eq!(
        state.select_blocker(),
        Some("Enter the instrument symbol to bind.")
    );

    state.symbol = "AAA".to_string();
    assert!(state.select_blocker().is_none());
    assert!(state.can_select());

    // An in-flight request suppresses the action even when the slots are full,
    // so one click cannot queue two binds.
    state.begin_request();
    assert!(!state.can_select());
}

/// A completed selection is reported with the authority it actually has. A
/// non-authoritative pair is bound and *labelled*, never presented as
/// exchange-backed.
#[test]
fn a_selection_carries_its_real_authority() {
    let mut state = ReferenceDataState::new();
    let request_id = state.begin_request();
    state.apply_event(ReferenceDataWorkerEvent::Selected {
        request_id,
        config_id: digest('c'),
        settings: Box::new(
            typhoon_engine::core::strategy_ir::ExecutionSettings::conservative_defaults(),
        ),
        calendar_artifact_id: digest('a'),
        corporate_action_artifact_id: digest('b'),
        authoritative: false,
    });

    let selection = state.selection.as_ref().expect("a selection");
    assert!(!selection.authoritative);
    assert_eq!(selection.config_id, digest('c'));
    assert!(
        state.status.contains("NOT exchange/vendor authoritative"),
        "an unverified pair must say so: {}",
        state.status
    );
    assert!(state.prepared_settings.is_some());
}

/// Changing a slot discards the prepared settings: they were sealed against the
/// previous choice, and showing them beside a new one would misstate the run.
#[test]
fn changing_a_slot_discards_the_previous_preparation() {
    let mut state = ReferenceDataState::new();
    state.select(ReferenceSelectionSlot::Calendar, &digest('a'));
    state.select(ReferenceSelectionSlot::CorporateActions, &digest('b'));
    let request_id = state.begin_request();
    state.apply_event(ReferenceDataWorkerEvent::Selected {
        request_id,
        config_id: digest('c'),
        settings: Box::new(
            typhoon_engine::core::strategy_ir::ExecutionSettings::conservative_defaults(),
        ),
        calendar_artifact_id: digest('a'),
        corporate_action_artifact_id: digest('b'),
        authoritative: true,
    });
    assert!(state.prepared_settings.is_some());

    // Re-picking the same artifact is a no-op.
    state.select(ReferenceSelectionSlot::Calendar, &digest('a'));
    assert!(state.prepared_settings.is_some());

    state.select(ReferenceSelectionSlot::Calendar, &digest('d'));
    assert!(state.prepared_settings.is_none());
    assert!(state.selection.is_none());
}

/// A listing that no longer contains a chosen artifact clears that slot rather
/// than leaving it pointing at something the panel cannot show.
#[test]
fn a_vanished_artifact_clears_its_slot_and_the_preparation() {
    let mut state = ReferenceDataState::new();
    state.select(ReferenceSelectionSlot::Calendar, &digest('a'));
    state.select(ReferenceSelectionSlot::CorporateActions, &digest('b'));

    listed(&mut state, vec![corporate_artifact(&digest('b'), true)], 0);
    assert!(state.selected_calendar.is_none());
    assert_eq!(
        state.selected_corporate_actions.as_deref(),
        Some(&*digest('b'))
    );
    assert!(state.prepared_settings.is_none());
}

/// A truncated listing says how many artifacts it is not showing. A silent
/// truncation would read as "this is everything in the store".
#[test]
fn a_truncated_listing_reports_what_it_omitted() {
    let mut state = ReferenceDataState::new();
    let summaries: Vec<_> = (0..REFERENCE_LIST_LIMIT + 5)
        .map(|index| calendar_artifact(&format!("{index:064x}")))
        .collect();
    listed(&mut state, summaries, 7);

    assert_eq!(state.artifacts.len(), REFERENCE_LIST_LIMIT);
    assert_eq!(state.artifacts_omitted, 12);
    assert!(
        state.status.contains("12 more not listed"),
        "omissions must be visible: {}",
        state.status
    );
}

/// Backpressure frees the pending slot so the next frame can retry. A stranded
/// slot would wedge every button in the panel.
#[test]
fn submit_failures_free_the_pending_slot() {
    let mut state = ReferenceDataState::new();
    state.begin_request();
    state.note_submit_failure(ReferenceSubmitError::QueueFull);
    assert!(state.pending.is_none());
    assert!(state.status.contains("busy"));

    state.begin_request();
    state.note_submit_failure(ReferenceSubmitError::WorkerStopped);
    assert!(state.pending.is_none());
    assert!(state.status.contains("not running"));
}

/// The panel's own cap never exceeds the worker's, so a request can always be
/// satisfied in one reply.
#[test]
fn the_list_limit_is_within_the_worker_bound() {
    assert!(ReferenceDataState::list_limit() <= MAX_LISTED_ARTIFACTS);
    assert_eq!(ReferenceDataState::list_limit(), REFERENCE_LIST_LIMIT);
}

#[test]
fn short_ids_never_panic_on_odd_lengths() {
    assert_eq!(short_id(""), "");
    assert_eq!(short_id("abc"), "abc");
    assert_eq!(short_id(&digest('a')), "aaaaaaaaaaaa");
}
