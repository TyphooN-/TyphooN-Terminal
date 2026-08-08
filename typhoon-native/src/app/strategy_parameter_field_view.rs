//! Bounded native projections for a sealed ADR-135 §7.4 parameter-field study.
//!
//! The file boundary deliberately performs the bounded read, JSON decode, embedded-report
//! decompression, identity verification, evidence replay and projection on a worker. Egui receives
//! only the capped, display-ready values below; repaint code never owns artifact bytes or invokes
//! the engine verifier.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, sync_channel};
use std::thread::ThreadId;

use typhoon_engine::core::strategy_ir::ParamValue;
use typhoon_engine::core::strategy_optimization::MAX_ARTIFACT_BYTES;
use typhoon_engine::core::strategy_parameter_field::{
    ParameterFieldPhase, ParameterFieldStudyArtifact,
};

use super::*;

pub(crate) const MAX_PARALLEL_PARAMETER_AXES: usize = 12;
pub(crate) const MAX_PARAMETER_FIELD_POINTS: usize = 128;
pub(crate) const MAX_SURFACE_SEGMENTS: usize = 256;
const MAX_AXIS_LABEL_CHARS: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedParameterAxis {
    pub(crate) id: String,
    pub(crate) first_value: String,
    pub(crate) last_value: String,
    pub(crate) value_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedParameterPoint {
    pub(crate) evaluation_n: usize,
    pub(crate) phase: &'static str,
    pub(crate) ordinal: usize,
    pub(crate) candidate_id: String,
    pub(crate) value: f64,
    pub(crate) rank: usize,
    pub(crate) parameter_coordinates: Vec<Option<f32>>,
    pub(crate) metric_coordinate: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParameterFieldView {
    pub(crate) artifact_id: String,
    pub(crate) source_dataset_id: String,
    pub(crate) source_manifest_id: String,
    pub(crate) config_id: String,
    pub(crate) metric_id: String,
    pub(crate) evaluations_n: usize,
    pub(crate) range_start: usize,
    pub(crate) range_end: usize,
    pub(crate) axes: Vec<PreparedParameterAxis>,
    pub(crate) omitted_axes: usize,
    pub(crate) points: Vec<PreparedParameterPoint>,
    pub(crate) omitted_points: usize,
    pub(crate) metric_range: Option<(f64, f64)>,
    pub(crate) undefined_metric_points: usize,
    /// Pairs index `points` and are prepared on the worker, never discovered during paint.
    pub(crate) surface_segments: Vec<[usize; 2]>,
    pub(crate) omitted_surface_segments: usize,
    pub(crate) total_surface_segments: usize,
    pub(crate) spp_estimate: f64,
    pub(crate) selected_value: f64,
    pub(crate) stability_bps: u32,
}

impl ParameterFieldView {
    fn from_verified(artifact: &ParameterFieldStudyArtifact) -> Result<Self, String> {
        artifact.verify().map_err(|error| error.to_string())?;
        let axes: Vec<_> = artifact
            .axes()
            .iter()
            .map(|axis| AxisInput {
                id: axis.id().to_string(),
                values: axis.values().to_vec(),
            })
            .collect();
        let points: Vec<_> = artifact
            .points()
            .iter()
            .map(|point| PointInput {
                evaluation_n: point.evaluation_n,
                phase: phase_label(point.phase),
                ordinal: point.ordinal,
                candidate_id: &point.candidate_id,
                axis_indices: &point.axis_indices,
                value: point.value,
                rank: point.rank,
            })
            .collect();
        let projected = project_parts(&axes, &points)?;
        let range = artifact.range();
        Ok(Self {
            artifact_id: artifact.artifact_id().to_string(),
            source_dataset_id: artifact.source_dataset_id().to_string(),
            source_manifest_id: artifact.source_manifest_id().to_string(),
            config_id: artifact.config_id().to_string(),
            metric_id: artifact.metric_id().to_string(),
            evaluations_n: artifact.evaluations_n(),
            range_start: range.start,
            range_end: range.end,
            axes: projected.axes,
            omitted_axes: projected.omitted_axes,
            points: projected.points,
            omitted_points: projected.omitted_points,
            metric_range: projected.metric_range,
            undefined_metric_points: projected.undefined_metric_points,
            surface_segments: projected.surface_segments,
            omitted_surface_segments: projected.omitted_surface_segments,
            total_surface_segments: projected.total_surface_segments,
            spp_estimate: artifact.spp().estimate(),
            selected_value: artifact.spp().selected_value(),
            stability_bps: artifact.profile().stability_bps(),
        })
    }
}

#[derive(Debug)]
struct AxisInput {
    id: String,
    values: Vec<ParamValue>,
}

#[derive(Clone, Copy, Debug)]
struct PointInput<'a> {
    evaluation_n: usize,
    phase: &'static str,
    ordinal: usize,
    candidate_id: &'a str,
    axis_indices: &'a [usize],
    value: f64,
    rank: usize,
}

#[derive(Debug)]
struct ProjectedParts {
    axes: Vec<PreparedParameterAxis>,
    omitted_axes: usize,
    points: Vec<PreparedParameterPoint>,
    omitted_points: usize,
    metric_range: Option<(f64, f64)>,
    undefined_metric_points: usize,
    surface_segments: Vec<[usize; 2]>,
    omitted_surface_segments: usize,
    total_surface_segments: usize,
}

fn project_parts(axes: &[AxisInput], points: &[PointInput<'_>]) -> Result<ProjectedParts, String> {
    if axes.is_empty() {
        return Err("parameter-field artifact has no axes".into());
    }
    for axis in axes {
        if axis.id.is_empty() || axis.values.is_empty() {
            return Err("parameter-field axis is empty".into());
        }
        if axis.values.iter().any(param_is_non_finite) {
            return Err(format!("parameter-field axis {} is non-finite", axis.id));
        }
    }

    let mut minimum = f64::INFINITY;
    let mut maximum = f64::NEG_INFINITY;
    for point in points {
        if point.evaluation_n == 0
            || point.candidate_id.is_empty()
            || point.axis_indices.len() != axes.len()
            || !point.value.is_finite()
        {
            return Err("invalid or non-finite parameter-field point".into());
        }
        for (axis_index, value_index) in point.axis_indices.iter().copied().enumerate() {
            if value_index >= axes[axis_index].values.len() {
                return Err("parameter-field point axis index is out of bounds".into());
            }
        }
        minimum = minimum.min(point.value);
        maximum = maximum.max(point.value);
    }
    let metric_range = if !points.is_empty() && minimum < maximum {
        Some((minimum, maximum))
    } else {
        None
    };

    let omitted_axes = axes.len().saturating_sub(MAX_PARALLEL_PARAMETER_AXES);
    let prepared_axes: Vec<_> = axes
        .iter()
        .take(MAX_PARALLEL_PARAMETER_AXES)
        .map(|axis| PreparedParameterAxis {
            id: clamp_label(&axis.id),
            first_value: format_param(&axis.values[0]),
            last_value: format_param(axis.values.last().expect("axis is non-empty")),
            value_count: axis.values.len(),
        })
        .collect();
    let omitted_points = points.len().saturating_sub(MAX_PARAMETER_FIELD_POINTS);
    let prepared_points: Vec<_> = points
        .iter()
        .take(MAX_PARAMETER_FIELD_POINTS)
        .map(|point| PreparedParameterPoint {
            evaluation_n: point.evaluation_n,
            phase: point.phase,
            ordinal: point.ordinal,
            candidate_id: point.candidate_id.to_string(),
            value: point.value,
            rank: point.rank,
            parameter_coordinates: point
                .axis_indices
                .iter()
                .copied()
                .zip(axes.iter())
                .take(MAX_PARALLEL_PARAMETER_AXES)
                .map(|(index, axis)| Some(normalize_index(index, axis.values.len())))
                .collect(),
            metric_coordinate: metric_range
                .map(|(low, high)| ((point.value - low) / (high - low)) as f32),
        })
        .collect();
    let undefined_metric_points = prepared_points
        .iter()
        .filter(|point| point.metric_coordinate.is_none())
        .count();
    let all_segments = prepare_surface_segments(&prepared_points);
    let total_surface_segments = all_segments.len();
    let omitted_surface_segments = total_surface_segments.saturating_sub(MAX_SURFACE_SEGMENTS);
    let surface_segments = all_segments
        .into_iter()
        .take(MAX_SURFACE_SEGMENTS)
        .collect();

    Ok(ProjectedParts {
        axes: prepared_axes,
        omitted_axes,
        points: prepared_points,
        omitted_points,
        metric_range,
        undefined_metric_points,
        surface_segments,
        omitted_surface_segments,
        total_surface_segments,
    })
}

fn prepare_surface_segments(points: &[PreparedParameterPoint]) -> Vec<[usize; 2]> {
    if points
        .iter()
        .any(|point| point.parameter_coordinates.len() < 2)
    {
        return Vec::new();
    }
    let mut edges = BTreeSet::new();
    for index in 0..points.len() {
        let x = points[index].parameter_coordinates[0];
        let y = points[index].parameter_coordinates[1];
        let mut nearest_x: Option<(f32, usize)> = None;
        let mut nearest_y: Option<(f32, usize)> = None;
        for other in 0..points.len() {
            if index == other {
                continue;
            }
            let other_x = points[other].parameter_coordinates[0];
            let other_y = points[other].parameter_coordinates[1];
            if x == other_x && y != other_y {
                let distance = (y.unwrap_or_default() - other_y.unwrap_or_default()).abs();
                if nearest_y.is_none_or(|nearest| distance < nearest.0) {
                    nearest_y = Some((distance, other));
                }
            }
            if y == other_y && x != other_x {
                let distance = (x.unwrap_or_default() - other_x.unwrap_or_default()).abs();
                if nearest_x.is_none_or(|nearest| distance < nearest.0) {
                    nearest_x = Some((distance, other));
                }
            }
        }
        for other in [nearest_x, nearest_y].into_iter().flatten() {
            let pair = [index.min(other.1), index.max(other.1)];
            edges.insert(pair);
        }
    }
    edges.into_iter().collect()
}

fn normalize_index(index: usize, count: usize) -> f32 {
    if count <= 1 {
        0.5
    } else {
        index as f32 / (count - 1) as f32
    }
}

fn param_is_non_finite(value: &ParamValue) -> bool {
    matches!(value, ParamValue::Float(value) if !value.is_finite())
}

fn format_param(value: &ParamValue) -> String {
    let value = match value {
        ParamValue::Bool(value) => value.to_string(),
        ParamValue::Int(value) => value.to_string(),
        ParamValue::Float(value) => format!("{value:.6}"),
        ParamValue::Text(value) => value.clone(),
    };
    clamp_label(&value)
}

fn clamp_label(value: &str) -> String {
    value.chars().take(MAX_AXIS_LABEL_CHARS).collect()
}

fn phase_label(phase: ParameterFieldPhase) -> &'static str {
    match phase {
        ParameterFieldPhase::FieldSample => "field sample",
        ParameterFieldPhase::PlateauNeighbour => "plateau neighbour",
    }
}

pub(crate) fn load_parameter_field(path: &Path) -> Result<ParameterFieldView, String> {
    let bytes = read_bounded(path, MAX_ARTIFACT_BYTES)?;
    // This call verifies identity and replays all derived evidence while inflating every sealed
    // report. It must remain on the worker side of `spawn_parameter_field_load`.
    let artifact =
        ParameterFieldStudyArtifact::from_json_slice(&bytes).map_err(|error| error.to_string())?;
    ParameterFieldView::from_verified(&artifact)
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let length = file
        .metadata()
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
        .len();
    if length > limit as u64 {
        return Err(format!(
            "{} exceeds byte limit {limit} ({length} bytes)",
            path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!("{} exceeds byte limit {limit}", path.display()));
    }
    Ok(bytes)
}

#[derive(Debug)]
pub(crate) struct LoadCompletion {
    pub(crate) request: u64,
    pub(crate) worker_thread: ThreadId,
    pub(crate) result: Result<ParameterFieldView, String>,
}

fn spawn_load_with<F>(request: u64, load: F) -> Receiver<LoadCompletion>
where
    F: FnOnce() -> Result<ParameterFieldView, String> + Send + 'static,
{
    let (sender, receiver) = sync_channel(1);
    std::thread::Builder::new()
        .name("parameter-field-view".into())
        .spawn(move || {
            let result = load();
            let _ = sender.send(LoadCompletion {
                request,
                worker_thread: std::thread::current().id(),
                result,
            });
        })
        .expect("parameter-field view worker spawn");
    receiver
}

pub(crate) fn spawn_parameter_field_load(
    request: u64,
    path: PathBuf,
    repaint: egui::Context,
) -> Receiver<LoadCompletion> {
    spawn_load_with(request, move || {
        let result = load_parameter_field(&path);
        repaint.request_repaint();
        result
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Completion {
    Installed,
    Failed,
    Stale,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ParameterFieldLoadState {
    generation: u64,
    active: Option<u64>,
    pub(crate) status: String,
    pub(crate) installed: Option<ParameterFieldView>,
}

impl ParameterFieldLoadState {
    pub(crate) fn begin_request(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.active = Some(self.generation);
        self.status =
            "Reading, decompressing and replay-verifying parameter-field evidence on worker…"
                .into();
        self.generation
    }

    pub(crate) fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.active = None;
        self.status = "Parameter-field load cancelled; late completion is stale".into();
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.active.is_some()
    }

    pub(crate) fn complete(
        &mut self,
        request: u64,
        result: Result<ParameterFieldView, String>,
    ) -> Completion {
        if self.active != Some(request) {
            return Completion::Stale;
        }
        self.active = None;
        match result {
            Ok(view) => {
                self.status = format!(
                    "Verified field {} · N={} · {} displayed points",
                    short_id(&view.artifact_id),
                    view.evaluations_n,
                    view.points.len()
                );
                self.installed = Some(view);
                Completion::Installed
            }
            Err(error) => {
                self.status = format!("Error: {error}");
                Completion::Failed
            }
        }
    }
}

pub(crate) fn render_parameter_field(ui: &mut egui::Ui, view: &ParameterFieldView) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("Verified parameter field").strong());
        ui.monospace(short_id(&view.artifact_id));
        ui.label(format!("N={}", view.evaluations_n));
        ui.label(format!("metric {}", view.metric_id));
    });
    ui.small(format!(
        "dataset {} · manifest {} · config {} · bars [{}, {})",
        short_id(&view.source_dataset_id),
        short_id(&view.source_manifest_id),
        short_id(&view.config_id),
        view.range_start,
        view.range_end
    ));
    ui.small(format!(
        "SPP field estimate {:.6} · selected point {:.6} · optimization-profile stability {:.2}%",
        view.spp_estimate,
        view.selected_value,
        f64::from(view.stability_bps) / 100.0
    ));
    if view.omitted_axes > 0 || view.omitted_points > 0 {
        ui.colored_label(
            egui::Color32::from_rgb(255, 200, 50),
            format!(
                "Display caps omitted {} parameter axes and {} sealed points; retained points remain in sealed order",
                view.omitted_axes, view.omitted_points
            ),
        );
    }
    ui.add_space(4.0);
    render_surface(ui, view);
    ui.add_space(6.0);
    render_parallel_coordinates(ui, view);
}

fn render_surface(ui: &mut egui::Ui, view: &ParameterFieldView) {
    ui.label(egui::RichText::new("3D parameter surface").strong());
    if view.axes.len() < 2 {
        ui.small("undefined: a 3D surface requires at least two sealed parameter axes");
        return;
    }
    if view.metric_range.is_none() {
        ui.small("undefined metric height: every displayed sealed point has the same metric value");
    }
    if view.omitted_surface_segments > 0 {
        ui.small(format!(
            "{} surface segments omitted (segment cap {MAX_SURFACE_SEGMENTS})",
            view.omitted_surface_segments
        ));
    }
    let width = ui.available_width().clamp(240.0, 760.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 260.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(10, 13, 22));
    let projected: Vec<Option<egui::Pos2>> = view
        .points
        .iter()
        .map(|point| {
            let x = point.parameter_coordinates.first().copied().flatten()?;
            let y = point.parameter_coordinates.get(1).copied().flatten()?;
            let z = point.metric_coordinate?;
            Some(surface_position(rect, x, y, z))
        })
        .collect();
    for [from, to] in &view.surface_segments {
        if let (Some(Some(from)), Some(Some(to))) = (projected.get(*from), projected.get(*to)) {
            painter.line_segment(
                [*from, *to],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(72, 145, 190)),
            );
        }
    }
    for (index, position) in projected.into_iter().enumerate() {
        let Some(position) = position else { continue };
        let point = &view.points[index];
        let color = if point.rank == 1 { UP } else { ACCENT };
        painter.circle_filled(position, if point.rank == 1 { 4.0 } else { 2.5 }, color);
    }
    painter.text(
        rect.left_bottom() + egui::vec2(8.0, -8.0),
        egui::Align2::LEFT_BOTTOM,
        &view.axes[0].id,
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(150),
    );
    painter.text(
        rect.right_bottom() + egui::vec2(-8.0, -8.0),
        egui::Align2::RIGHT_BOTTOM,
        &view.axes[1].id,
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(150),
    );
    painter.text(
        rect.left_top() + egui::vec2(8.0, 8.0),
        egui::Align2::LEFT_TOP,
        &view.metric_id,
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(150),
    );
}

