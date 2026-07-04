//! Universal drawing interaction layer — one exhaustive implementation of
//! hit-testing, anchor (control-point) enumeration/editing, whole-drawing
//! translation, and placement previews for **every** `Drawing` variant.
//!
//! History: selection, drag, resize and eraser each carried their own partial
//! per-variant `match` with a `_ =>` fallback, so most of the 80 tools were
//! silently unselectable/undraggable, and each site re-derived its own
//! screen mapping that disagreed with the painted pixels (no log scale, no
//! free-look camera, a padding bug in the control-point centre). Everything
//! here is pure, screen-mapping comes exclusively from [`PriceViewGeometry`]
//! (the exact geometry the frame painted), and the matches are exhaustive —
//! adding a `Drawing` variant now fails compilation until interaction is
//! defined for it.

use crate::drawing::{DrawMode, Drawing};
use crate::render::PriceViewGeometry;

/// One grabbable anchor of a drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnchorPos {
    /// Anchored to a (bar, price) data point.
    Data(usize, f64),
    /// Horizontal element — price only (handle renders at a fixed x).
    PriceOnly(f64),
    /// Vertical element — bar only (handle renders at vertical center).
    BarOnly(usize),
}

impl AnchorPos {
    /// Screen position; price-only anchors sit near the right edge, bar-only
    /// anchors at the vertical center of the pane.
    pub fn to_screen(&self, g: &PriceViewGeometry) -> egui::Pos2 {
        match self {
            AnchorPos::Data(bar, price) => egui::pos2(g.bar_to_x(*bar), g.price_to_y(*price)),
            AnchorPos::PriceOnly(price) => {
                egui::pos2(g.chart_rect.right() - 40.0, g.price_to_y(*price))
            }
            AnchorPos::BarOnly(bar) => egui::pos2(g.bar_to_x(*bar), g.chart_rect.center().y),
        }
    }
}

/// Slope-handle horizontal offset (bars) for slope/scale tools (Ray, GannFan).
const SLOPE_HANDLE_BARS: usize = 20;

fn mid(p1: (usize, f64), p2: (usize, f64)) -> (usize, f64) {
    ((p1.0 + p2.0) / 2, (p1.1 + p2.1) * 0.5)
}

/// Every grabbable anchor of `d`, in a stable order that
/// [`drawing_set_anchor`] understands.
pub fn drawing_anchors(d: &Drawing) -> Vec<AnchorPos> {
    use AnchorPos::*;
    match d {
        Drawing::HLine { price, .. } | Drawing::MagnetLevel { price, .. } => vec![PriceOnly(*price)],
        Drawing::PriceNote { price, .. } => vec![PriceOnly(*price)],
        Drawing::VLine { bar_idx, .. }
        | Drawing::SessionBreak { bar_idx, .. }
        | Drawing::FibTimeZones { bar_idx, .. }
        | Drawing::AnchoredVwapLine { bar_idx, .. } => vec![BarOnly(*bar_idx)],
        Drawing::CyclicLines { bar_start, bar_end, .. }
        | Drawing::TimeCycle { bar_start, bar_end, .. } => {
            vec![BarOnly(*bar_start), BarOnly(*bar_end)]
        }
        Drawing::TrendLine { p1, p2, .. }
        | Drawing::ExtendedLine { p1, p2, .. }
        | Drawing::ArrowLine { p1, p2, .. }
        | Drawing::InfoLine { p1, p2, .. }
        | Drawing::TrendAngle { p1, p2, .. }
        | Drawing::Rectangle { p1, p2, .. }
        | Drawing::Highlighter { p1, p2, .. }
        | Drawing::Ellipse { p1, p2, .. }
        | Drawing::Ruler { p1, p2, .. }
        | Drawing::MeasureTool { p1, p2, .. }
        | Drawing::Forecast { p1, p2, .. }
        | Drawing::GhostFeed { p1, p2, .. }
        | Drawing::PriceRange { p1, p2 }
        | Drawing::DateRange { p1, p2 }
        | Drawing::DatePriceRange { p1, p2 }
        | Drawing::RegressionChannel { p1, p2, .. }
        | Drawing::GannBox { p1, p2, .. }
        | Drawing::SineWave { p1, p2, .. }
        | Drawing::Circle { p1, p2, .. }
        | Drawing::PitchFan { p1, p2, .. }
        | Drawing::TrendFibTime { p1, p2, .. }
        | Drawing::GannSquare { p1, p2, .. }
        | Drawing::GannSquareFixed { p1, p2, .. }
        | Drawing::BarsPattern { p1, p2, .. }
        | Drawing::Projection { p1, p2, .. }
        | Drawing::DoubleCurve { p1, p2, .. } => vec![Data(p1.0, p1.1), Data(p2.0, p2.1)],
        Drawing::FiboRetrace {
            high,
            low,
            bar_start,
            bar_end,
        } => vec![Data(*bar_start, *high), Data(*bar_end, *low)],
        Drawing::Ray { origin, slope, .. } => {
            let handle_bar = origin.0 + SLOPE_HANDLE_BARS;
            vec![
                Data(origin.0, origin.1),
                Data(handle_bar, origin.1 + slope * SLOPE_HANDLE_BARS as f64),
            ]
        }
        Drawing::GannFan { origin, scale, .. } => {
            let handle_bar = origin.0 + SLOPE_HANDLE_BARS;
            vec![
                Data(origin.0, origin.1),
                Data(handle_bar, origin.1 + scale * SLOPE_HANDLE_BARS as f64),
            ]
        }
        Drawing::Channel { p1, p2, width, .. } => {
            let m = mid(*p1, *p2);
            vec![Data(p1.0, p1.1), Data(p2.0, p2.1), Data(m.0, m.1 + width)]
        }
        Drawing::ParallelChannel { p1, p2, offset, .. } => {
            let m = mid(*p1, *p2);
            vec![Data(p1.0, p1.1), Data(p2.0, p2.1), Data(m.0, m.1 + offset)]
        }
        Drawing::HRay { bar_idx, price, .. } | Drawing::CrossLine { bar_idx, price, .. } => {
            vec![Data(*bar_idx, *price)]
        }
        Drawing::TextLabel { bar_idx, price, .. }
        | Drawing::ArrowMarker { bar_idx, price, .. }
        | Drawing::CrossMarker { bar_idx, price, .. }
        | Drawing::PriceLabel { bar_idx, price, .. }
        | Drawing::AnchorNote { bar_idx, price, .. }
        | Drawing::Emoji { bar_idx, price, .. }
        | Drawing::Flag { bar_idx, price, .. }
        | Drawing::Signpost { bar_idx, price, .. }
        | Drawing::AnchoredText { bar_idx, price, .. }
        | Drawing::Comment { bar_idx, price, .. }
        | Drawing::ArrowMarkerLeft { bar_idx, price, .. }
        | Drawing::ArrowMarkerRight { bar_idx, price, .. } => vec![Data(*bar_idx, *price)],
        Drawing::Callout { anchor, label_pos, .. } | Drawing::Balloon { anchor, label_pos, .. } => {
            vec![Data(anchor.0, anchor.1), Data(label_pos.0, label_pos.1)]
        }
        Drawing::Pitchfork { pivot, p2, p3, .. }
        | Drawing::SchiffPitchfork { pivot, p2, p3, .. }
        | Drawing::ModSchiffPitchfork { pivot, p2, p3, .. }
        | Drawing::InsidePitchfork { pivot, p2, p3, .. } => {
            vec![Data(pivot.0, pivot.1), Data(p2.0, p2.1), Data(p3.0, p3.1)]
        }
        Drawing::FiboExtension { p1, p2, p3, .. }
        | Drawing::FibChannel { p1, p2, p3, .. }
        | Drawing::Triangle { p1, p2, p3, .. }
        | Drawing::TrendChannel { p1, p2, p3, .. }
        | Drawing::FibWedge { p1, p2, p3, .. }
        | Drawing::ArcDraw { p1, p2, p3, .. }
        | Drawing::SpeedResistanceFan { p1, p2, p3, .. }
        | Drawing::SpeedResistanceArc { p1, p2, p3, .. }
        | Drawing::RotatedRectangle { p1, p2, p3, .. } => {
            vec![Data(p1.0, p1.1), Data(p2.0, p2.1), Data(p3.0, p3.1)]
        }
        Drawing::CurveDraw {
            p1,
            ctrl1,
            ctrl2,
            p2,
            ..
        } => vec![
            Data(p1.0, p1.1),
            Data(ctrl1.0, ctrl1.1),
            Data(ctrl2.0, ctrl2.1),
            Data(p2.0, p2.1),
        ],
        Drawing::LongPosition {
            entry,
            stop,
            target,
        }
        | Drawing::ShortPosition {
            entry,
            stop,
            target,
        }
        | Drawing::RiskRewardBox {
            entry,
            stop,
            target,
        } => vec![
            Data(entry.0, entry.1),
            Data(entry.0, *stop),
            Data(entry.0, *target),
        ],
        Drawing::FibCircle { center, radius_pt, .. } | Drawing::FibSpiral { center, radius_pt, .. } => {
            vec![Data(center.0, center.1), Data(radius_pt.0, radius_pt.1)]
        }
        Drawing::Polyline { points, .. }
        | Drawing::ElliottWave { points, .. }
        | Drawing::AbcCorrection { points, .. }
        | Drawing::HeadShoulders { points, .. }
        | Drawing::XabcdPattern { points, .. }
        | Drawing::Brush { points, .. }
        | Drawing::PathDraw { points, .. }
        | Drawing::TrianglePattern { points, .. }
        | Drawing::ThreeDrives { points, .. }
        | Drawing::ElliottDouble { points, .. }
        | Drawing::AbcdPattern { points, .. }
        | Drawing::CypherPattern { points, .. }
        | Drawing::ElliottTriangle { points, .. }
        | Drawing::ElliottTripleCombo { points, .. } => points
            .iter()
            .map(|(b, p)| AnchorPos::Data(*b, *p))
            .collect(),
    }
}

