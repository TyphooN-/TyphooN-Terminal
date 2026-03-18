//! TyphooN GPU Charts — WebGL2-accelerated candlestick renderer.
//!
//! Renders candlesticks, indicator lines, price scale, time axis, and crosshair
//! entirely on the GPU via WebGL2. Compiled to Wasm for use in Tauri WebView.
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

// Per-vertex: x position, y_top, y_bottom
layout(location = 0) in vec2 a_pos;
layout(location = 1) in float a_color_flag; // 1.0 = bullish, 0.0 = bearish, 0.5 = wick

uniform vec2 u_viewport;    // canvas width, height
uniform vec2 u_price_range;  // min_price, max_price
uniform vec2 u_time_range;   // min_time_idx, max_time_idx
uniform float u_candle_width;

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

uniform vec3 u_bull_color;   // default green
uniform vec3 u_bear_color;   // default red
uniform vec3 u_wick_color;   // default gray

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

// Grid shader (dotted lines for price levels)
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
// CHART ENGINE
// ══════════════════════════════════════════════════════════════

/// Chart rendering mode.
#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq)]
pub enum ChartType {
    Candles = 0,
    HeikinAshi = 1,
    Line = 2,
    Bars = 3,     // OHLC bars
    Renko = 4,
}

#[wasm_bindgen]
pub struct GpuChart {
    gl: GL,
    canvas: HtmlCanvasElement,
    candle_program: WebGlProgram,
    line_program: WebGlProgram,
    grid_program: WebGlProgram,
    // Uniform locations
    candle_viewport: WebGlUniformLocation,
    candle_price_range: WebGlUniformLocation,
    candle_time_range: WebGlUniformLocation,
    candle_width: WebGlUniformLocation,
    candle_bull_color: WebGlUniformLocation,
    candle_bear_color: WebGlUniformLocation,
    candle_wick_color: WebGlUniformLocation,
    line_price_range: WebGlUniformLocation,
    line_time_range: WebGlUniformLocation,
    line_color: WebGlUniformLocation,
    grid_color: WebGlUniformLocation,
    // Buffers
    candle_vbo: WebGlBuffer,
    candle_vertex_count: i32,
    // Line chart buffer (separate from candle VBO)
    line_chart_vbo: WebGlBuffer,
    line_chart_count: i32,
    // Chart type
    chart_type: ChartType,
    // View state
    min_price: f64,
    max_price: f64,
    visible_start: f64,
    visible_end: f64,
    total_bars: usize,
    // Bar data (kept for interaction + Heikin-Ashi/Renko computation)
    bar_opens: Vec<f32>,
    bar_highs: Vec<f32>,
    bar_lows: Vec<f32>,
    bar_closes: Vec<f32>,
    // Indicator line buffers
    line_buffers: Vec<(WebGlBuffer, i32, [f32; 4])>, // (vbo, vertex_count, rgba)
    // Grid lines
    grid_vbo: WebGlBuffer,
    grid_vertex_count: i32,
}

