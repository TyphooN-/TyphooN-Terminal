//! TyphooN GPU Charts — WebGL2-accelerated chart engine.
//!
//! Full chart engine: candlesticks, indicator lines, drawing tools, price lines,
//! sub-panes (Fisher/Volume/RSI), grid, crosshair — all on GPU via WebGL2.
//! Compiled to Wasm for use in Tauri WebView.
//!
//! Supports 1M+ candles at 60fps. Works on all GPUs (AMD/Intel/Nvidia) via
//! WebGL2 which is universally supported in WebKitGTK, Chromium, and Firefox.

use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext as GL, WebGlProgram, WebGlBuffer, WebGlUniformLocation, HtmlCanvasElement};

// ══════════════════════════════════════════════════════════════
// SHADER SOURCES
// ══════════════════════════════════════════════════════════════

const CANDLE_VS: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec2 a_pos;
layout(location = 1) in float a_color_flag;
uniform vec2 u_viewport;
uniform vec2 u_price_range;
uniform vec2 u_time_range;
out float v_color_flag;
void main() {
    float x_ndc = (a_pos.x - u_time_range.x) / (u_time_range.y - u_time_range.x) * 2.0 - 1.0;
    float y_ndc = (a_pos.y - u_price_range.x) / (u_price_range.y - u_price_range.x) * 2.0 - 1.0;
    gl_Position = vec4(x_ndc, y_ndc, 0.0, 1.0);
    v_color_flag = a_color_flag;
}
"#;

const CANDLE_FS: &str = r#"#version 300 es
precision highp float;
in float v_color_flag;
out vec4 frag_color;
uniform vec3 u_bull_color;
uniform vec3 u_bear_color;
uniform vec3 u_wick_color;
void main() {
    if (v_color_flag > 0.75) {
        frag_color = vec4(u_bull_color, 1.0);
    } else if (v_color_flag < 0.25) {
        frag_color = vec4(u_bear_color, 1.0);
    } else {
        frag_color = vec4(u_wick_color, 1.0);
    }
}
"#;

// Shared line shader — used for indicator lines AND drawing tools
const LINE_VS: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec2 a_pos;
uniform vec2 u_price_range;
uniform vec2 u_time_range;
void main() {
    float x_ndc = (a_pos.x - u_time_range.x) / (u_time_range.y - u_time_range.x) * 2.0 - 1.0;
    float y_ndc = (a_pos.y - u_price_range.x) / (u_price_range.y - u_price_range.x) * 2.0 - 1.0;
    gl_Position = vec4(x_ndc, y_ndc, 0.0, 1.0);
}
"#;

const LINE_FS: &str = r#"#version 300 es
precision highp float;
uniform vec4 u_line_color;
out vec4 frag_color;
void main() {
    frag_color = u_line_color;
}
"#;

// Per-vertex color shader — used for main-pane histograms and fills
const VERTEX_COLOR_VS: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec4 a_color;
uniform vec2 u_price_range;
uniform vec2 u_time_range;
out vec4 v_color;
void main() {
    float x_ndc = (a_pos.x - u_time_range.x) / (u_time_range.y - u_time_range.x) * 2.0 - 1.0;
    float y_ndc = (a_pos.y - u_price_range.x) / (u_price_range.y - u_price_range.x) * 2.0 - 1.0;
    gl_Position = vec4(x_ndc, y_ndc, 0.0, 1.0);
    v_color = a_color;
}
"#;

const VERTEX_COLOR_FS: &str = r#"#version 300 es
precision highp float;
in vec4 v_color;
out vec4 frag_color;
void main() {
    frag_color = v_color;
}
"#;

const GRID_VS: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec2 a_pos;
void main() { gl_Position = vec4(a_pos, 0.0, 1.0); }
"#;

const GRID_FS: &str = r#"#version 300 es
precision highp float;
uniform vec4 u_grid_color;
out vec4 frag_color;
void main() { frag_color = u_grid_color; }
"#;

// ══════════════════════════════════════════════════════════════
// DRAWING TYPES (Phase 1 + Phase 3)
// ══════════════════════════════════════════════════════════════

/// Drawing tool types — matches frontend drawing type strings.
#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u32)]
pub enum DrawType {
    TrendLine = 0,
    Ray = 1,
    Segment = 2,
    ExtendedLine = 3,
    ArrowLine = 4,
    Horizontal = 5,
    Vertical = 6,
    Rectangle = 7,
    Triangle = 8,
    Circle = 9,
    Ellipse = 10,
    Channel = 11,
    ParallelChannel = 12,
    Pitchfork = 13,
    SchiffPitchfork = 14,
    Fibonacci = 15,
    FibFan = 16,
    FibArcs = 17,
    FibChannel = 18,
    FibExtension = 19,
    GannFan = 20,
    GannLine = 21,
    GannBox = 22,
    CycleLines = 23,
    ArrowUp = 24,
    ArrowDown = 25,
    PriceLabel = 26,
    TextLabel = 27,
    // Phase 3 new tools
    RegressionChannel = 28,
    StdDevChannel = 29,
    RiskReward = 30,
    PositionBox = 31,
    DateRange = 32,
    PriceRange = 33,
    ElliottImpulse = 34,
    ElliottCorrective = 35,
    SpeedLines = 36,
    EquidistantChannel = 37,
}

/// Line style for indicator lines.
/// 0 = solid, 1 = dashed, 2 = dotted.
#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u32)]
pub enum LineStyle {
    Solid = 0,
    Dashed = 1,
    Dotted = 2,
}

/// An indicator line series on the main chart.
#[derive(Clone)]
struct IndicatorLine {
    vbo: WebGlBuffer,
    count: i32,
    color: [f32; 4],
    width: f32,
    style: LineStyle,
}

/// A histogram series on the main chart (volume, RVOL, etc.).
/// Uses per-vertex coloring via the vertex_color shader.
struct HistogramBuffer {
    vbo: WebGlBuffer,
    vertex_count: i32,
}

/// A filled area between two lines on the main chart.
struct FillBuffer {
    vbo: WebGlBuffer,
    vertex_count: i32,
    color: [f32; 4],
}

/// A single drawing on the chart.
#[derive(Clone)]
struct Drawing {
    draw_type: DrawType,
    /// Points as (bar_index, price) pairs — flat: [bar0, price0, bar1, price1, ...]
    points: Vec<f64>,
    color: [f32; 4],     // RGBA 0-1
    line_width: f32,
    fill_color: [f32; 4], // RGBA for filled shapes (0 alpha = no fill)
}

/// A horizontal price line (SL/TP/entry).
#[derive(Clone)]
struct PriceLine {
    price: f64,
    color: [f32; 4],
    line_width: f32,
    label: String,
}

/// A sub-pane for indicators (Fisher, Volume, RSI, etc.)
struct SubPane {
    /// Y start in normalized canvas coords (0.0 = bottom)
    y_start: f32,
    /// Height in normalized canvas coords
    height: f32,
    min_value: f64,
    max_value: f64,
    /// Line series: (vbo, vertex_count, rgba)
    lines: Vec<(WebGlBuffer, i32, [f32; 4])>,
    /// Histogram bars: (vbo, vertex_count) — uses candle shader for coloring
    histograms: Vec<(WebGlBuffer, i32)>,
}

// ══════════════════════════════════════════════════════════════
// CHART ENGINE
// ══════════════════════════════════════════════════════════════

#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq)]
pub enum ChartType {
    Candles = 0,
    HeikinAshi = 1,
    Line = 2,
    Bars = 3,
    Renko = 4,
}

#[wasm_bindgen]
pub struct GpuChart {
    gl: GL,
    canvas: HtmlCanvasElement,
    candle_program: WebGlProgram,
    line_program: WebGlProgram,
    grid_program: WebGlProgram,
    vertex_color_program: WebGlProgram,
    // Uniform locations
    candle_viewport: Option<WebGlUniformLocation>,
    candle_price_range: Option<WebGlUniformLocation>,
    candle_time_range: Option<WebGlUniformLocation>,
    candle_bull_color: Option<WebGlUniformLocation>,
    candle_bear_color: Option<WebGlUniformLocation>,
    candle_wick_color: Option<WebGlUniformLocation>,
    line_price_range: Option<WebGlUniformLocation>,
    line_time_range: Option<WebGlUniformLocation>,
    line_color: Option<WebGlUniformLocation>,
    grid_color: Option<WebGlUniformLocation>,
    vc_price_range: Option<WebGlUniformLocation>,
    vc_time_range: Option<WebGlUniformLocation>,
    // Candle buffers
    candle_vbo: WebGlBuffer,
    candle_vertex_count: i32,
    wick_vbo: WebGlBuffer,
    wick_vertex_count: i32,
    line_chart_vbo: WebGlBuffer,
    line_chart_count: i32,
    chart_type: ChartType,
    // View state
    min_price: f64,
    max_price: f64,
    visible_start: f64,
    visible_end: f64,
    total_bars: usize,
    // Bar data
    bar_opens: Vec<f32>,
    bar_highs: Vec<f32>,
    bar_lows: Vec<f32>,
    bar_closes: Vec<f32>,
    // Indicator line buffers
    line_buffers: Vec<IndicatorLine>,
    // Main-pane histogram buffers
    histogram_buffers: Vec<HistogramBuffer>,
    // Main-pane fill buffers
    fill_buffers: Vec<FillBuffer>,
    // Grid
    grid_vbo: WebGlBuffer,
    grid_vertex_count: i32,
    // Phase 1: Drawing tools
    drawings: Vec<Drawing>,
    // Phase 2: Price lines (SL/TP/entry)
    price_lines: Vec<PriceLine>,
    // Phase 4: Sub-panes
    panes: Vec<SubPane>,
    /// Main chart height fraction (1.0 = full, reduced when panes added)
    main_pane_height: f32,
}