/// Move anchor `idx` of `d` to (`bar`, `price`). Anchor order matches
/// [`drawing_anchors`]. Out-of-range indices are ignored.
pub fn drawing_set_anchor(d: &mut Drawing, idx: usize, bar: usize, price: f64, max_bar: usize) {
    let bar = bar.min(max_bar);
    let set = |pt: &mut (usize, f64)| {
        pt.0 = bar;
        pt.1 = price;
    };
    match d {
        Drawing::HLine { price: p, .. }
        | Drawing::MagnetLevel { price: p, .. }
        | Drawing::PriceNote { price: p, .. } => *p = price,
        Drawing::VLine { bar_idx, .. }
        | Drawing::SessionBreak { bar_idx, .. }
        | Drawing::FibTimeZones { bar_idx, .. }
        | Drawing::AnchoredVwapLine { bar_idx, .. } => *bar_idx = bar,
        Drawing::CyclicLines { bar_start, bar_end, .. }
        | Drawing::TimeCycle { bar_start, bar_end, .. } => {
            if idx == 0 {
                *bar_start = bar;
            } else {
                *bar_end = bar;
            }
        }
        Drawing::TrendLine { p1, p2, .. }
        | Drawing::ExtendedLine { p1, p2, .. }
        | Drawing::ArrowLine { p1, p2, .. }
        | Drawing::InfoLine { p1, p2, .. }
        | Drawing::TrendAngle { p1, p2, .. }
        | Drawing::Rectangle { p1, p2, .. }
        | Drawing::Highlighter { p1, p2, .. }
        | Drawing::Ellipse { p1, p2, .. }
        | Drawing::Ruler { p1, p2, .. }
        | Drawing::MeasureTool { p1, p2, .. }
        | Drawing::Forecast { p1, p2, .. }
        | Drawing::GhostFeed { p1, p2, .. }
        | Drawing::PriceRange { p1, p2 }
        | Drawing::DateRange { p1, p2 }
        | Drawing::DatePriceRange { p1, p2 }
        | Drawing::RegressionChannel { p1, p2, .. }
        | Drawing::GannBox { p1, p2, .. }
        | Drawing::SineWave { p1, p2, .. }
        | Drawing::Circle { p1, p2, .. }
        | Drawing::PitchFan { p1, p2, .. }
        | Drawing::TrendFibTime { p1, p2, .. }
        | Drawing::GannSquare { p1, p2, .. }
        | Drawing::GannSquareFixed { p1, p2, .. }
        | Drawing::BarsPattern { p1, p2, .. }
        | Drawing::Projection { p1, p2, .. }
        | Drawing::DoubleCurve { p1, p2, .. } => {
            if idx == 0 {
                set(p1);
            } else {
                set(p2);
            }
        }
        Drawing::FiboRetrace {
            high,
            low,
            bar_start,
            bar_end,
        } => {
            if idx == 0 {
                *bar_start = bar;
                *high = price;
            } else {
                *bar_end = bar;
                *low = price;
            }
        }
        Drawing::Ray { origin, slope, .. } => {
            if idx == 0 {
                origin.0 = bar;
                origin.1 = price;
            } else {
                let db = bar as i64 - origin.0 as i64;
                if db != 0 {
                    *slope = (price - origin.1) / db as f64;
                }
            }
        }
        Drawing::GannFan { origin, scale, .. } => {
            if idx == 0 {
                origin.0 = bar;
                origin.1 = price;
            } else {
                let db = bar as i64 - origin.0 as i64;
                if db != 0 {
                    *scale = (price - origin.1) / db as f64;
                }
            }
        }
        Drawing::Channel { p1, p2, width, .. } => match idx {
            0 => set(p1),
            1 => set(p2),
            _ => *width = price - (p1.1 + p2.1) * 0.5,
        },
        Drawing::ParallelChannel { p1, p2, offset, .. } => match idx {
            0 => set(p1),
            1 => set(p2),
            _ => *offset = price - (p1.1 + p2.1) * 0.5,
        },
        Drawing::HRay { bar_idx, price: p, .. } | Drawing::CrossLine { bar_idx, price: p, .. } => {
            *bar_idx = bar;
            *p = price;
        }
        Drawing::TextLabel { bar_idx, price: p, .. }
        | Drawing::ArrowMarker { bar_idx, price: p, .. }
        | Drawing::CrossMarker { bar_idx, price: p, .. }
        | Drawing::PriceLabel { bar_idx, price: p, .. }
        | Drawing::AnchorNote { bar_idx, price: p, .. }
        | Drawing::Emoji { bar_idx, price: p, .. }
        | Drawing::Flag { bar_idx, price: p, .. }
        | Drawing::Signpost { bar_idx, price: p, .. }
        | Drawing::AnchoredText { bar_idx, price: p, .. }
        | Drawing::Comment { bar_idx, price: p, .. }
        | Drawing::ArrowMarkerLeft { bar_idx, price: p, .. }
        | Drawing::ArrowMarkerRight { bar_idx, price: p, .. } => {
            *bar_idx = bar;
            *p = price;
        }
        Drawing::Callout { anchor, label_pos, .. } | Drawing::Balloon { anchor, label_pos, .. } => {
            if idx == 0 {
                set(anchor);
            } else {
                set(label_pos);
            }
        }
        Drawing::Pitchfork { pivot, p2, p3, .. }
        | Drawing::SchiffPitchfork { pivot, p2, p3, .. }
        | Drawing::ModSchiffPitchfork { pivot, p2, p3, .. }
        | Drawing::InsidePitchfork { pivot, p2, p3, .. } => match idx {
            0 => set(pivot),
            1 => set(p2),
            _ => set(p3),
        },
        Drawing::FiboExtension { p1, p2, p3, .. }
        | Drawing::FibChannel { p1, p2, p3, .. }
        | Drawing::Triangle { p1, p2, p3, .. }
        | Drawing::TrendChannel { p1, p2, p3, .. }
        | Drawing::FibWedge { p1, p2, p3, .. }
        | Drawing::ArcDraw { p1, p2, p3, .. }
        | Drawing::SpeedResistanceFan { p1, p2, p3, .. }
        | Drawing::SpeedResistanceArc { p1, p2, p3, .. }
        | Drawing::RotatedRectangle { p1, p2, p3, .. } => match idx {
            0 => set(p1),
            1 => set(p2),
            _ => set(p3),
        },
        Drawing::CurveDraw {
            p1,
            ctrl1,
            ctrl2,
            p2,
            ..
        } => match idx {
            0 => set(p1),
            1 => set(ctrl1),
            2 => set(ctrl2),
            _ => set(p2),
        },
        Drawing::LongPosition {
            entry,
            stop,
            target,
        }
        | Drawing::ShortPosition {
            entry,
            stop,
            target,
        }
        | Drawing::RiskRewardBox {
            entry,
            stop,
            target,
        } => match idx {
            0 => {
                entry.0 = bar;
                entry.1 = price;
            }
            1 => *stop = price,
            _ => *target = price,
        },
        Drawing::FibCircle { center, radius_pt, .. }
        | Drawing::FibSpiral { center, radius_pt, .. } => {
            if idx == 0 {
                set(center);
            } else {
                set(radius_pt);
            }
        }
        Drawing::Polyline { points, .. }
        | Drawing::ElliottWave { points, .. }
        | Drawing::AbcCorrection { points, .. }
        | Drawing::HeadShoulders { points, .. }
        | Drawing::XabcdPattern { points, .. }
        | Drawing::Brush { points, .. }
        | Drawing::PathDraw { points, .. }
        | Drawing::TrianglePattern { points, .. }
        | Drawing::ThreeDrives { points, .. }
        | Drawing::ElliottDouble { points, .. }
        | Drawing::AbcdPattern { points, .. }
        | Drawing::CypherPattern { points, .. }
        | Drawing::ElliottTriangle { points, .. }
        | Drawing::ElliottTripleCombo { points, .. } => {
            if let Some(pt) = points.get_mut(idx) {
                set(pt);
            }
        }
    }
}