fn surface_position(rect: egui::Rect, x: f32, y: f32, z: f32) -> egui::Pos2 {
    let centre_x = rect.center().x;
    let base_y = rect.bottom() - 30.0;
    egui::pos2(
        centre_x + (x - y) * rect.width() * 0.36,
        base_y - (x + y) * rect.height() * 0.18 - z * rect.height() * 0.52,
    )
}

fn render_parallel_coordinates(ui: &mut egui::Ui, view: &ParameterFieldView) {
    ui.label(egui::RichText::new("Parallel coordinates").strong());
    if view.axes.len() <= 2 {
        ui.small("available for sealed fields with more than two parameter axes");
        return;
    }
    let axis_count = view.axes.len() + 1;
    let width = ui.available_width().clamp(260.0, 900.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 250.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(10, 13, 22));
    let inner = rect.shrink2(egui::vec2(24.0, 28.0));
    let x_for = |axis: usize| {
        inner.left() + axis as f32 * inner.width() / (axis_count.saturating_sub(1)) as f32
    };
    for axis in 0..axis_count {
        let x = x_for(axis);
        painter.line_segment(
            [egui::pos2(x, inner.top()), egui::pos2(x, inner.bottom())],
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
        );
        let label = if axis < view.axes.len() {
            &view.axes[axis].id
        } else {
            &view.metric_id
        };
        painter.text(
            egui::pos2(x, rect.bottom() - 8.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            egui::FontId::monospace(9.0),
            egui::Color32::from_gray(150),
        );
    }
    for point in &view.points {
        let Some(metric) = point.metric_coordinate else {
            continue;
        };
        let mut positions = Vec::with_capacity(axis_count);
        for (axis, coordinate) in point.parameter_coordinates.iter().enumerate() {
            let Some(coordinate) = coordinate else {
                continue;
            };
            positions.push(egui::pos2(
                x_for(axis),
                inner.bottom() - coordinate * inner.height(),
            ));
        }
        positions.push(egui::pos2(
            x_for(axis_count - 1),
            inner.bottom() - metric * inner.height(),
        ));
        if positions.len() == axis_count {
            let color = if point.rank == 1 {
                egui::Color32::from_rgb(255, 200, 50)
            } else {
                egui::Color32::from_rgba_unmultiplied(70, 165, 210, 70)
            };
            painter.add(egui::Shape::line(positions, egui::Stroke::new(1.0, color)));
        }
    }
    if view.undefined_metric_points > 0 {
        painter.text(
            rect.left_top() + egui::vec2(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            format!(
                "{} points undefined on constant metric scale",
                view.undefined_metric_points
            ),
            egui::FontId::monospace(9.0),
            egui::Color32::from_rgb(255, 200, 50),
        );
    }
}

fn short_id(identity: &str) -> &str {
    identity.get(..8).unwrap_or(identity)
}

#[cfg(test)]
mod tests;