#[wasm_bindgen]
impl GpuChart {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<GpuChart, JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("Canvas not found"))?
            .dyn_into::<HtmlCanvasElement>()?;

        let gl = canvas.get_context("webgl2")?
            .ok_or_else(|| JsValue::from_str("WebGL2 not supported"))?
            .dyn_into::<GL>()?;

        let candle_program = compile_program(&gl, CANDLE_VS, CANDLE_FS)?;
        let line_program = compile_program(&gl, LINE_VS, LINE_FS)?;
        let grid_program = compile_program(&gl, GRID_VS, GRID_FS)?;
        let vertex_color_program = compile_program(&gl, VERTEX_COLOR_VS, VERTEX_COLOR_FS)?;

        let candle_viewport = gl.get_uniform_location(&candle_program, "u_viewport");
        let candle_price_range = gl.get_uniform_location(&candle_program, "u_price_range");
        let candle_time_range = gl.get_uniform_location(&candle_program, "u_time_range");
        let candle_bull_color = gl.get_uniform_location(&candle_program, "u_bull_color");
        let candle_bear_color = gl.get_uniform_location(&candle_program, "u_bear_color");
        let candle_wick_color = gl.get_uniform_location(&candle_program, "u_wick_color");
        let line_price_range = gl.get_uniform_location(&line_program, "u_price_range");
        let line_time_range = gl.get_uniform_location(&line_program, "u_time_range");
        let line_color = gl.get_uniform_location(&line_program, "u_line_color");
        let grid_color = gl.get_uniform_location(&grid_program, "u_grid_color");
        let vc_price_range = gl.get_uniform_location(&vertex_color_program, "u_price_range");
        let vc_time_range = gl.get_uniform_location(&vertex_color_program, "u_time_range");

        let candle_vbo = gl.create_buffer().ok_or("Failed to create buffer")?;
        let wick_vbo = gl.create_buffer().ok_or("Failed to create wick buffer")?;
        let line_chart_vbo = gl.create_buffer().ok_or("Failed to create line chart buffer")?;
        let grid_vbo = gl.create_buffer().ok_or("Failed to create grid buffer")?;

        gl.clear_color(0.04, 0.04, 0.08, 1.0);
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        Ok(GpuChart {
            gl, canvas,
            candle_program, line_program, grid_program, vertex_color_program,
            candle_viewport, candle_price_range, candle_time_range,
            candle_bull_color, candle_bear_color, candle_wick_color,
            line_price_range, line_time_range, line_color,
            grid_color,
            vc_price_range, vc_time_range,
            candle_vbo, candle_vertex_count: 0,
            wick_vbo, wick_vertex_count: 0,
            line_chart_vbo, line_chart_count: 0,
            chart_type: ChartType::Candles,
            min_price: 0.0, max_price: 100.0,
            visible_start: 0.0, visible_end: 100.0,
            total_bars: 0,
            bar_opens: vec![], bar_highs: vec![], bar_lows: vec![], bar_closes: vec![],
            line_buffers: vec![],
            histogram_buffers: vec![],
            fill_buffers: vec![],
            grid_vbo, grid_vertex_count: 0,
            drawings: vec![],
            price_lines: vec![],
            panes: vec![],
            main_pane_height: 1.0,
        })
    }

    // ── Data Loading ────────────────────────────────────────────

    // Note: prices stored as f32 for GPU performance. At $71,000, f32 provides
    // ~$0.01 precision (7 significant digits). Sufficient for charting; crosshair
    // tooltips use the original f64 data from the frontend for exact values.
    #[wasm_bindgen]
    pub fn set_data(&mut self, data: &[f64]) {
        let n = data.len() / 5;
        self.total_bars = n;
        self.bar_opens.clear();
        self.bar_highs.clear();
        self.bar_lows.clear();
        self.bar_closes.clear();

        let mut min_p = f64::MAX;
        let mut max_p = f64::MIN;

        for i in 0..n {
            let o = data[i * 5] as f32;
            let h = data[i * 5 + 1] as f32;
            let l = data[i * 5 + 2] as f32;
            let c = data[i * 5 + 3] as f32;
            self.bar_opens.push(o);
            self.bar_highs.push(h);
            self.bar_lows.push(l);
            self.bar_closes.push(c);
            if (h as f64) > max_p { max_p = h as f64; }
            if (l as f64) < min_p { min_p = l as f64; }
        }

        let padding = (max_p - min_p) * 0.05;
        self.min_price = min_p - padding;
        self.max_price = max_p + padding;
        self.visible_start = if n > 100 { (n - 100) as f64 } else { 0.0 };
        self.visible_end = n as f64 + 2.0;

        self.rebuild_geometry();
        self.build_grid_geometry();
    }

    #[wasm_bindgen]
    pub fn set_chart_type(&mut self, ct: ChartType) {
        self.chart_type = ct;
        if self.total_bars > 0 { self.rebuild_geometry(); }
    }

    #[wasm_bindgen]
    pub fn set_visible_range(&mut self, start: f64, end: f64) {
        self.visible_start = start;
        self.visible_end = end;
        let s = start.max(0.0) as usize;
        let e = (end as usize).min(self.total_bars);
        if s < e {
            let mut min_p = f64::MAX;
            let mut max_p = f64::MIN;
            for i in s..e {
                if (self.bar_highs[i] as f64) > max_p { max_p = self.bar_highs[i] as f64; }
                if (self.bar_lows[i] as f64) < min_p { min_p = self.bar_lows[i] as f64; }
            }
            let padding = (max_p - min_p) * 0.05;
            self.min_price = min_p - padding;
            self.max_price = max_p + padding;
        }
        self.build_grid_geometry();
    }

    #[wasm_bindgen]
    pub fn scroll(&mut self, delta: f64) {
        let range = self.visible_end - self.visible_start;
        self.visible_start = (self.visible_start + delta).max(0.0);
        self.visible_end = self.visible_start + range;
        if self.visible_end > self.total_bars as f64 + 5.0 {
            self.visible_end = self.total_bars as f64 + 5.0;
            self.visible_start = self.visible_end - range;
        }
        self.set_visible_range(self.visible_start, self.visible_end);
    }

    #[wasm_bindgen]
    pub fn zoom(&mut self, factor: f64, center_x: f64) {
        let range = self.visible_end - self.visible_start;
        let center = self.visible_start + range * center_x;
        let new_range = (range / factor).max(10.0).min(self.total_bars as f64);
        self.visible_start = center - new_range * center_x;
        self.visible_end = self.visible_start + new_range;
        self.set_visible_range(self.visible_start, self.visible_end);
    }

    // ── Indicator Lines ─────────────────────────────────────────

    /// Add a styled indicator line with custom width and dash style.
    #[wasm_bindgen]
    pub fn add_line_styled(&mut self, values: &[f64], r: f32, g: f32, b: f32, a: f32, width: f32, style: LineStyle) {
        let mut vertices: Vec<f32> = Vec::with_capacity(values.len() * 2);
        for (i, &v) in values.iter().enumerate() {
            vertices.push(i as f32);
            vertices.push(v as f32);
        }
        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
        self.line_buffers.push(IndicatorLine {
            vbo,
            count: values.len() as i32,
            color: [r, g, b, a],
            width,
            style,
        });
    }

    /// Add a solid indicator line (convenience wrapper).
    #[wasm_bindgen]
    pub fn add_line(&mut self, values: &[f64], r: f32, g: f32, b: f32, a: f32) {
        self.add_line_styled(values, r, g, b, a, 1.0, LineStyle::Solid);
    }

    #[wasm_bindgen]
    pub fn clear_lines(&mut self) {
        for line in &self.line_buffers {
            self.gl.delete_buffer(Some(&line.vbo));
        }
        self.line_buffers.clear();
    }

    // ── Histogram Series (main pane) ────────────────────────────

    /// Add a histogram series to the main chart.
    /// `values`: one value per bar.
    /// `colors`: flat [r,g,b,a, r,g,b,a, ...] per bar (must be values.len() * 4).
    /// `base`: the zero/baseline value.
    #[wasm_bindgen]
    pub fn add_histogram(&mut self, values: &[f64], colors: &[f32], base: f64) {
        let n = values.len();
        if n == 0 { return; }
        let hw = 0.35f32;
        let base_f = base as f32;
        // 6 vertices per bar, each vertex = 2 pos + 4 color = 6 floats
        let mut vertices: Vec<f32> = Vec::with_capacity(n * 6 * 6);
        for i in 0..n {
            let x = i as f32;
            let val = values[i] as f32;
            let top = val.max(base_f);
            let bot = val.min(base_f);
            // Per-bar color (default white if colors array too short)
            let (cr, cg, cb, ca) = if i * 4 + 3 < colors.len() {
                (colors[i*4], colors[i*4+1], colors[i*4+2], colors[i*4+3])
            } else {
                (1.0, 1.0, 1.0, 1.0)
            };
            // Triangle 1: top-left, top-right, bottom-right
            vertices.extend_from_slice(&[x-hw, top, cr, cg, cb, ca]);
            vertices.extend_from_slice(&[x+hw, top, cr, cg, cb, ca]);
            vertices.extend_from_slice(&[x+hw, bot, cr, cg, cb, ca]);
            // Triangle 2: top-left, bottom-right, bottom-left
            vertices.extend_from_slice(&[x-hw, top, cr, cg, cb, ca]);
            vertices.extend_from_slice(&[x+hw, bot, cr, cg, cb, ca]);
            vertices.extend_from_slice(&[x-hw, bot, cr, cg, cb, ca]);
        }
        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
        self.histogram_buffers.push(HistogramBuffer {
            vbo,
            vertex_count: (n * 6) as i32,
        });
    }

    /// Clear all main-pane histogram series.
    #[wasm_bindgen]
    pub fn clear_histograms(&mut self) {
        for hb in &self.histogram_buffers {
            self.gl.delete_buffer(Some(&hb.vbo));
        }
        self.histogram_buffers.clear();
    }

    // ── Area/Baseline Fill (main pane) ──────────────────────────

    /// Add a filled area between two price-level series.
    /// `top_values` and `bottom_values` are one value per bar (same length).
    /// Color is uniform for the entire fill.
    #[wasm_bindgen]
    pub fn add_fill(&mut self, top_values: &[f64], bottom_values: &[f64], r: f32, g: f32, b: f32, a: f32) {
        let n = top_values.len().min(bottom_values.len());
        if n < 2 { return; }
        // Build triangle strip: for each bar, emit (x, top) then (x, bottom)
        let mut vertices: Vec<f32> = Vec::with_capacity(n * 2 * 2);
        for i in 0..n {
            vertices.push(i as f32);
            vertices.push(top_values[i] as f32);
            vertices.push(i as f32);
            vertices.push(bottom_values[i] as f32);
        }
        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
        self.fill_buffers.push(FillBuffer {
            vbo,
            vertex_count: (n * 2) as i32,
            color: [r, g, b, a],
        });
    }

    /// Clear all main-pane fill areas.
    #[wasm_bindgen]
    pub fn clear_fills(&mut self) {
        for fb in &self.fill_buffers {
            self.gl.delete_buffer(Some(&fb.vbo));
        }
        self.fill_buffers.clear();
    }

    // ── Update Last Bar (real-time) ─────────────────────────────

    /// Update the last bar's OHLC data and rebuild only its geometry.
    /// Avoids rebuilding the entire candle buffer for real-time tick updates.
    #[wasm_bindgen]
    pub fn update_last_bar(&mut self, open: f64, high: f64, low: f64, close: f64) {
        if self.total_bars == 0 { return; }
        let idx = self.total_bars - 1;
        self.bar_opens[idx] = open as f32;
        self.bar_highs[idx] = high as f32;
        self.bar_lows[idx] = low as f32;
        self.bar_closes[idx] = close as f32;

        // Update price range if needed
        let h = high;
        let l = low;
        let padding = (self.max_price - self.min_price) * 0.05;
        if h > self.max_price - padding { self.max_price = h + padding; }
        if l < self.min_price + padding { self.min_price = l - padding; }

        // Rebuild all geometry (chart type may be HA/Renko which needs full recalc)
        self.rebuild_geometry();
    }

    // ── Phase 1: Drawing Tools ──────────────────────────────────

    /// Add a drawing. Points are flat [bar0, price0, bar1, price1, ...].
    /// Color is [r, g, b, a]. Fill color is [r, g, b, a] (alpha=0 for no fill).
    #[wasm_bindgen]
    pub fn add_drawing(&mut self, draw_type: DrawType, points: &[f64], color: &[f32], line_width: f32, fill: &[f32]) {
        let c = if color.len() >= 4 { [color[0], color[1], color[2], color[3]] } else { [1.0, 1.0, 1.0, 1.0] };
        let f = if fill.len() >= 4 { [fill[0], fill[1], fill[2], fill[3]] } else { [0.0; 4] };
        self.drawings.push(Drawing {
            draw_type,
            points: points.to_vec(),
            color: c,
            line_width,
            fill_color: f,
        });
    }

    /// Remove drawing by index.
    #[wasm_bindgen]
    pub fn remove_drawing(&mut self, index: usize) {
        if index < self.drawings.len() {
            self.drawings.remove(index);
        }
    }

    /// Clear all drawings.
    #[wasm_bindgen]
    pub fn clear_drawings(&mut self) {
        self.drawings.clear();
    }

    /// Get drawing count.
    #[wasm_bindgen]
    pub fn drawing_count(&self) -> usize {
        self.drawings.len()
    }

    /// Update a drawing's points.
    #[wasm_bindgen]
    pub fn update_drawing_points(&mut self, index: usize, points: &[f64]) {
        if index < self.drawings.len() {
            self.drawings[index].points = points.to_vec();
        }
    }

    /// Update a drawing's color.
    #[wasm_bindgen]
    pub fn update_drawing_color(&mut self, index: usize, color: &[f32]) {
        if index < self.drawings.len() && color.len() >= 4 {
            self.drawings[index].color = [color[0], color[1], color[2], color[3]];
        }
    }

    /// Hit-test: returns drawing index at canvas (x, y), or -1 if none.
    #[wasm_bindgen]
    pub fn hit_test_drawing(&self, canvas_x: f64, canvas_y: f64, tolerance: f64) -> i32 {
        let bar = self.bar_at_x(canvas_x);
        let price = self.price_at_y(canvas_y);
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;
        if w <= 0.0 || h <= 0.0 { return -1; }

        // Convert tolerance from pixels to price/bar units
        let price_range = self.max_price - self.min_price;
        let bar_range = self.visible_end - self.visible_start;
        let tol_price = tolerance / h * price_range;
        let tol_bar = tolerance / w * bar_range;

        for (idx, d) in self.drawings.iter().enumerate().rev() {
            if d.points.len() < 2 { continue; }
            let b0 = d.points[0];
            let p0 = d.points[1];

            match d.draw_type {
                DrawType::Horizontal => {
                    if (price - p0).abs() < tol_price { return idx as i32; }
                }
                DrawType::Vertical => {
                    if (bar - b0).abs() < tol_bar { return idx as i32; }
                }
                DrawType::ArrowUp | DrawType::ArrowDown | DrawType::PriceLabel | DrawType::TextLabel => {
                    if (bar - b0).abs() < tol_bar * 2.0 && (price - p0).abs() < tol_price * 2.0 {
                        return idx as i32;
                    }
                }
                _ => {
                    if d.points.len() >= 4 {
                        let b1 = d.points[2];
                        let p1 = d.points[3];
                        // Point-to-line-segment distance
                        let dist = point_to_segment_dist(bar, price, b0, p0, b1, p1, tol_bar, tol_price);
                        if dist < 1.0 { return idx as i32; }

                        // For filled shapes, check if point is inside
                        match d.draw_type {
                            DrawType::Rectangle | DrawType::RiskReward | DrawType::PositionBox |
                            DrawType::DateRange | DrawType::PriceRange => {
                                let min_b = b0.min(b1);
                                let max_b = b0.max(b1);
                                let min_p = p0.min(p1);
                                let max_p = p0.max(p1);
                                if bar >= min_b && bar <= max_b && price >= min_p && price <= max_p {
                                    return idx as i32;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        -1
    }

    // ── Phase 2: Price Lines (SL/TP) ────────────────────────────

    /// Add a price line (SL/TP/entry). Returns index.
    #[wasm_bindgen]
    pub fn add_price_line(&mut self, price: f64, r: f32, g: f32, b: f32, a: f32, line_width: f32) -> usize {
        self.price_lines.push(PriceLine {
            price,
            color: [r, g, b, a],
            line_width,
            label: String::new(),
        });
        self.price_lines.len() - 1
    }

    /// Update a price line's price (for dragging).
    #[wasm_bindgen]
    pub fn update_price_line(&mut self, index: usize, price: f64) {
        if index < self.price_lines.len() {
            self.price_lines[index].price = price;
        }
    }

    /// Remove a price line by index.
    #[wasm_bindgen]
    pub fn remove_price_line(&mut self, index: usize) {
        if index < self.price_lines.len() {
            self.price_lines.remove(index);
        }
    }

    /// Clear all price lines.
    #[wasm_bindgen]
    pub fn clear_price_lines(&mut self) {
        self.price_lines.clear();
    }

    /// Get price line count.
    #[wasm_bindgen]
    pub fn price_line_count(&self) -> usize {
        self.price_lines.len()
    }

    /// Get price of a price line (for drag read-back).
    #[wasm_bindgen]
    pub fn get_price_line_price(&self, index: usize) -> f64 {
        if index < self.price_lines.len() { self.price_lines[index].price } else { 0.0 }
    }

    /// Hit-test price lines. Returns index or -1.
    #[wasm_bindgen]
    pub fn hit_test_price_line(&self, canvas_y: f64, tolerance: f64) -> i32 {
        let h = self.canvas.height() as f64;
        if h <= 0.0 { return -1; }
        let price = self.price_at_y(canvas_y);
        let price_range = self.max_price - self.min_price;
        let tol_price = tolerance / h * price_range;

        for (idx, pl) in self.price_lines.iter().enumerate() {
            if (price - pl.price).abs() < tol_price {
                return idx as i32;
            }
        }
        -1
    }

    // ── Phase 4: Sub-Panes ──────────────────────────────────────

    /// Add a sub-pane below the main chart. Returns pane index.
    /// `height` is fraction of total canvas (e.g., 0.15 = 15%).
    #[wasm_bindgen]
    pub fn add_pane(&mut self, height: f32) -> usize {
        self.panes.push(SubPane {
            y_start: 0.0,  // computed in recalc_pane_layout
            height,
            min_value: -2.0,
            max_value: 2.0,
            lines: vec![],
            histograms: vec![],
        });
        self.recalc_pane_layout();
        self.panes.len() - 1
    }

    /// Set value range for a sub-pane.
    #[wasm_bindgen]
    pub fn set_pane_range(&mut self, pane: usize, min_val: f64, max_val: f64) {
        if pane < self.panes.len() {
            self.panes[pane].min_value = min_val;
            self.panes[pane].max_value = max_val;
        }
    }

    /// Add a line series to a sub-pane. Values are indicator values (one per bar).
    #[wasm_bindgen]
    pub fn add_pane_line(&mut self, pane: usize, values: &[f64], r: f32, g: f32, b: f32, a: f32) {
        if pane >= self.panes.len() { return; }
        let mut vertices: Vec<f32> = Vec::with_capacity(values.len() * 2);
        for (i, &v) in values.iter().enumerate() {
            vertices.push(i as f32);
            vertices.push(v as f32);
        }
        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
        self.panes[pane].lines.push((vbo, values.len() as i32, [r, g, b, a]));
    }

    /// Add a histogram series to a sub-pane. Data is flat [value, colorFlag, value, colorFlag, ...].
    #[wasm_bindgen]
    pub fn add_pane_histogram(&mut self, pane: usize, data: &[f64]) {
        if pane >= self.panes.len() { return; }
        let n = data.len() / 2;
        let mut vertices: Vec<f32> = Vec::with_capacity(n * 6 * 3);
        let hw = 0.35f32;
        for i in 0..n {
            let val = data[i * 2] as f32;
            let flag = data[i * 2 + 1] as f32;
            let x = i as f32;
            // Bar from 0 to val
            let top = val.max(0.0);
            let bot = val.min(0.0);
            vertices.extend_from_slice(&[x - hw, top, flag, x + hw, top, flag, x + hw, bot, flag]);
            vertices.extend_from_slice(&[x - hw, top, flag, x + hw, bot, flag, x - hw, bot, flag]);
        }
        let vbo = self.gl.create_buffer().unwrap();
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
        self.panes[pane].histograms.push((vbo, (n * 6) as i32));
    }

    /// Clear all data from a sub-pane.
    #[wasm_bindgen]
    pub fn clear_pane(&mut self, pane: usize) {
        if pane >= self.panes.len() { return; }
        for (vbo, _, _) in &self.panes[pane].lines {
            self.gl.delete_buffer(Some(vbo));
        }
        for (vbo, _) in &self.panes[pane].histograms {
            self.gl.delete_buffer(Some(vbo));
        }
        self.panes[pane].lines.clear();
        self.panes[pane].histograms.clear();
    }

    /// Remove all sub-panes.
    #[wasm_bindgen]
    pub fn clear_panes(&mut self) {
        for p in &self.panes {
            for (vbo, _, _) in &p.lines { self.gl.delete_buffer(Some(vbo)); }
            for (vbo, _) in &p.histograms { self.gl.delete_buffer(Some(vbo)); }
        }
        self.panes.clear();
        self.main_pane_height = 1.0;
    }

    // ── Coordinate Helpers ──────────────────────────────────────

    #[wasm_bindgen]
    pub fn price_at_y(&self, y: f64) -> f64 {
        let h = self.canvas.height() as f64;
        if h <= 0.0 { return self.min_price; }
        // Only consider main pane area
        let main_h = h * self.main_pane_height as f64;
        let t = 1.0 - y / main_h;
        self.min_price + t.clamp(0.0, 1.0) * (self.max_price - self.min_price)
    }

    #[wasm_bindgen]
    pub fn bar_at_x(&self, x: f64) -> f64 {
        let w = self.canvas.width() as f64;
        if w <= 0.0 { return self.visible_start; }
        self.visible_start + (x / w) * (self.visible_end - self.visible_start)
    }

    /// Convert a price value to canvas Y coordinate (inverse of price_at_y).
    #[wasm_bindgen]
    pub fn y_at_price(&self, price: f64) -> f64 {
        let h = self.canvas.height() as f64;
        if h <= 0.0 { return 0.0; }
        let main_h = h * self.main_pane_height as f64;
        let price_range = self.max_price - self.min_price;
        if price_range <= 0.0 { return 0.0; }
        let t = (price - self.min_price) / price_range;
        main_h * (1.0 - t)
    }

    /// Convert a bar index to canvas X coordinate (inverse of bar_at_x).
    #[wasm_bindgen]
    pub fn x_at_bar(&self, bar: f64) -> f64 {
        let w = self.canvas.width() as f64;
        if w <= 0.0 { return 0.0; }
        let bar_range = self.visible_end - self.visible_start;
        if bar_range <= 0.0 { return 0.0; }
        (bar - self.visible_start) / bar_range * w
    }

    // ── Rendering ───────────────────────────────────────────────

    #[wasm_bindgen]
    pub fn render(&self) {
        let w = self.canvas.width() as i32;
        let h = self.canvas.height() as i32;
        self.gl.viewport(0, 0, w, h);
        self.gl.clear(GL::COLOR_BUFFER_BIT);

        // Main chart pane (top portion)
        let main_h = (h as f32 * self.main_pane_height) as i32;
        let pane_area_h = h - main_h;
        self.gl.viewport(0, pane_area_h, w, main_h);

        self.render_grid();

        match self.chart_type {
            ChartType::Line => self.render_line_chart(),
            _ => self.render_candles(w, main_h),
        }

        self.render_fills();
        self.render_histograms();
        self.render_lines();
        self.render_drawings();
        self.render_price_lines();

        // Sub-panes (bottom portion)
        self.render_panes(w, h);
    }

    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.canvas.set_width(width);
        self.canvas.set_height(height);
    }

    #[wasm_bindgen]
    pub fn get_price_labels(&self) -> Vec<f64> {
        let h = self.canvas.height() as f64 * self.main_pane_height as f64;
        let price_range = self.max_price - self.min_price;
        if price_range <= 0.0 { return vec![]; }
        let step = nice_step(price_range, 6.0);
        let first = (self.min_price / step).ceil() * step;
        let mut result = Vec::new();
        let mut p = first;
        while p < self.max_price {
            let y = h - ((p - self.min_price) / price_range * h);
            result.push(p);
            result.push(y);
            p += step;
        }
        result
    }

    #[wasm_bindgen]
    pub fn get_time_labels(&self) -> Vec<f64> {
        let w = self.canvas.width() as f64;
        let range = self.visible_end - self.visible_start;
        if range <= 0.0 { return vec![]; }
        let step = nice_step(range, 8.0);
        let first = (self.visible_start / step).ceil() * step;
        let mut result = Vec::new();
        let mut t = first;
        while t < self.visible_end {
            let x = (t - self.visible_start) / range * w;
            result.push(t);
            result.push(x);
            t += step;
        }
        result
    }

    #[wasm_bindgen]
    pub fn get_crosshair_data(&self, canvas_x: f64, canvas_y: f64) -> Vec<f64> {
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;
        if w <= 0.0 || h <= 0.0 { return vec![]; }

        let price = self.price_at_y(canvas_y);
        let bar_f = self.bar_at_x(canvas_x);
        if bar_f.is_nan() || bar_f < 0.0 {
            return vec![price, bar_f, 0.0, 0.0, 0.0, 0.0];
        }
        let bar_idx = bar_f.round() as usize;
        if bar_idx >= self.bar_opens.len() {
            return vec![price, bar_f, 0.0, 0.0, 0.0, 0.0];
        }
        vec![
            price, bar_f,
            self.bar_opens[bar_idx] as f64,
            self.bar_highs[bar_idx] as f64,
            self.bar_lows[bar_idx] as f64,
            self.bar_closes[bar_idx] as f64,
        ]
    }

    #[wasm_bindgen]
    pub fn render_crosshair(&self, canvas_x: f64, canvas_y: f64) {
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;
        if w <= 0.0 || h <= 0.0 { return; }
        let x_ndc = (canvas_x / w * 2.0 - 1.0) as f32;
        let y_ndc = (1.0 - canvas_y / h * 2.0) as f32;

        let h_verts: [f32; 4] = [-1.0, y_ndc, 1.0, y_ndc];
        let v_verts: [f32; 4] = [x_ndc, -1.0, x_ndc, 1.0];

        // Reset viewport to full canvas for crosshair
        self.gl.viewport(0, 0, w as i32, h as i32);
        self.gl.use_program(Some(&self.grid_program));
        self.gl.uniform4f(self.grid_color.as_ref(), 0.6, 0.6, 0.6, 0.5);

        if let Some(buf) = &self.gl.create_buffer() {
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(buf));
            unsafe {
                let view = js_sys::Float32Array::view(&h_verts);
                self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            self.gl.draw_arrays(GL::LINES, 0, 2);
            unsafe {
                let view = js_sys::Float32Array::view(&v_verts);
                self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }
            self.gl.draw_arrays(GL::LINES, 0, 2);

            self.gl.disable_vertex_attrib_array(0);
            self.gl.bind_buffer(GL::ARRAY_BUFFER, None);
            self.gl.delete_buffer(Some(buf));
        }
    }

    #[wasm_bindgen]
    pub fn get_price_range(&self) -> Vec<f64> { vec![self.min_price, self.max_price] }

    #[wasm_bindgen]
    pub fn get_time_range(&self) -> Vec<f64> { vec![self.visible_start, self.visible_end] }

    #[wasm_bindgen]
    pub fn visible_bars(&self) -> f64 { self.visible_end - self.visible_start }

    #[wasm_bindgen]
    pub fn total_bar_count(&self) -> usize { self.total_bars }
}

// ══════════════════════════════════════════════════════════════
// INTERNAL RENDERING
// ══════════════════════════════════════════════════════════════

impl GpuChart {
    fn rebuild_geometry(&mut self) {
        match self.chart_type {
            ChartType::Candles => self.build_candle_geometry(),
            ChartType::HeikinAshi => self.build_heikin_ashi_geometry(),
            ChartType::Line => self.build_line_chart_geometry(),
            ChartType::Bars => self.build_ohlc_bars_geometry(),
            ChartType::Renko => self.build_renko_geometry(),
        }
    }

    fn render_line_chart(&self) {
        if self.line_chart_count < 2 { return; }
        self.gl.use_program(Some(&self.line_program));
        self.gl.uniform2f(self.line_price_range.as_ref(), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(self.line_time_range.as_ref(), self.visible_start as f32, self.visible_end as f32);
        self.gl.uniform4f(self.line_color.as_ref(), 0.30, 0.69, 0.31, 1.0);
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.line_chart_vbo));
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
        self.gl.draw_arrays(GL::LINE_STRIP, 0, self.line_chart_count);
    }

    fn build_line_chart_geometry(&mut self) {
        let n = self.total_bars;
        let mut vertices: Vec<f32> = Vec::with_capacity(n * 2);
        for i in 0..n {
            vertices.push(i as f32);
            vertices.push(self.bar_closes[i]);
        }
        self.line_chart_count = n as i32;
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.line_chart_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
    }

    fn build_heikin_ashi_geometry(&mut self) {
        let n = self.total_bars;
        if n == 0 { return; }
        let mut ha_o = Vec::with_capacity(n);
        let mut ha_h = Vec::with_capacity(n);
        let mut ha_l = Vec::with_capacity(n);
        let mut ha_c = Vec::with_capacity(n);

        for i in 0..n {
            let c = (self.bar_opens[i] + self.bar_highs[i] + self.bar_lows[i] + self.bar_closes[i]) / 4.0;
            let o = if i == 0 { (self.bar_opens[0] + self.bar_closes[0]) / 2.0 } else { (ha_o[i-1] + ha_c[i-1]) / 2.0 };
            ha_o.push(o); ha_h.push(self.bar_highs[i].max(o).max(c));
            ha_l.push(self.bar_lows[i].min(o).min(c)); ha_c.push(c);
        }

        let mut body_verts: Vec<f32> = Vec::with_capacity(n * 6 * 3);
        let mut wick_verts: Vec<f32> = Vec::with_capacity(n * 4 * 3);
        for i in 0..n {
            let x = i as f32;
            let (o, h, l, c) = (ha_o[i], ha_h[i], ha_l[i], ha_c[i]);
            let bullish = if c >= o { 1.0f32 } else { 0.0 };
            let (bt, bb) = (o.max(c), o.min(c));
            let hw = 0.35;
            body_verts.extend_from_slice(&[x-hw,bt,bullish, x+hw,bt,bullish, x+hw,bb,bullish]);
            body_verts.extend_from_slice(&[x-hw,bt,bullish, x+hw,bb,bullish, x-hw,bb,bullish]);
            wick_verts.extend_from_slice(&[x,bt,0.5, x,h,0.5, x,bb,0.5, x,l,0.5]);
        }
        self.upload_candle_data(&body_verts, &wick_verts, n);
    }

    fn build_candle_geometry(&mut self) {
        let n = self.total_bars;
        let mut body_verts: Vec<f32> = Vec::with_capacity(n * 6 * 3);
        let mut wick_verts: Vec<f32> = Vec::with_capacity(n * 4 * 3);

        for i in 0..n {
            let x = i as f32;
            let (o, h, l, c) = (self.bar_opens[i], self.bar_highs[i], self.bar_lows[i], self.bar_closes[i]);
            let bullish = if c >= o { 1.0f32 } else { 0.0 };
            let (bt, bb) = (o.max(c), o.min(c));
            let hw = 0.35;
            body_verts.extend_from_slice(&[x-hw,bt,bullish, x+hw,bt,bullish, x+hw,bb,bullish]);
            body_verts.extend_from_slice(&[x-hw,bt,bullish, x+hw,bb,bullish, x-hw,bb,bullish]);
            wick_verts.extend_from_slice(&[x,bt,0.5, x,h,0.5, x,bb,0.5, x,l,0.5]);
        }
        self.upload_candle_data(&body_verts, &wick_verts, n);
    }

    fn upload_candle_data(&mut self, body_verts: &[f32], wick_verts: &[f32], n: usize) {
        self.candle_vertex_count = (n * 6) as i32;
        self.wick_vertex_count = (n * 4) as i32;
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.candle_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(body_verts);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.wick_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(wick_verts);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
    }

    fn build_ohlc_bars_geometry(&mut self) {
        let n = self.total_bars;
        let mut vertices: Vec<f32> = Vec::with_capacity(n * 6 * 3);
        for i in 0..n {
            let x = i as f32;
            let (o, h, l, c) = (self.bar_opens[i], self.bar_highs[i], self.bar_lows[i], self.bar_closes[i]);
            let bullish = if c >= o { 1.0f32 } else { 0.0 };
            let hw = 0.3;
            vertices.extend_from_slice(&[x,h,bullish, x,l,bullish]);
            vertices.extend_from_slice(&[x-hw,o,bullish, x,o,bullish]);
            vertices.extend_from_slice(&[x,c,bullish, x+hw,c,bullish]);
        }
        self.candle_vertex_count = (n * 6) as i32;
        self.wick_vertex_count = 0;
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.candle_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
    }

    fn build_renko_geometry(&mut self) {
        let n = self.total_bars;
        if n < 15 { return; }
        let period = 14;
        let mut trs = Vec::with_capacity(n);
        for i in 1..n {
            let tr = (self.bar_highs[i] - self.bar_lows[i])
                .max((self.bar_highs[i] - self.bar_closes[i-1]).abs())
                .max((self.bar_lows[i] - self.bar_closes[i-1]).abs());
            trs.push(tr);
        }
        let mut atr = trs[..period.min(trs.len())].iter().sum::<f32>() / period as f32;
        for i in period..trs.len() { atr = (atr * (period as f32 - 1.0) + trs[i]) / period as f32; }
        if atr <= 0.0 { return; }

        struct Brick { x: f32, top: f32, bot: f32, bull: bool }
        let mut bricks: Vec<Brick> = Vec::new();
        let mut base = self.bar_closes[0];
        let mut brick_x = 0.0f32;
        for i in 1..n {
            let price = self.bar_closes[i];
            while price >= base + atr { bricks.push(Brick { x: brick_x, top: base + atr, bot: base, bull: true }); base += atr; brick_x += 1.0; }
            while price <= base - atr { bricks.push(Brick { x: brick_x, top: base, bot: base - atr, bull: false }); base -= atr; brick_x += 1.0; }
        }
        self.total_bars = bricks.len();
        if bricks.is_empty() { self.candle_vertex_count = 0; return; }

        let (mut min_p, mut max_p) = (f32::MAX, f32::MIN);
        for b in &bricks { min_p = min_p.min(b.bot); max_p = max_p.max(b.top); }
        let padding = (max_p - min_p) * 0.05;
        self.min_price = (min_p - padding) as f64;
        self.max_price = (max_p + padding) as f64;
        self.visible_start = if bricks.len() > 100 { (bricks.len() - 100) as f64 } else { 0.0 };
        self.visible_end = bricks.len() as f64 + 2.0;

        let mut vertices: Vec<f32> = Vec::with_capacity(bricks.len() * 6 * 3);
        for b in &bricks {
            let bullish = if b.bull { 1.0f32 } else { 0.0 };
            let hw = 0.4;
            vertices.extend_from_slice(&[b.x-hw,b.top,bullish, b.x+hw,b.top,bullish, b.x+hw,b.bot,bullish]);
            vertices.extend_from_slice(&[b.x-hw,b.top,bullish, b.x+hw,b.bot,bullish, b.x-hw,b.bot,bullish]);
        }
        self.candle_vertex_count = (bricks.len() * 6) as i32;
        self.wick_vertex_count = 0;
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.candle_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
    }

    fn build_grid_geometry(&mut self) {
        let price_range = self.max_price - self.min_price;
        if price_range <= 0.0 { return; }
        let step = nice_step(price_range, 6.0);
        let first = (self.min_price / step).ceil() * step;
        let mut vertices: Vec<f32> = Vec::new();
        let mut p = first;
        while p < self.max_price {
            let y_ndc = ((p - self.min_price) / price_range * 2.0 - 1.0) as f32;
            vertices.extend_from_slice(&[-1.0, y_ndc, 1.0, y_ndc]);
            p += step;
        }
        self.grid_vertex_count = (vertices.len() / 2) as i32;
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.grid_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
    }

    fn render_grid(&self) {
        if self.grid_vertex_count == 0 { return; }
        self.gl.use_program(Some(&self.grid_program));
        self.gl.uniform4f(self.grid_color.as_ref(), 0.15, 0.15, 0.2, 1.0);
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.grid_vbo));
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
        self.gl.draw_arrays(GL::LINES, 0, self.grid_vertex_count);
    }

    fn render_candles(&self, _w: i32, _h: i32) {
        if self.candle_vertex_count == 0 { return; }
        self.gl.use_program(Some(&self.candle_program));
        self.gl.uniform2f(self.candle_viewport.as_ref(), self.canvas.width() as f32, self.canvas.height() as f32);
        self.gl.uniform2f(self.candle_price_range.as_ref(), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(self.candle_time_range.as_ref(), self.visible_start as f32, self.visible_end as f32);
        self.gl.uniform3f(self.candle_bull_color.as_ref(), 0.30, 0.69, 0.31);
        self.gl.uniform3f(self.candle_bear_color.as_ref(), 0.96, 0.26, 0.21);
        self.gl.uniform3f(self.candle_wick_color.as_ref(), 0.6, 0.6, 0.6);

        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.candle_vbo));
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 12, 0);
        self.gl.enable_vertex_attrib_array(1);
        self.gl.vertex_attrib_pointer_with_i32(1, 1, GL::FLOAT, false, 12, 8);

        match self.chart_type {
            ChartType::Bars => { self.gl.draw_arrays(GL::LINES, 0, self.candle_vertex_count); }
            ChartType::Renko => { self.gl.draw_arrays(GL::TRIANGLES, 0, self.candle_vertex_count); }
            _ => {
                self.gl.draw_arrays(GL::TRIANGLES, 0, self.candle_vertex_count);
                if self.wick_vertex_count > 0 {
                    self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.wick_vbo));
                    self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 12, 0);
                    self.gl.vertex_attrib_pointer_with_i32(1, 1, GL::FLOAT, false, 12, 8);
                    self.gl.draw_arrays(GL::LINES, 0, self.wick_vertex_count);
                }
            }
        }
    }

    fn render_lines(&self) {
        if self.line_buffers.is_empty() { return; }
        self.gl.use_program(Some(&self.line_program));
        self.gl.uniform2f(self.line_price_range.as_ref(), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(self.line_time_range.as_ref(), self.visible_start as f32, self.visible_end as f32);
        for line in &self.line_buffers {
            self.gl.line_width(line.width.max(1.0));
            self.gl.uniform4f(self.line_color.as_ref(), line.color[0], line.color[1], line.color[2], line.color[3]);
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&line.vbo));
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            match line.style {
                LineStyle::Solid => {
                    self.gl.draw_arrays(GL::LINE_STRIP, 0, line.count);
                }
                LineStyle::Dashed => {
                    // Draw every other segment (pairs of points) for dashed effect
                    // Dash pattern: 4 on, 4 off (in bar units)
                    let dash_on = 4i32;
                    let dash_off = 4i32;
                    let cycle = dash_on + dash_off;
                    let mut start = 0i32;
                    while start < line.count {
                        let end = (start + dash_on).min(line.count);
                        if end > start {
                            self.gl.draw_arrays(GL::LINE_STRIP, start, end - start);
                        }
                        start += cycle;
                    }
                }
                LineStyle::Dotted => {
                    // Draw every other segment (pairs of points) for dotted effect
                    // Dot pattern: 1 on, 2 off
                    let dot_on = 2i32;
                    let dot_off = 2i32;
                    let cycle = dot_on + dot_off;
                    let mut start = 0i32;
                    while start < line.count {
                        let end = (start + dot_on).min(line.count);
                        if end > start {
                            self.gl.draw_arrays(GL::LINE_STRIP, start, end - start);
                        }
                        start += cycle;
                    }
                }
            }
        }
        self.gl.line_width(1.0);
    }

    fn render_histograms(&self) {
        if self.histogram_buffers.is_empty() { return; }
        self.gl.use_program(Some(&self.vertex_color_program));
        self.gl.uniform2f(self.vc_price_range.as_ref(), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(self.vc_time_range.as_ref(), self.visible_start as f32, self.visible_end as f32);
        for hb in &self.histogram_buffers {
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&hb.vbo));
            self.gl.enable_vertex_attrib_array(0);
            // stride = 6 floats (2 pos + 4 color) = 24 bytes
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 24, 0);
            self.gl.enable_vertex_attrib_array(1);
            self.gl.vertex_attrib_pointer_with_i32(1, 4, GL::FLOAT, false, 24, 8);
            self.gl.draw_arrays(GL::TRIANGLES, 0, hb.vertex_count);
            self.gl.disable_vertex_attrib_array(1);
        }
    }

    fn render_fills(&self) {
        if self.fill_buffers.is_empty() { return; }
        self.gl.use_program(Some(&self.line_program));
        self.gl.uniform2f(self.line_price_range.as_ref(), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(self.line_time_range.as_ref(), self.visible_start as f32, self.visible_end as f32);
        for fb in &self.fill_buffers {
            self.gl.uniform4f(self.line_color.as_ref(), fb.color[0], fb.color[1], fb.color[2], fb.color[3]);
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&fb.vbo));
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            self.gl.draw_arrays(GL::TRIANGLE_STRIP, 0, fb.vertex_count);
        }
    }

    // ── Drawing Tool Rendering (Phase 1 + 3) ────────────────────

    fn render_drawings(&self) {
        if self.drawings.is_empty() { return; }
        self.gl.use_program(Some(&self.line_program));
        self.gl.uniform2f(self.line_price_range.as_ref(), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(self.line_time_range.as_ref(), self.visible_start as f32, self.visible_end as f32);

        for d in &self.drawings {
            if d.points.len() < 2 { continue; }
            let (b0, p0) = (d.points[0] as f32, d.points[1] as f32);
            let (b1, p1) = if d.points.len() >= 4 { (d.points[2] as f32, d.points[3] as f32) } else { (b0, p0) };

            self.gl.line_width(d.line_width.max(1.0));
            self.gl.uniform4f(self.line_color.as_ref(), d.color[0], d.color[1], d.color[2], d.color[3]);

            match d.draw_type {
                DrawType::Horizontal => {
                    self.draw_line_verts(&[self.visible_start as f32, p0, self.visible_end as f32, p0]);
                }
                DrawType::Vertical => {
                    self.draw_line_verts(&[b0, self.min_price as f32, b0, self.max_price as f32]);
                }
                DrawType::TrendLine | DrawType::Segment | DrawType::GannLine => {
                    self.draw_line_verts(&[b0, p0, b1, p1]);
                }
                DrawType::Ray => {
                    let db = b1 - b0;
                    let dp = p1 - p0;
                    let ext = self.visible_end as f32;
                    let t = if db.abs() > 1e-10 { (ext - b0) / db } else { 100.0 };
                    self.draw_line_verts(&[b0, p0, b0 + db * t, p0 + dp * t]);
                }
                DrawType::ExtendedLine => {
                    let db = b1 - b0;
                    let dp = p1 - p0;
                    let t_fwd = if db.abs() > 1e-10 { (self.visible_end as f32 - b0) / db } else { 100.0 };
                    let t_bak = if db.abs() > 1e-10 { (self.visible_start as f32 - b0) / db } else { -100.0 };
                    self.draw_line_verts(&[b0 + db * t_bak, p0 + dp * t_bak, b0 + db * t_fwd, p0 + dp * t_fwd]);
                }
                DrawType::ArrowLine => {
                    self.draw_line_verts(&[b0, p0, b1, p1]);
                    // Arrowhead rendered as 2 extra lines
                    let db = b1 - b0;
                    let dp = p1 - p0;
                    let len = (db * db + dp * dp).sqrt().max(0.001);
                    let ux = db / len;
                    let uy = dp / len;
                    let arrow_len = len * 0.1;
                    let ax = b1 - ux * arrow_len + uy * arrow_len * 0.5;
                    let ay = p1 - uy * arrow_len - ux * arrow_len * 0.5;
                    let bx = b1 - ux * arrow_len - uy * arrow_len * 0.5;
                    let by = p1 - uy * arrow_len + ux * arrow_len * 0.5;
                    self.draw_line_verts(&[b1, p1, ax, ay]);
                    self.draw_line_verts(&[b1, p1, bx, by]);
                }
                DrawType::Rectangle | DrawType::RiskReward | DrawType::PositionBox |
                DrawType::DateRange | DrawType::PriceRange => {
                    // Fill
                    if d.fill_color[3] > 0.0 {
                        self.gl.uniform4f(self.line_color.as_ref(), d.fill_color[0], d.fill_color[1], d.fill_color[2], d.fill_color[3]);
                        self.draw_filled_rect(b0, p0, b1, p1);
                        self.gl.uniform4f(self.line_color.as_ref(), d.color[0], d.color[1], d.color[2], d.color[3]);
                    }
                    // Border
                    self.draw_line_verts(&[b0,p0, b1,p0, b1,p0, b1,p1, b1,p1, b0,p1, b0,p1, b0,p0]);
                }
                DrawType::Triangle => {
                    if d.points.len() >= 6 {
                        let (b2, p2) = (d.points[4] as f32, d.points[5] as f32);
                        if d.fill_color[3] > 0.0 {
                            self.gl.uniform4f(self.line_color.as_ref(), d.fill_color[0], d.fill_color[1], d.fill_color[2], d.fill_color[3]);
                            self.draw_triangle_fill(b0,p0, b1,p1, b2,p2);
                            self.gl.uniform4f(self.line_color.as_ref(), d.color[0], d.color[1], d.color[2], d.color[3]);
                        }
                        self.draw_line_verts(&[b0,p0, b1,p1, b1,p1, b2,p2, b2,p2, b0,p0]);
                    }
                }
                DrawType::Circle | DrawType::Ellipse => {
                    self.draw_ellipse(b0, p0, (b1-b0).abs(), (p1-p0).abs(), d.draw_type == DrawType::Circle, &d.fill_color);
                }
                DrawType::Channel | DrawType::ParallelChannel | DrawType::EquidistantChannel => {
                    let offset = if d.points.len() >= 6 { d.points[5] as f32 - d.points[1] as f32 } else { (p1-p0).abs() * 0.5 };
                    self.draw_line_verts(&[b0,p0, b1,p1]); // Main line
                    self.draw_line_verts(&[b0,p0+offset, b1,p1+offset]); // Parallel
                    if d.fill_color[3] > 0.0 && matches!(d.draw_type, DrawType::ParallelChannel) {
                        self.gl.uniform4f(self.line_color.as_ref(), d.fill_color[0], d.fill_color[1], d.fill_color[2], d.fill_color[3]);
                        self.draw_filled_rect(b0, p0, b1, p1+offset);
                        self.gl.uniform4f(self.line_color.as_ref(), d.color[0], d.color[1], d.color[2], d.color[3]);
                    }
                }
                DrawType::Pitchfork | DrawType::SchiffPitchfork => {
                    if d.points.len() >= 6 {
                        let (b2, p2) = (d.points[4] as f32, d.points[5] as f32);
                        let mid_b = (b1 + b2) / 2.0;
                        let mid_p = (p1 + p2) / 2.0;
                        // Median line from pivot through midpoint, extending right
                        let db = mid_b - b0;
                        let dp = mid_p - p0;
                        let ext_t = if db.abs() > 1e-10 { (self.visible_end as f32 - b0) / db } else { 10.0 };
                        self.draw_line_verts(&[b0, p0, b0 + db * ext_t, p0 + dp * ext_t]);
                        // Prongs
                        self.draw_line_verts(&[b1, p1, b1 + db * ext_t, p1 + dp * ext_t]);
                        self.draw_line_verts(&[b2, p2, b2 + db * ext_t, p2 + dp * ext_t]);
                    }
                }
                DrawType::Fibonacci | DrawType::FibChannel | DrawType::FibExtension => {
                    let levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.272, 1.618];
                    let range = p1 - p0;
                    for &lvl in &levels {
                        let y = p0 + range * lvl as f32;
                        self.draw_line_verts(&[b0.min(b1), y, b0.max(b1), y]);
                    }
                }
                DrawType::FibFan => {
                    let levels = [0.236, 0.382, 0.5, 0.618, 0.786];
                    let range = p1 - p0;
                    for &lvl in &levels {
                        let target_p = p0 + range * lvl as f32;
                        let db = b1 - b0;
                        let dp = target_p - p0;
                        let ext_t = if db.abs() > 1e-10 { (self.visible_end as f32 - b0) / db } else { 10.0 };
                        self.draw_line_verts(&[b0, p0, b0 + db * ext_t.max(1.0), p0 + dp * ext_t.max(1.0)]);
                    }
                }
                DrawType::FibArcs => {
                    for &ratio in &[0.382f32, 0.5, 0.618] {
                        let dist_b = (b1 - b0).abs();
                        let dist_p = (p1 - p0).abs();
                        let rb = dist_b * ratio;
                        let rp = dist_p * ratio;
                        self.draw_ellipse(b1, p1, rb, rp, false, &[0.0; 4]);
                    }
                }
                DrawType::GannFan => {
                    let angles = [0.125, 0.25, 0.333, 0.5, 1.0, 2.0, 3.0, 4.0, 8.0];
                    let scale_p = (p1 - p0).abs().max(1.0);
                    let scale_b = (b1 - b0).abs().max(1.0);
                    let unit = scale_p / scale_b;
                    for &a in &angles {
                        let end_b = self.visible_end as f32;
                        let end_p = p0 + (end_b - b0) * unit * a;
                        self.draw_line_verts(&[b0, p0, end_b, end_p]);
                    }
                }
                DrawType::GannBox => {
                    // Grid between p1→p2 with Gann divisions
                    let steps = [0.0, 0.25, 0.5, 0.75, 1.0];
                    for &s in &steps {
                        let y = p0 + (p1 - p0) * s;
                        self.draw_line_verts(&[b0, y, b1, y]);
                        let x = b0 + (b1 - b0) * s;
                        self.draw_line_verts(&[x, p0, x, p1]);
                    }
                    // Diagonals
                    self.draw_line_verts(&[b0,p0, b1,p1]);
                    self.draw_line_verts(&[b0,p1, b1,p0]);
                }
                DrawType::CycleLines => {
                    let interval = (b1 - b0).abs().max(1.0);
                    let mut x = b0;
                    while x <= self.visible_end as f32 {
                        self.draw_line_verts(&[x, self.min_price as f32, x, self.max_price as f32]);
                        x += interval;
                    }
                }
                DrawType::ArrowUp | DrawType::ArrowDown => {
                    let size = (self.max_price - self.min_price) as f32 * 0.015;
                    let tip_p = p0;
                    let base_p = if d.draw_type == DrawType::ArrowUp { p0 - size } else { p0 + size };
                    let hw = (self.visible_end - self.visible_start) as f32 * 0.008;
                    self.draw_triangle_fill(b0, tip_p, b0 - hw, base_p, b0 + hw, base_p);
                }
                DrawType::RegressionChannel | DrawType::StdDevChannel => {
                    // Linear regression through visible close prices
                    let s = (b0.min(b1) as usize).max(0);
                    let e = (b0.max(b1) as usize).min(self.bar_closes.len());
                    if e > s + 2 {
                        let (slope, intercept, std_dev) = linear_regression(&self.bar_closes[s..e]);
                        let n_bars = (e - s) as f32;
                        let start_b = s as f32;
                        let end_b = (e - 1) as f32;
                        let y0r = intercept;
                        let y1r = intercept + slope * (n_bars - 1.0);
                        // Center line
                        self.draw_line_verts(&[start_b, y0r, end_b, y1r]);
                        // ±1 std dev channels
                        let dev = if d.draw_type == DrawType::StdDevChannel { std_dev * 2.0 } else { std_dev };
                        self.draw_line_verts(&[start_b, y0r + dev, end_b, y1r + dev]);
                        self.draw_line_verts(&[start_b, y0r - dev, end_b, y1r - dev]);
                    }
                }
                DrawType::SpeedLines => {
                    let range = (p1 - p0).abs();
                    for &frac in &[0.333f32, 0.5, 0.667] {
                        let target = p0 + (p1 - p0) * frac;
                        let db = b1 - b0;
                        let dp = target - p0;
                        let ext = self.visible_end as f32;
                        let t = if db.abs() > 1e-10 { (ext - b0) / db } else { 10.0 };
                        self.draw_line_verts(&[b0, p0, b0 + db * t.max(1.0), p0 + dp * t.max(1.0)]);
                    }
                }
                DrawType::ElliottImpulse | DrawType::ElliottCorrective => {
                    // Zigzag lines connecting all points
                    let mut i = 0;
                    while i + 3 < d.points.len() {
                        let ba = d.points[i] as f32;
                        let pa = d.points[i+1] as f32;
                        let bb = d.points[i+2] as f32;
                        let pb = d.points[i+3] as f32;
                        self.draw_line_verts(&[ba, pa, bb, pb]);
                        i += 2;
                    }
                    // Labels rendered via Canvas2D text overlay (not in GPU)
                }
                DrawType::PriceLabel | DrawType::TextLabel => {
                    // Text labels rendered via Canvas2D text overlay
                    // GPU just draws a small marker
                    let size = (self.max_price - self.min_price) as f32 * 0.005;
                    self.draw_line_verts(&[b0 - 0.5, p0, b0 + 0.5, p0]);
                }
            }
            self.gl.line_width(1.0);
        }
    }

    // ── Price Line Rendering (Phase 2) ──────────────────────────

    fn render_price_lines(&self) {
        if self.price_lines.is_empty() { return; }
        self.gl.use_program(Some(&self.line_program));
        self.gl.uniform2f(self.line_price_range.as_ref(), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(self.line_time_range.as_ref(), self.visible_start as f32, self.visible_end as f32);

        for pl in &self.price_lines {
            self.gl.line_width(pl.line_width.max(1.0));
            self.gl.uniform4f(self.line_color.as_ref(), pl.color[0], pl.color[1], pl.color[2], pl.color[3]);
            let p = pl.price as f32;
            self.draw_line_verts(&[self.visible_start as f32, p, self.visible_end as f32, p]);
        }
        self.gl.line_width(1.0);
    }

    // ── Sub-Pane Rendering (Phase 4) ────────────────────────────

    fn recalc_pane_layout(&mut self) {
        let total_pane_height: f32 = self.panes.iter().map(|p| p.height).sum();
        self.main_pane_height = (1.0 - total_pane_height).max(0.3);
        let mut y = 0.0f32;
        for pane in self.panes.iter_mut().rev() {
            pane.y_start = y;
            y += pane.height;
        }
    }

    fn render_panes(&self, w: i32, h: i32) {
        for pane in &self.panes {
            let py = (pane.y_start * h as f32) as i32;
            let ph = (pane.height * h as f32) as i32;
            if ph <= 0 { continue; }
            self.gl.viewport(0, py, w, ph);

            // Pane grid (zero line + border)
            self.gl.use_program(Some(&self.grid_program));
            self.gl.uniform4f(self.grid_color.as_ref(), 0.2, 0.2, 0.25, 1.0);
            // Zero line
            let val_range = pane.max_value - pane.min_value;
            if val_range > 0.0 {
                let zero_ndc = ((0.0 - pane.min_value) / val_range * 2.0 - 1.0) as f32;
                if zero_ndc > -1.0 && zero_ndc < 1.0 {
                    if let Some(buf) = &self.gl.create_buffer() {
                        let verts = [-1.0f32, zero_ndc, 1.0, zero_ndc];
                        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(buf));
                        unsafe {
                            let view = js_sys::Float32Array::view(&verts);
                            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
                        }
                        self.gl.enable_vertex_attrib_array(0);
                        self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
                        self.gl.draw_arrays(GL::LINES, 0, 2);
                        self.gl.delete_buffer(Some(buf));
                    }
                }
            }

            // Top border
            self.gl.uniform4f(self.grid_color.as_ref(), 0.3, 0.3, 0.35, 1.0);
            if let Some(buf) = &self.gl.create_buffer() {
                let verts = [-1.0f32, 1.0, 1.0, 1.0];
                self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(buf));
                unsafe {
                    let view = js_sys::Float32Array::view(&verts);
                    self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
                }
                self.gl.enable_vertex_attrib_array(0);
                self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
                self.gl.draw_arrays(GL::LINES, 0, 2);
                self.gl.delete_buffer(Some(buf));
            }

            // Pane line series
            self.gl.use_program(Some(&self.line_program));
            self.gl.uniform2f(self.line_price_range.as_ref(), pane.min_value as f32, pane.max_value as f32);
            self.gl.uniform2f(self.line_time_range.as_ref(), self.visible_start as f32, self.visible_end as f32);

            for (vbo, count, color) in &pane.lines {
                self.gl.uniform4f(self.line_color.as_ref(), color[0], color[1], color[2], color[3]);
                self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(vbo));
                self.gl.enable_vertex_attrib_array(0);
                self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
                self.gl.draw_arrays(GL::LINE_STRIP, 0, *count);
            }

            // Pane histogram series (uses candle program for coloring)
            for (vbo, count) in &pane.histograms {
                self.gl.use_program(Some(&self.candle_program));
                self.gl.uniform2f(self.candle_viewport.as_ref(), w as f32, ph as f32);
                self.gl.uniform2f(self.candle_price_range.as_ref(), pane.min_value as f32, pane.max_value as f32);
                self.gl.uniform2f(self.candle_time_range.as_ref(), self.visible_start as f32, self.visible_end as f32);
                self.gl.uniform3f(self.candle_bull_color.as_ref(), 0.30, 0.69, 0.31);
                self.gl.uniform3f(self.candle_bear_color.as_ref(), 0.96, 0.26, 0.21);
                self.gl.uniform3f(self.candle_wick_color.as_ref(), 0.6, 0.6, 0.6);
                self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(vbo));
                self.gl.enable_vertex_attrib_array(0);
                self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 12, 0);
                self.gl.enable_vertex_attrib_array(1);
                self.gl.vertex_attrib_pointer_with_i32(1, 1, GL::FLOAT, false, 12, 8);
                self.gl.draw_arrays(GL::TRIANGLES, 0, *count);
            }
        }
    }

    // ── Drawing Geometry Helpers ─────────────────────────────────

    /// Draw a line from vertices [x0,y0, x1,y1, ...] using a temp buffer.
    fn draw_line_verts(&self, verts: &[f32]) {
        if let Some(buf) = &self.gl.create_buffer() {
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(buf));
            unsafe {
                let view = js_sys::Float32Array::view(verts);
                self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            self.gl.draw_arrays(GL::LINES, 0, (verts.len() / 2) as i32);
            self.gl.delete_buffer(Some(buf));
        }
    }

    /// Draw a filled rectangle as 2 triangles.
    fn draw_filled_rect(&self, b0: f32, p0: f32, b1: f32, p1: f32) {
        let verts = [b0,p0, b1,p0, b1,p1, b0,p0, b1,p1, b0,p1];
        if let Some(buf) = &self.gl.create_buffer() {
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(buf));
            unsafe {
                let view = js_sys::Float32Array::view(&verts);
                self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            self.gl.draw_arrays(GL::TRIANGLES, 0, 6);
            self.gl.delete_buffer(Some(buf));
        }
    }

    /// Draw a filled triangle.
    fn draw_triangle_fill(&self, b0: f32, p0: f32, b1: f32, p1: f32, b2: f32, p2: f32) {
        let verts = [b0,p0, b1,p1, b2,p2];
        if let Some(buf) = &self.gl.create_buffer() {
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(buf));
            unsafe {
                let view = js_sys::Float32Array::view(&verts);
                self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            self.gl.draw_arrays(GL::TRIANGLES, 0, 3);
            self.gl.delete_buffer(Some(buf));
        }
    }

    /// Draw an ellipse outline (and optional fill) using line segments.
    fn draw_ellipse(&self, cx: f32, cy: f32, rx: f32, ry: f32, is_circle: bool, fill: &[f32; 4]) {
        let segments = 48;
        let ry_actual = if is_circle { rx } else { ry };
        let mut line_verts: Vec<f32> = Vec::with_capacity((segments + 1) * 2);
        let mut fill_verts: Vec<f32> = Vec::new();

        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let x = cx + rx * angle.cos();
            let y = cy + ry_actual * angle.sin();
            line_verts.push(x);
            line_verts.push(y);
        }

        // Outline
        if let Some(buf) = &self.gl.create_buffer() {
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(buf));
            unsafe {
                let view = js_sys::Float32Array::view(&line_verts);
                self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            self.gl.draw_arrays(GL::LINE_STRIP, 0, (segments + 1) as i32);
            self.gl.delete_buffer(Some(buf));
        }

        // Fill
        if fill[3] > 0.0 {
            fill_verts.reserve((segments + 1) * 2 + 2);
            fill_verts.push(cx);
            fill_verts.push(cy);
            fill_verts.extend_from_slice(&line_verts);

            self.gl.uniform4f(self.line_color.as_ref(), fill[0], fill[1], fill[2], fill[3]);
            if let Some(buf) = &self.gl.create_buffer() {
                self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(buf));
                unsafe {
                    let view = js_sys::Float32Array::view(&fill_verts);
                    self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
                }
                self.gl.enable_vertex_attrib_array(0);
                self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
                self.gl.draw_arrays(GL::TRIANGLE_FAN, 0, (segments + 2) as i32);
                self.gl.delete_buffer(Some(buf));
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// UTILITIES
// ══════════════════════════════════════════════════════════════

fn compile_shader(gl: &GL, shader_type: u32, source: &str) -> Result<web_sys::WebGlShader, JsValue> {
    let shader = gl.create_shader(shader_type).ok_or("Failed to create shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if !gl.get_shader_parameter(&shader, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        let info = gl.get_shader_info_log(&shader).unwrap_or_default();
        return Err(JsValue::from_str(&format!("Shader compile error: {info}")));
    }
    Ok(shader)
}

fn compile_program(gl: &GL, vs_src: &str, fs_src: &str) -> Result<WebGlProgram, JsValue> {
    let vs = compile_shader(gl, GL::VERTEX_SHADER, vs_src)?;
    let fs = compile_shader(gl, GL::FRAGMENT_SHADER, fs_src)?;
    let program = gl.create_program().ok_or("Failed to create program")?;
    gl.attach_shader(&program, &vs);
    gl.attach_shader(&program, &fs);
    gl.link_program(&program);
    if !gl.get_program_parameter(&program, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        let info = gl.get_program_info_log(&program).unwrap_or_default();
        return Err(JsValue::from_str(&format!("Program link error: {info}")));
    }
    gl.delete_shader(Some(&vs));
    gl.delete_shader(Some(&fs));
    Ok(program)
}

fn nice_step(range: f64, target_lines: f64) -> f64 {
    let rough = range / target_lines;
    let mag = 10.0_f64.powf(rough.log10().floor());
    let norm = rough / mag;
    let step = if norm < 1.5 { 1.0 } else if norm < 3.0 { 2.0 } else if norm < 7.0 { 5.0 } else { 10.0 };
    step * mag
}

/// Point-to-line-segment distance in normalized coordinates.
fn point_to_segment_dist(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64, tol_x: f64, tol_y: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-20 {
        let nx = (px - x1) / tol_x;
        let ny = (py - y1) / tol_y;
        return (nx * nx + ny * ny).sqrt();
    }
    let t = ((px - x1) * dx + (py - y1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    let nx = (px - proj_x) / tol_x;
    let ny = (py - proj_y) / tol_y;
    (nx * nx + ny * ny).sqrt()
}

/// Linear regression: returns (slope, intercept, std_dev).
fn linear_regression(data: &[f32]) -> (f32, f32, f32) {
    let n = data.len() as f32;
    if n < 2.0 { return (0.0, data.first().copied().unwrap_or(0.0), 0.0); }
    let mut sum_x = 0.0f32;
    let mut sum_y = 0.0f32;
    let mut sum_xy = 0.0f32;
    let mut sum_xx = 0.0f32;
    for (i, &y) in data.iter().enumerate() {
        let x = i as f32;
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_xx += x * x;
    }
    let denom = n * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-10 { return (0.0, sum_y / n, 0.0); }
    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n;
    // Standard deviation of residuals
    let mut sum_sq = 0.0f32;
    for (i, &y) in data.iter().enumerate() {
        let predicted = intercept + slope * i as f32;
        let residual = y - predicted;
        sum_sq += residual * residual;
    }
    let std_dev = (sum_sq / n).sqrt();
    (slope, intercept, std_dev)
}
