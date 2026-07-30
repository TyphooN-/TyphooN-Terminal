use super::*;

use typhoon_engine::core::strategy_problem_recognition::{
    AbsurdMetricObservations, EdgeConcentration, ParameterStepObservations,
};

// ── Helpers ────────────────────────────────────────────────────────

fn policy() -> ProblemRecognitionPolicy {
    ProblemRecognitionPolicy {
        minimum_trades: 100,
        maximum_top_trade_share_bps: 2_000,
        maximum_time_in_market_bps: 8_000,
        boundary_width_bps: 1_000,
        maximum_boundary_trade_share_bps: 3_000,
        minimum_cost_2x_ratio_bps: 5_000,
        minimum_cost_3x_ratio_bps: 3_000,
        minimum_oos_is_ratio_bps: 5_000,
        maximum_edge_concentration_bps: 6_000,
        maximum_absolute_sharpe_bps: 50_000,
        minimum_max_drawdown_bps: 100,
        minimum_parameter_step_ratio_bps: 5_000,
    }
}

fn observations() -> ReportProblemObservations {
    ReportProblemObservations {
        trade_count: 312,
        top_trade_share_bps: 1_250,
        time_in_market_bps: 4_400,
        boundary_trade_share_bps: 900,
        cost_2x_ratio_bps: 7_100,
        cost_3x_ratio_bps: 4_050,
        oos_is_ratio_bps: 6_600,
        edge_concentration: EdgeConcentration {
            calendar_granularity: CalendarGranularity::Monthly,
            calendar_periods: 24,
            calendar_share_bps: Some(1_800),
            symbols: 3,
            symbol_share_bps: Some(5_500),
            sides: 2,
            side_share_bps: Some(5_100),
            worst: Some((ConcentrationFamily::Symbol, 5_500)),
        },
        absurd_metrics: AbsurdMetricObservations {
            absolute_sharpe_bps: Some(14_200),
            max_drawdown_bps: Some(1_650),
            profit_factor_at_sentinel: false,
        },
        parameter_step: ParameterStepObservations {
            steps_n: 8,
            worst_step_ratio_bps: 6_900,
        },
    }
}

fn measured(rows: &[ProblemObservationRow], label: &str) -> String {
    rows.iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("row {label} is missing"))
        .measured
        .clone()
}

fn bound(rows: &[ProblemObservationRow], label: &str) -> String {
    rows.iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("row {label} is missing"))
        .bound
        .clone()
}

// ── Stage projection ───────────────────────────────────────────────

#[test]
fn stage_rows_carry_the_sealed_verdict_and_evaluation_n_unchanged() {
    let sealed = vec![
        StageEvidence::pass("trade-count", 312, "312 trades clears the 100 minimum"),
        StageEvidence::fail(
            "cost-degradation-3x",
            40,
            "4050 bps retention below 3000 bps",
        ),
    ];

    let (rows, omitted) = project_stages(&sealed);

    assert_eq!(omitted, 0);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].stage, "trade-count");
    assert_eq!(rows[0].verdict, StageVerdict::Pass);
    assert_eq!(rows[0].observations_n, 312);
    assert_eq!(rows[0].reason, "312 trades clears the 100 minimum");
    assert!(!rows[0].reason_truncated);
    assert_eq!(rows[1].verdict, StageVerdict::Fail);
    assert_eq!(rows[1].observations_n, 40);
}

#[test]
fn a_stage_list_above_the_display_cap_is_bounded_and_counts_what_it_hid() {
    // A verified artifact seals exactly the current gate set, so this is the defensive path for a
    // later schema — the projection must stay bounded and stay honest about the overflow.
    let sealed: Vec<_> = (0..MAX_PROBLEM_STAGE_ROWS + 8)
        .map(|index| StageEvidence::pass(format!("gate-{index}"), index, "reason"))
        .collect();

    let (rows, omitted) = project_stages(&sealed);

    assert_eq!(rows.len(), MAX_PROBLEM_STAGE_ROWS);
    assert_eq!(omitted, 8);
    assert_eq!(rows[0].stage, "gate-0");
    assert_eq!(
        rows[MAX_PROBLEM_STAGE_ROWS - 1].stage,
        format!("gate-{}", MAX_PROBLEM_STAGE_ROWS - 1)
    );
}

#[test]
fn an_oversized_sealed_reason_is_clamped_and_says_so() {
    let long: String = std::iter::repeat_n('x', MAX_REASON_CHARS + 40).collect();
    let short: String = std::iter::repeat_n('y', MAX_REASON_CHARS).collect();

    let (rows, _) = project_stages(&[
        StageEvidence::fail("long", 1, long),
        StageEvidence::pass("exactly-at-cap", 1, short.clone()),
    ]);

    assert!(rows[0].reason_truncated);
    assert_eq!(rows[0].reason.chars().count(), MAX_REASON_CHARS);
    // The control: a reason at the cap is passed through whole and not marked truncated.
    assert!(!rows[1].reason_truncated);
    assert_eq!(rows[1].reason, short);
}