/// Translate an entire drawing by (`bar_delta`, `price_delta`) — every
/// variant, no fallback arm.
pub fn translate_drawing(d: &mut Drawing, bar_delta: i64, price_delta: f64, max_bar: usize) {
    let mb = |idx: &mut usize| {
        *idx = (*idx as i64 + bar_delta).clamp(0, max_bar as i64) as usize;
    };
    let mv = |pt: &mut (usize, f64)| {
        let mut b = pt.0;
        mb(&mut b);
        pt.0 = b;
        pt.1 += price_delta;
    };
    match d {
        Drawing::HLine { price, .. }
        | Drawing::MagnetLevel { price, .. }
        | Drawing::PriceNote { price, .. } => *price += price_delta,
        Drawing::VLine { bar_idx, .. }
        | Drawing::SessionBreak { bar_idx, .. }
        | Drawing::FibTimeZones { bar_idx, .. }
        | Drawing::AnchoredVwapLine { bar_idx, .. } => mb(bar_idx),
        Drawing::CyclicLines { bar_start, bar_end, .. }
        | Drawing::TimeCycle { bar_start, bar_end, .. } => {
            mb(bar_start);
            mb(bar_end);
        }
        Drawing::TrendLine { p1, p2, .. }
        | Drawing::ExtendedLine { p1, p2, .. }
        | Drawing::ArrowLine { p1, p2, .. }
        | Drawing::InfoLine { p1, p2, .. }
        | Drawing::TrendAngle { p1, p2, .. }
        | Drawing::Rectangle { p1, p2, .. }
        | Drawing::Highlighter { p1, p2, .. }
        | Drawing::Ellipse { p1, p2, .. }
        | Drawing::Ruler { p1, p2, .. }
        | Drawing::MeasureTool { p1, p2, .. }
        | Drawing::Forecast { p1, p2, .. }
        | Drawing::GhostFeed { p1, p2, .. }
        | Drawing::PriceRange { p1, p2 }
        | Drawing::DateRange { p1, p2 }
        | Drawing::DatePriceRange { p1, p2 }
        | Drawing::RegressionChannel { p1, p2, .. }
        | Drawing::GannBox { p1, p2, .. }
        | Drawing::SineWave { p1, p2, .. }
        | Drawing::Circle { p1, p2, .. }
        | Drawing::PitchFan { p1, p2, .. }
        | Drawing::TrendFibTime { p1, p2, .. }
        | Drawing::GannSquare { p1, p2, .. }
        | Drawing::GannSquareFixed { p1, p2, .. }
        | Drawing::BarsPattern { p1, p2, .. }
        | Drawing::Projection { p1, p2, .. }
        | Drawing::DoubleCurve { p1, p2, .. }
        | Drawing::Channel { p1, p2, .. }
        | Drawing::ParallelChannel { p1, p2, .. } => {
            mv(p1);
            mv(p2);
        }
        Drawing::FiboRetrace {
            high,
            low,
            bar_start,
            bar_end,
        } => {
            *high += price_delta;
            *low += price_delta;
            mb(bar_start);
            mb(bar_end);
        }
        Drawing::Ray { origin, .. } => mv(origin),
        Drawing::GannFan { origin, .. } => mv(origin),
        Drawing::HRay { bar_idx, price, .. }
        | Drawing::CrossLine { bar_idx, price, .. }
        | Drawing::TextLabel { bar_idx, price, .. }
        | Drawing::ArrowMarker { bar_idx, price, .. }
        | Drawing::CrossMarker { bar_idx, price, .. }
        | Drawing::PriceLabel { bar_idx, price, .. }
        | Drawing::AnchorNote { bar_idx, price, .. }
        | Drawing::Emoji { bar_idx, price, .. }
        | Drawing::Flag { bar_idx, price, .. }
        | Drawing::Signpost { bar_idx, price, .. }
        | Drawing::AnchoredText { bar_idx, price, .. }
        | Drawing::Comment { bar_idx, price, .. }
        | Drawing::ArrowMarkerLeft { bar_idx, price, .. }
        | Drawing::ArrowMarkerRight { bar_idx, price, .. } => {
            mb(bar_idx);
            *price += price_delta;
        }
        Drawing::Callout { anchor, label_pos, .. } | Drawing::Balloon { anchor, label_pos, .. } => {
            mv(anchor);
            mv(label_pos);
        }
        Drawing::Pitchfork { pivot, p2, p3, .. }
        | Drawing::SchiffPitchfork { pivot, p2, p3, .. }
        | Drawing::ModSchiffPitchfork { pivot, p2, p3, .. }
        | Drawing::InsidePitchfork { pivot, p2, p3, .. } => {
            mv(pivot);
            mv(p2);
            mv(p3);
        }
        Drawing::FiboExtension { p1, p2, p3, .. }
        | Drawing::FibChannel { p1, p2, p3, .. }
        | Drawing::Triangle { p1, p2, p3, .. }
        | Drawing::TrendChannel { p1, p2, p3, .. }
        | Drawing::FibWedge { p1, p2, p3, .. }
        | Drawing::ArcDraw { p1, p2, p3, .. }
        | Drawing::SpeedResistanceFan { p1, p2, p3, .. }
        | Drawing::SpeedResistanceArc { p1, p2, p3, .. }
        | Drawing::RotatedRectangle { p1, p2, p3, .. } => {
            mv(p1);
            mv(p2);
            mv(p3);
        }
        Drawing::CurveDraw {
            p1,
            ctrl1,
            ctrl2,
            p2,
            ..
        } => {
            mv(p1);
            mv(ctrl1);
            mv(ctrl2);
            mv(p2);
        }
        Drawing::LongPosition {
            entry,
            stop,
            target,
        }
        | Drawing::ShortPosition {
            entry,
            stop,
            target,
        }
        | Drawing::RiskRewardBox {
            entry,
            stop,
            target,
        } => {
            mv(entry);
            *stop += price_delta;
            *target += price_delta;
        }
        Drawing::FibCircle { center, radius_pt, .. }
        | Drawing::FibSpiral { center, radius_pt, .. } => {
            mv(center);
            mv(radius_pt);
        }
        Drawing::Polyline { points, .. }
        | Drawing::ElliottWave { points, .. }
        | Drawing::AbcCorrection { points, .. }
        | Drawing::HeadShoulders { points, .. }
        | Drawing::XabcdPattern { points, .. }
        | Drawing::Brush { points, .. }
        | Drawing::PathDraw { points, .. }
        | Drawing::TrianglePattern { points, .. }
        | Drawing::ThreeDrives { points, .. }
        | Drawing::ElliottDouble { points, .. }
        | Drawing::AbcdPattern { points, .. }
        | Drawing::CypherPattern { points, .. }
        | Drawing::ElliottTriangle { points, .. }
        | Drawing::ElliottTripleCombo { points, .. } => {
            for pt in points.iter_mut() {
                mv(pt);
            }
        }
    }
}

