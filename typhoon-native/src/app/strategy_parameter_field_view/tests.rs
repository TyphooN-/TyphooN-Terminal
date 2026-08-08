use super::*;

use typhoon_engine::core::strategy_ir::ParamValue;

fn axis(id: &str, count: usize) -> AxisInput {
    AxisInput {
        id: id.to_string(),
        values: (0..count)
            .map(|value| ParamValue::Int(value as i64))
            .collect(),
    }
}

fn point<'a>(
    evaluation_n: usize,
    ordinal: usize,
    candidate_id: &'a str,
    axis_indices: &'a [usize],
    value: f64,
) -> PointInput<'a> {
    PointInput {
        evaluation_n,
        phase: "field sample",
        ordinal,
        candidate_id,
        axis_indices,
        value,
        rank: ordinal + 1,
    }
}

#[test]
fn projection_preserves_sealed_order_identity_and_evaluation_n() {
    let axes = vec![axis("fast", 3), axis("slow", 4), axis("threshold", 2)];
    let indices = [vec![2, 0, 1], vec![0, 3, 0], vec![1, 1, 1]];
    let points = [
        point(7, 10, "candidate-z", &indices[0], 3.0),
        point(8, 2, "candidate-a", &indices[1], 1.0),
        point(9, 6, "candidate-m", &indices[2], 2.0),
    ];

    let projected = project_parts(&axes, &points).expect("projection");

    assert_eq!(
        projected
            .axes
            .iter()
            .map(|axis| axis.id.as_str())
            .collect::<Vec<_>>(),
        ["fast", "slow", "threshold"]
    );
    assert_eq!(
        projected
            .points
            .iter()
            .map(|row| row.candidate_id.as_str())
            .collect::<Vec<_>>(),
        ["candidate-z", "candidate-a", "candidate-m"]
    );
    assert_eq!(
        projected
            .points
            .iter()
            .map(|row| row.evaluation_n)
            .collect::<Vec<_>>(),
        [7, 8, 9]
    );
    assert_eq!(
        projected
            .points
            .iter()
            .map(|row| row.ordinal)
            .collect::<Vec<_>>(),
        [10, 2, 6]
    );
    assert_eq!(
        projected.points[0].parameter_coordinates,
        vec![Some(1.0), Some(0.0), Some(1.0)]
    );
    assert_eq!(projected.points[1].metric_coordinate, Some(0.0));
    assert_eq!(projected.points[0].metric_coordinate, Some(1.0));
}

#[test]
fn projection_caps_axes_points_and_surface_segments_with_explicit_omissions() {
    let axes: Vec<_> = (0..MAX_PARALLEL_PARAMETER_AXES + 4)
        .map(|index| axis(&format!("axis-{index}"), 2))
        .collect();
    let all_indices: Vec<Vec<usize>> = (0..MAX_PARAMETER_FIELD_POINTS + 9)
        .map(|index| (0..axes.len()).map(|axis| (index + axis) % 2).collect())
        .collect();
    let ids: Vec<String> = (0..all_indices.len())
        .map(|index| format!("candidate-{index}"))
        .collect();
    let points: Vec<_> = all_indices
        .iter()
        .zip(ids.iter())
        .enumerate()
        .map(|(index, (indices, id))| point(index + 1, index, id, indices, index as f64))
        .collect();

    let projected = project_parts(&axes, &points).expect("projection");

    assert_eq!(projected.axes.len(), MAX_PARALLEL_PARAMETER_AXES);
    assert_eq!(projected.omitted_axes, 4);
    assert_eq!(projected.points.len(), MAX_PARAMETER_FIELD_POINTS);
    assert_eq!(projected.omitted_points, 9);
    assert!(projected.surface_segments.len() <= MAX_SURFACE_SEGMENTS);
    assert_eq!(
        projected.surface_segments.len() + projected.omitted_surface_segments,
        projected.total_surface_segments
    );
}