// ── Observation projection ─────────────────────────────────────────

#[test]
fn every_observation_is_paired_with_the_policy_bound_it_faced() {
    let rows = project_observations(&observations(), &policy());

    assert_eq!(measured(&rows, "Trades"), "312");
    assert_eq!(bound(&rows, "Trades"), "min 100");
    assert_eq!(measured(&rows, "Top-trade PnL share"), "12.50% (1250 bps)");
    assert_eq!(bound(&rows, "Top-trade PnL share"), "max 20.00% (2000 bps)");
    assert_eq!(
        measured(&rows, "Retention at 3x costs"),
        "40.50% (4050 bps)"
    );
    assert_eq!(
        bound(&rows, "Retention at 3x costs"),
        "min 30.00% (3000 bps)"
    );
    assert_eq!(
        bound(&rows, "Sample-boundary trade share"),
        "max 30.00% (3000 bps) (edge band 10.00% (1000 bps))"
    );
    assert_eq!(
        measured(&rows, "Worst edge concentration"),
        "symbol 55.00% (5500 bps)"
    );
    assert_eq!(
        measured(&rows, "Calendar concentration"),
        "18.00% (1800 bps) over 24 monthly periods"
    );
    assert_eq!(
        measured(&rows, "Worst +/-1 parameter step retention"),
        "69.00% (6900 bps) over 8 sealed neighbours"
    );
    // Context-only rows must not invent a bound they were never judged against.
    assert_eq!(bound(&rows, "Symbol concentration"), CONTEXT_ONLY);
    assert_eq!(bound(&rows, "Side concentration"), CONTEXT_ONLY);
}

#[test]
fn registry_values_the_engine_left_undefined_are_shown_as_undefined_not_zero() {
    let mut undefined = observations();
    undefined.absurd_metrics.absolute_sharpe_bps = None;
    undefined.absurd_metrics.max_drawdown_bps = None;
    undefined.absurd_metrics.profit_factor_at_sentinel = true;
    undefined.edge_concentration.worst = None;
    undefined.edge_concentration.symbol_share_bps = None;

    let rows = project_observations(&undefined, &policy());

    assert_eq!(measured(&rows, "|Sharpe ratio|"), "undefined");
    assert_eq!(measured(&rows, "Max drawdown"), "undefined");
    assert_eq!(measured(&rows, "Profit factor at sentinel"), "yes");
    assert_eq!(
        measured(&rows, "Worst edge concentration"),
        "no evaluable family"
    );
    assert_eq!(
        measured(&rows, "Symbol concentration"),
        "undefined over 3 symbols"
    );
    // The bound is still stated: an undefined measurement does not erase the gate it faced.
    assert_eq!(bound(&rows, "|Sharpe ratio|"), "max 500.00% (50000 bps)");
}

// ── Sealed source lineage ──────────────────────────────────────────

fn cross_id() -> String {
    std::iter::repeat_n('a', 64).collect()
}

fn dataset_id() -> String {
    std::iter::repeat_n('b', 64).collect()
}

#[test]
fn each_lineage_row_carries_its_exact_sealed_identities_and_evaluation_scope() {
    let cross = cross_check_row(
        &cross_id(),
        &std::iter::repeat_n('c', 64).collect::<String>(),
        &dataset_id(),
        "net_profit",
        4_096,
        3,
    );
    assert_eq!(cross.study, CROSS_CHECK_STUDY);
    // The full sealed identity is carried, not a shortened one — the panel shortens at render.
    assert_eq!(cross.artifact_id, cross_id());
    assert_eq!(
        cross.binds,
        "strategy cccccccc · dataset bbbbbbbb · metric net_profit"
    );
    assert_eq!(cross.scope, "4096 evaluations · 3 sealed checks");
    assert!(cross.error.is_none());

    let oos = oos_row(
        &cross_id(),
        &std::iter::repeat_n('d', 64).collect::<String>(),
        &dataset_id(),
        "net_profit",
        &std::iter::repeat_n('e', 64).collect::<String>(),
        2_500,
        4,
    );
    assert_eq!(
        oos.binds,
        "candidate dddddddd · dataset bbbbbbbb · config eeeeeeee · metric net_profit"
    );
    assert_eq!(oos.scope, "2500 bars · 4 executed partitions");

    let significance = significance_row(&cross_id(), &dataset_id(), "sharpe_ratio", 4_096, 128);
    assert_eq!(significance.study, SIGNIFICANCE_STUDY);
    assert_eq!(significance.binds, "dataset bbbbbbbb · metric sharpe_ratio");
    // Evaluation N is what bounds what the verdict can speak for, so it is stated verbatim.
    assert_eq!(significance.scope, "4096 evaluations · 128 candidates");
}

