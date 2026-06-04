//! Extracted from app.rs: drawing helpers.

use super::*;

// ─── drawing tools ───────────────────────────────────────────────────────────

pub(crate) const TRENDLINE_COL: egui::Color32 = egui::Color32::from_rgb(100, 200, 255);
pub(crate) const FIBO_COL: egui::Color32 = egui::Color32::from_rgb(200, 160, 100);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

/// Wrapper around Drawing with per-drawing style properties.
#[derive(Clone, Debug)]
pub(crate) enum Drawing {
    /// Horizontal price line.
    HLine { price: f64, color: egui::Color32 },
    /// Trendline between two (bar_index, price) points.
    TrendLine {
        p1: (usize, f64), // (absolute bar index, price)
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Fibonacci retracement between two price levels.
    FiboRetrace {
        high: f64,
        low: f64,
        bar_start: usize,
        bar_end: usize,
    },
    /// Vertical line at a bar index.
    VLine {
        bar_idx: usize,
        color: egui::Color32,
    },
    /// Rectangle between two (bar, price) corners.
    Rectangle {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Ray from one point extending infinitely to the right.
    Ray {
        origin: (usize, f64),
        slope: f64, // price per bar
        color: egui::Color32,
    },
    /// Parallel channel (trendline + offset).
    Channel {
        p1: (usize, f64),
        p2: (usize, f64),
        width: f64, // price offset for parallel line
        color: egui::Color32,
    },
    /// Extended line (infinite both directions through two points).
    ExtendedLine {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Horizontal ray (extends right from a price level at a specific bar).
    HRay {
        bar_idx: usize,
        price: f64,
        color: egui::Color32,
    },
    /// Cross line (horizontal + vertical through one point).
    CrossLine {
        bar_idx: usize,
        price: f64,
        color: egui::Color32,
    },
    /// Arrow line (trendline with arrowhead at p2).
    ArrowLine {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Info line (trendline showing distance, percent change, bars count).
    InfoLine {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Andrews Pitchfork (3-point: pivot + two channel points).
    Pitchfork {
        pivot: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Fibonacci Extension (3-point trend-based).
    FiboExtension {
        p1: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Gann Fan (8 angle lines from a pivot point).
    GannFan {
        origin: (usize, f64),
        scale: f64, // price per bar for 1×1 angle
        color: egui::Color32,
    },
    /// Long Position (risk/reward box).
    LongPosition {
        entry: (usize, f64),
        stop: f64,
        target: f64,
    },
    /// Short Position (risk/reward box).
    ShortPosition {
        entry: (usize, f64),
        stop: f64,
        target: f64,
    },
    /// Price Range measurement (two price levels).
    PriceRange { p1: (usize, f64), p2: (usize, f64) },
    /// Text annotation at a specific point.
    TextLabel {
        bar_idx: usize,
        price: f64,
        text: String,
        color: egui::Color32,
    },
    /// Arrow marker (up/down directional).
    ArrowMarker {
        bar_idx: usize,
        price: f64,
        is_up: bool,
        color: egui::Color32,
    },
    /// Circle/Ellipse between two corner points.
    Ellipse {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Triangle (3 points).
    Triangle {
        p1: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Trend Angle (trendline with angle display).
    TrendAngle {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Parallel Channel (two parallel trendlines, 2 clicks + width from midpoint offset).
    ParallelChannel {
        p1: (usize, f64),
        p2: (usize, f64),
        offset: f64, // price offset for the parallel line (half-width above & below)
        color: egui::Color32,
    },
    /// Fib Channel (Fibonacci levels applied to a channel, 3 clicks).
    FibChannel {
        p1: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64), // defines channel width direction
        color: egui::Color32,
    },
    /// Fib Time Zones (vertical lines at Fibonacci intervals from a start bar).
    FibTimeZones {
        bar_idx: usize,
        color: egui::Color32,
    },
    /// Price Label (horizontal line with price badge at a specific point).
    PriceLabel {
        bar_idx: usize,
        price: f64,
        color: egui::Color32,
    },
    /// Callout (text box with arrow pointing to chart location, 2 clicks).
    Callout {
        anchor: (usize, f64),    // point the arrow points to
        label_pos: (usize, f64), // where the text box sits
        text: String,
        color: egui::Color32,
    },
    /// Highlighter (semi-transparent colored rectangle for marking zones).
    Highlighter {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Cross Marker (+ marker at a specific point).
    CrossMarker {
        bar_idx: usize,
        price: f64,
        color: egui::Color32,
    },
    /// Polyline (multi-segment line, series of connected points).
    Polyline {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Anchor Note (note pinned to a bar with background box).
    AnchorNote {
        bar_idx: usize,
        price: f64,
        text: String,
        color: egui::Color32,
    },
    /// Regression Channel (linear regression line with standard deviation bands, 2 clicks).
    RegressionChannel {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Gann Box (rectangle with Gann grid lines, 2 clicks).
    GannBox {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Elliott Wave Labels (5 swing points connected by lines, labeled 1-5).
    ElliottWave {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// ABC Correction Labels (3 swing points connected by lines, labeled A-B-C).
    AbcCorrection {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Date Range measurement (bars/days between two points).
    DateRange { p1: (usize, f64), p2: (usize, f64) },
    /// Date & Price Range combined measurement (bars, price, % change).
    DatePriceRange { p1: (usize, f64), p2: (usize, f64) },
    /// Head & Shoulders pattern markup (5 points: LS, Head, RS + neckline).
    HeadShoulders {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// XABCD Harmonic Pattern (5 labeled points connected by lines).
    XabcdPattern {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Brush/Freehand (series of closely-spaced dots following mouse drag).
    Brush {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Schiff Pitchfork (pivot shifted to midpoint of pivot-endpoint1).
    SchiffPitchfork {
        pivot: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Modified Schiff Pitchfork (pivot shifted to midpoint of both pivot-endpoint pairs).
    ModSchiffPitchfork {
        pivot: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Cyclic Lines (vertical lines at regular intervals from two click points).
    CyclicLines {
        bar_start: usize,
        bar_end: usize,
        color: egui::Color32,
    },
    /// Sine Wave (sine curve from two click points defining period/amplitude).
    SineWave {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Emoji annotation (single character placed at a chart point).
    Emoji {
        bar_idx: usize,
        price: f64,
        emoji: String,
    },
    /// Flag marker at a chart point.
    Flag {
        bar_idx: usize,
        price: f64,
        color: egui::Color32,
    },
    /// Balloon / speech bubble (anchor + box position with text).
    Balloon {
        anchor: (usize, f64),
        label_pos: (usize, f64),
        text: String,
        color: egui::Color32,
    },
    /// Session Break (vertical dashed line with label).
    SessionBreak {
        bar_idx: usize,
        color: egui::Color32,
    },
    /// Magnet Level (horizontal line that glows when price is within 0.5%).
    MagnetLevel { price: f64, color: egui::Color32 },
    /// Risk/Reward Box (entry, SL, TP with green/red zones and R:R ratio).
    RiskRewardBox {
        entry: (usize, f64),
        stop: f64,
        target: f64,
    },
    /// Fib Circle (center + radius click, draws circles at Fib ratios).
    FibCircle {
        center: (usize, f64),
        radius_pt: (usize, f64),
        color: egui::Color32,
    },
    /// Arc through 3 points (start, midpoint, end).
    ArcDraw {
        p1: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Bezier Curve with 2 control points (start, ctrl1, ctrl2, end).
    CurveDraw {
        p1: (usize, f64),
        ctrl1: (usize, f64),
        ctrl2: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Smooth Bezier-interpolated path through multiple points.
    PathDraw {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Forecast (2 clicks defining period, projects trend forward as dashed line).
    Forecast {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Ghost Feed (mirrors historical price action forward).
    GhostFeed {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Signpost icon with direction arrow at a point.
    Signpost {
        bar_idx: usize,
        price: f64,
        color: egui::Color32,
    },
    /// Ruler (distance in price, bars, percentage between two points).
    Ruler {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Time Cycle (vertical lines at cycle intervals with semi-circles).
    TimeCycle {
        bar_start: usize,
        bar_end: usize,
        color: egui::Color32,
    },
    /// Speed Resistance Fan (low, high, time ref — draws 1/3 and 2/3 speed lines).
    SpeedResistanceFan {
        p1: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Speed Resistance Arc (same as fan but with arcs).
    SpeedResistanceArc {
        p1: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Fib Spiral (center + radius, draws golden spiral).
    FibSpiral {
        center: (usize, f64),
        radius_pt: (usize, f64),
        color: egui::Color32,
    },
    /// Rotated Rectangle (2 clicks for baseline, 1 for height).
    RotatedRectangle {
        p1: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Anchored VWAP Line (from anchor bar forward using actual bar data).
    AnchoredVwapLine {
        bar_idx: usize,
        color: egui::Color32,
    },
    /// Trend Channel (2 clicks for trendline + 1 for channel width, with fill).
    TrendChannel {
        p1: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64), // defines offset / width
        color: egui::Color32,
    },
    /// Inside Pitchfork (3 clicks, pitchfork drawn inside the price action).
    InsidePitchfork {
        pivot: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Fib Wedge (3 clicks, fibonacci levels on converging trendlines).
    FibWedge {
        p1: (usize, f64),
        p2: (usize, f64),
        p3: (usize, f64),
        color: egui::Color32,
    },
    /// Price Note (text annotation pinned to a price level, not a bar).
    PriceNote {
        price: f64,
        text: String,
        color: egui::Color32,
    },
    /// Measure Tool (2 clicks, shows bars, price, %, angle, R:R).
    MeasureTool {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Anchored Text (1-click, text pinned to bar+price).
    AnchoredText {
        bar_idx: usize,
        price: f64,
        text: String,
        color: egui::Color32,
    },
    /// Comment (1-click, text note pinned to bar+price).
    Comment {
        bar_idx: usize,
        price: f64,
        text: String,
        color: egui::Color32,
    },
    /// Arrow Marker Left (1-click, left-pointing triangle).
    ArrowMarkerLeft {
        bar_idx: usize,
        price: f64,
        color: egui::Color32,
    },
    /// Arrow Marker Right (1-click, right-pointing triangle).
    ArrowMarkerRight {
        bar_idx: usize,
        price: f64,
        color: egui::Color32,
    },
    /// Circle (2 clicks: center + radius point).
    Circle {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Pitch Fan (2 clicks).
    PitchFan {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Trend-Based Fib Time (2 clicks).
    TrendFibTime {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Gann Square (2 clicks).
    GannSquare {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Gann Square Fixed (2 clicks).
    GannSquareFixed {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Bars Pattern (2 clicks: source range to mirror).
    BarsPattern {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Projection (2 clicks).
    Projection {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Double Curve (2 clicks).
    DoubleCurve {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Triangle Pattern (3 clicks).
    TrianglePattern {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Three Drives Pattern (3 clicks).
    ThreeDrives {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Elliott Double Combo WXY (3 clicks).
    ElliottDouble {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// ABCD Pattern (4 clicks).
    AbcdPattern {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Cypher Pattern (5 clicks).
    CypherPattern {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Elliott Triangle ABCDE (5 clicks).
    ElliottTriangle {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
    /// Elliott Triple Combo WXYXZ (5 clicks).
    ElliottTripleCombo {
        points: Vec<(usize, f64)>,
        color: egui::Color32,
    },
}

/// Trade marker for chart overlay (DARWIN deals, broker fills).
/// Aggregated: multiple deals at the same bar+price become one marker with combined volume.
#[derive(Clone, Debug)]
pub(crate) struct TradeMarker {
    pub(crate) bar_idx: usize, // index into bars array
    pub(crate) price: f64,
    pub(crate) volume: f64,    // aggregated total lots
    pub(crate) is_buy: bool,   // true=buy, false=sell
    pub(crate) count: u32,     // number of individual deals aggregated
    pub(crate) ticker: String, // DARWIN ticker (e.g., "HAKR", "MFSO")
}

/// Open position line for chart overlay (entry, SL, TP).
#[derive(Clone, Debug)]
pub(crate) struct PositionLine {
    pub(crate) price: f64,
    pub(crate) volume: f64, // aggregated lots at this price
    pub(crate) is_buy: bool,
    pub(crate) line_type: u8, // 0=entry, 1=SL, 2=TP
}

/// Trade overlay data passed to draw_chart for DARWIN/broker position rendering.
#[derive(Clone, Debug, Default)]
pub(crate) struct TradeOverlay {
    pub(crate) markers: Vec<TradeMarker>,
    pub(crate) position_lines: Vec<PositionLine>,
}

/// Drawing interaction mode.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum DrawMode {
    None,
    Eraser, // click near a drawing to delete it instantly
    PlacingHLine,
    PlacingTrendP1,
    PlacingTrendP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingFiboP1,
    PlacingFiboP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingVLine,
    PlacingRectP1,
    PlacingRectP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingRayP1,
    PlacingRayP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingChannelP1,
    PlacingChannelP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingChannelP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingExtLineP1,
    PlacingExtLineP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingHRay,
    PlacingCrossLine,
    PlacingArrowP1,
    PlacingArrowP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingInfoLineP1,
    PlacingInfoLineP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingPitchforkP1,
    PlacingPitchforkP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingPitchforkP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingFiboExtP1,
    PlacingFiboExtP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingFiboExtP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingGannFan,
    PlacingLongPosP1,
    PlacingLongPosP2 {
        bar1: usize,
        entry: f64,
    },
    PlacingLongPosP3 {
        bar1: usize,
        entry: f64,
        stop: f64,
    },
    PlacingShortPosP1,
    PlacingShortPosP2 {
        bar1: usize,
        entry: f64,
    },
    PlacingShortPosP3 {
        bar1: usize,
        entry: f64,
        stop: f64,
    },
    PlacingPriceRangeP1,
    PlacingPriceRangeP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingTextLabel,
    PlacingArrowMarkerUp,
    PlacingArrowMarkerDown,
    PlacingEllipseP1,
    PlacingEllipseP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingTriangleP1,
    PlacingTriangleP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingTriangleP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingTrendAngleP1,
    PlacingTrendAngleP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingParallelChP1,
    PlacingParallelChP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingFibChannelP1,
    PlacingFibChannelP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingFibChannelP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingFibTimeZones,
    PlacingPriceLabel,
    PlacingCalloutP1,
    PlacingCalloutP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingHighlighterP1,
    PlacingHighlighterP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingCrossMarker,
    PlacingPolyline,
    PlacingAnchorNote,
    PlacingRegressionChP1,
    PlacingRegressionChP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingGannBoxP1,
    PlacingGannBoxP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingElliottWave,
    PlacingAbcCorrection,
    PlacingDateRangeP1,
    PlacingDateRangeP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingDatePriceRangeP1,
    PlacingDatePriceRangeP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingHeadShoulders,
    PlacingXabcdPattern,
    PlacingBrush,
    PlacingSchiffPitchforkP1,
    PlacingSchiffPitchforkP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingSchiffPitchforkP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingModSchiffPitchforkP1,
    PlacingModSchiffPitchforkP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingModSchiffPitchforkP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingCyclicLinesP1,
    PlacingCyclicLinesP2 {
        bar1: usize,
    },
    PlacingSineWaveP1,
    PlacingSineWaveP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingEmoji,
    PlacingFlag,
    PlacingBalloonP1,
    PlacingBalloonP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingSessionBreak,
    PlacingMagnetLevel,
    PlacingRiskRewardP1,
    PlacingRiskRewardP2 {
        bar1: usize,
        entry: f64,
    },
    PlacingRiskRewardP3 {
        bar1: usize,
        entry: f64,
        stop: f64,
    },
    PlacingFibCircleP1,
    PlacingFibCircleP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingArcP1,
    PlacingArcP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingArcP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingCurveP1,
    PlacingCurveP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingCurveP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingCurveP4 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
        bar3: usize,
        price3: f64,
    },
    PlacingPath,
    PlacingForecastP1,
    PlacingForecastP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingGhostFeedP1,
    PlacingGhostFeedP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingSignpost,
    PlacingRulerP1,
    PlacingRulerP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingTimeCycleP1,
    PlacingTimeCycleP2 {
        bar1: usize,
    },
    PlacingSpeedFanP1,
    PlacingSpeedFanP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingSpeedFanP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingSpeedArcP1,
    PlacingSpeedArcP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingSpeedArcP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingFibSpiralP1,
    PlacingFibSpiralP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingRotatedRectP1,
    PlacingRotatedRectP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingRotatedRectP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingAnchoredVwap,
    PlacingTrendChannelP1,
    PlacingTrendChannelP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingTrendChannelP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingInsidePitchforkP1,
    PlacingInsidePitchforkP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingInsidePitchforkP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingFibWedgeP1,
    PlacingFibWedgeP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingFibWedgeP3 {
        bar1: usize,
        price1: f64,
        bar2: usize,
        price2: f64,
    },
    PlacingPriceNote,
    PlacingMeasureToolP1,
    PlacingMeasureToolP2 {
        bar1: usize,
        price1: f64,
    },
    // ── New drawing tools ──
    PlacingAnchoredText,
    PlacingComment,
    PlacingArrowMarkerLeft,
    PlacingArrowMarkerRight,
    PlacingCircleP1,
    PlacingCircleP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingPitchFanP1,
    PlacingPitchFanP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingTrendFibTimeP1,
    PlacingTrendFibTimeP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingGannSquareP1,
    PlacingGannSquareP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingGannSquareFixedP1,
    PlacingGannSquareFixedP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingBarsPatternP1,
    PlacingBarsPatternP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingProjectionP1,
    PlacingProjectionP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingDoubleCurveP1,
    PlacingDoubleCurveP2 {
        bar1: usize,
        price1: f64,
    },
    PlacingTrianglePattern,
    PlacingThreeDrives,
    PlacingElliottDouble,
    PlacingAbcdPattern,
    PlacingCypherPattern,
    PlacingElliottTriangle,
    PlacingElliottTripleCombo,
}