fn seg_dist(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = egui::vec2(b.x - a.x, b.y - a.y);
    let ap = egui::vec2(p.x - a.x, p.y - a.y);
    let len_sq = ab.x * ab.x + ab.y * ab.y;
    if len_sq < 0.001 {
        return (ap.x * ap.x + ap.y * ap.y).sqrt();
    }
    let t = ((ap.x * ab.x + ap.y * ab.y) / len_sq).clamp(0.0, 1.0);
    let proj = egui::pos2(a.x + t * ab.x, a.y + t * ab.y);
    ((p.x - proj.x).powi(2) + (p.y - proj.y).powi(2)).sqrt()
}

fn poly_dist(p: egui::Pos2, pts: &[egui::Pos2]) -> f32 {
    if pts.len() == 1 {
        return ((p.x - pts[0].x).powi(2) + (p.y - pts[0].y).powi(2)).sqrt();
    }
    pts.windows(2)
        .map(|w| seg_dist(p, w[0], w[1]))
        .fold(f32::MAX, f32::min)
}

fn rect_dist(p: egui::Pos2, r: egui::Rect) -> f32 {
    if r.contains(p) {
        return 0.0;
    }
    let dx = (p.x - r.center().x).abs() - r.width() * 0.5;
    let dy = (p.y - r.center().y).abs() - r.height() * 0.5;
    dx.max(0.0).hypot(dy.max(0.0))
}

/// Extend the infinite line through (a, b) to the clip rect's x-span.
fn line_across_rect(a: egui::Pos2, b: egui::Pos2, rect: egui::Rect) -> (egui::Pos2, egui::Pos2) {
    let dx = b.x - a.x;
    if dx.abs() < 0.001 {
        return (
            egui::pos2(a.x, rect.top()),
            egui::pos2(a.x, rect.bottom()),
        );
    }
    let m = (b.y - a.y) / dx;
    let y_at = |x: f32| a.y + m * (x - a.x);
    (
        egui::pos2(rect.left(), y_at(rect.left())),
        egui::pos2(rect.right(), y_at(rect.right())),
    )
}

fn sample_quad(p0: egui::Pos2, c: egui::Pos2, p1: egui::Pos2, n: usize) -> Vec<egui::Pos2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            egui::pos2(
                u * u * p0.x + 2.0 * u * t * c.x + t * t * p1.x,
                u * u * p0.y + 2.0 * u * t * c.y + t * t * p1.y,
            )
        })
        .collect()
}

fn sample_cubic(
    p0: egui::Pos2,
    c1: egui::Pos2,
    c2: egui::Pos2,
    p1: egui::Pos2,
    n: usize,
) -> Vec<egui::Pos2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            egui::pos2(
                u.powi(3) * p0.x
                    + 3.0 * u * u * t * c1.x
                    + 3.0 * u * t * t * c2.x
                    + t.powi(3) * p1.x,
                u.powi(3) * p0.y
                    + 3.0 * u * u * t * c1.y
                    + 3.0 * u * t * t * c2.y
                    + t.powi(3) * p1.y,
            )
        })
        .collect()
}

/// Fibonacci counting offsets used by FibTimeZones.
const FIB_ZONE_OFFSETS: [usize; 9] = [0, 1, 2, 3, 5, 8, 13, 21, 34];

