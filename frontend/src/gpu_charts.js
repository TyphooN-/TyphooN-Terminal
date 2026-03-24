/* @ts-self-types="./typhoon_gpu_charts.d.ts" */

/**
 * @enum {0 | 1 | 2 | 3 | 4}
 */
export const ChartType = Object.freeze({
    Candles: 0, "0": "Candles",
    HeikinAshi: 1, "1": "HeikinAshi",
    Line: 2, "2": "Line",
    Bars: 3, "3": "Bars",
    Renko: 4, "4": "Renko",
});

/**
 * Drawing tool types — matches frontend drawing type strings.
 * @enum {0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 | 24 | 25 | 26 | 27 | 28 | 29 | 30 | 31 | 32 | 33 | 34 | 35 | 36 | 37}
 */
export const DrawType = Object.freeze({
    TrendLine: 0, "0": "TrendLine",
    Ray: 1, "1": "Ray",
    Segment: 2, "2": "Segment",
    ExtendedLine: 3, "3": "ExtendedLine",
    ArrowLine: 4, "4": "ArrowLine",
    Horizontal: 5, "5": "Horizontal",
    Vertical: 6, "6": "Vertical",
    Rectangle: 7, "7": "Rectangle",
    Triangle: 8, "8": "Triangle",
    Circle: 9, "9": "Circle",
    Ellipse: 10, "10": "Ellipse",
    Channel: 11, "11": "Channel",
    ParallelChannel: 12, "12": "ParallelChannel",
    Pitchfork: 13, "13": "Pitchfork",
    SchiffPitchfork: 14, "14": "SchiffPitchfork",
    Fibonacci: 15, "15": "Fibonacci",
    FibFan: 16, "16": "FibFan",
    FibArcs: 17, "17": "FibArcs",
    FibChannel: 18, "18": "FibChannel",
    FibExtension: 19, "19": "FibExtension",
    GannFan: 20, "20": "GannFan",
    GannLine: 21, "21": "GannLine",
    GannBox: 22, "22": "GannBox",
    CycleLines: 23, "23": "CycleLines",
    ArrowUp: 24, "24": "ArrowUp",
    ArrowDown: 25, "25": "ArrowDown",
    PriceLabel: 26, "26": "PriceLabel",
    TextLabel: 27, "27": "TextLabel",
    RegressionChannel: 28, "28": "RegressionChannel",
    StdDevChannel: 29, "29": "StdDevChannel",
    RiskReward: 30, "30": "RiskReward",
    PositionBox: 31, "31": "PositionBox",
    DateRange: 32, "32": "DateRange",
    PriceRange: 33, "33": "PriceRange",
    ElliottImpulse: 34, "34": "ElliottImpulse",
    ElliottCorrective: 35, "35": "ElliottCorrective",
    SpeedLines: 36, "36": "SpeedLines",
    EquidistantChannel: 37, "37": "EquidistantChannel",
});