#[test]
fn non_finite_and_invalid_projection_inputs_fail_closed() {
    let axes = vec![axis("x", 2), axis("y", 2), axis("z", 2)];
    let valid = [0, 1, 0];
    let invalid_index = [0, 2, 0];

    assert!(project_parts(&axes, &[point(1, 0, "candidate", &valid, f64::NAN)]).is_err());
    assert!(project_parts(&axes, &[point(1, 0, "candidate", &valid, f64::INFINITY)]).is_err());
    assert!(project_parts(&axes, &[point(1, 0, "candidate", &invalid_index, 1.0)]).is_err());
    assert!(project_parts(&axes, &[point(0, 0, "candidate", &valid, 1.0)]).is_err());
    assert!(project_parts(&[axis("x", 0), axis("y", 2)], &[]).is_err());
}

#[test]
fn constant_metric_scale_is_explicitly_undefined_not_fabricated_as_zero() {
    let axes = vec![axis("x", 2), axis("y", 2), axis("z", 2)];
    let a = [0, 0, 0];
    let b = [1, 1, 1];

    let projected = project_parts(
        &axes,
        &[point(1, 0, "a", &a, 4.0), point(2, 1, "b", &b, 4.0)],
    )
    .expect("projection");

    assert_eq!(projected.metric_range, None);
    assert!(
        projected
            .points
            .iter()
            .all(|point| point.metric_coordinate.is_none())
    );
    assert_eq!(projected.undefined_metric_points, 2);
}

fn dummy_view(identity: &str) -> ParameterFieldView {
    ParameterFieldView {
        artifact_id: identity.to_string(),
        source_dataset_id: "dataset".into(),
        source_manifest_id: "manifest".into(),
        config_id: "config".into(),
        metric_id: "metric".into(),
        evaluations_n: 3,
        range_start: 0,
        range_end: 3,
        axes: Vec::new(),
        omitted_axes: 0,
        points: Vec::new(),
        omitted_points: 0,
        metric_range: None,
        undefined_metric_points: 0,
        surface_segments: Vec::new(),
        omitted_surface_segments: 0,
        total_surface_segments: 0,
        spp_estimate: 1.0,
        selected_value: 1.0,
        stability_bps: 5_000,
    }
}

#[test]
fn stale_cancel_and_failure_preserve_the_last_verified_view() {
    let mut state = ParameterFieldLoadState::default();
    state.installed = Some(dummy_view("old"));

    let first = state.begin_request();
    let second = state.begin_request();
    assert_eq!(
        state.complete(first, Ok(dummy_view("stale"))),
        Completion::Stale
    );
    assert_eq!(state.installed.as_ref().unwrap().artifact_id, "old");

    assert_eq!(
        state.complete(second, Err("decode failed".into())),
        Completion::Failed
    );
    assert_eq!(state.installed.as_ref().unwrap().artifact_id, "old");

    let cancelled = state.begin_request();
    state.cancel();
    assert_eq!(
        state.complete(cancelled, Ok(dummy_view("cancelled"))),
        Completion::Stale
    );
    assert_eq!(state.installed.as_ref().unwrap().artifact_id, "old");

    let current = state.begin_request();
    assert_eq!(
        state.complete(current, Ok(dummy_view("new"))),
        Completion::Installed
    );
    assert_eq!(state.installed.as_ref().unwrap().artifact_id, "new");
}

#[test]
fn loader_runs_the_verifying_boundary_on_a_worker_thread() {
    let caller = std::thread::current().id();
    let request = 42;
    let receiver = spawn_load_with(request, move || {
        assert_ne!(std::thread::current().id(), caller);
        Err("expected fixture refusal".to_string())
    });

    let completion = receiver.recv().expect("worker completion");
    assert_eq!(completion.request, request);
    assert_ne!(completion.worker_thread, caller);
    assert_eq!(completion.result.unwrap_err(), "expected fixture refusal");
}