/// Screen-space distance from `pos` to the nearest interactive part of `d`,
/// using the frame's painted geometry. Exhaustive — every variant is
/// hit-testable, so every variant can be selected, dragged, and erased.
pub fn drawing_hit_distance(d: &Drawing, pos: egui::Pos2, g: &PriceViewGeometry) -> f32 {
    let rect = g.chart_rect;
    let sp = |bar: usize, price: f64| egui::pos2(g.bar_to_x(bar), g.price_to_y(price));
    let pt2 = |p: &(usize, f64)| sp(p.0, p.1);
    match d {
        Drawing::HLine { price, .. }
        | Drawing::MagnetLevel { price, .. }
        | Drawing::PriceNote { price, .. } => (pos.y - g.price_to_y(*price)).abs(),
        Drawing::VLine { bar_idx, .. }
        | Drawing::SessionBreak { bar_idx, .. }
        | Drawing::AnchoredVwapLine { bar_idx, .. } => (pos.x - g.bar_to_x(*bar_idx)).abs(),
        Drawing::FibTimeZones { bar_idx, .. } => FIB_ZONE_OFFSETS
            .iter()
            .map(|off| (pos.x - g.bar_to_x(bar_idx + off)).abs())
            .fold(f32::MAX, f32::min),
        Drawing::CyclicLines { bar_start, bar_end, .. }
        | Drawing::TimeCycle { bar_start, bar_end, .. } => {
            let interval = (*bar_end as i64 - *bar_start as i64).unsigned_abs().max(1) as usize;
            (0..8)
                .map(|k| (pos.x - g.bar_to_x(bar_start + k * interval)).abs())
                .fold(f32::MAX, f32::min)
        }
        Drawing::TrendLine { p1, p2, .. }
        | Drawing::ArrowLine { p1, p2, .. }
        | Drawing::InfoLine { p1, p2, .. }
        | Drawing::TrendAngle { p1, p2, .. }
        | Drawing::Ruler { p1, p2, .. }
        | Drawing::MeasureTool { p1, p2, .. }
        | Drawing::SineWave { p1, p2, .. }
        | Drawing::PitchFan { p1, p2, .. }
        | Drawing::TrendFibTime { p1, p2, .. }
        | Drawing::BarsPattern { p1, p2, .. }
        | Drawing::Projection { p1, p2, .. }
        | Drawing::DoubleCurve { p1, p2, .. } => seg_dist(pos, pt2(p1), pt2(p2)),
        Drawing::Forecast { p1, p2, .. } | Drawing::GhostFeed { p1, p2, .. } => {
            // Segment plus its forward projection to the right edge.
            let (a, b) = (pt2(p1), pt2(p2));
            let (_, far) = line_across_rect(a, b, rect);
            seg_dist(pos, a, b).min(seg_dist(pos, b, far))
        }
        Drawing::ExtendedLine { p1, p2, .. } => {
            let (a, b) = line_across_rect(pt2(p1), pt2(p2), rect);
            seg_dist(pos, a, b)
        }
        Drawing::Ray { origin, slope, .. } => {
            let a = pt2(origin);
            let bars_to_edge = ((rect.right() - a.x) / g.bar_w.max(0.001)) as f64;
            let b = egui::pos2(rect.right(), g.price_to_y(origin.1 + slope * bars_to_edge));
            seg_dist(pos, a, b)
        }
        Drawing::HRay { bar_idx, price, .. } => {
            let y = g.price_to_y(*price);
            let x0 = g.bar_to_x(*bar_idx).max(rect.left());
            seg_dist(pos, egui::pos2(x0, y), egui::pos2(rect.right(), y))
        }
        Drawing::CrossLine { bar_idx, price, .. } => {
            let x = g.bar_to_x(*bar_idx);
            let y = g.price_to_y(*price);
            (pos.x - x).abs().min((pos.y - y).abs())
        }
        Drawing::Rectangle { p1, p2, .. }
        | Drawing::Highlighter { p1, p2, .. }
        | Drawing::GannBox { p1, p2, .. }
        | Drawing::GannSquare { p1, p2, .. }
        | Drawing::GannSquareFixed { p1, p2, .. }
        | Drawing::DateRange { p1, p2 }
        | Drawing::DatePriceRange { p1, p2 }
        | Drawing::PriceRange { p1, p2 } => {
            rect_dist(pos, egui::Rect::from_two_pos(pt2(p1), pt2(p2)))
        }
        Drawing::FiboRetrace {
            high,
            low,
            bar_start,
            bar_end,
        } => {
            // Grab any of the fib level lines across the bar span.
            let x1 = g.bar_to_x(*bar_start);
            let x2 = g.bar_to_x(*bar_end);
            let (xl, xr) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
            [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0]
                .iter()
                .map(|r| {
                    let y = g.price_to_y(high - (high - low) * r);
                    seg_dist(pos, egui::pos2(xl, y), egui::pos2(xr, y))
                })
                .fold(f32::MAX, f32::min)
        }
        Drawing::Channel { p1, p2, width, .. } => {
            let a = pt2(p1);
            let b = pt2(p2);
            let a2 = sp(p1.0, p1.1 + width);
            let b2 = sp(p2.0, p2.1 + width);
            seg_dist(pos, a, b).min(seg_dist(pos, a2, b2))
        }
        Drawing::ParallelChannel { p1, p2, offset, .. } => {
            let up = seg_dist(pos, sp(p1.0, p1.1 + offset), sp(p2.0, p2.1 + offset));
            let dn = seg_dist(pos, sp(p1.0, p1.1 - offset), sp(p2.0, p2.1 - offset));
            seg_dist(pos, pt2(p1), pt2(p2)).min(up).min(dn)
        }
        Drawing::Ellipse { p1, p2, .. } => {
            let (a, b) = (pt2(p1), pt2(p2));
            let cx = (a.x + b.x) * 0.5;
            let cy = (a.y + b.y) * 0.5;
            let rx = ((a.x - b.x).abs() * 0.5).max(1.0);
            let ry = ((a.y - b.y).abs() * 0.5).max(1.0);
            let norm = ((pos.x - cx) / rx).powi(2) + ((pos.y - cy) / ry).powi(2);
            if norm <= 1.0 {
                0.0
            } else {
                (norm.sqrt() - 1.0) * rx.min(ry)
            }
        }
        Drawing::Circle { p1, p2, .. } => {
            let c = pt2(p1);
            let e = pt2(p2);
            let r = ((c.x - e.x).powi(2) + (c.y - e.y).powi(2)).sqrt();
            let dc = ((pos.x - c.x).powi(2) + (pos.y - c.y).powi(2)).sqrt();
            (dc - r).abs()
        }
        Drawing::FibCircle { center, radius_pt, .. } => {
            let c = pt2(center);
            let e = pt2(radius_pt);
            let r = ((c.x - e.x).powi(2) + (c.y - e.y).powi(2)).sqrt();
            let dc = ((pos.x - c.x).powi(2) + (pos.y - c.y).powi(2)).sqrt();
            [0.382, 0.5, 0.618, 1.0, 1.618]
                .iter()
                .map(|f| (dc - r * f).abs())
                .fold(f32::MAX, f32::min)
        }
        Drawing::FibSpiral { center, radius_pt, .. } => {
            let c = pt2(center);
            let e = pt2(radius_pt);
            let r = ((c.x - e.x).powi(2) + (c.y - e.y).powi(2)).sqrt();
            let dc = ((pos.x - c.x).powi(2) + (pos.y - c.y).powi(2)).sqrt();
            dc.min((dc - r).abs())
        }
        Drawing::GannFan { origin, scale, .. } => {
            let a = pt2(origin);
            let bars_to_edge = ((rect.right() - a.x) / g.bar_w.max(0.001)) as f64;
            [8.0, 4.0, 3.0, 2.0, 1.0, 0.5, 1.0 / 3.0, 0.25]
                .iter()
                .map(|m| {
                    let b = egui::pos2(
                        rect.right(),
                        g.price_to_y(origin.1 + scale * m * bars_to_edge),
                    );
                    seg_dist(pos, a, b)
                })
                .fold(f32::MAX, f32::min)
        }
        Drawing::Pitchfork { pivot, p2, p3, .. }
        | Drawing::SchiffPitchfork { pivot, p2, p3, .. }
        | Drawing::ModSchiffPitchfork { pivot, p2, p3, .. }
        | Drawing::InsidePitchfork { pivot, p2, p3, .. } => {
            let pv = pt2(pivot);
            let a = pt2(p2);
            let b = pt2(p3);
            let m = egui::pos2((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
            let (_, median_far) = line_across_rect(pv, m, rect);
            seg_dist(pos, pv, median_far)
                .min(seg_dist(pos, a, b))
                .min(seg_dist(pos, pv, a))
                .min(seg_dist(pos, pv, b))
        }
        Drawing::FiboExtension { p1, p2, p3, .. } => {
            poly_dist(pos, &[pt2(p1), pt2(p2), pt2(p3)])
        }
        Drawing::FibChannel { p1, p2, p3, .. } | Drawing::TrendChannel { p1, p2, p3, .. } => {
            let base = seg_dist(pos, pt2(p1), pt2(p2));
            let off_y = pt2(p3).y - pt2(p1).y;
            let a2 = egui::pos2(pt2(p1).x, pt2(p1).y + off_y);
            let b2 = egui::pos2(pt2(p2).x, pt2(p2).y + off_y);
            base.min(seg_dist(pos, a2, b2))
        }
        Drawing::Triangle { p1, p2, p3, .. } => {
            let (a, b, c) = (pt2(p1), pt2(p2), pt2(p3));
            seg_dist(pos, a, b)
                .min(seg_dist(pos, b, c))
                .min(seg_dist(pos, a, c))
        }
        Drawing::FibWedge { p1, p2, p3, .. } => {
            let (a, b, c) = (pt2(p1), pt2(p2), pt2(p3));
            seg_dist(pos, a, b).min(seg_dist(pos, a, c))
        }
        Drawing::SpeedResistanceFan { p1, p2, p3, .. }
        | Drawing::SpeedResistanceArc { p1, p2, p3, .. } => {
            poly_dist(pos, &[pt2(p1), pt2(p2), pt2(p3)])
        }
        Drawing::RotatedRectangle { p1, p2, p3, .. } => {
            // Baseline + the two offset edges of the rotated box.
            let (a, b, c) = (pt2(p1), pt2(p2), pt2(p3));
            let off = egui::vec2(c.x - b.x, c.y - b.y);
            let a2 = egui::pos2(a.x + off.x, a.y + off.y);
            let b2 = c;
            seg_dist(pos, a, b)
                .min(seg_dist(pos, a2, b2))
                .min(seg_dist(pos, a, a2))
                .min(seg_dist(pos, b, b2))
        }
        Drawing::ArcDraw { p1, p2, p3, .. } => {
            let samples = sample_quad(pt2(p1), pt2(p2), pt2(p3), 24);
            poly_dist(pos, &samples)
        }
        Drawing::CurveDraw {
            p1,
            ctrl1,
            ctrl2,
            p2,
            ..
        } => {
            let samples = sample_cubic(pt2(p1), pt2(ctrl1), pt2(ctrl2), pt2(p2), 32);
            poly_dist(pos, &samples)
        }
        Drawing::RegressionChannel { p1, p2, .. } => seg_dist(pos, pt2(p1), pt2(p2)),
        Drawing::LongPosition {
            entry,
            stop,
            target,
        }
        | Drawing::ShortPosition {
            entry,
            stop,
            target,
        }
        | Drawing::RiskRewardBox {
            entry,
            stop,
            target,
        } => {
            // The rendered box spans from the entry bar rightward; grab
            // anywhere inside it or on its three price lines.
            let x0 = g.bar_to_x(entry.0);
            let x1 = x0 + 24.0 * g.bar_w.max(1.0);
            let ys = [
                g.price_to_y(entry.1),
                g.price_to_y(*stop),
                g.price_to_y(*target),
            ];
            let top = ys.iter().fold(f32::MAX, |a, b| a.min(*b));
            let bot = ys.iter().fold(f32::MIN, |a, b| a.max(*b));
            rect_dist(
                pos,
                egui::Rect::from_min_max(egui::pos2(x0, top), egui::pos2(x1, bot)),
            )
        }
        Drawing::TextLabel { bar_idx, price, .. }
        | Drawing::AnchorNote { bar_idx, price, .. }
        | Drawing::AnchoredText { bar_idx, price, .. }
        | Drawing::Comment { bar_idx, price, .. }
        | Drawing::PriceLabel { bar_idx, price, .. } => {
            // Text-ish: generous grab box around the anchor.
            let c = sp(*bar_idx, *price);
            rect_dist(pos, egui::Rect::from_center_size(c, egui::vec2(64.0, 20.0)))
        }
        Drawing::ArrowMarker { bar_idx, price, .. }
        | Drawing::CrossMarker { bar_idx, price, .. }
        | Drawing::Emoji { bar_idx, price, .. }
        | Drawing::Flag { bar_idx, price, .. }
        | Drawing::Signpost { bar_idx, price, .. }
        | Drawing::ArrowMarkerLeft { bar_idx, price, .. }
        | Drawing::ArrowMarkerRight { bar_idx, price, .. } => {
            let c = sp(*bar_idx, *price);
            ((pos.x - c.x).powi(2) + (pos.y - c.y).powi(2)).sqrt()
        }
        Drawing::Callout { anchor, label_pos, .. } | Drawing::Balloon { anchor, label_pos, .. } => {
            let a = pt2(anchor);
            let l = pt2(label_pos);
            seg_dist(pos, a, l)
                .min(rect_dist(
                    pos,
                    egui::Rect::from_center_size(l, egui::vec2(80.0, 24.0)),
                ))
        }
        Drawing::Polyline { points, .. }
        | Drawing::ElliottWave { points, .. }
        | Drawing::AbcCorrection { points, .. }
        | Drawing::HeadShoulders { points, .. }
        | Drawing::XabcdPattern { points, .. }
        | Drawing::Brush { points, .. }
        | Drawing::PathDraw { points, .. }
        | Drawing::TrianglePattern { points, .. }
        | Drawing::ThreeDrives { points, .. }
        | Drawing::ElliottDouble { points, .. }
        | Drawing::AbcdPattern { points, .. }
        | Drawing::CypherPattern { points, .. }
        | Drawing::ElliottTriangle { points, .. }
        | Drawing::ElliottTripleCombo { points, .. } => {
            let pts: Vec<egui::Pos2> = points.iter().map(|p| pt2(p)).collect();
            if pts.is_empty() {
                f32::MAX
            } else {
                poly_dist(pos, &pts)
            }
        }
    }
}

/// Ghost color used for placement previews.
pub const PREVIEW_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(160, 160, 220, 140);

/// The drawing that WOULD be committed if the user clicked at
/// (`bar`, `price`) now — rendered as a live ghost during placement so every
/// tool tracks the cursor exactly like the final result (TradingView-style).
/// `pending` carries already-clicked points for multi-click tools.
pub fn preview_drawing(
    mode: &DrawMode,
    pending: &[(usize, f64)],
    bar: usize,
    price: f64,
) -> Option<Drawing> {
    let c = PREVIEW_COLOR;
    let cur = (bar, price);
    let with_cursor = || -> Vec<(usize, f64)> {
        let mut pts = pending.to_vec();
        pts.push(cur);
        pts
    };
    Some(match *mode {
        DrawMode::None | DrawMode::Eraser => return None,
        DrawMode::PlacingHLine => Drawing::HLine { price, color: c },
        DrawMode::PlacingVLine => Drawing::VLine { bar_idx: bar, color: c },
        DrawMode::PlacingHRay => Drawing::HRay {
            bar_idx: bar,
            price,
            color: c,
        },
        DrawMode::PlacingCrossLine => Drawing::CrossLine {
            bar_idx: bar,
            price,
            color: c,
        },
        DrawMode::PlacingTrendP1
        | DrawMode::PlacingFiboP1
        | DrawMode::PlacingRectP1
        | DrawMode::PlacingRayP1
        | DrawMode::PlacingChannelP1
        | DrawMode::PlacingExtLineP1
        | DrawMode::PlacingArrowP1
        | DrawMode::PlacingInfoLineP1
        | DrawMode::PlacingPitchforkP1
        | DrawMode::PlacingFiboExtP1
        | DrawMode::PlacingLongPosP1
        | DrawMode::PlacingShortPosP1
        | DrawMode::PlacingPriceRangeP1
        | DrawMode::PlacingEllipseP1
        | DrawMode::PlacingTriangleP1
        | DrawMode::PlacingTrendAngleP1
        | DrawMode::PlacingParallelChP1
        | DrawMode::PlacingFibChannelP1
        | DrawMode::PlacingCalloutP1
        | DrawMode::PlacingHighlighterP1
        | DrawMode::PlacingRegressionChP1
        | DrawMode::PlacingGannBoxP1
        | DrawMode::PlacingDateRangeP1
        | DrawMode::PlacingDatePriceRangeP1
        | DrawMode::PlacingSchiffPitchforkP1
        | DrawMode::PlacingModSchiffPitchforkP1
        | DrawMode::PlacingCyclicLinesP1
        | DrawMode::PlacingSineWaveP1
        | DrawMode::PlacingBalloonP1
        | DrawMode::PlacingRiskRewardP1
        | DrawMode::PlacingFibCircleP1
        | DrawMode::PlacingArcP1
        | DrawMode::PlacingCurveP1
        | DrawMode::PlacingForecastP1
        | DrawMode::PlacingGhostFeedP1
        | DrawMode::PlacingRulerP1
        | DrawMode::PlacingTimeCycleP1
        | DrawMode::PlacingSpeedFanP1
        | DrawMode::PlacingSpeedArcP1
        | DrawMode::PlacingFibSpiralP1
        | DrawMode::PlacingRotatedRectP1
        | DrawMode::PlacingTrendChannelP1
        | DrawMode::PlacingInsidePitchforkP1
        | DrawMode::PlacingFibWedgeP1
        | DrawMode::PlacingMeasureToolP1
        | DrawMode::PlacingCircleP1
        | DrawMode::PlacingPitchFanP1
        | DrawMode::PlacingTrendFibTimeP1
        | DrawMode::PlacingGannSquareP1
        | DrawMode::PlacingGannSquareFixedP1
        | DrawMode::PlacingBarsPatternP1
        | DrawMode::PlacingProjectionP1
        | DrawMode::PlacingDoubleCurveP1 => Drawing::CrossMarker {
            bar_idx: bar,
            price,
            color: c,
        },
        DrawMode::PlacingTrendP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingFiboP2 { bar1, price1 } => Drawing::FiboRetrace {
            high: price1.max(price),
            low: price1.min(price),
            bar_start: bar1.min(bar),
            bar_end: bar1.max(bar),
        },
        DrawMode::PlacingRectP2 { bar1, price1 } => Drawing::Rectangle {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingRayP2 { bar1, price1 } => {
            let db = bar as i64 - bar1 as i64;
            Drawing::Ray {
                origin: (bar1, price1),
                slope: if db != 0 {
                    (price - price1) / db as f64
                } else {
                    0.0
                },
                color: c,
            }
        }
        DrawMode::PlacingChannelP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingChannelP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::Channel {
            p1: (bar1, price1),
            p2: (bar2, price2),
            width: price - (price1 + price2) * 0.5,
            color: c,
        },
        DrawMode::PlacingExtLineP2 { bar1, price1 } => Drawing::ExtendedLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingArrowP2 { bar1, price1 } => Drawing::ArrowLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingInfoLineP2 { bar1, price1 } => Drawing::InfoLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingPitchforkP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingPitchforkP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::Pitchfork {
            pivot: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingFiboExtP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingFiboExtP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::FiboExtension {
            p1: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingGannFan => Drawing::GannFan {
            origin: cur,
            scale: 0.0,
            color: c,
        },
        DrawMode::PlacingLongPosP2 { bar1, entry } => Drawing::LongPosition {
            entry: (bar1, entry),
            stop: price,
            target: entry + (entry - price),
        },
        DrawMode::PlacingLongPosP3 { bar1, entry, stop } => Drawing::LongPosition {
            entry: (bar1, entry),
            stop,
            target: price,
        },
        DrawMode::PlacingShortPosP2 { bar1, entry } => Drawing::ShortPosition {
            entry: (bar1, entry),
            stop: price,
            target: entry - (price - entry),
        },
        DrawMode::PlacingShortPosP3 { bar1, entry, stop } => Drawing::ShortPosition {
            entry: (bar1, entry),
            stop,
            target: price,
        },
        DrawMode::PlacingPriceRangeP2 { bar1, price1 } => Drawing::PriceRange {
            p1: (bar1, price1),
            p2: cur,
        },
        DrawMode::PlacingTextLabel => Drawing::TextLabel {
            bar_idx: bar,
            price,
            text: "Text".into(),
            color: c,
        },
        DrawMode::PlacingArrowMarkerUp => Drawing::ArrowMarker {
            bar_idx: bar,
            price,
            is_up: true,
            color: c,
        },
        DrawMode::PlacingArrowMarkerDown => Drawing::ArrowMarker {
            bar_idx: bar,
            price,
            is_up: false,
            color: c,
        },
        DrawMode::PlacingEllipseP2 { bar1, price1 } => Drawing::Ellipse {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingTriangleP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingTriangleP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::Triangle {
            p1: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingTrendAngleP2 { bar1, price1 } => Drawing::TrendAngle {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingParallelChP2 { bar1, price1 } => Drawing::ParallelChannel {
            p1: (bar1, price1),
            p2: cur,
            offset: 0.0,
            color: c,
        },
        DrawMode::PlacingFibChannelP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingFibChannelP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::FibChannel {
            p1: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingFibTimeZones => Drawing::FibTimeZones { bar_idx: bar, color: c },
        DrawMode::PlacingPriceLabel => Drawing::PriceLabel {
            bar_idx: bar,
            price,
            color: c,
        },
        DrawMode::PlacingCalloutP2 { bar1, price1 } => Drawing::Callout {
            anchor: (bar1, price1),
            label_pos: cur,
            text: "Callout".into(),
            color: c,
        },
        DrawMode::PlacingHighlighterP2 { bar1, price1 } => Drawing::Highlighter {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingCrossMarker => Drawing::CrossMarker {
            bar_idx: bar,
            price,
            color: c,
        },
        DrawMode::PlacingPolyline => Drawing::Polyline {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingAnchorNote => Drawing::AnchorNote {
            bar_idx: bar,
            price,
            text: "Note".into(),
            color: c,
        },
        DrawMode::PlacingRegressionChP2 { bar1, price1 } => Drawing::RegressionChannel {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingGannBoxP2 { bar1, price1 } => Drawing::GannBox {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingElliottWave => Drawing::ElliottWave {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingAbcCorrection => Drawing::AbcCorrection {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingDateRangeP2 { bar1, price1 } => Drawing::DateRange {
            p1: (bar1, price1),
            p2: cur,
        },
        DrawMode::PlacingDatePriceRangeP2 { bar1, price1 } => Drawing::DatePriceRange {
            p1: (bar1, price1),
            p2: cur,
        },
        DrawMode::PlacingHeadShoulders => Drawing::HeadShoulders {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingXabcdPattern => Drawing::XabcdPattern {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingBrush => Drawing::Brush {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingSchiffPitchforkP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingSchiffPitchforkP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::SchiffPitchfork {
            pivot: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingModSchiffPitchforkP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingModSchiffPitchforkP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::ModSchiffPitchfork {
            pivot: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingCyclicLinesP2 { bar1 } => Drawing::CyclicLines {
            bar_start: bar1,
            bar_end: bar,
            color: c,
        },
        DrawMode::PlacingSineWaveP2 { bar1, price1 } => Drawing::SineWave {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingEmoji => Drawing::Emoji {
            bar_idx: bar,
            price,
            emoji: "😀".into(),
        },
        DrawMode::PlacingFlag => Drawing::Flag {
            bar_idx: bar,
            price,
            color: c,
        },
        DrawMode::PlacingBalloonP2 { bar1, price1 } => Drawing::Balloon {
            anchor: (bar1, price1),
            label_pos: cur,
            text: "Balloon".into(),
            color: c,
        },
        DrawMode::PlacingSessionBreak => Drawing::SessionBreak { bar_idx: bar, color: c },
        DrawMode::PlacingMagnetLevel => Drawing::MagnetLevel { price, color: c },
        DrawMode::PlacingRiskRewardP2 { bar1, entry } => Drawing::RiskRewardBox {
            entry: (bar1, entry),
            stop: price,
            target: entry + (entry - price),
        },
        DrawMode::PlacingRiskRewardP3 { bar1, entry, stop } => Drawing::RiskRewardBox {
            entry: (bar1, entry),
            stop,
            target: price,
        },
        DrawMode::PlacingFibCircleP2 { bar1, price1 } => Drawing::FibCircle {
            center: (bar1, price1),
            radius_pt: cur,
            color: c,
        },
        DrawMode::PlacingArcP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingArcP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::ArcDraw {
            p1: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingCurveP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingCurveP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::CurveDraw {
            p1: (bar1, price1),
            ctrl1: (bar2, price2),
            ctrl2: cur,
            p2: cur,
            color: c,
        },
        DrawMode::PlacingCurveP4 {
            bar1,
            price1,
            bar2,
            price2,
            bar3,
            price3,
        } => Drawing::CurveDraw {
            p1: (bar1, price1),
            ctrl1: (bar2, price2),
            ctrl2: (bar3, price3),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingPath => Drawing::PathDraw {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingForecastP2 { bar1, price1 } => Drawing::Forecast {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingGhostFeedP2 { bar1, price1 } => Drawing::GhostFeed {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingSignpost => Drawing::Signpost {
            bar_idx: bar,
            price,
            color: c,
        },
        DrawMode::PlacingRulerP2 { bar1, price1 } => Drawing::Ruler {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingTimeCycleP2 { bar1 } => Drawing::TimeCycle {
            bar_start: bar1,
            bar_end: bar,
            color: c,
        },
        DrawMode::PlacingSpeedFanP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingSpeedFanP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::SpeedResistanceFan {
            p1: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingSpeedArcP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingSpeedArcP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::SpeedResistanceArc {
            p1: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingFibSpiralP2 { bar1, price1 } => Drawing::FibSpiral {
            center: (bar1, price1),
            radius_pt: cur,
            color: c,
        },
        DrawMode::PlacingRotatedRectP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingRotatedRectP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::RotatedRectangle {
            p1: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingAnchoredVwap => Drawing::AnchoredVwapLine { bar_idx: bar, color: c },
        DrawMode::PlacingTrendChannelP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingTrendChannelP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::TrendChannel {
            p1: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingInsidePitchforkP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingInsidePitchforkP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::InsidePitchfork {
            pivot: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingFibWedgeP2 { bar1, price1 } => Drawing::TrendLine {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingFibWedgeP3 {
            bar1,
            price1,
            bar2,
            price2,
        } => Drawing::FibWedge {
            p1: (bar1, price1),
            p2: (bar2, price2),
            p3: cur,
            color: c,
        },
        DrawMode::PlacingPriceNote => Drawing::PriceNote {
            price,
            text: "Note".into(),
            color: c,
        },
        DrawMode::PlacingMeasureToolP2 { bar1, price1 } => Drawing::MeasureTool {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingAnchoredText => Drawing::AnchoredText {
            bar_idx: bar,
            price,
            text: "Text".into(),
            color: c,
        },
        DrawMode::PlacingComment => Drawing::Comment {
            bar_idx: bar,
            price,
            text: "Comment".into(),
            color: c,
        },
        DrawMode::PlacingArrowMarkerLeft => Drawing::ArrowMarkerLeft {
            bar_idx: bar,
            price,
            color: c,
        },
        DrawMode::PlacingArrowMarkerRight => Drawing::ArrowMarkerRight {
            bar_idx: bar,
            price,
            color: c,
        },
        DrawMode::PlacingCircleP2 { bar1, price1 } => Drawing::Circle {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingPitchFanP2 { bar1, price1 } => Drawing::PitchFan {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingTrendFibTimeP2 { bar1, price1 } => Drawing::TrendFibTime {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingGannSquareP2 { bar1, price1 } => Drawing::GannSquare {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingGannSquareFixedP2 { bar1, price1 } => Drawing::GannSquareFixed {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingBarsPatternP2 { bar1, price1 } => Drawing::BarsPattern {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingProjectionP2 { bar1, price1 } => Drawing::Projection {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingDoubleCurveP2 { bar1, price1 } => Drawing::DoubleCurve {
            p1: (bar1, price1),
            p2: cur,
            color: c,
        },
        DrawMode::PlacingTrianglePattern => Drawing::TrianglePattern {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingThreeDrives => Drawing::ThreeDrives {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingElliottDouble => Drawing::ElliottDouble {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingAbcdPattern => Drawing::AbcdPattern {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingCypherPattern => Drawing::CypherPattern {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingElliottTriangle => Drawing::ElliottTriangle {
            points: with_cursor(),
            color: c,
        },
        DrawMode::PlacingElliottTripleCombo => Drawing::ElliottTripleCombo {
            points: with_cursor(),
            color: c,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> PriceViewGeometry {
        PriceViewGeometry {
            chart_rect: egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 400.0)),
            price_min: 50.0,
            price_max: 150.0,
            log_scale: false,
            data_left: 0.0,
            bar_w: 8.0,
            start_idx: 100,
        }
    }

    #[test]
    fn geometry_bar_mapping_round_trips_including_offscreen() {
        let g = geometry();
        assert_eq!(g.x_to_bar(g.bar_to_x(150), 10_000), 150);
        // Off-screen-left bar still maps consistently (signed, no underflow).
        assert!(g.bar_to_x(50) < g.chart_rect.left());
        assert_eq!(g.x_to_bar(g.bar_to_x(50), 10_000), 50);
    }

    #[test]
    fn hit_distance_reaches_partially_offscreen_trendline() {
        let g = geometry();
        // P1 far off-screen left, P2 on-screen: grabbing the visible middle
        // must hit (the old per-site hit-test required BOTH endpoints
        // visible, making long lines unselectable).
        let d = Drawing::TrendLine {
            p1: (0, 100.0),
            p2: (180, 100.0),
            color: egui::Color32::WHITE,
        };
        let mid = egui::pos2(400.0, g.price_to_y(100.0));
        assert!(drawing_hit_distance(&d, mid, &g) < 2.0);
    }

    #[test]
    fn every_variant_is_hit_testable_and_translatable() {
        // Representative instance of each shape family (compile-time
        // exhaustiveness already guards new variants; this exercises runtime
        // sanity: finite distance + translate moves anchors).
        let mk = |d: Drawing| d;
        let c = egui::Color32::WHITE;
        let samples = vec![
            mk(Drawing::HLine { price: 100.0, color: c }),
            mk(Drawing::FiboRetrace {
                high: 120.0,
                low: 80.0,
                bar_start: 110,
                bar_end: 140,
            }),
            mk(Drawing::LongPosition {
                entry: (120, 100.0),
                stop: 90.0,
                target: 120.0,
            }),
            mk(Drawing::Callout {
                anchor: (120, 100.0),
                label_pos: (130, 110.0),
                text: "x".into(),
                color: c,
            }),
            mk(Drawing::CyclicLines {
                bar_start: 110,
                bar_end: 120,
                color: c,
            }),
            mk(Drawing::CurveDraw {
                p1: (110, 90.0),
                ctrl1: (115, 130.0),
                ctrl2: (125, 70.0),
                p2: (130, 110.0),
                color: c,
            }),
            mk(Drawing::GannFan {
                origin: (120, 100.0),
                scale: 0.5,
                color: c,
            }),
            mk(Drawing::ElliottWave {
                points: vec![(105, 90.0), (115, 110.0), (125, 95.0)],
                color: c,
            }),
        ];
        let g = geometry();
        for mut d in samples {
            let dist = drawing_hit_distance(&d, g.chart_rect.center(), &g);
            assert!(dist.is_finite(), "{d:?}");
            let anchors_before = drawing_anchors(&d);
            assert!(!anchors_before.is_empty(), "{d:?}");
            translate_drawing(&mut d, 5, 2.5, 10_000);
            let anchors_after = drawing_anchors(&d);
            // Data anchors moved by exactly (+5 bars, +2.5 price).
            for (a, b) in anchors_before.iter().zip(anchors_after.iter()) {
                if let (AnchorPos::Data(b0, p0), AnchorPos::Data(b1, p1)) = (a, b) {
                    assert_eq!(*b1 as i64 - *b0 as i64, 5, "{d:?}");
                    assert!((p1 - p0 - 2.5).abs() < 1e-9, "{d:?}");
                }
            }
        }
    }

    #[test]
    fn set_anchor_reshapes_slope_and_position_tools() {
        let g = geometry();
        let mut ray = Drawing::Ray {
            origin: (100, 100.0),
            slope: 0.0,
            color: egui::Color32::WHITE,
        };
        // Dragging the slope handle re-aims the ray.
        drawing_set_anchor(&mut ray, 1, 120, 110.0, 10_000);
        if let Drawing::Ray { slope, .. } = ray {
            assert!((slope - 0.5).abs() < 1e-9);
        } else {
            unreachable!()
        }
        let mut long = Drawing::LongPosition {
            entry: (120, 100.0),
            stop: 90.0,
            target: 120.0,
        };
        drawing_set_anchor(&mut long, 1, 120, 85.0, 10_000);
        if let Drawing::LongPosition { stop, .. } = long {
            assert!((stop - 85.0).abs() < 1e-9);
        } else {
            unreachable!()
        }
        let _ = g;
    }

    #[test]
    fn preview_tracks_cursor_for_multi_click_tools() {
        let pending = vec![(100, 90.0), (110, 110.0)];
        let p = preview_drawing(&DrawMode::PlacingElliottWave, &pending, 120, 95.0).unwrap();
        if let Drawing::ElliottWave { points, .. } = p {
            assert_eq!(points.len(), 3);
            assert_eq!(points[2], (120, 95.0));
        } else {
            unreachable!()
        }
        let p2 = preview_drawing(
            &DrawMode::PlacingTrendP2 {
                bar1: 100,
                price1: 90.0,
            },
            &[],
            140,
            120.0,
        )
        .unwrap();
        assert!(matches!(p2, Drawing::TrendLine { p2: (140, p), .. } if (p - 120.0).abs() < 1e-9));
        assert!(preview_drawing(&DrawMode::None, &[], 0, 0.0).is_none());
    }
}