#[test]
fn the_lineage_projection_is_deterministic_for_the_same_sealed_inputs() {
    let first = significance_row(&cross_id(), &dataset_id(), "net_profit", 12, 7);
    let second = significance_row(&cross_id(), &dataset_id(), "net_profit", 12, 7);

    assert_eq!(first, second);
    // A different sealed input must not collapse onto the same row.
    assert_ne!(
        first,
        significance_row(&cross_id(), &dataset_id(), "net_profit", 13, 7)
    );
}

#[test]
fn an_undecodable_source_reports_the_refusal_under_the_existing_reason_bound() {
    let long: String = std::iter::repeat_n('z', MAX_REASON_CHARS + 60).collect();

    let row = unreadable_source_row(OOS_STUDY, &long);

    assert_eq!(row.study, OOS_STUDY);
    assert_eq!(
        row.error.as_deref().expect("refusal").chars().count(),
        MAX_REASON_CHARS
    );
    // An unreadable source claims no identity and no scope rather than showing a blank-looking one.
    assert!(row.artifact_id.is_empty());
    assert!(row.binds.is_empty());
    assert!(row.scope.is_empty());
}

#[test]
fn the_lineage_row_owns_only_display_strings_not_a_decoded_source_or_its_sealed_bytes() {
    // The row is `&'static str` + 3 `String` + `Option<String>`. Smuggling a decoded
    // CrossCheckStudyArtifact / ExecutedOosScheme / SignificanceStudyArtifact, or the zstd bytes
    // they were inflated from, into the view would change this size and fail here.
    let expected =
        size_of::<&'static str>() + size_of::<String>() * 3 + size_of::<Option<String>>();
    assert_eq!(size_of::<ProblemSourceRow>(), expected);

    // And what it does own stays small: identities are fixed-width digests and the composed
    // strings are bounded by them plus counts.
    let row = cross_check_row(
        &cross_id(),
        &dataset_id(),
        &dataset_id(),
        "net_profit",
        usize::MAX,
        usize::MAX,
    );
    let owned = row.artifact_id.len() + row.binds.len() + row.scope.len();
    assert!(owned < 512, "lineage row owns {owned} bytes");
}

// ── Off-thread request identity ────────────────────────────────────

#[test]
fn only_the_outstanding_request_is_accepted() {
    let mut state = ProblemLoadState::default();
    assert!(!state.is_busy());

    let first = state.begin_request();
    assert!(state.is_busy());
    let second = state.begin_request();

    // A superseded load's late completion is dropped.
    assert!(!state.accept(first));
    assert!(state.is_busy());
    assert!(state.accept(second));
    assert!(!state.is_busy());
    // And a completion cannot be accepted twice.
    assert!(!state.accept(second));
}

#[test]
fn a_cancelled_load_never_accepts_its_own_completion() {
    let mut state = ProblemLoadState::default();
    let request = state.begin_request();
    state.cancel();

    assert!(!state.is_busy());
    assert!(!state.accept(request));
    // The next request still gets a fresh identity that the cancelled one cannot impersonate.
    let next = state.begin_request();
    assert_ne!(next, request);
    assert!(state.accept(next));
}

// ── Bounded read before the verifying decode ───────────────────────

#[test]
fn an_artifact_file_above_the_engine_byte_cap_is_refused_before_it_is_decoded() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "typhoon-problem-view-{}-{nonce}.json",
        std::process::id()
    ));
    std::fs::write(&path, vec![b'x'; MAX_ARTIFACT_BYTES + 1]).expect("fixture");

    let error = load_problem_recognition(&path).expect_err("oversized artifact must be refused");

    let _ = std::fs::remove_file(&path);
    assert!(
        error.contains(&format!("exceeds byte limit {MAX_ARTIFACT_BYTES}")),
        "unexpected error: {error}"
    );
}

#[test]
fn a_missing_artifact_reports_the_path_instead_of_panicking() {
    let path = std::env::temp_dir().join("typhoon-problem-view-does-not-exist.json");
    let _ = std::fs::remove_file(&path);

    let error = load_problem_recognition(&path).expect_err("missing artifact must be refused");

    assert!(
        error.starts_with("cannot open "),
        "unexpected error: {error}"
    );
}

#[test]
fn garbage_bytes_fail_the_verifying_decode_rather_than_producing_a_view() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "typhoon-problem-view-garbage-{}-{nonce}.json",
        std::process::id()
    ));
    std::fs::write(&path, b"{\"schema_version\":2}").expect("fixture");

    let result = load_problem_recognition(&path);

    let _ = std::fs::remove_file(&path);
    assert!(
        result.is_err(),
        "a partial artifact must not project a view"
    );
}