export class GpuChart {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GpuChartFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_gpuchart_free(ptr, 0);
    }
    /**
     * Add a drawing. Points are flat [bar0, price0, bar1, price1, ...].
     * Color is [r, g, b, a]. Fill color is [r, g, b, a] (alpha=0 for no fill).
     * @param {DrawType} draw_type
     * @param {Float64Array} points
     * @param {Float32Array} color
     * @param {number} line_width
     * @param {Float32Array} fill
     */
    add_drawing(draw_type, points, color, line_width, fill) {
        const ptr0 = passArrayF64ToWasm0(points, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(color, wasm.__wbindgen_export2);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passArrayF32ToWasm0(fill, wasm.__wbindgen_export2);
        const len2 = WASM_VECTOR_LEN;
        wasm.gpuchart_add_drawing(this.__wbg_ptr, draw_type, ptr0, len0, ptr1, len1, line_width, ptr2, len2);
    }
    /**
     * Add a filled area between two price-level series.
     * `top_values` and `bottom_values` are one value per bar (same length).
     * Color is uniform for the entire fill.
     * @param {Float64Array} top_values
     * @param {Float64Array} bottom_values
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     */
    add_fill(top_values, bottom_values, r, g, b, a) {
        const ptr0 = passArrayF64ToWasm0(top_values, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF64ToWasm0(bottom_values, wasm.__wbindgen_export2);
        const len1 = WASM_VECTOR_LEN;
        wasm.gpuchart_add_fill(this.__wbg_ptr, ptr0, len0, ptr1, len1, r, g, b, a);
    }
    /**
     * Add a histogram series to the main chart.
     * `values`: one value per bar.
     * `colors`: flat [r,g,b,a, r,g,b,a, ...] per bar (must be values.len() * 4).
     * `base`: the zero/baseline value.
     * @param {Float64Array} values
     * @param {Float32Array} colors
     * @param {number} base
     */
    add_histogram(values, colors, base) {
        const ptr0 = passArrayF64ToWasm0(values, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayF32ToWasm0(colors, wasm.__wbindgen_export2);
        const len1 = WASM_VECTOR_LEN;
        wasm.gpuchart_add_histogram(this.__wbg_ptr, ptr0, len0, ptr1, len1, base);
    }
    /**
     * Add a solid indicator line (convenience wrapper).
     * @param {Float64Array} values
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     */
    add_line(values, r, g, b, a) {
        const ptr0 = passArrayF64ToWasm0(values, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.gpuchart_add_line(this.__wbg_ptr, ptr0, len0, r, g, b, a);
    }
    /**
     * Add a styled indicator line with custom width and dash style.
     * @param {Float64Array} values
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @param {number} width
     * @param {LineStyle} style
     */
    add_line_styled(values, r, g, b, a, width, style) {
        const ptr0 = passArrayF64ToWasm0(values, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.gpuchart_add_line_styled(this.__wbg_ptr, ptr0, len0, r, g, b, a, width, style);
    }
    /**
     * Add a sub-pane below the main chart. Returns pane index.
     * `height` is fraction of total canvas (e.g., 0.15 = 15%).
     * @param {number} height
     * @returns {number}
     */
    add_pane(height) {
        const ret = wasm.gpuchart_add_pane(this.__wbg_ptr, height);
        return ret >>> 0;
    }
    /**
     * Add a histogram series to a sub-pane. Data is flat [value, colorFlag, value, colorFlag, ...].
     * @param {number} pane
     * @param {Float64Array} data
     */
    add_pane_histogram(pane, data) {
        const ptr0 = passArrayF64ToWasm0(data, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.gpuchart_add_pane_histogram(this.__wbg_ptr, pane, ptr0, len0);
    }
    /**
     * Add a line series to a sub-pane. Values are indicator values (one per bar).
     * @param {number} pane
     * @param {Float64Array} values
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     */
    add_pane_line(pane, values, r, g, b, a) {
        const ptr0 = passArrayF64ToWasm0(values, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.gpuchart_add_pane_line(this.__wbg_ptr, pane, ptr0, len0, r, g, b, a);
    }
    /**
     * Add a price line (SL/TP/entry). Returns index.
     * @param {number} price
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     * @param {number} line_width
     * @returns {number}
     */
    add_price_line(price, r, g, b, a, line_width) {
        const ret = wasm.gpuchart_add_price_line(this.__wbg_ptr, price, r, g, b, a, line_width);
        return ret >>> 0;
    }
    /**
     * @param {number} x
     * @returns {number}
     */
    bar_at_x(x) {
        const ret = wasm.gpuchart_bar_at_x(this.__wbg_ptr, x);
        return ret;
    }
    /**
     * Clear all drawings.
     */
    clear_drawings() {
        wasm.gpuchart_clear_drawings(this.__wbg_ptr);
    }
    /**
     * Clear all main-pane fill areas.
     */
    clear_fills() {
        wasm.gpuchart_clear_fills(this.__wbg_ptr);
    }
    /**
     * Clear all main-pane histogram series.
     */
    clear_histograms() {
        wasm.gpuchart_clear_histograms(this.__wbg_ptr);
    }
    clear_lines() {
        wasm.gpuchart_clear_lines(this.__wbg_ptr);
    }
    /**
     * Clear all data from a sub-pane.
     * @param {number} pane
     */
    clear_pane(pane) {
        wasm.gpuchart_clear_pane(this.__wbg_ptr, pane);
    }
    /**
     * Remove all sub-panes.
     */
    clear_panes() {
        wasm.gpuchart_clear_panes(this.__wbg_ptr);
    }
    /**
     * Clear all price lines.
     */
    clear_price_lines() {
        wasm.gpuchart_clear_price_lines(this.__wbg_ptr);
    }
    /**
     * Get drawing count.
     * @returns {number}
     */
    drawing_count() {
        const ret = wasm.gpuchart_drawing_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} canvas_x
     * @param {number} canvas_y
     * @returns {Float64Array}
     */
    get_crosshair_data(canvas_x, canvas_y) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.gpuchart_get_crosshair_data(retptr, this.__wbg_ptr, canvas_x, canvas_y);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var v1 = getArrayF64FromWasm0(r0, r1).slice();
            wasm.__wbindgen_export4(r0, r1 * 8, 8);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * @returns {Float64Array}
     */
    get_price_labels() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.gpuchart_get_price_labels(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var v1 = getArrayF64FromWasm0(r0, r1).slice();
            wasm.__wbindgen_export4(r0, r1 * 8, 8);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Get price of a price line (for drag read-back).
     * @param {number} index
     * @returns {number}
     */
    get_price_line_price(index) {
        const ret = wasm.gpuchart_get_price_line_price(this.__wbg_ptr, index);
        return ret;
    }
    /**
     * @returns {Float64Array}
     */
    get_price_range() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.gpuchart_get_price_range(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var v1 = getArrayF64FromWasm0(r0, r1).slice();
            wasm.__wbindgen_export4(r0, r1 * 8, 8);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * @returns {Float64Array}
     */
    get_time_labels() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.gpuchart_get_time_labels(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var v1 = getArrayF64FromWasm0(r0, r1).slice();
            wasm.__wbindgen_export4(r0, r1 * 8, 8);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * @returns {Float64Array}
     */
    get_time_range() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.gpuchart_get_time_range(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var v1 = getArrayF64FromWasm0(r0, r1).slice();
            wasm.__wbindgen_export4(r0, r1 * 8, 8);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Hit-test: returns drawing index at canvas (x, y), or -1 if none.
     * @param {number} canvas_x
     * @param {number} canvas_y
     * @param {number} tolerance
     * @returns {number}
     */
    hit_test_drawing(canvas_x, canvas_y, tolerance) {
        const ret = wasm.gpuchart_hit_test_drawing(this.__wbg_ptr, canvas_x, canvas_y, tolerance);
        return ret;
    }
    /**
     * Hit-test price lines. Returns index or -1.
     * @param {number} canvas_y
     * @param {number} tolerance
     * @returns {number}
     */
    hit_test_price_line(canvas_y, tolerance) {
        const ret = wasm.gpuchart_hit_test_price_line(this.__wbg_ptr, canvas_y, tolerance);
        return ret;
    }
    /**
     * @param {string} canvas_id
     */
    constructor(canvas_id) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(canvas_id, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
            const len0 = WASM_VECTOR_LEN;
            wasm.gpuchart_new(retptr, ptr0, len0);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            this.__wbg_ptr = r0 >>> 0;
            GpuChartFinalization.register(this, this.__wbg_ptr, this);
            return this;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * @param {number} y
     * @returns {number}
     */
    price_at_y(y) {
        const ret = wasm.gpuchart_price_at_y(this.__wbg_ptr, y);
        return ret;
    }
    /**
     * Get price line count.
     * @returns {number}
     */
    price_line_count() {
        const ret = wasm.gpuchart_price_line_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Remove drawing by index.
     * @param {number} index
     */
    remove_drawing(index) {
        wasm.gpuchart_remove_drawing(this.__wbg_ptr, index);
    }
    /**
     * Remove a price line by index.
     * @param {number} index
     */
    remove_price_line(index) {
        wasm.gpuchart_remove_price_line(this.__wbg_ptr, index);
    }
    render() {
        wasm.gpuchart_render(this.__wbg_ptr);
    }
    /**
     * @param {number} canvas_x
     * @param {number} canvas_y
     */
    render_crosshair(canvas_x, canvas_y) {
        wasm.gpuchart_render_crosshair(this.__wbg_ptr, canvas_x, canvas_y);
    }
    /**
     * @param {number} width
     * @param {number} height
     */
    resize(width, height) {
        wasm.gpuchart_resize(this.__wbg_ptr, width, height);
    }
    /**
     * @param {number} delta
     */
    scroll(delta) {
        wasm.gpuchart_scroll(this.__wbg_ptr, delta);
    }
    /**
     * @param {ChartType} ct
     */
    set_chart_type(ct) {
        wasm.gpuchart_set_chart_type(this.__wbg_ptr, ct);
    }
    /**
     * @param {Float64Array} data
     */
    set_data(data) {
        const ptr0 = passArrayF64ToWasm0(data, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.gpuchart_set_data(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Set value range for a sub-pane.
     * @param {number} pane
     * @param {number} min_val
     * @param {number} max_val
     */
    set_pane_range(pane, min_val, max_val) {
        wasm.gpuchart_set_pane_range(this.__wbg_ptr, pane, min_val, max_val);
    }
    /**
     * @param {number} start
     * @param {number} end
     */
    set_visible_range(start, end) {
        wasm.gpuchart_set_visible_range(this.__wbg_ptr, start, end);
    }
    /**
     * @returns {number}
     */
    total_bar_count() {
        const ret = wasm.gpuchart_total_bar_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Update a drawing's color.
     * @param {number} index
     * @param {Float32Array} color
     */
    update_drawing_color(index, color) {
        const ptr0 = passArrayF32ToWasm0(color, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.gpuchart_update_drawing_color(this.__wbg_ptr, index, ptr0, len0);
    }
    /**
     * Update a drawing's points.
     * @param {number} index
     * @param {Float64Array} points
     */
    update_drawing_points(index, points) {
        const ptr0 = passArrayF64ToWasm0(points, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        wasm.gpuchart_update_drawing_points(this.__wbg_ptr, index, ptr0, len0);
    }
    /**
     * Update the last bar's OHLC data and rebuild only its geometry.
     * Avoids rebuilding the entire candle buffer for real-time tick updates.
     * @param {number} open
     * @param {number} high
     * @param {number} low
     * @param {number} close
     */
    update_last_bar(open, high, low, close) {
        wasm.gpuchart_update_last_bar(this.__wbg_ptr, open, high, low, close);
    }
    /**
     * Update a price line's price (for dragging).
     * @param {number} index
     * @param {number} price
     */
    update_price_line(index, price) {
        wasm.gpuchart_update_price_line(this.__wbg_ptr, index, price);
    }
    /**
     * @returns {number}
     */
    visible_bars() {
        const ret = wasm.gpuchart_visible_bars(this.__wbg_ptr);
        return ret;
    }
    /**
     * Convert a bar index to canvas X coordinate (inverse of bar_at_x).
     * @param {number} bar
     * @returns {number}
     */
    x_at_bar(bar) {
        const ret = wasm.gpuchart_x_at_bar(this.__wbg_ptr, bar);
        return ret;
    }
    /**
     * Convert a price value to canvas Y coordinate (inverse of price_at_y).
     * @param {number} price
     * @returns {number}
     */
    y_at_price(price) {
        const ret = wasm.gpuchart_y_at_price(this.__wbg_ptr, price);
        return ret;
    }
    /**
     * @param {number} factor
     * @param {number} center_x
     */
    zoom(factor, center_x) {
        wasm.gpuchart_zoom(this.__wbg_ptr, factor, center_x);
    }
}
if (Symbol.dispose) GpuChart.prototype[Symbol.dispose] = GpuChart.prototype.free;

/**
 * Line style for indicator lines.
 * 0 = solid, 1 = dashed, 2 = dotted.
 * @enum {0 | 1 | 2}
 */
export const LineStyle = Object.freeze({
    Solid: 0, "0": "Solid",
    Dashed: 1, "1": "Dashed",
    Dotted: 2, "2": "Dotted",
});

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_boolean_get_c0f3f60bac5a78d1: function(arg0) {
            const v = getObject(arg0);
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_is_undefined_52709e72fb9f179c: function(arg0) {
            const ret = getObject(arg0) === undefined;
            return ret;
        },
        __wbg___wbindgen_throw_6ddd609b62940d55: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_attachShader_e557f37438249ff7: function(arg0, arg1, arg2) {
            getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
        },
        __wbg_bindBuffer_142694a9732bc098: function(arg0, arg1, arg2) {
            getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
        },
        __wbg_blendFunc_2e98c5f57736e5f3: function(arg0, arg1, arg2) {
            getObject(arg0).blendFunc(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_bufferData_d20232e3d5dcdc62: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
        },
        __wbg_clearColor_080c8446c8438f8e: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearColor(arg1, arg2, arg3, arg4);
        },
        __wbg_clear_3d6ad4729e206aac: function(arg0, arg1) {
            getObject(arg0).clear(arg1 >>> 0);
        },
        __wbg_compileShader_7ca66245c2798601: function(arg0, arg1) {
            getObject(arg0).compileShader(getObject(arg1));
        },
        __wbg_createBuffer_1aa34315dc9585a2: function(arg0) {
            const ret = getObject(arg0).createBuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createProgram_1fa32901e4db13cd: function(arg0) {
            const ret = getObject(arg0).createProgram();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createShader_a00913b8c6489e6b: function(arg0, arg1) {
            const ret = getObject(arg0).createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_deleteBuffer_b053c58b4ed1ab1c: function(arg0, arg1) {
            getObject(arg0).deleteBuffer(getObject(arg1));
        },
        __wbg_deleteShader_5b6992b5e5894d44: function(arg0, arg1) {
            getObject(arg0).deleteShader(getObject(arg1));
        },
        __wbg_disableVertexAttribArray_124a165b099b763b: function(arg0, arg1) {
            getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_document_c0320cd4183c6d9b: function(arg0) {
            const ret = getObject(arg0).document;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_drawArrays_c20dedf441392005: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
        },
        __wbg_enableVertexAttribArray_60dadea3a00e104a: function(arg0, arg1) {
            getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_enable_91dff7f43064bb54: function(arg0, arg1) {
            getObject(arg0).enable(arg1 >>> 0);
        },
        __wbg_getContext_f04bf8f22dcb2d53: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_getElementById_d1f25d287b19a833: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getElementById(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_getProgramInfoLog_50443ddea7475f57: function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getProgramParameter_46e2d49878b56edd: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_getShaderInfoLog_22f9e8c90a52f38d: function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getShaderParameter_46f64f7ca5d534db: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_getUniformLocation_5eb08673afa04eee: function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_height_6568c4427c3b889d: function(arg0) {
            const ret = getObject(arg0).height;
            return ret;
        },
        __wbg_instanceof_HtmlCanvasElement_26125339f936be50: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof HTMLCanvasElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_WebGl2RenderingContext_349f232f715e6bc2: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof WebGL2RenderingContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_23e677d2c6843922: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_lineWidth_1b57aff251eb2695: function(arg0, arg1) {
            getObject(arg0).lineWidth(arg1);
        },
        __wbg_linkProgram_b969f67969a850b5: function(arg0, arg1) {
            getObject(arg0).linkProgram(getObject(arg1));
        },
        __wbg_set_height_b6548a01bdcb689a: function(arg0, arg1) {
            getObject(arg0).height = arg1 >>> 0;
        },
        __wbg_set_width_c0fcaa2da53cd540: function(arg0, arg1) {
            getObject(arg0).width = arg1 >>> 0;
        },
        __wbg_shaderSource_2bca0edc97475e95: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
        },
        __wbg_static_accessor_GLOBAL_8adb955bd33fac2f: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_ad356e0db91c7913: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_static_accessor_SELF_f207c857566db248: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_static_accessor_WINDOW_bb9f1ba69d61b386: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_uniform2f_8fc2c40c50fd770c: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform2f(getObject(arg1), arg2, arg3);
        },
        __wbg_uniform3f_1f319f9f4d116e54: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniform3f(getObject(arg1), arg2, arg3, arg4);
        },
        __wbg_uniform4f_0b00a34f4789ad14: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).uniform4f(getObject(arg1), arg2, arg3, arg4, arg5);
        },
        __wbg_useProgram_5405b431988b837b: function(arg0, arg1) {
            getObject(arg0).useProgram(getObject(arg1));
        },
        __wbg_vertexAttribPointer_ea73fc4cc5b7d647: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        },
        __wbg_viewport_b60aceadb9166023: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).viewport(arg1, arg2, arg3, arg4);
        },
        __wbg_width_4d6fc7fecd877217: function(arg0) {
            const ret = getObject(arg0).width;
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(F32)) -> NamedExternref("Float32Array")`.
            const ret = getArrayF32FromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_object_clone_ref: function(arg0) {
            const ret = getObject(arg0);
            return addHeapObject(ret);
        },
        __wbindgen_object_drop_ref: function(arg0) {
            takeObject(arg0);
        },
    };
    return {
        __proto__: null,
        "./typhoon_gpu_charts_bg.js": import0,
    };
}

const GpuChartFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_gpuchart_free(ptr >>> 0, 1));

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function dropObject(idx) {
    if (idx < 1028) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getObject(idx) { return heap[idx]; }

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_export(addHeapObject(e));
    }
}

let heap = new Array(1024).fill(undefined);
heap.push(undefined, null, true, false);

let heap_next = heap.length;

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArrayF32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getFloat32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayF64ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 8, 8) >>> 0;
    getFloat64ArrayMemory0().set(arg, ptr / 8);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedFloat64ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('typhoon_gpu_charts_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