#[wasm_bindgen]
impl GpuChart {
    /// Create a new GPU chart on the given canvas element ID.
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<GpuChart, JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("Canvas not found"))?
            .dyn_into::<HtmlCanvasElement>()?;

        let gl = canvas.get_context("webgl2")?
            .ok_or_else(|| JsValue::from_str("WebGL2 not supported"))?
            .dyn_into::<GL>()?;

        // Compile shaders
        let candle_program = compile_program(&gl, CANDLE_VS, CANDLE_FS)?;
        let line_program = compile_program(&gl, LINE_VS, LINE_FS)?;
        let grid_program = compile_program(&gl, GRID_VS, GRID_FS)?;

        // Get uniform locations
        let candle_viewport = gl.get_uniform_location(&candle_program, "u_viewport").unwrap();
        let candle_price_range = gl.get_uniform_location(&candle_program, "u_price_range").unwrap();
        let candle_time_range = gl.get_uniform_location(&candle_program, "u_time_range").unwrap();
        let candle_width = gl.get_uniform_location(&candle_program, "u_candle_width").unwrap();
        let candle_bull_color = gl.get_uniform_location(&candle_program, "u_bull_color").unwrap();
        let candle_bear_color = gl.get_uniform_location(&candle_program, "u_bear_color").unwrap();
        let candle_wick_color = gl.get_uniform_location(&candle_program, "u_wick_color").unwrap();
        let line_price_range = gl.get_uniform_location(&line_program, "u_price_range").unwrap();
        let line_time_range = gl.get_uniform_location(&line_program, "u_time_range").unwrap();
        let line_color = gl.get_uniform_location(&line_program, "u_line_color").unwrap();
        let grid_color = gl.get_uniform_location(&grid_program, "u_grid_color").unwrap();

        let candle_vbo = gl.create_buffer().ok_or("Failed to create buffer")?;
        let line_chart_vbo = gl.create_buffer().ok_or("Failed to create line chart buffer")?;
        let grid_vbo = gl.create_buffer().ok_or("Failed to create grid buffer")?;

        // Dark background
        gl.clear_color(0.04, 0.04, 0.08, 1.0);
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        Ok(GpuChart {
            gl, canvas,
            candle_program, line_program, grid_program,
            candle_viewport, candle_price_range, candle_time_range, candle_width,
            candle_bull_color, candle_bear_color, candle_wick_color,
            line_price_range, line_time_range, line_color,
            grid_color,
            candle_vbo, candle_vertex_count: 0,
            line_chart_vbo, line_chart_count: 0,
            chart_type: ChartType::Candles,
            min_price: 0.0, max_price: 100.0,
            visible_start: 0.0, visible_end: 100.0,
            total_bars: 0,
            bar_opens: vec![], bar_highs: vec![], bar_lows: vec![], bar_closes: vec![],
            line_buffers: vec![],
            grid_vbo, grid_vertex_count: 0,
        })
    }

    /// Load OHLCV bar data (flat f64 array: [O,H,L,C,V, O,H,L,C,V, ...]).
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

    /// Set chart type: Candles, HeikinAshi, Line, Bars, Renko.
    #[wasm_bindgen]
    pub fn set_chart_type(&mut self, ct: ChartType) {
        self.chart_type = ct;
        if self.total_bars > 0 {
            self.rebuild_geometry();
        }
    }

    /// Set visible range (bar indices).
    #[wasm_bindgen]
    pub fn set_visible_range(&mut self, start: f64, end: f64) {
        self.visible_start = start;
        self.visible_end = end;
        // Recalculate price range for visible bars
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

    /// Scroll by delta bars (positive = right, negative = left).
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

    /// Zoom in/out (factor > 1 = zoom in, < 1 = zoom out).
    #[wasm_bindgen]
    pub fn zoom(&mut self, factor: f64, center_x: f64) {
        let range = self.visible_end - self.visible_start;
        let center = self.visible_start + range * center_x;
        let new_range = (range / factor).max(10.0).min(self.total_bars as f64);
        self.visible_start = center - new_range * center_x;
        self.visible_end = self.visible_start + new_range;
        self.set_visible_range(self.visible_start, self.visible_end);
    }

    /// Add an indicator line overlay. Color as [r, g, b, a] (0-1 range).
    #[wasm_bindgen]
    pub fn add_line(&mut self, values: &[f64], r: f32, g: f32, b: f32, a: f32) {
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

        self.line_buffers.push((vbo, values.len() as i32, [r, g, b, a]));
    }

    /// Clear all indicator lines.
    #[wasm_bindgen]
    pub fn clear_lines(&mut self) {
        for (vbo, _, _) in &self.line_buffers {
            self.gl.delete_buffer(Some(vbo));
        }
        self.line_buffers.clear();
    }

    /// Get price at canvas Y coordinate.
    #[wasm_bindgen]
    pub fn price_at_y(&self, y: f64) -> f64 {
        let h = self.canvas.height() as f64;
        let t = 1.0 - y / h; // flip Y
        self.min_price + t * (self.max_price - self.min_price)
    }

    /// Get bar index at canvas X coordinate.
    #[wasm_bindgen]
    pub fn bar_at_x(&self, x: f64) -> f64 {
        let w = self.canvas.width() as f64;
        self.visible_start + (x / w) * (self.visible_end - self.visible_start)
    }

    /// Render the full chart.
    #[wasm_bindgen]
    pub fn render(&self) {
        let w = self.canvas.width() as i32;
        let h = self.canvas.height() as i32;
        self.gl.viewport(0, 0, w, h);
        self.gl.clear(GL::COLOR_BUFFER_BIT);

        self.render_grid();

        match self.chart_type {
            ChartType::Line => self.render_line_chart(),
            _ => self.render_candles(w, h), // Candles, HeikinAshi, Bars, Renko all use candle geometry
        }

        self.render_lines();
    }

    /// Resize canvas to container.
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.canvas.set_width(width);
        self.canvas.set_height(height);
    }

    /// Get OHLCV data for bar at index (for crosshair tooltip).
    /// Returns [open, high, low, close, volume] or empty if out of range.
    #[wasm_bindgen]
    pub fn get_bar_ohlcv(&self, idx: usize) -> Vec<f64> {
        if idx >= self.bar_opens.len() { return vec![]; }
        vec![
            self.bar_opens[idx] as f64,
            self.bar_highs[idx] as f64,
            self.bar_lows[idx] as f64,
            self.bar_closes[idx] as f64,
            0.0, // volume not stored in f32 arrays
        ]
    }

    /// Get price scale labels: returns [price0, y0, price1, y1, ...] in canvas coordinates.
    #[wasm_bindgen]
    pub fn get_price_labels(&self) -> Vec<f64> {
        let h = self.canvas.height() as f64;
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

    /// Get time scale labels: returns [bar_idx0, x0, bar_idx1, x1, ...] in canvas coordinates.
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

    /// Get crosshair data at canvas position: returns [price, bar_idx, open, high, low, close].
    #[wasm_bindgen]
    pub fn get_crosshair_data(&self, canvas_x: f64, canvas_y: f64) -> Vec<f64> {
        let price = self.price_at_y(canvas_y);
        let bar_f = self.bar_at_x(canvas_x);
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

    /// Render crosshair lines at given canvas coordinates.
    #[wasm_bindgen]
    pub fn render_crosshair(&self, canvas_x: f64, canvas_y: f64) {
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;
        // Convert to NDC
        let x_ndc = (canvas_x / w * 2.0 - 1.0) as f32;
        let y_ndc = (1.0 - canvas_y / h * 2.0) as f32; // flip Y

        // Horizontal line
        let h_verts: [f32; 4] = [-1.0, y_ndc, 1.0, y_ndc];
        // Vertical line
        let v_verts: [f32; 4] = [x_ndc, -1.0, x_ndc, 1.0];

        self.gl.use_program(Some(&self.grid_program));
        self.gl.uniform4f(Some(&self.grid_color), 0.6, 0.6, 0.6, 0.5); // semi-transparent gray

        let buf = self.gl.create_buffer();
        if let Some(buf) = &buf {
            // Horizontal
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(buf));
            unsafe {
                let view = js_sys::Float32Array::view(&h_verts);
                self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            self.gl.draw_arrays(GL::LINES, 0, 2);

            // Vertical
            unsafe {
                let view = js_sys::Float32Array::view(&v_verts);
                self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }
            self.gl.draw_arrays(GL::LINES, 0, 2);

            self.gl.delete_buffer(Some(buf));
        }
    }

    /// Get min/max price for current view.
    #[wasm_bindgen]
    pub fn get_price_range(&self) -> Vec<f64> {
        vec![self.min_price, self.max_price]
    }

    /// Get visible time range.
    #[wasm_bindgen]
    pub fn get_time_range(&self) -> Vec<f64> {
        vec![self.visible_start, self.visible_end]
    }

    /// Get current visible bar count.
    #[wasm_bindgen]
    pub fn visible_bars(&self) -> f64 {
        self.visible_end - self.visible_start
    }

    /// Get total bar count.
    #[wasm_bindgen]
    pub fn total_bar_count(&self) -> usize {
        self.total_bars
    }
}

// ── Internal methods ────────────────────────────────────────────

impl GpuChart {
    /// Rebuild geometry for the current chart type.
    fn rebuild_geometry(&mut self) {
        match self.chart_type {
            ChartType::Candles => self.build_candle_geometry(),
            ChartType::HeikinAshi => self.build_heikin_ashi_geometry(),
            ChartType::Line => self.build_line_chart_geometry(),
            ChartType::Bars => self.build_ohlc_bars_geometry(),
            ChartType::Renko => self.build_renko_geometry(),
        }
    }

    /// Render line chart (simple close-price line).
    fn render_line_chart(&self) {
        if self.line_chart_count < 2 { return; }
        self.gl.use_program(Some(&self.line_program));
        self.gl.uniform2f(Some(&self.line_price_range), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(Some(&self.line_time_range), self.visible_start as f32, self.visible_end as f32);
        self.gl.uniform4f(Some(&self.line_color), 0.30, 0.69, 0.31, 1.0); // green line
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.line_chart_vbo));
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
        self.gl.draw_arrays(GL::LINE_STRIP, 0, self.line_chart_count);
    }

    /// Build line chart geometry (close prices as LINE_STRIP).
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

    /// Build Heikin-Ashi geometry — smoothed candles.
    /// HA Close = (O+H+L+C)/4, HA Open = (prev_HA_O + prev_HA_C)/2
    /// HA High = max(H, HA_O, HA_C), HA Low = min(L, HA_O, HA_C)
    fn build_heikin_ashi_geometry(&mut self) {
        let n = self.total_bars;
        if n == 0 { return; }
        let mut ha_o = Vec::with_capacity(n);
        let mut ha_h = Vec::with_capacity(n);
        let mut ha_l = Vec::with_capacity(n);
        let mut ha_c = Vec::with_capacity(n);

        for i in 0..n {
            let c = (self.bar_opens[i] + self.bar_highs[i] + self.bar_lows[i] + self.bar_closes[i]) / 4.0;
            let o = if i == 0 {
                (self.bar_opens[0] + self.bar_closes[0]) / 2.0
            } else {
                (ha_o[i - 1] + ha_c[i - 1]) / 2.0
            };
            let h = self.bar_highs[i].max(o).max(c);
            let l = self.bar_lows[i].min(o).min(c);
            ha_o.push(o);
            ha_h.push(h);
            ha_l.push(l);
            ha_c.push(c);
        }

        // Build geometry using HA values (same structure as regular candles)
        let mut vertices: Vec<f32> = Vec::with_capacity(n * 10 * 3);
        for i in 0..n {
            let x = i as f32;
            let o = ha_o[i];
            let h = ha_h[i];
            let l = ha_l[i];
            let c = ha_c[i];
            let bullish = if c >= o { 1.0f32 } else { 0.0 };
            let body_top = o.max(c);
            let body_bot = o.min(c);
            let hw = 0.35;
            vertices.extend_from_slice(&[x - hw, body_top, bullish, x + hw, body_top, bullish, x + hw, body_bot, bullish]);
            vertices.extend_from_slice(&[x - hw, body_top, bullish, x + hw, body_bot, bullish, x - hw, body_bot, bullish]);
            vertices.extend_from_slice(&[x, body_top, 0.5, x, h, 0.5]);
            vertices.extend_from_slice(&[x, body_bot, 0.5, x, l, 0.5]);
        }
        self.candle_vertex_count = (n * 10) as i32;
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.candle_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
    }

    /// Build OHLC bars geometry — vertical line (H-L) + left tick (O) + right tick (C).
    fn build_ohlc_bars_geometry(&mut self) {
        let n = self.total_bars;
        // Each bar: 6 vertices (3 lines × 2 vertices each)
        let mut vertices: Vec<f32> = Vec::with_capacity(n * 6 * 3);
        for i in 0..n {
            let x = i as f32;
            let o = self.bar_opens[i];
            let h = self.bar_highs[i];
            let l = self.bar_lows[i];
            let c = self.bar_closes[i];
            let bullish = if c >= o { 1.0f32 } else { 0.0 };
            let hw = 0.3;
            // Vertical line: high to low
            vertices.extend_from_slice(&[x, h, bullish, x, l, bullish]);
            // Left tick: open
            vertices.extend_from_slice(&[x - hw, o, bullish, x, o, bullish]);
            // Right tick: close
            vertices.extend_from_slice(&[x, c, bullish, x + hw, c, bullish]);
        }
        self.candle_vertex_count = (n * 6) as i32;
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.candle_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
    }

    /// Build Renko geometry — fixed-size bricks based on ATR.
    fn build_renko_geometry(&mut self) {
        let n = self.total_bars;
        if n < 15 { return; }
        // Calculate ATR(14) for brick size
        let period = 14;
        let mut trs = Vec::with_capacity(n);
        for i in 1..n {
            let tr = (self.bar_highs[i] - self.bar_lows[i])
                .max((self.bar_highs[i] - self.bar_closes[i - 1]).abs())
                .max((self.bar_lows[i] - self.bar_closes[i - 1]).abs());
            trs.push(tr);
        }
        let mut atr = trs[..period.min(trs.len())].iter().sum::<f32>() / period as f32;
        for i in period..trs.len() {
            atr = (atr * (period as f32 - 1.0) + trs[i]) / period as f32;
        }
        let brick_size = atr;
        if brick_size <= 0.0 { return; }

        // Generate bricks
        struct Brick { x: f32, top: f32, bot: f32, bull: bool }
        let mut bricks: Vec<Brick> = Vec::new();
        let mut base = self.bar_closes[0];
        let mut brick_x = 0.0f32;
        for i in 1..n {
            let price = self.bar_closes[i];
            while price >= base + brick_size {
                bricks.push(Brick { x: brick_x, top: base + brick_size, bot: base, bull: true });
                base += brick_size;
                brick_x += 1.0;
            }
            while price <= base - brick_size {
                bricks.push(Brick { x: brick_x, top: base, bot: base - brick_size, bull: false });
                base -= brick_size;
                brick_x += 1.0;
            }
        }

        self.total_bars = bricks.len();
        if bricks.is_empty() { self.candle_vertex_count = 0; return; }

        // Update price range for renko
        let mut min_p = f32::MAX;
        let mut max_p = f32::MIN;
        for b in &bricks { min_p = min_p.min(b.bot); max_p = max_p.max(b.top); }
        let padding = (max_p - min_p) * 0.05;
        self.min_price = (min_p - padding) as f64;
        self.max_price = (max_p + padding) as f64;
        self.visible_start = if bricks.len() > 100 { (bricks.len() - 100) as f64 } else { 0.0 };
        self.visible_end = bricks.len() as f64 + 2.0;

        // Build box geometry (no wicks for Renko)
        let mut vertices: Vec<f32> = Vec::with_capacity(bricks.len() * 6 * 3);
        for b in &bricks {
            let bullish = if b.bull { 1.0f32 } else { 0.0 };
            let hw = 0.4;
            vertices.extend_from_slice(&[b.x - hw, b.top, bullish, b.x + hw, b.top, bullish, b.x + hw, b.bot, bullish]);
            vertices.extend_from_slice(&[b.x - hw, b.top, bullish, b.x + hw, b.bot, bullish, b.x - hw, b.bot, bullish]);
        }
        self.candle_vertex_count = (bricks.len() * 6) as i32;
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.candle_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
    }

    /// Build standard candlestick geometry.
    fn build_candle_geometry(&mut self) {
        let n = self.total_bars;
        // Each candle: 6 vertices for body (2 triangles) + 4 vertices for wicks (2 lines)
        // Vertex format: [x, y, color_flag] = 3 floats per vertex
        let mut vertices: Vec<f32> = Vec::with_capacity(n * 10 * 3);

        for i in 0..n {
            let x = i as f32;
            let o = self.bar_opens[i];
            let h = self.bar_highs[i];
            let l = self.bar_lows[i];
            let c = self.bar_closes[i];
            let bullish = if c >= o { 1.0f32 } else { 0.0 };
            let body_top = o.max(c);
            let body_bot = o.min(c);
            let hw = 0.35; // half-width of candle body

            // Body: 2 triangles (6 vertices)
            // Triangle 1: top-left, top-right, bottom-right
            vertices.extend_from_slice(&[x - hw, body_top, bullish]);
            vertices.extend_from_slice(&[x + hw, body_top, bullish]);
            vertices.extend_from_slice(&[x + hw, body_bot, bullish]);
            // Triangle 2: top-left, bottom-right, bottom-left
            vertices.extend_from_slice(&[x - hw, body_top, bullish]);
            vertices.extend_from_slice(&[x + hw, body_bot, bullish]);
            vertices.extend_from_slice(&[x - hw, body_bot, bullish]);

            // Upper wick: line from body_top to high
            vertices.extend_from_slice(&[x, body_top, 0.5]); // wick color
            vertices.extend_from_slice(&[x, h, 0.5]);

            // Lower wick: line from body_bot to low
            vertices.extend_from_slice(&[x, body_bot, 0.5]);
            vertices.extend_from_slice(&[x, l, 0.5]);
        }

        self.candle_vertex_count = (n * 10) as i32;

        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.candle_vbo));
        unsafe {
            let buf = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &buf, GL::STATIC_DRAW);
        }
    }

    fn build_grid_geometry(&mut self) {
        // Horizontal price grid lines (5-8 lines across visible range)
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
        self.gl.uniform4f(Some(&self.grid_color), 0.15, 0.15, 0.2, 1.0);

        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.grid_vbo));
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);

        self.gl.draw_arrays(GL::LINES, 0, self.grid_vertex_count);
    }

    fn render_candles(&self, _w: i32, _h: i32) {
        if self.candle_vertex_count == 0 { return; }

        self.gl.use_program(Some(&self.candle_program));
        self.gl.uniform2f(Some(&self.candle_viewport), self.canvas.width() as f32, self.canvas.height() as f32);
        self.gl.uniform2f(Some(&self.candle_price_range), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(Some(&self.candle_time_range), self.visible_start as f32, self.visible_end as f32);
        self.gl.uniform1f(Some(&self.candle_width), 0.7);
        self.gl.uniform3f(Some(&self.candle_bull_color), 0.30, 0.69, 0.31);
        self.gl.uniform3f(Some(&self.candle_bear_color), 0.96, 0.26, 0.21);
        self.gl.uniform3f(Some(&self.candle_wick_color), 0.6, 0.6, 0.6);

        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.candle_vbo));
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 12, 0);
        self.gl.enable_vertex_attrib_array(1);
        self.gl.vertex_attrib_pointer_with_i32(1, 1, GL::FLOAT, false, 12, 8);

        match self.chart_type {
            ChartType::Bars => {
                // OHLC bars: all lines (3 lines × 2 vertices = 6 per bar)
                self.gl.draw_arrays(GL::LINES, 0, self.candle_vertex_count);
            }
            ChartType::Renko => {
                // Renko: triangles only (6 per brick, no wicks)
                self.gl.draw_arrays(GL::TRIANGLES, 0, self.candle_vertex_count);
            }
            _ => {
                // Candles / Heikin-Ashi: triangles (body) + lines (wicks)
                let body_count = (self.total_bars * 6) as i32;
                self.gl.draw_arrays(GL::TRIANGLES, 0, body_count);
                for i in 0..self.total_bars {
                    let offset = (i * 10 + 6) as i32;
                    self.gl.draw_arrays(GL::LINES, offset, 4);
                }
            }
        }
    }

    fn render_lines(&self) {
        if self.line_buffers.is_empty() { return; }
        self.gl.use_program(Some(&self.line_program));
        self.gl.uniform2f(Some(&self.line_price_range), self.min_price as f32, self.max_price as f32);
        self.gl.uniform2f(Some(&self.line_time_range), self.visible_start as f32, self.visible_end as f32);

        for (vbo, count, color) in &self.line_buffers {
            self.gl.uniform4f(Some(&self.line_color), color[0], color[1], color[2], color[3]);
            self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(vbo));
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            self.gl.draw_arrays(GL::LINE_STRIP, 0, *count);
        }
    }
}

// ── Utilities ───────────────────────────────────────────────────

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

/// Calculate a "nice" step size for grid lines.
fn nice_step(range: f64, target_lines: f64) -> f64 {
    let rough = range / target_lines;
    let mag = 10.0_f64.powf(rough.log10().floor());
    let norm = rough / mag;
    let step = if norm < 1.5 { 1.0 } else if norm < 3.0 { 2.0 } else if norm < 7.0 { 5.0 } else { 10.0 };
    step * mag
}
