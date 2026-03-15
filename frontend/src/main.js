/**
 * TyphooN Terminal — Frontend
 *
 * Lightweight-charts candlestick rendering with draggable SL/TP lines.
 * Communicates with Rust backend via Tauri invoke().
 *
 * Matches TyphooN EA workflow:
 * 1. Drag SL/TP lines to desired levels
 * 2. Click "Open Trade" — system calculates lots and places order
 */

import { createChart, CrosshairMode } from "lightweight-charts";
import { createWindow, openArticleWindow, openFundamentalsWindow, openFilingsWindow, tileWindows, closeAllWindows } from "./windows.js";
import "./windows.css";

// ── Logging ─────────────────────────────────────────────────

function log(msg, level = "info") {
  const content = document.getElementById("log-content");
  if (!content) { console.log(`[${level}] ${msg}`); return; }
  const entry = document.createElement("div");
  const time = new Date().toLocaleTimeString("en-GB", { hour12: false });
  entry.className = `log-entry log-${level}`;
  const timeSpan = document.createElement("span");
  timeSpan.className = "log-time";
  timeSpan.textContent = time;
  entry.appendChild(timeSpan);
  entry.appendChild(document.createTextNode(msg));
  content.appendChild(entry);
  content.scrollTop = content.scrollHeight;
  // Also mirror to console
  if (level === "error") console.error(msg);
  else if (level === "warn") console.warn(msg);
  else console.log(msg);
}

// Tauri v2 invoke — with logging
function invoke(cmd, args) {
  if (!window.__TAURI__ || !window.__TAURI__.core) {
    log("Tauri not loaded yet", "error");
    return Promise.reject("Tauri not loaded yet");
  }
  return window.__TAURI__.core.invoke(cmd, args).then(result => {
    log(`${cmd} → OK`, "ok");
    return result;
  }).catch(err => {
    log(`${cmd} → ${err}`, "error");
    throw err;
  });
}

// ── State ───────────────────────────────────────────────────

let chart = null;
let fisherChart = null;
let volumeChart = null;
let candleSeries = null;
let fisherSeries = {};
let volumeSeries = {};
let slLine = null;
let tpLine = null;
let currentSymbol = "";
let currentTimeframe = "1Hour";
let lastPrice = 0;
let mtfData = {};

// ── Tab State ───────────────────────────────────────────────

let tabs = []; // [{ id, symbol, timeframe, barCount, lastPrice, chartData }]
let activeTabId = null;
let nextTabId = 1;

function createTab(symbol = "", timeframe = "1Month") {
  const id = nextTabId++;
  const tab = { id, symbol, timeframe, barCount: "50000", lastPrice: 0, chartData: null };
  tabs.push(tab);
  renderTabs();
  switchTab(id);
  return tab;
}

function switchTab(id) {
  // Save current tab state before switching
  if (activeTabId !== null) {
    const cur = tabs.find(t => t.id === activeTabId);
    if (cur) {
      cur.symbol = currentSymbol;
      cur.timeframe = currentTimeframe;
      cur.lastPrice = lastPrice;
      cur.barCount = document.getElementById("bar-count").value;
    }
  }

  activeTabId = id;
  const tab = tabs.find(t => t.id === id);
  if (!tab) return;

  // Restore tab state to UI
  document.getElementById("symbol-input").value = tab.symbol;
  document.getElementById("timeframe-select").value = tab.timeframe;
  document.getElementById("bar-count").value = tab.barCount;
  currentSymbol = tab.symbol;
  currentTimeframe = tab.timeframe;
  lastPrice = tab.lastPrice;

  renderTabs();

  // Stop live polling from previous tab
  if (liveBarInterval) { clearInterval(liveBarInterval); liveBarInterval = null; }

  // Clear current chart immediately before loading new
  candleSeries.setData([]);
  clearIndicators();
  for (const [, s] of Object.entries(fisherSeries)) fisherChart.removeSeries(s);
  for (const [, s] of Object.entries(volumeSeries)) volumeChart.removeSeries(s);
  fisherSeries = {};
  volumeSeries = {};
  setText("connect-status-bar", "");

  // Load chart if symbol set
  if (tab.symbol) {
    loadChart(tab.symbol, tab.timeframe);
  }
}

function closeTab(id) {
  if (tabs.length <= 1) return; // Keep at least one tab
  const idx = tabs.findIndex(t => t.id === id);
  if (idx < 0) return;
  tabs.splice(idx, 1);
  if (activeTabId === id) {
    // Switch to nearest tab
    const newIdx = Math.min(idx, tabs.length - 1);
    switchTab(tabs[newIdx].id);
  }
  renderTabs();
}

function renderTabs() {
  const list = document.getElementById("tab-list");
  list.textContent = "";
  for (const tab of tabs) {
    const el = document.createElement("div");
    el.className = `chart-tab${tab.id === activeTabId ? " active" : ""}`;

    const label = document.createElement("span");
    label.textContent = tab.symbol || "New";
    el.appendChild(label);

    if (tabs.length > 1) {
      const close = document.createElement("span");
      close.className = "tab-close";
      close.textContent = "×";
      close.addEventListener("click", (e) => { e.stopPropagation(); closeTab(tab.id); });
      el.appendChild(close);
    }

    el.addEventListener("click", () => switchTab(tab.id));
    list.appendChild(el);
  }
}

function updateTabLabel() {
  if (activeTabId === null) return;
  const tab = tabs.find(t => t.id === activeTabId);
  if (tab && currentSymbol) {
    tab.symbol = currentSymbol;
    tab.timeframe = currentTimeframe;
    renderTabs();
  }
}

// ── Chart Setup ─────────────────────────────────────────────

function initChart() {
  const container = document.getElementById("chart-container");
  chart = createChart(container, {
    width: container.clientWidth,
    height: container.clientHeight,
    layout: {
      background: { color: "#000000" },
      textColor: "#d1d4dc",
      fontFamily: "Consolas, Courier New, monospace",
      attributionLogo: false,
    },
    grid: {
      vertLines: { color: "#333333", style: 3 },
      horzLines: { color: "#333333", style: 3 },
    },
    crosshair: { mode: CrosshairMode.Normal },
    rightPriceScale: { borderColor: "#333" },
    timeScale: { borderColor: "#333", timeVisible: true },
  });

  // MT5 default: filled green up, filled red down
  candleSeries = chart.addCandlestickSeries({
    upColor: "#00ff00",
    downColor: "#ff0000",
    borderDownColor: "#ff0000",
    borderUpColor: "#00ff00",
    wickDownColor: "#ff0000",
    wickUpColor: "#00ff00",
  });

  // Fisher pane — separate chart instance
  const fisherContainer = document.getElementById("fisher-pane");
  fisherChart = createChart(fisherContainer, {
    width: fisherContainer.clientWidth,
    height: fisherContainer.clientHeight,
    layout: { background: { color: "#000000" }, textColor: "#888", fontFamily: "Consolas, Courier New, monospace", attributionLogo: false },
    grid: { vertLines: { color: "#111" }, horzLines: { color: "#111" } },
    rightPriceScale: { borderColor: "#333" },
    timeScale: { visible: false }, // synced with main chart
    crosshair: { mode: CrosshairMode.Normal },
  });

  // Volume pane — separate chart instance
  const volumeContainer = document.getElementById("volume-pane");
  volumeChart = createChart(volumeContainer, {
    width: volumeContainer.clientWidth,
    height: volumeContainer.clientHeight,
    layout: { background: { color: "#000000" }, textColor: "#888", fontFamily: "Consolas, Courier New, monospace", attributionLogo: false },
    grid: { vertLines: { color: "#111" }, horzLines: { color: "#111" } },
    rightPriceScale: { borderColor: "#333" },
    timeScale: { visible: false },
    crosshair: { mode: CrosshairMode.Normal },
  });

  // Sync time scales: when main chart scrolls, sub-panes follow
  let syncing = false;
  chart.timeScale().subscribeVisibleLogicalRangeChange((range) => {
    if (range && !syncing) {
      syncing = true;
      fisherChart.timeScale().setVisibleLogicalRange(range);
      volumeChart.timeScale().setVisibleLogicalRange(range);
      syncing = false;
    }
  });

  // Sync crosshair across panes
  chart.subscribeCrosshairMove((param) => {
    if (param.time) {
      fisherChart.setCrosshairPosition(undefined, undefined, param.time);
      volumeChart.setCrosshairPosition(undefined, undefined, param.time);
    }
  });
  fisherChart.subscribeCrosshairMove((param) => {
    if (param.time) {
      chart.setCrosshairPosition(undefined, undefined, param.time);
      volumeChart.setCrosshairPosition(undefined, undefined, param.time);
    }
  });
  volumeChart.subscribeCrosshairMove((param) => {
    if (param.time) {
      chart.setCrosshairPosition(undefined, undefined, param.time);
      fisherChart.setCrosshairPosition(undefined, undefined, param.time);
    }
  });

  // Resize all charts together
  const ro = new ResizeObserver(() => {
    chart.resize(container.clientWidth, container.clientHeight);
    fisherChart.resize(fisherContainer.clientWidth, fisherContainer.clientHeight);
    volumeChart.resize(volumeContainer.clientWidth, volumeContainer.clientHeight);
  });
  ro.observe(container);
  ro.observe(fisherContainer);
  ro.observe(volumeContainer);

  // Tooltip for indicator values on crosshair
  setupTooltip();
}

function setupTooltip() {
  const tooltip = document.createElement("div");
  tooltip.id = "chart-tooltip";
  tooltip.className = "chart-tooltip hidden";
  document.getElementById("chart-stack").appendChild(tooltip);

  chart.subscribeCrosshairMove((param) => {
    if (!param.time || !param.point || param.point.x < 0) {
      tooltip.classList.add("hidden");
      return;
    }
    const lines = [];
    for (const [key, series] of Object.entries(indicatorSeries)) {
      const data = param.seriesData.get(series);
      if (data && data.value !== undefined) {
        // Derive label from key
        const label = key.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase());
        lines.push(`${label}: ${data.value.toFixed(4)}`);
      }
    }
    if (lines.length > 0) {
      tooltip.textContent = lines.join("\n");
      tooltip.style.left = param.point.x + 16 + "px";
      tooltip.style.top = param.point.y + "px";
      tooltip.classList.remove("hidden");
    } else {
      tooltip.classList.add("hidden");
    }
  });
}

// ── SL/TP Lines ─────────────────────────────────────────────

function createSLLine(price) {
  removeSLLine();
  slLine = candleSeries.createPriceLine({
    price,
    color: "#f44336",
    lineWidth: 2,
    lineStyle: 0,
    axisLabelVisible: true,
    title: "SL",
  });
  if (currentSymbol) invoke("set_sl_level", { symbol: currentSymbol, price }).catch(() => {});
}

function createTPLine(price) {
  removeTPLine();
  tpLine = candleSeries.createPriceLine({
    price,
    color: "#4caf50",
    lineWidth: 2,
    lineStyle: 0,
    axisLabelVisible: true,
    title: "TP",
  });
  if (currentSymbol) invoke("set_tp_level", { symbol: currentSymbol, price }).catch(() => {});
}

function removeSLLine() {
  if (slLine) { candleSeries.removePriceLine(slLine); slLine = null; }
}
function removeTPLine() {
  if (tpLine) { candleSeries.removePriceLine(tpLine); tpLine = null; }
}
function getSLPrice() { return slLine ? slLine.options().price : null; }
function getTPPrice() { return tpLine ? tpLine.options().price : null; }

// ── Draggable SL/TP Lines (MT5-style) ──────────────────────
// Double-click near a line to grab it, drag to new price, release to set.
// Uses chart price scale coordinateToPrice/priceToCoordinate for pixel↔price.

let draggingLine = null; // "sl" | "tp" | null

function setupLineDrag() {
  const container = document.getElementById("chart-container");
  const HIT_TOLERANCE = 8; // pixels

  function getLineYCoord(line) {
    if (!line || !candleSeries) return null;
    const price = line.options().price;
    const priceScale = candleSeries.priceScale();
    // Use series coordinate conversion
    const y = candleSeries.priceToCoordinate(price);
    return y;
  }

  function hitTestLine(clientY) {
    const rect = container.getBoundingClientRect();
    const y = clientY - rect.top;

    const slY = slLine ? getLineYCoord(slLine) : null;
    const tpY = tpLine ? getLineYCoord(tpLine) : null;

    const slDist = slY !== null ? Math.abs(y - slY) : Infinity;
    const tpDist = tpY !== null ? Math.abs(y - tpY) : Infinity;

    if (slDist <= HIT_TOLERANCE && slDist <= tpDist) return "sl";
    if (tpDist <= HIT_TOLERANCE) return "tp";
    return null;
  }

  // Double-click to start dragging
  container.addEventListener("dblclick", (e) => {
    const hit = hitTestLine(e.clientY);
    if (!hit) return;
    draggingLine = hit;
    container.style.cursor = "ns-resize";
    e.preventDefault();
    e.stopPropagation();
  });

  // Drag — update line price in real-time
  container.addEventListener("mousemove", (e) => {
    if (!draggingLine) {
      // Show resize cursor when hovering near a line
      const hit = hitTestLine(e.clientY);
      container.style.cursor = hit ? "ns-resize" : "";
      return;
    }
    const rect = container.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const newPrice = candleSeries.coordinateToPrice(y);
    if (newPrice === null || newPrice <= 0) return;

    const line = draggingLine === "sl" ? slLine : tpLine;
    if (line) {
      line.applyOptions({ price: newPrice });
    }
  });

  // Release — finalize price and sync to backend
  container.addEventListener("mouseup", () => {
    if (!draggingLine) return;
    const line = draggingLine === "sl" ? slLine : tpLine;
    if (line && currentSymbol) {
      const finalPrice = line.options().price;
      if (draggingLine === "sl") {
        invoke("set_sl_level", { symbol: currentSymbol, price: finalPrice }).catch(() => {});
        log(`SL moved to ${finalPrice.toFixed(4)}`, "info");
      } else {
        invoke("set_tp_level", { symbol: currentSymbol, price: finalPrice }).catch(() => {});
        log(`TP moved to ${finalPrice.toFixed(4)}`, "info");
      }
    }
    draggingLine = null;
    container.style.cursor = "";
  });

  // Cancel drag if mouse leaves chart
  container.addEventListener("mouseleave", () => {
    if (draggingLine) {
      draggingLine = null;
      container.style.cursor = "";
    }
  });
}

// ── Drawing Tools (Trend Lines + Fibonacci) ─────────────────
// Canvas overlay approach: draw on a transparent canvas synced with the chart.
// Lines stored as [{ type, p1: {time, price}, p2: {time, price} }].
// Fibonacci retracements auto-compute 0/23.6/38.2/50/61.8/78.6/100% levels.

let drawings = []; // [{ type: "trendline"|"fibonacci", p1, p2 }]
let drawingMode = null; // null | "trendline" | "fibonacci"
let drawingAnchor = null; // first click point { time, price }
let drawCanvas = null;
const DRAWINGS_KEY = "typhoon_drawings";

function loadDrawings() {
  try { drawings = JSON.parse(localStorage.getItem(DRAWINGS_KEY) || "[]"); } catch { drawings = []; }
}
function saveDrawings() { localStorage.setItem(DRAWINGS_KEY, JSON.stringify(drawings)); }

function setupDrawingCanvas() {
  const container = document.getElementById("chart-container");
  drawCanvas = document.createElement("canvas");
  drawCanvas.id = "draw-overlay";
  drawCanvas.style.cssText = "position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:10;";
  container.style.position = "relative";
  container.appendChild(drawCanvas);

  // Resize canvas to match container
  const resizeCanvas = () => {
    drawCanvas.width = container.clientWidth;
    drawCanvas.height = container.clientHeight;
    renderDrawings();
  };
  new ResizeObserver(resizeCanvas).observe(container);
  resizeCanvas();

  // Re-render when chart scrolls or zooms
  chart.timeScale().subscribeVisibleLogicalRangeChange(() => renderDrawings());

  // Click to place anchor points (only when in drawing mode)
  container.addEventListener("click", (e) => {
    if (!drawingMode) return;
    const rect = container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const time = chart.timeScale().coordinateToTime(x);
    const price = candleSeries.coordinateToPrice(y);
    if (time === null || price === null) return;

    if (!drawingAnchor) {
      drawingAnchor = { time, price };
      log(`Drawing: anchor set at $${price.toFixed(4)}`, "info");
    } else {
      drawings.push({ type: drawingMode, p1: drawingAnchor, p2: { time, price } });
      saveDrawings();
      log(`${drawingMode} drawn`, "ok");
      drawingAnchor = null;
      drawingMode = null;
      container.style.cursor = "";
      renderDrawings();
    }
  });
}

function renderDrawings() {
  // Delegate to extended version if available (supports horizontal, rectangle, channel)
  if (typeof renderDrawingsExtended === "function") {
    renderDrawingsExtended();
    return;
  }
  if (!drawCanvas || !chart || !candleSeries) return;
  const ctx = drawCanvas.getContext("2d");
  ctx.clearRect(0, 0, drawCanvas.width, drawCanvas.height);

  for (const d of drawings) {
    const x1 = chart.timeScale().timeToCoordinate(d.p1.time);
    const y1 = candleSeries.priceToCoordinate(d.p1.price);
    const x2 = chart.timeScale().timeToCoordinate(d.p2.time);
    const y2 = candleSeries.priceToCoordinate(d.p2.price);
    if (x1 === null || y1 === null || x2 === null || y2 === null) continue;

    if (d.type === "trendline") {
      ctx.beginPath();
      ctx.strokeStyle = "#00bcd4";
      ctx.lineWidth = 1.5;
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
    } else if (d.type === "fibonacci") {
      const high = Math.max(d.p1.price, d.p2.price);
      const low = Math.min(d.p1.price, d.p2.price);
      const range = high - low;
      const levels = [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
      const colors = ["#f44336", "#ff9800", "#ffeb3b", "#8bc34a", "#00bcd4", "#3f51b5", "#9c27b0"];
      const xLeft = Math.min(x1, x2);
      const xRight = drawCanvas.width;

      for (let i = 0; i < levels.length; i++) {
        const price = high - range * levels[i];
        const y = candleSeries.priceToCoordinate(price);
        if (y === null) continue;
        ctx.beginPath();
        ctx.strokeStyle = colors[i];
        ctx.lineWidth = 0.8;
        ctx.setLineDash([4, 4]);
        ctx.moveTo(xLeft, y);
        ctx.lineTo(xRight, y);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.fillStyle = colors[i];
        ctx.font = "10px Consolas";
        ctx.fillText(`${(levels[i] * 100).toFixed(1)}% $${price.toFixed(2)}`, xLeft + 4, y - 3);
      }
    }
  }
}

// ══════════════════════════════════════════════════════════════
// INDICATOR CALCULATIONS — Exact ports from MQL5 NNFX system
// ══════════════════════════════════════════════════════════════

let indicatorSeries = {};

function clearIndicators() {
  for (const [, series] of Object.entries(indicatorSeries)) {
    chart.removeSeries(series);
  }
  indicatorSeries = {};
}

// ── KAMA (Kaufman Adaptive Moving Average) ──────────────────
// Port of KAMA.mqh: period=10, fast=2, slow=30, applied to OPEN
// Colors: clrWhite (#FFFFFF), width 2
function calcKAMA(data, period = 10, fastP = 2, slowP = 30) {
  const fastSC = 2.0 / (fastP + 1.0);
  const slowSC = 2.0 / (slowP + 1.0);
  const result = [];
  if (data.length < period + 1) return result;
  // Applied to CLOSE price (MQL5 default price[] parameter)
  let kama = data[period].close;
  for (let i = period; i < data.length; i++) {
    const signal = Math.abs(data[i].close - data[i - period].close);
    let noise = 0;
    for (let j = i - period + 1; j <= i; j++) {
      noise += Math.abs(data[j].close - data[j - 1].close);
    }
    const er = noise !== 0 ? signal / noise : 0;
    const ssc = er * (fastSC - slowSC) + slowSC;
    const ssc2 = ssc * ssc;
    kama = ssc2 * (data[i].close - kama) + kama;
    result.push({ time: data[i].time, value: kama });
  }
  return result;
}

// ── Previous Candle Levels ──────────────────────────────────
// Port of PreviousCandleLevels.mqh
// Colors: clrWhite (#FFFFFF) for H1/H4, clrMagenta (#FF00FF) for D1/W1/MN1
// Width: 2, Style: SOLID
function calcPrevCandleLevels(data) {
  const highs = [], lows = [];
  for (let i = 1; i < data.length; i++) {
    highs.push({ time: data[i].time, value: data[i - 1].high });
    lows.push({ time: data[i].time, value: data[i - 1].low });
  }
  return { highs, lows };
}

// ── ATR Projection ──────────────────────────────────────────
// Port of ATR_Projection.mqh: period=14
// Colors: clrYellow (#FFFF00), style=STYLE_DOT, width=2
// Projection: currentOpen ± ATR
function calcATRProjection(data, period = 14) {
  const trs = [];
  for (let i = 1; i < data.length; i++) {
    trs.push(Math.max(
      data[i].high - data[i].low,
      Math.abs(data[i].high - data[i - 1].close),
      Math.abs(data[i].low - data[i - 1].close)
    ));
  }
  if (trs.length < period) return { upper: [], lower: [], atrValues: [] };

  let atr = trs.slice(0, period).reduce((a, b) => a + b, 0) / period;
  const upper = [], lower = [], atrValues = [];
  for (let i = period; i < trs.length; i++) {
    atr = (atr * (period - 1) + trs[i]) / period;
    const idx = i + 1;
    if (idx < data.length) {
      // MQL5: currentOpen + ATR / currentOpen - ATR
      upper.push({ time: data[idx].time, value: data[idx].open + atr });
      lower.push({ time: data[idx].time, value: data[idx].open - atr });
      atrValues.push({ time: data[idx].time, value: atr });
    }
  }
  return { upper, lower, atrValues };
}

// ── Ehlers Fisher Transform ─────────────────────────────────
// Port of EhlersFisherTransform.mqh: period=32, price=MEDIAN
// Colors: clrMediumSeaGreen (#3CB371) bullish, clrOrangeRed (#FF4500) bearish, clrDarkGray (#A9A9A9) neutral
// Signal line: clrDarkGray, width 1
// Fisher line: width 2, color changes based on Fisher vs Signal
function calcEhlersFisher(data, period = 32) {
  if (data.length < period + 1) return { fisher: [], signal: [], colors: [] };
  const fisher = [], signal = [], colors = [];
  let prevSmoothed = 0, prevFisher = 0;

  for (let i = period; i < data.length; i++) {
    // Find highest/lowest over period (excluding current bar — calc_no mode)
    let maxH = -Infinity, minL = Infinity;
    for (let j = i - period; j < i; j++) {
      if (data[j].high > maxH) maxH = data[j].high;
      if (data[j].low < minL) minL = data[j].low;
    }
    // Median price = (high + low) / 2
    const price = (data[i].high + data[i].low) / 2;
    const range = maxH - minL;
    // Normalize to 0-1, then center to -1..+1
    const normalized = range > 0 ? (price - minL) / range : 0.5;
    const os = 2.0 * (normalized - 0.5);
    // Smooth
    let smoothed = 0.5 * os + 0.5 * prevSmoothed;
    smoothed = Math.max(-0.999, Math.min(0.999, smoothed));
    // Fisher transform with smoothing
    const ft = 0.25 * Math.log((1 + smoothed) / (1 - smoothed)) + 0.5 * prevFisher;

    // Color: green if fisher > signal (bullish), red if < (bearish), gray if equal
    const sig = prevFisher;
    let color;
    if (ft > sig) color = "#3CB371";      // clrMediumSeaGreen
    else if (ft < sig) color = "#FF4500"; // clrOrangeRed
    else color = "#A9A9A9";               // clrDarkGray

    fisher.push({ time: data[i].time, value: ft });
    signal.push({ time: data[i].time, value: sig });
    colors.push(color);

    prevSmoothed = smoothed;
    prevFisher = ft;
  }
  return { fisher, signal, colors };
}

// ── RVOL (Relative Volume) ──────────────────────────────────
// Port of RVOL.mqh: averagingDays=10
// Colors: clrGreen (#00FF00) >1.25, clrOrange (#FFA500) 0.8-1.25, clrRed (#FF0000) <0.8
// Style: DRAW_COLOR_HISTOGRAM, width 3
function calcRVOL(data, avgDays = 10) {
  const result = [];
  if (data.length < avgDays + 1) return result;
  // Sliding window
  let windowSum = 0;
  for (let i = 0; i < avgDays; i++) windowSum += (data[i].volume || 0);
  for (let i = avgDays; i < data.length; i++) {
    const mean = windowSum / avgDays;
    const vol = data[i].volume || 0;
    const rvol = mean > 0 ? vol / mean : 0;
    let color;
    if (rvol > 1.25) color = "#00FF00";       // clrGreen — above average
    else if (rvol >= 0.8) color = "#FFA500";   // clrOrange — average
    else color = "#FF0000";                     // clrRed — below average
    result.push({ time: data[i].time, value: rvol, color });
    // Slide window
    windowSum += vol;
    windowSum -= (data[i - avgDays].volume || 0);
  }
  return result;
}

// ── SMA ─────────────────────────────────────────────────────
function calcSMA(data, period) {
  const result = [];
  for (let i = period - 1; i < data.length; i++) {
    let sum = 0;
    for (let j = i - period + 1; j <= i; j++) sum += data[j].close;
    result.push({ time: data[i].time, value: sum / period });
  }
  return result;
}

// ── EMA ─────────────────────────────────────────────────────
function calcEMA(data, period) {
  const k = 2 / (period + 1);
  const result = [];
  let ema = data[0].close;
  for (let i = 0; i < data.length; i++) {
    ema = data[i].close * k + ema * (1 - k);
    if (i >= period - 1) result.push({ time: data[i].time, value: ema });
  }
  return result;
}

// ── DEMA ────────────────────────────────────────────────────
function calcDEMA(data, period) {
  const ema1 = calcEMA(data, period);
  if (ema1.length < period) return [];
  const ema2data = ema1.map(e => ({ close: e.value, time: e.time }));
  const k = 2 / (period + 1);
  let ema2 = ema2data[0].close;
  const result = [];
  for (let i = 0; i < ema2data.length; i++) {
    ema2 = ema2data[i].close * k + ema2 * (1 - k);
    if (i >= period - 1) result.push({ time: ema2data[i].time, value: 2 * ema2data[i].close - ema2 });
  }
  return result;
}

// ── RSI ─────────────────────────────────────────────────────
function calcRSI(data, period) {
  const result = [];
  let gains = 0, losses = 0;
  for (let i = 1; i <= period && i < data.length; i++) {
    const change = data[i].close - data[i - 1].close;
    if (change > 0) gains += change; else losses -= change;
  }
  let avgGain = gains / period, avgLoss = losses / period;
  for (let i = period; i < data.length; i++) {
    if (i > period) {
      const change = data[i].close - data[i - 1].close;
      avgGain = (avgGain * (period - 1) + (change > 0 ? change : 0)) / period;
      avgLoss = (avgLoss * (period - 1) + (change < 0 ? -change : 0)) / period;
    }
    const rs = avgLoss === 0 ? 100 : avgGain / avgLoss;
    result.push({ time: data[i].time, value: 100 - 100 / (1 + rs) });
  }
  return result;
}

// ── MACD ────────────────────────────────────────────────────
function calcMACD(data, fastP = 12, slowP = 26, signalP = 9) {
  const fastEMA = calcEMA(data, fastP);
  const slowEMA = calcEMA(data, slowP);
  const slowMap = new Map(slowEMA.map(e => [e.time, e.value]));
  const macdLine = [];
  for (const fe of fastEMA) {
    const sv = slowMap.get(fe.time);
    if (sv !== undefined) macdLine.push({ time: fe.time, value: fe.value - sv });
  }
  if (macdLine.length < signalP) return { macd: macdLine, signal: [], histogram: [] };
  const k = 2 / (signalP + 1);
  let sig = macdLine[0].value;
  const signalData = [], histogram = [];
  for (let i = 0; i < macdLine.length; i++) {
    sig = macdLine[i].value * k + sig * (1 - k);
    if (i >= signalP - 1) {
      signalData.push({ time: macdLine[i].time, value: sig });
      const diff = macdLine[i].value - sig;
      histogram.push({ time: macdLine[i].time, value: diff, color: diff >= 0 ? "#26a69a" : "#ef5350" });
    }
  }
  return { macd: macdLine, signal: signalData, histogram };
}

// ── Bollinger Bands ─────────────────────────────────────────
function calcBollinger(data, period) {
  const upper = [], lower = [];
  for (let i = period - 1; i < data.length; i++) {
    let sum = 0, sumSq = 0;
    for (let j = i - period + 1; j <= i; j++) { sum += data[j].close; sumSq += data[j].close ** 2; }
    const mean = sum / period;
    const std = Math.sqrt(sumSq / period - mean ** 2);
    upper.push({ time: data[i].time, value: mean + 2 * std });
    lower.push({ time: data[i].time, value: mean - 2 * std });
  }
  return { upper, lower };
}

// ── ATR (standalone, for separate pane) ─────────────────────
function calcATR(data, period) {
  const result = [], trs = [];
  for (let i = 1; i < data.length; i++) {
    trs.push(Math.max(data[i].high - data[i].low, Math.abs(data[i].high - data[i - 1].close), Math.abs(data[i].low - data[i - 1].close)));
  }
  let atr = trs.slice(0, period).reduce((a, b) => a + b, 0) / period;
  for (let i = period; i < trs.length; i++) {
    atr = (atr * (period - 1) + trs[i]) / period;
    if (i + 1 < data.length) result.push({ time: data[i + 1].time, value: atr });
  }
  return result;
}

// ── VWAP ────────────────────────────────────────────────────
function calcVWAP(data) {
  const result = [];
  let cumVol = 0, cumTPV = 0, lastDate = "";
  for (const d of data) {
    const date = typeof d.time === "number" ? new Date(d.time * 1000).toISOString().slice(0, 10) : "";
    if (date !== lastDate) { cumVol = 0; cumTPV = 0; lastDate = date; }
    const tp = (d.high + d.low + d.close) / 3;
    const vol = d.volume || 1;
    cumVol += vol; cumTPV += tp * vol;
    if (cumVol > 0) result.push({ time: d.time, value: cumTPV / cumVol });
  }
  return result;
}

// ── BetterVolume ────────────────────────────────────────────
// Exact port of BetterVolume.mqh — Emini-Watch classification
// Colors: Yellow=LowVol, Red=ClimaxUp, White=ClimaxDn, Green=Churn, Magenta=Climax+Churn, SteelBlue=Normal
function calcBetterVolume(data, lookback = 20) {
  const result = [];
  if (data.length < lookback + 2) return result;

  // Estimate buy/sell volume (matches EstimateBuySell in MQL5)
  function estimateBuySell(bar) {
    const vol = data[bar].volume || 0;
    const range = data[bar].high - data[bar].low;
    if (range <= 0) return { buy: vol * 0.5, sell: vol * 0.5 };
    const o = data[bar].open, c = data[bar].close;
    let buyVol;
    if (c > o) {
      const denom = 2.0 * range + o - c;
      buyVol = (range / (denom > 0 ? denom : range)) * vol;
    } else if (c < o) {
      const denom = 2.0 * range + c - o;
      buyVol = ((range + c - o) / (denom > 0 ? denom : range)) * vol;
    } else {
      buyVol = vol * 0.5;
    }
    return { buy: buyVol, sell: vol - buyVol };
  }

  for (let pos = lookback + 1; pos < data.length; pos++) {
    const vol = data[pos].volume || 0;
    const range = data[pos].high - data[pos].low || 0.0001;
    const { buy: buyVol, sell: sellVol } = estimateBuySell(pos);

    // Current bar metrics
    const buyRange = buyVol * range;
    const sellRange = sellVol * range;
    const volDivR = vol / range;
    const sellDivR = sellVol / range;
    const buyDivR = buyVol / range;

    // Find lookback extremes (1-bar)
    let highBuyRange = 0, highSellRange = 0, highVolDivR = 0;
    let lowSellDivR = Infinity, lowBuyDivR = Infinity, lowTotalVol = Infinity;

    for (let i = 0; i < lookback; i++) {
      const b = pos - 1 - i;
      if (b < 0) break;
      const { buy: bv, sell: sv } = estimateBuySell(b);
      const r = data[b].high - data[b].low || 0.0001;
      const v = data[b].volume || 0;
      const br = bv * r, sr = sv * r, vr = v / r;
      const sdr = sv / r, bdr = bv / r;
      if (br > highBuyRange) highBuyRange = br;
      if (sr > highSellRange) highSellRange = sr;
      if (vr > highVolDivR) highVolDivR = vr;
      if (sdr < lowSellDivR) lowSellDivR = sdr;
      if (bdr < lowBuyDivR) lowBuyDivR = bdr;
      if (v < lowTotalVol) lowTotalVol = v;
    }

    // Classification flags
    let isClimaxUp = false, isClimaxDn = false, isChurn = false, isLowVol = false;

    // Low Volume
    if (vol <= lowTotalVol) isLowVol = true;
    // Climax Up: bullish bar with highest buy pressure
    if (data[pos].close > data[pos].open && (buyRange >= highBuyRange || sellDivR <= lowSellDivR))
      isClimaxUp = true;
    // Climax Down: bearish bar with highest sell pressure
    if (data[pos].close < data[pos].open && (sellRange >= highSellRange || buyDivR <= lowBuyDivR))
      isClimaxDn = true;
    // Churn: highest volume/range ratio
    if (volDivR >= highVolDivR) isChurn = true;

    // Priority: ClimaxChurn > LowVol > ClimaxUp > ClimaxDown > Churn > Normal
    let color;
    if ((isClimaxUp || isClimaxDn) && isChurn) color = "#FF00FF";     // Magenta — Climax+Churn
    else if (isLowVol) color = "#FFFF00";                              // Yellow — Low Volume
    else if (isClimaxUp) color = "#FF0000";                            // Red — Climax Up
    else if (isClimaxDn) color = "#FFFFFF";                            // White — Climax Down
    else if (isChurn) color = "#00FF00";                                // Green — Churn
    else color = "#4682B4";                                             // SteelBlue — Normal

    result.push({ time: data[pos].time, value: vol, color });
  }
  return result;
}

// ── Supply/Demand Zones ─────────────────────────────────────
// Detect strong move-away candles and project their origin as zones
// Exact port of SupplyDemand.mqh — fractal-based detection with strength tiers
function calcSupplyDemandZones(data, fractalLookback = 5, backLimit = 1000) {
  if (data.length < fractalLookback * 2 + 1) return [];

  const limit = Math.min(backLimit, data.length - fractalLookback - 1);

  // Fractal detection (matches IsFractalHigh/IsFractalLow in MQL5)
  function isFractalHigh(bar) {
    const val = data[bar].high;
    for (let i = 1; i <= fractalLookback; i++) {
      if (bar - i < 0 || bar + i >= data.length) return false;
      if (data[bar - i].high >= val || data[bar + i].high >= val) return false;
    }
    return true;
  }
  function isFractalLow(bar) {
    const val = data[bar].low;
    for (let i = 1; i <= fractalLookback; i++) {
      if (bar - i < 0 || bar + i >= data.length) return false;
      if (data[bar - i].low <= val || data[bar + i].low <= val) return false;
    }
    return true;
  }

  // Find zones at fractals (matches FindZones in MQL5)
  const zones = [];
  const startBar = Math.max(fractalLookback, data.length - limit);
  for (let i = startBar; i < data.length - fractalLookback; i++) {
    // Supply zone at fractal high: zone = [min(close,open) → high]
    if (isFractalHigh(i)) {
      const hi = data[i].high;
      let lo = Math.min(data[i].close, data[i].open);
      if (hi - lo < 0.0001) lo = hi - 0.0001;
      zones.push({ type: "supply", high: hi, low: lo, startTime: data[i].time, barIdx: i, touches: 0, strength: "untested" });
    }
    // Demand zone at fractal low: zone = [low → max(close,open)]
    if (isFractalLow(i)) {
      let hi = Math.max(data[i].close, data[i].open);
      const lo = data[i].low;
      if (hi - lo < 0.0001) hi = lo + 0.0001;
      zones.push({ type: "demand", high: hi, low: lo, startTime: data[i].time, barIdx: i, touches: 0, strength: "untested" });
    }
  }

  // Test zones against subsequent price action (matches TestZones in MQL5)
  for (const z of zones) {
    const scanFrom = Math.min(z.barIdx + fractalLookback + 1, data.length - 1);
    for (let b = scanFrom; b < data.length; b++) {
      // Does bar overlap zone?
      if (data[b].high >= z.low && data[b].low <= z.high) {
        // Broken: close pierces beyond zone boundary
        if (z.type === "supply" && data[b].close > z.high) { z.strength = "broken"; break; }
        if (z.type === "demand" && data[b].close < z.low) { z.strength = "broken"; break; }
        z.touches++;
      }
    }
    if (z.strength !== "broken") {
      if (z.touches === 0) z.strength = "untested";
      else if (z.touches <= 2) z.strength = "tested";
      else z.strength = "proven";
    }
  }

  // Merge overlapping same-type zones (matches MergeZones in MQL5)
  zones.sort((a, b) => a.type === b.type ? a.low - b.low : a.type.localeCompare(b.type));
  const merged = [];
  for (const z of zones) {
    const last = merged.length > 0 ? merged[merged.length - 1] : null;
    if (last && last.type === z.type && z.low <= last.high) {
      last.high = Math.max(last.high, z.high);
      last.low = Math.min(last.low, z.low);
      last.touches += z.touches;
      if (z.strength === "broken") last.strength = "broken";
      if (z.startTime < last.startTime) last.startTime = z.startTime;
    } else {
      merged.push({ ...z });
    }
  }

  // Filter: remove broken, keep active zones
  return merged.filter(z => z.strength !== "broken");
}

// ══════════════════════════════════════════════════════════════
// MULTI-TIMEFRAME INDICATORS — ports from MultiKAMA, ATR_Projection, PreviousCandleLevels
// ══════════════════════════════════════════════════════════════

const MTF_TIMEFRAMES = ["15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week"];
const ALL_MTF_KAMA_TFS = ["1Hour", "4Hour", "1Day", "1Week"];
const ALL_MTF_ATR_TFS = ["1Hour", "4Hour", "1Day", "1Week"];
const ALL_MTF_PCL_TFS = ["1Hour", "4Hour", "1Day", "1Week"];

// Timeframe hierarchy — only show TFs HIGHER than current chart (like MT5)
const TF_RANK = { "1Min": 0, "5Min": 1, "15Min": 2, "30Min": 3, "1Hour": 4, "4Hour": 5, "1Day": 6, "1Week": 7, "1Month": 8 };

function getRelevantMTFs(allTFs) {
  const currentRank = TF_RANK[currentTimeframe] ?? 3;
  return allTFs.filter(tf => (TF_RANK[tf] ?? 0) > currentRank);
}

// PreviousCandleLevels.mqh defaults:
//   PreviousCandleColour = clrWhite (H1/H4 prev levels)
//   JudasLevelColour = clrMagenta (D1/W1/MN1 prev + current levels)
const MTF_PCL_COLORS = {
  "1Hour": "#FFFFFF",
  "4Hour": "#FFFFFF",
  "1Day":  "#FF00FF",
  "1Week": "#FF00FF",
};

// MTF_MA indicator colors from MQL5 defaults:
//   H1 200SMA = clrTomato (#FF6347)
//   H4/D1/W1 200SMA = clrMagenta (#FF00FF)
//   W1 100SMA = clrMagenta
//   MN1 100SMA = clrMagenta
const MTF_MA_COLORS = {
  "1Hour": "#FF6347",  // Tomato
  "4Hour": "#FF00FF",  // Magenta
  "1Day":  "#FF00FF",  // Magenta
  "1Week": "#FF00FF",  // Magenta
};

const MTF_LABELS = { "15Min": "M15", "30Min": "M30", "1Hour": "H1", "4Hour": "H4", "1Day": "D1", "1Week": "W1" };

async function loadMTFData(symbol) {
  try {
    const json = await invoke("get_multi_tf_bars", {
      symbol,
      timeframes: MTF_TIMEFRAMES,
      limit: 500,
    });
    mtfData = {};
    const parsed = JSON.parse(json);
    for (const [tf, bars] of Object.entries(parsed)) {
      mtfData[tf] = bars.map(b => ({
        time: Math.floor(new Date(b.timestamp).getTime() / 1000),
        open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume,
      }));
    }
    log(`MTF data loaded: ${Object.keys(mtfData).map(tf => `${MTF_LABELS[tf] || tf}=${(mtfData[tf]||[]).length}`).join(", ")}`, "ok");
  } catch (e) {
    log(`MTF data load failed: ${e}`, "error");
  }
}

// MultiKAMA: project HTF KAMA values onto current chart's time axis
function projectHTFToChartTime(htfData, chartData) {
  if (!htfData || htfData.length === 0 || chartData.length === 0) return [];
  const result = [];
  let htfIdx = 0;
  for (const bar of chartData) {
    while (htfIdx < htfData.length - 1 && htfData[htfIdx + 1].time <= bar.time) htfIdx++;
    if (htfData[htfIdx].time <= bar.time) {
      result.push({ time: bar.time, value: htfData[htfIdx].value });
    }
  }
  return result;
}

// Previous candle levels from HTF: project prev bar's high/low as horizontal lines
function calcHTFPrevLevels(htfBars, chartData) {
  if (!htfBars || htfBars.length < 2 || chartData.length === 0) return null;
  const prevBar = htfBars[htfBars.length - 2]; // previous completed bar
  const curBar = htfBars[htfBars.length - 1];  // current bar
  return {
    prevHigh: prevBar.high,
    prevLow: prevBar.low,
    curHigh: curBar.high,
    curLow: curBar.low,
  };
}

// ATR projection from HTF: project open ± ATR onto chart
function calcHTFATRProjection(htfBars, period = 14) {
  if (!htfBars || htfBars.length < period + 2) return null;
  // Calculate ATR from HTF bars
  const trs = [];
  for (let i = 1; i < htfBars.length; i++) {
    trs.push(Math.max(
      htfBars[i].high - htfBars[i].low,
      Math.abs(htfBars[i].high - htfBars[i - 1].close),
      Math.abs(htfBars[i].low - htfBars[i - 1].close)
    ));
  }
  let atr = trs.slice(0, period).reduce((a, b) => a + b, 0) / period;
  for (let i = period; i < trs.length; i++) {
    atr = (atr * (period - 1) + trs[i]) / period;
  }
  const curOpen = htfBars[htfBars.length - 1].open;
  return { atr, upper: curOpen + atr, lower: curOpen - atr };
}

// Truncate indicator data to not extend past last candle
function clipToChart(indData, chartData) {
  if (!chartData || chartData.length === 0 || !indData || indData.length === 0) return indData;
  const lastTime = chartData[chartData.length - 1].time;
  return indData.filter(d => d.time <= lastTime);
}

function applyIndicators(chartData) {
  clearIndicators();
  for (const [, s] of Object.entries(fisherSeries)) fisherChart.removeSeries(s);
  for (const [, s] of Object.entries(volumeSeries)) volumeChart.removeSeries(s);
  fisherSeries = {};
  volumeSeries = {};
  const checkboxes = document.querySelectorAll("#indicator-list input[type=checkbox]:checked");
  const lastTime = chartData.length > 0 ? chartData[chartData.length - 1].time : Infinity;
  // Helper: clip any data array to not exceed last candle time
  const clip = (data) => data.filter(d => d.time <= lastTime);

  for (const cb of checkboxes) {
    const ind = cb.dataset.ind;
    const period = parseInt(cb.dataset.period) || 14;
    const key = `${ind}_${period}`;

    // ══════════════════════════════════════════════════════════
    // NNFX SYSTEM INDICATORS — exact MQL5 ports
    // ══════════════════════════════════════════════════════════

    if (ind === "kama") {
      // MultiKAMA.mqh: KAMA from multiple timeframes, all clrWhite, width 2
      // Current chart's own KAMA
      if (chartData.length > period + 1) {
        const s = chart.addLineSeries({ color: "#FFFFFF", lineWidth: 2, title: "", lastValueVisible: false, priceLineVisible: false });
        s.setData(clip(calcKAMA(chartData, period)));
        indicatorSeries[key] = s;
      }
      // HTF KAMAs projected onto current chart
      for (const tf of getRelevantMTFs(ALL_MTF_KAMA_TFS)) {
        if (tf === currentTimeframe) continue; // skip current TF (already drawn above)
        const tfBars = mtfData[tf];
        if (!tfBars || tfBars.length < period + 1) continue;
        const kamaData = calcKAMA(tfBars, period);
        const projected = projectHTFToChartTime(kamaData, chartData);
        if (projected.length === 0) continue;
        const maColor = MTF_MA_COLORS[tf] || "#FF00FF";
        const s = chart.addLineSeries({ color: maColor, lineWidth: 2, title: "", lastValueVisible: false, priceLineVisible: false });
        s.setData(clip(projected));
        indicatorSeries[`${key}_${tf}`] = s;
      }

    } else if (ind === "prev-levels") {
      // PreviousCandleLevels.mqh: multi-TF previous high/low
      // Current chart prev candle levels
      if (chartData.length > 1) {
        const pcl = calcPrevCandleLevels(chartData);
        const sh = chart.addLineSeries({ color: "#FFFFFF", lineWidth: 2, lineStyle: 0, title: "", lastValueVisible: false, priceLineVisible: false });
        const sl2 = chart.addLineSeries({ color: "#FFFFFF", lineWidth: 2, lineStyle: 0, title: "", lastValueVisible: false, priceLineVisible: false });
        sh.setData(clip(pcl.highs)); sl2.setData(clip(pcl.lows));
        indicatorSeries[key + "_h"] = sh;
        indicatorSeries[key + "_l"] = sl2;
      }
      // HTF previous candle levels — solid lines from HTF bar start to last candle
      for (const tf of getRelevantMTFs(ALL_MTF_PCL_TFS)) {
        const tfBars = mtfData[tf];
        const levels = calcHTFPrevLevels(tfBars, chartData);
        if (!levels) continue;
        const color = MTF_PCL_COLORS[tf] || "#FFFFFF";
        // Previous bar levels span from previous HTF bar start to current
        const prevStart = tfBars.length >= 2 ? tfBars[tfBars.length - 2].time : 0;
        const levelBars = clip(chartData.filter(d => d.time >= prevStart));
        if (levelBars.length < 2) continue;
        const drawLevel = (val, clr, k) => {
          const s = chart.addLineSeries({ color: clr, lineWidth: 2, lineStyle: 0, title: "", lastValueVisible: false, priceLineVisible: false });
          s.setData(levelBars.map(d => ({ time: d.time, value: val })));
          indicatorSeries[`pcl_${tf}_${k}`] = s;
        };
        drawLevel(levels.prevHigh, color, "ph");
        drawLevel(levels.prevLow, color, "pl");
        // D1/W1 current bar levels (Judas)
        if (tf === "1Day" || tf === "1Week") {
          const curStart = tfBars[tfBars.length - 1].time;
          const curBars = clip(chartData.filter(d => d.time >= curStart));
          if (curBars.length >= 2) {
            drawLevel(levels.curHigh, "#FF00FF", "ch");
            drawLevel(levels.curLow, "#FF00FF", "cl");
          }
        }
      }

    } else if (ind === "atr-proj") {
      // ATR_Projection.mqh: ATR from multiple timeframes, clrYellow, STYLE_DOT, width 2
      // Current chart ATR projection
      if (chartData.length > period + 1) {
        const atrp = calcATRProjection(chartData, period);
        const su = chart.addLineSeries({ color: "#FFFF00", lineWidth: 2, lineStyle: 0, title: "", lastValueVisible: false, priceLineVisible: false });
        const sl3 = chart.addLineSeries({ color: "#FFFF00", lineWidth: 2, lineStyle: 0, title: "", lastValueVisible: false, priceLineVisible: false });
        su.setData(clip(atrp.upper)); sl3.setData(clip(atrp.lower));
        indicatorSeries[key + "_u"] = su;
        indicatorSeries[key + "_l"] = sl3;
      }
      // HTF ATR projections — solid yellow lines, clipped to chart range
      for (const tf of getRelevantMTFs(ALL_MTF_ATR_TFS)) {
        const tfBars = mtfData[tf];
        const proj = calcHTFATRProjection(tfBars, period);
        if (!proj) continue;
        // Draw as line series from HTF current bar start to last candle (not edge-to-edge price lines)
        const htfStart = tfBars[tfBars.length - 1].time;
        const projBars = clip(chartData.filter(d => d.time >= htfStart));
        if (projBars.length < 2) continue;
        const sU = chart.addLineSeries({ color: "#FFFF00", lineWidth: 2, lineStyle: 0, title: "", lastValueVisible: false, priceLineVisible: false });
        const sL = chart.addLineSeries({ color: "#FFFF00", lineWidth: 2, lineStyle: 0, title: "", lastValueVisible: false, priceLineVisible: false });
        sU.setData(projBars.map(d => ({ time: d.time, value: proj.upper })));
        sL.setData(projBars.map(d => ({ time: d.time, value: proj.lower })));
        indicatorSeries[`atr_mtf_${tf}_u`] = sU;
        indicatorSeries[`atr_mtf_${tf}_l`] = sL;
      }

    } else if (ind === "fisher" && chartData.length > period) {
      // EhlersFisherTransform.mqh — DRAW_COLOR_LINE in separate pane
      // Green (#3CB371) when fisher > signal (bullish), Red (#FF4500) when < (bearish), Gray neutral
      const ef = calcEhlersFisher(chartData, period);

      // MQL5 DRAW_COLOR_LINE: ONE line that changes color per bar.
      // Split into contiguous same-color segments, each as its own line series.
      // Each segment includes the last point of the previous segment for continuity.
      const segments = [];
      let curColor = ef.colors[0];
      let curSeg = [{ time: ef.fisher[0].time, value: ef.fisher[0].value }];

      for (let i = 1; i < ef.fisher.length; i++) {
        const d = { time: ef.fisher[i].time, value: ef.fisher[i].value };
        if (ef.colors[i] !== curColor) {
          // Close current segment with this transition point
          curSeg.push(d);
          segments.push({ color: curColor, data: curSeg });
          // Start new segment from this point
          curColor = ef.colors[i];
          curSeg = [d];
        } else {
          curSeg.push(d);
        }
      }
      if (curSeg.length > 0) segments.push({ color: curColor, data: curSeg });

      // Draw each segment as its own line series
      for (let si = 0; si < segments.length; si++) {
        const seg = segments[si];
        if (seg.data.length < 2) continue;
        const s = fisherChart.addLineSeries({
          color: seg.color, lineWidth: 2,
          lastValueVisible: si === segments.length - 1, // only last segment shows value
          priceLineVisible: false, crosshairMarkerVisible: false,
        });
        s.setData(seg.data);
        fisherSeries[`seg_${si}`] = s;
      }

      // Signal line (gray, thin)
      const sSignal = fisherChart.addLineSeries({
        color: "#A9A9A9", lineWidth: 1, lastValueVisible: false, priceLineVisible: false,
      });
      sSignal.setData(ef.signal);

      // Zero line
      const sZero = fisherChart.addLineSeries({
        color: "#FFFFFF33", lineWidth: 1, lineStyle: 2, lastValueVisible: false, priceLineVisible: false,
      });
      sZero.setData(ef.fisher.map(d => ({ time: d.time, value: 0 })));

      fisherSeries.signal = sSignal;
      fisherSeries.zero = sZero;
      fisherChart.timeScale().setVisibleLogicalRange(chart.timeScale().getVisibleLogicalRange());

    } else if (ind === "better-vol" && chartData.length > 2) {
      // BetterVolume — rendered in dedicated volumeChart pane
      const bvData = calcBetterVolume(chartData);
      const s = volumeChart.addHistogramSeries({
        priceFormat: { type: "volume" },
      });
      s.setData(bvData);
      volumeSeries.hist = s;
      volumeChart.timeScale().setVisibleLogicalRange(chart.timeScale().getVisibleLogicalRange());

    } else if (ind === "supply-demand" && chartData.length > 10) {
      // SupplyDemand.mqh: fractal-based zones with strength-tier colors
      const zones = calcSupplyDemandZones(chartData);
      for (let zi = 0; zi < zones.length; zi++) {
        const z = zones[zi];
        // MQL5 default colors by type and strength
        const colors = z.type === "supply" ? {
          untested: "#87CEEB",  // clrSkyBlue
          tested:   "#00BFFF",  // clrDeepSkyBlue
          proven:   "#1E90FF",  // clrDodgerBlue
        } : {
          untested: "#8FBC8F",  // clrDarkSeaGreen
          tested:   "#3CB371",  // clrMediumSeaGreen
          proven:   "#2E8B57",  // clrSeaGreen
        };
        const zoneColor = colors[z.strength] || colors.untested;
        const fillColor = zoneColor + "30"; // semi-transparent fill
        const lineColor = zoneColor + "88"; // slightly transparent border
        const zoneBars = clip(chartData.filter(d => d.time >= z.startTime));
        if (zoneBars.length < 2) continue;

        // Top line of zone
        const topLine = chart.addLineSeries({
          color: lineColor, lineWidth: 1, lastValueVisible: false,
          priceLineVisible: false, crosshairMarkerVisible: false,
        });
        topLine.setData(zoneBars.map(d => ({ time: d.time, value: z.high })));

        // Bottom line of zone
        const botLine = chart.addLineSeries({
          color: lineColor, lineWidth: 1, lastValueVisible: false,
          priceLineVisible: false, crosshairMarkerVisible: false,
        });
        botLine.setData(zoneBars.map(d => ({ time: d.time, value: z.low })));

        // Fill: baseline series — fills between line and a fixed price level
        // This creates a proper bounded rectangle between zone top and bottom
        const fillArea = chart.addBaselineSeries({
          topFillColor1: fillColor,
          topFillColor2: fillColor,
          bottomFillColor1: fillColor,
          bottomFillColor2: fillColor,
          topLineColor: "transparent",
          bottomLineColor: "transparent",
          lineWidth: 0,
          baseValue: { type: "price", price: z.low },
          lastValueVisible: false,
          priceLineVisible: false,
          crosshairMarkerVisible: false,
        });
        fillArea.setData(zoneBars.map(d => ({ time: d.time, value: z.high })));

        indicatorSeries[`sd_${zi}_t`] = topLine;
        indicatorSeries[`sd_${zi}_b`] = botLine;
        indicatorSeries[`sd_${zi}_f`] = fillArea;
      }

    } else if (ind === "rvol" && chartData.length > 11) {
      // RVOL.mqh: DRAW_COLOR_HISTOGRAM
      const rvolData = calcRVOL(chartData, period);
      const s = chart.addHistogramSeries({
        priceScaleId: "rvol", lastValueVisible: true,
        priceFormat: { type: "price", precision: 2, minMove: 0.01 },
      });
      chart.priceScale("rvol").applyOptions({ scaleMargins: { top: 0.87, bottom: 0 }, borderVisible: false });
      s.setData(rvolData);
      indicatorSeries[key] = s;

    } else if (ind === "volume") {
      // Standard volume
      const s = chart.addHistogramSeries({ priceFormat: { type: "volume" }, priceScaleId: "volume" });
      chart.priceScale("volume").applyOptions({ scaleMargins: { top: 0.85, bottom: 0 } });
      s.setData(chartData.map(d => ({
        time: d.time, value: d.volume || 0,
        color: d.close >= d.open ? "#26a69a80" : "#ef535080",
      })));
      indicatorSeries[key] = s;

    } else if (ind === "mtf-ma") {
      // MTF_MA: SMA from current chart + higher timeframes
      // MQL5 defaults: H1 200SMA=Tomato, H4/D1/W1 200SMA=Magenta, W1/MN1 100SMA=Magenta
      if (chartData.length > period) {
        const currentColor = period === 200 ? "#FFFF00" : "#FF00FF"; // 200=yellow on current, 100=magenta
        const s = chart.addLineSeries({ color: currentColor, lineWidth: 1, title: "", lastValueVisible: false, priceLineVisible: false });
        s.setData(clip(calcSMA(chartData, period)));
        indicatorSeries[key] = s;
      }
      // HTF SMA projected onto current chart
      for (const tf of getRelevantMTFs(ALL_MTF_KAMA_TFS)) {
        const tfBars = mtfData[tf];
        if (!tfBars || tfBars.length < period + 1) continue;
        const smaData = calcSMA(tfBars, period);
        const projected = projectHTFToChartTime(smaData, chartData);
        if (projected.length === 0) continue;
        // MQL5 color: H1=Tomato, H4/D1/W1=Magenta (from MTF_MA indicator)
        const maColor = tf === "1Hour" ? "#FF6347" : "#FF00FF";
        const s = chart.addLineSeries({ color: maColor, lineWidth: 2, title: "", lastValueVisible: false, priceLineVisible: false });
        s.setData(clip(projected));
        indicatorSeries[`${key}_${tf}`] = s;
      }

    // ══════════════════════════════════════════════════════════
    // STANDARD INDICATORS
    // ══════════════════════════════════════════════════════════

    } else if (ind === "sma" && chartData.length > period) {
      const colors = { 200: "#FFFF00", 50: "#2196f3" };
      const s = chart.addLineSeries({ color: colors[period] || "#FFFFFF", lineWidth: 1, title: "", lastValueVisible: false, priceLineVisible: false });
      s.setData(clip(calcSMA(chartData, period)));
      indicatorSeries[key] = s;

    } else if (ind === "ema" && chartData.length > period) {
      const colors = { 50: "#2196f3", 200: "#ff9800" };
      const s = chart.addLineSeries({ color: colors[period] || "#FFFFFF", lineWidth: 1, title: "", lastValueVisible: false, priceLineVisible: false });
      s.setData(clip(calcEMA(chartData, period)));
      indicatorSeries[key] = s;

    } else if (ind === "dema" && chartData.length > period * 2) {
      const s = chart.addLineSeries({ color: "#00e676", lineWidth: 1, title: "", lastValueVisible: false, priceLineVisible: false });
      s.setData(clip(calcDEMA(chartData, period)));
      indicatorSeries[key] = s;

    } else if (ind === "bollinger" && chartData.length > period) {
      const bb = calcBollinger(chartData, period);
      const su = chart.addLineSeries({ color: "#9c27b0", lineWidth: 1, lineStyle: 2, title: "BB+", priceLineVisible: false });
      const sl = chart.addLineSeries({ color: "#9c27b0", lineWidth: 1, lineStyle: 2, title: "BB-", priceLineVisible: false });
      su.setData(bb.upper); sl.setData(bb.lower);
      indicatorSeries[key + "_u"] = su;
      indicatorSeries[key + "_l"] = sl;

    } else if (ind === "rsi" && chartData.length > period + 1) {
      const rsiData = calcRSI(chartData, period);
      const s = chart.addLineSeries({ color: "#ab47bc", lineWidth: 1, title: `RSI${period}`, priceScaleId: "rsi", lastValueVisible: true });
      chart.priceScale("rsi").applyOptions({ scaleMargins: { top: 0.82, bottom: 0 }, borderVisible: false });
      s.setData(rsiData);
      const ob = chart.addLineSeries({ color: "#f4433644", lineWidth: 1, lineStyle: 2, priceScaleId: "rsi", lastValueVisible: false, priceLineVisible: false });
      const os = chart.addLineSeries({ color: "#4caf5044", lineWidth: 1, lineStyle: 2, priceScaleId: "rsi", lastValueVisible: false, priceLineVisible: false });
      ob.setData(rsiData.map(d => ({ time: d.time, value: 70 }))); os.setData(rsiData.map(d => ({ time: d.time, value: 30 })));
      indicatorSeries[key] = s; indicatorSeries[key + "_ob"] = ob; indicatorSeries[key + "_os"] = os;

    } else if (ind === "macd" && chartData.length > 26) {
      const m = calcMACD(chartData);
      const sLine = chart.addLineSeries({ color: "#2196f3", lineWidth: 1, title: "MACD", priceScaleId: "macd", lastValueVisible: true });
      const sSig = chart.addLineSeries({ color: "#ff9800", lineWidth: 1, title: "Signal", priceScaleId: "macd", lastValueVisible: false });
      const sHist = chart.addHistogramSeries({ priceScaleId: "macd", lastValueVisible: false });
      chart.priceScale("macd").applyOptions({ scaleMargins: { top: 0.87, bottom: 0 }, borderVisible: false });
      sLine.setData(m.macd); sSig.setData(m.signal); sHist.setData(m.histogram);
      indicatorSeries[key + "_l"] = sLine; indicatorSeries[key + "_s"] = sSig; indicatorSeries[key + "_h"] = sHist;

    } else if (ind === "atr" && chartData.length > period + 1) {
      const s = chart.addLineSeries({ color: "#ff5722", lineWidth: 1, title: `ATR${period}`, priceScaleId: "atr", lastValueVisible: true });
      chart.priceScale("atr").applyOptions({ scaleMargins: { top: 0.87, bottom: 0 }, borderVisible: false });
      s.setData(calcATR(chartData, period)); indicatorSeries[key] = s;

    } else if (ind === "vwap" && chartData.length > 1) {
      const s = chart.addLineSeries({ color: "#ff4081", lineWidth: 2, title: "VWAP", lastValueVisible: true });
      s.setData(calcVWAP(chartData)); indicatorSeries[key] = s;
    }
  }
}

// ── MTF MA Grid Update ──────────────────────────────────────

function updateMTFGrid() {
  const tfs = ["15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week"];
  for (const tf of tfs) {
    const bars = mtfData[tf];
    if (!bars || bars.length < 201) {
      // Not enough data
      setMTFDot(`mtf-sma200-${tf}`, "neutral");
      setMTFDot(`mtf-kama-${tf}`, "neutral");
      setMTFDot(`mtf-fisher-${tf}`, "neutral");
      continue;
    }
    const lastPrice = bars[bars.length - 1].close;

    // SMA 200
    const sma200 = calcSMA(bars, 200);
    if (sma200.length > 0) {
      setMTFDot(`mtf-sma200-${tf}`, lastPrice > sma200[sma200.length - 1].value ? "bullish" : "bearish");
    }

    // KAMA
    const kama = calcKAMA(bars, 10);
    if (kama.length > 0) {
      setMTFDot(`mtf-kama-${tf}`, lastPrice > kama[kama.length - 1].value ? "bullish" : "bearish");
    }

    // Fisher
    const ef = calcEhlersFisher(bars, 32);
    if (ef.colors.length > 0) {
      const lastColor = ef.colors[ef.colors.length - 1];
      setMTFDot(`mtf-fisher-${tf}`, lastColor === "#3CB371" ? "bullish" : lastColor === "#FF4500" ? "bearish" : "neutral");
    }
  }
}

function setMTFDot(id, state) {
  const el = document.getElementById(id);
  if (el) {
    el.className = `mtf-dot ${state}`;
  }
}

// ══════════════════════════════════════════════════════════════
// THREE-TIER BAR CACHE
//   Hot:  In-memory (instant, 1-min TTL for freshness)
//   Warm: IndexedDB (50MB+, survives restarts, structured)
//   Cold: zstd-compressed files via Rust (unlimited, persistent)
// ══════════════════════════════════════════════════════════════

const barCache = {}; // Hot: "SYMBOL:TF" → { data: [], timestamp: Date }
const CACHE_TTL_MS = 60 * 1000; // 1 minute — fresh threshold
let idb = null; // Warm: IndexedDB handle

function getCacheKey(symbol, tf) { return `${symbol}:${tf}`; }

// ── IndexedDB (Warm Cache) ──────────────────────────────────

function openIndexedDB() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open("typhoon_bars", 1);
    req.onupgradeneeded = (e) => {
      const db = e.target.result;
      if (!db.objectStoreNames.contains("bars")) {
        db.createObjectStore("bars", { keyPath: "key" });
      }
    };
    req.onsuccess = (e) => { idb = e.target.result; resolve(idb); };
    req.onerror = () => reject("IndexedDB open failed");
  });
}

async function idbGet(key) {
  if (!idb) return null;
  return new Promise((resolve) => {
    const tx = idb.transaction("bars", "readonly");
    const req = tx.objectStore("bars").get(key);
    req.onsuccess = () => resolve(req.result || null);
    req.onerror = () => resolve(null);
  });
}

async function idbPut(key, data, timestamp) {
  if (!idb) return;
  return new Promise((resolve) => {
    const tx = idb.transaction("bars", "readwrite");
    tx.objectStore("bars").put({ key, data, timestamp });
    tx.oncomplete = () => resolve();
    tx.onerror = () => resolve();
  });
}

// ── Cold Cache (zstd via Rust) ──────────────────────────────

async function coldSave(key, data) {
  try {
    await invoke("save_cold_cache", { key, data: JSON.stringify(data) });
  } catch (_) {}
}

async function coldLoad(key) {
  try {
    const json = await invoke("load_cold_cache", { key });
    return JSON.parse(json);
  } catch (_) {
    return null;
  }
}

// ── Unified Cache Operations ────────────────────────────────

// Load from all tiers on startup: cold → warm → hot
async function loadBarCacheFromDisk() {
  try {
    await openIndexedDB();
    // Load from IndexedDB (warm) into hot cache
    if (idb) {
      const tx = idb.transaction("bars", "readonly");
      const store = tx.objectStore("bars");
      const req = store.getAll();
      await new Promise((resolve) => {
        req.onsuccess = () => {
          for (const entry of req.result || []) {
            if (entry.key && entry.data) {
              barCache[entry.key] = { data: entry.data, timestamp: entry.timestamp || 0 };
            }
          }
          resolve();
        };
        req.onerror = () => resolve();
      });
    }
    const count = Object.keys(barCache).length;
    if (count > 0) log(`Loaded ${count} cached bar sets from IndexedDB`, "info");

    // Also check cold cache for anything not in warm
    try {
      const coldList = JSON.parse(await invoke("list_cold_cache"));
      let coldLoaded = 0;
      for (const entry of coldList) {
        if (!barCache[entry.key]) {
          const data = await coldLoad(entry.key);
          if (data) {
            barCache[entry.key] = { data, timestamp: Date.now() - 3600000 }; // mark as stale
            await idbPut(entry.key, data, Date.now() - 3600000); // promote to warm
            coldLoaded++;
          }
        }
      }
      if (coldLoaded > 0) log(`Promoted ${coldLoaded} cold cache entries to warm`, "info");
    } catch (_) {}
  } catch (e) {
    log(`Cache init: ${e}`, "warn");
  }
}

// Save to all tiers: hot → warm → cold (async, non-blocking)
function saveBarCacheToDisk(cacheKey, data) {
  const ts = Date.now();
  barCache[cacheKey] = { data, timestamp: ts };
  // Warm (IndexedDB) — async, fire-and-forget
  idbPut(cacheKey, data, ts);
  // Cold (zstd file) — async, fire-and-forget
  coldSave(cacheKey, data);
}

// Migrate old localStorage cache to IndexedDB on first run
async function migrateLocalStorageCache() {
  const BAR_CACHE_PREFIX = "typhoon_bars_";
  let migrated = 0;
  for (let i = localStorage.length - 1; i >= 0; i--) {
    const key = localStorage.key(i);
    if (key && key.startsWith(BAR_CACHE_PREFIX)) {
      try {
        const stored = JSON.parse(localStorage.getItem(key));
        if (stored && stored.data) {
          const cacheKey = key.substring(BAR_CACHE_PREFIX.length);
          await idbPut(cacheKey, stored.data, stored.timestamp || 0);
          barCache[cacheKey] = { data: stored.data, timestamp: stored.timestamp || 0 };
          migrated++;
        }
        localStorage.removeItem(key); // clean up old format
      } catch (_) {}
    }
  }
  if (migrated > 0) log(`Migrated ${migrated} cache entries from localStorage to IndexedDB`, "ok");
}

// ── Load Queue (shows all symbols loading across tabs) ──────

const loadingSymbols = new Map(); // symbol → status string

function setLoadingStatus(symbol, status) {
  if (status) loadingSymbols.set(symbol, status);
  else loadingSymbols.delete(symbol);
  updateLoadingIndicator();
}

function updateLoadingIndicator() {
  const el = document.getElementById("loading-indicator");
  if (loadingSymbols.size === 0) {
    el.classList.add("hidden");
  } else {
    el.classList.remove("hidden");
    const parts = [...loadingSymbols.entries()].map(([sym, st]) => `${sym} (${st})`);
    el.textContent = parts.join(" | ");
  }
}

// ── Load Chart Data ─────────────────────────────────────────

let liveBarInterval = null;

async function loadChart(symbol, timeframe) {
  setLoadingStatus(symbol, "loading...");

  // Set symbol immediately so tab identity is correct
  currentSymbol = symbol;
  currentTimeframe = timeframe;
  const loadTabId = activeTabId; // capture which tab initiated this load

  try {
    const limit = parseInt(document.getElementById("bar-count").value) || 1000;
    const cacheKey = getCacheKey(symbol, timeframe);
    let bars;

    // Strategy: show cached data IMMEDIATELY, then refresh in background
    const cached = barCache[cacheKey];
    const isFresh = cached && (Date.now() - cached.timestamp) < CACHE_TTL_MS;
    const hasEnough = cached && cached.data && cached.data.length >= limit * 0.5;

    if (hasEnough) {
      // Display cached data instantly
      bars = cached.data;
      log(`${symbol} @ ${timeframe}: ${bars.length} bars from cache (${isFresh ? "fresh" : "stale, refreshing..."})`, "info");

      if (!isFresh) {
        // Refresh in background — will update chart when done
        (async () => {
          try {
            const freshJson = await invoke("get_bars", { symbol, timeframe, limit });
            const freshBars = JSON.parse(freshJson);
            if (freshBars.length > 0) {
              barCache[cacheKey] = { data: freshBars, timestamp: Date.now() };
              saveBarCacheToDisk(cacheKey, freshBars);
              // If still on same tab/symbol, update chart
              if (currentSymbol === symbol && currentTimeframe === timeframe) {
                const freshChartData = freshBars.map(b => ({
                  time: Math.floor(new Date(b.timestamp).getTime() / 1000),
                  open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume,
                }));
                candleSeries.setData(freshChartData);
                lastPrice = freshChartData[freshChartData.length - 1].close;
                log(`${symbol} @ ${timeframe}: refreshed to ${freshBars.length} bars`, "ok");
              }
            }
          } catch (_) {}
        })();
      }
    } else {
      // No usable cache — fetch synchronously
      const barsJson = await invoke("get_bars", { symbol, timeframe, limit });
      bars = JSON.parse(barsJson);
      barCache[cacheKey] = { data: bars, timestamp: Date.now() };
      saveBarCacheToDisk(cacheKey, bars);
      if (bars.length > 0) {
        const first = bars[0].timestamp.substring(0, 10);
        const last = bars[bars.length - 1].timestamp.substring(0, 10);
        setLoadingStatus(symbol, `${first} → ${last} · ${bars.length} bars`);
      }
    }

    const chartData = bars.map((b) => ({
      time: Math.floor(new Date(b.timestamp).getTime() / 1000),
      open: b.open,
      high: b.high,
      low: b.low,
      close: b.close,
      volume: b.volume,
    }));

    if (chartData.length === 0) {
      log(`No bars returned for ${symbol} @ ${timeframe}`, "warn");
      setText("connect-status-bar", `No data for ${symbol} @ ${timeframe}`);
      setLoadingStatus(symbol, null);
      return;
    }

    // Guard: if user switched tabs during async load, don't overwrite wrong chart
    if (activeTabId !== loadTabId) {
      log(`Discarding late bars for ${symbol} (tab switched)`, "warn");
      setLoadingStatus(symbol, null);
      return;
    }

    candleSeries.setData(chartData);
    chart.timeScale().fitContent();
    currentSymbol = symbol;
    currentTimeframe = timeframe;
    if (chartData.length > 0) lastPrice = chartData[chartData.length - 1].close;

    // Load MTF data for multi-timeframe indicators, then apply all + update grid
    loadMTFData(symbol).then(() => {
      applyIndicators(chartData);
      updateMTFGrid();
    }).catch(() => applyIndicators(chartData));

    log(`${symbol} @ ${timeframe}: ${chartData.length} bars, last=$${lastPrice}`, "ok");
    setText("connect-status-bar", `${symbol} — ${chartData.length} bars`);
    setLoadingStatus(symbol, null);
    updateTabLabel();

    // Start live bar polling (update latest bar every 10s)
    if (liveBarInterval) clearInterval(liveBarInterval);
    liveBarInterval = setInterval(() => updateLatestBar(symbol, timeframe), 10000);

    // Background pre-fetch: load all other timeframes for this symbol
    prefetchAllTimeframes(symbol, timeframe, limit);

    // Load news and fundamentals for this symbol (background)
    loadNewsAndFundamentals(symbol);
  } catch (e) {
    log(`Chart load failed for ${symbol} @ ${timeframe}: ${e}`, "error");
    setText("connect-status-bar", `Chart error: ${e}`);
    setLoadingStatus(symbol, null);
  }
}

const ALL_TIMEFRAMES = ["1Min", "5Min", "15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week"];

async function prefetchAllTimeframes(symbol, currentTF, limit) {
  const toFetch = ALL_TIMEFRAMES.filter(tf => tf !== currentTF);
  log(`Pre-fetching ${toFetch.length} timeframes for ${symbol}...`, "info");
  for (const tf of toFetch) {
    const cacheKey = getCacheKey(symbol, tf);
    const cached = barCache[cacheKey];
    // Skip if already cached and fresh
    if (cached && (Date.now() - cached.timestamp) < CACHE_TTL_MS * 60) continue; // 60× TTL for prefetch (1 hour)
    try {
      const barsJson = await invoke("get_bars", { symbol, timeframe: tf, limit });
      const bars = JSON.parse(barsJson);
      if (bars.length > 0) {
        barCache[cacheKey] = { data: bars, timestamp: Date.now() };
        saveBarCacheToDisk(cacheKey, bars);
        log(`Pre-cached ${symbol} @ ${tf}: ${bars.length} bars`, "info");
      }
    } catch (_) {
      // Silent fail on prefetch — not critical
    }
  }
  log(`Pre-fetch complete for ${symbol}`, "ok");
}

let lastBarTime = 0;

async function updateLatestBar(symbol, timeframe) {
  if (symbol !== currentSymbol || timeframe !== currentTimeframe) return;
  try {
    const barsJson = await invoke("get_bars", { symbol, timeframe, limit: 5 });
    const bars = JSON.parse(barsJson);
    if (bars.length === 0) return;
    const latest = bars[bars.length - 1];
    const barTime = Math.floor(new Date(latest.timestamp).getTime() / 1000);
    const bar = {
      time: barTime,
      open: latest.open,
      high: latest.high,
      low: latest.low,
      close: latest.close,
    };
    candleSeries.update(bar);
    lastPrice = bar.close;

    // If a NEW bar has printed (different timestamp), refresh all indicators
    if (barTime !== lastBarTime && lastBarTime !== 0) {
      log(`New bar on ${symbol} @ ${timeframe}`, "info");
      const chartData = candleSeries.data();
      if (chartData && chartData.length > 0) {
        applyIndicators(chartData);
      }
    }
    lastBarTime = barTime;
  } catch (_) {}
}

// ── Dashboard Update (all 11 labels) ────────────────────────

async function updateDashboard() {
  try {
    // Margin info (includes equity, balance, ML, zone, spread tolerance)
    const marginJson = await invoke("get_margin_info");
    const mi = JSON.parse(marginJson);

    const hasPositions = mi.gross_lots > 0;
    const mlText = hasPositions ? `${mi.margin_level_pct.toFixed(1)}%` : "—";
    setText("account-info", `Eq: $${fmt(mi.equity)}${hasPositions ? ` | ML: ${mlText}` : ""}`);
    setText("info-equity", `Eq: $${mi.equity.toFixed(2)}`);
    setText("info-balance", `Bal: $${mi.balance.toFixed(2)}`);

    const mlEl = document.getElementById("info-margin");
    if (!hasPositions || !mi.zone) {
      mlEl.textContent = "ML: —";
      mlEl.className = "dash-row neutral";
    } else {
      mlEl.textContent = `ML: ${mi.margin_level_pct.toFixed(1)}% [${mi.zone}]`;
      mlEl.className = `dash-row ${mi.zone === "TRIM" ? "positive" : mi.zone === "DEAD ZONE" ? "neutral" : "negative"}`;
    }

    // Positions
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);

    let totalPL = 0;
    let posText = "Position: —";
    let posQty = 0;
    let posSide = "";
    let posEntry = 0;

    for (const p of positions) {
      if (p.symbol === currentSymbol || p.symbol === currentSymbol.replace("/", "")) {
        totalPL = p.unrealized_pl;
        posQty = Math.abs(p.qty);
        posSide = p.side;
        posEntry = p.avg_entry_price;
        posText = `${p.side === "long" ? "Long" : "Short"} ${posQty} lots`;
      }
    }

    setText("info-position", posText);
    const plEl = document.getElementById("info-pl");
    plEl.textContent = `P/L: $${totalPL.toFixed(2)}`;
    plEl.className = `dash-row ${totalPL >= 0 ? "positive" : "negative"}`;

    // SL/TP P/L, Risk, R:R
    if (posQty > 0 && posEntry > 0) {
      try {
        const stJson = await invoke("get_sl_tp_pl", {
          symbol: currentSymbol, qty: posQty, side: posSide, entryPrice: posEntry,
        });
        const st = JSON.parse(stJson);

        if (st.sl_pl !== null) {
          setTextClass("info-sl-pl", `SL P/L: $${st.sl_pl.toFixed(2)}`, st.sl_pl >= 0 ? "positive" : "negative");
          if (mi.balance > 0) {
            setText("info-risk", `Risk: $${Math.abs(st.sl_pl).toFixed(2)} (${(Math.abs(st.sl_pl) / mi.balance * 100).toFixed(2)}%)`);
          }
        } else {
          setText("info-sl-pl", "SL P/L: —");
          setText("info-risk", "Risk: —");
        }
        if (st.tp_pl !== null) setTextClass("info-tp-pl", `TP P/L: $${st.tp_pl.toFixed(2)}`, "positive");
        else setText("info-tp-pl", "TP P/L: —");
        if (st.rr !== null) setText("info-rr", `RR: ${st.rr.toFixed(2)}`);
        else setText("info-rr", "RR: —");
      } catch (_) {}

      // VaR
      if (lastPrice > 0) {
        try {
          const varJson = await invoke("calculate_position_var", {
            symbol: currentSymbol, positionSize: posQty, currentPrice: lastPrice,
          });
          const v = JSON.parse(varJson);
          setText("info-var", `VaR: $${v.var_dollars.toFixed(2)}`);
        } catch (_) { setText("info-var", "VaR: —"); }
      }
    } else {
      setText("info-sl-pl", "SL P/L: —");
      setText("info-tp-pl", "TP P/L: —");
      setText("info-rr", "RR: —");
      setText("info-var", "VaR: —");
      setText("info-risk", "Risk: —");
    }

    updateNextBarTime();
    updatePositionsPanel();
    checkAlerts();
  } catch (_) {}
}

function updateNextBarTime() {
  const tfMap = {
    "1Min": 60, "5Min": 300, "15Min": 900, "30Min": 1800, "1Hour": 3600,
    "4Hour": 14400, "1Day": 86400, "1Week": 604800,
  };
  const secs = tfMap[currentTimeframe] || 3600;
  const now = Math.floor(Date.now() / 1000);
  const remaining = Math.ceil(now / secs) * secs - now;
  const h = Math.floor(remaining / 3600);
  const m = Math.floor((remaining % 3600) / 60);
  const s = remaining % 60;
  setText("info-time", `Next bar: ${h > 0 ? `${h}H ${m}M ${s}s` : m > 0 ? `${m}M ${s}s` : `${s}s`}`);
}

function setText(id, text) {
  const el = document.getElementById(id);
  if (el && el.textContent !== text) el.textContent = text;
}
function setTextClass(id, text, cls) {
  const el = document.getElementById(id);
  if (el) { el.textContent = text; el.className = `dash-row ${cls}`; }
}
function fmt(n) { return Number(n).toLocaleString(undefined, { maximumFractionDigits: 0 }); }

// ── Symbol Autocomplete ─────────────────────────────────────

let symbolsLoaded = false;
let autocompleteIndex = -1;

async function loadSymbolList() {
  try {
    const count = await invoke("load_symbols");
    console.log(`Loaded ${count} tradable symbols`);
    symbolsLoaded = true;
  } catch (e) {
    console.error("Failed to load symbols:", e);
  }
}

function setupAutocomplete() {
  const input = document.getElementById("symbol-input");
  const list = document.getElementById("symbol-autocomplete");
  let debounceTimer = null;

  input.addEventListener("input", () => {
    clearTimeout(debounceTimer);
    const q = input.value.trim();
    if (q.length < 1 || !symbolsLoaded) {
      list.classList.add("hidden");
      return;
    }
    debounceTimer = setTimeout(async () => {
      try {
        const resultJson = await invoke("search_symbols", { query: q });
        const matches = JSON.parse(resultJson);
        list.textContent = "";
        autocompleteIndex = -1;
        if (matches.length === 0) {
          list.classList.add("hidden");
          return;
        }
        for (const [sym, name] of matches) {
          const item = document.createElement("div");
          item.className = "autocomplete-item";
          const symSpan = document.createElement("span");
          symSpan.className = "sym";
          symSpan.textContent = sym;
          const nameSpan = document.createElement("span");
          nameSpan.className = "name";
          nameSpan.textContent = name;
          item.appendChild(symSpan);
          item.appendChild(nameSpan);
          item.addEventListener("mousedown", (e) => {
            e.preventDefault();
            input.value = sym;
            list.classList.add("hidden");
            triggerLoad();
          });
          list.appendChild(item);
        }
        list.classList.remove("hidden");
      } catch (_) {
        list.classList.add("hidden");
      }
    }, 150);
  });

  input.addEventListener("keydown", (e) => {
    const items = list.querySelectorAll(".autocomplete-item");
    if (e.key === "ArrowDown") {
      e.preventDefault();
      autocompleteIndex = Math.min(autocompleteIndex + 1, items.length - 1);
      items.forEach((el, i) => el.classList.toggle("selected", i === autocompleteIndex));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      autocompleteIndex = Math.max(autocompleteIndex - 1, 0);
      items.forEach((el, i) => el.classList.toggle("selected", i === autocompleteIndex));
    } else if (e.key === "Enter") {
      if (autocompleteIndex >= 0 && items[autocompleteIndex]) {
        input.value = items[autocompleteIndex].querySelector(".sym").textContent;
        list.classList.add("hidden");
      }
      triggerLoad();
    } else if (e.key === "Escape") {
      list.classList.add("hidden");
    }
  });

  input.addEventListener("blur", () => {
    setTimeout(() => list.classList.add("hidden"), 200);
  });
}

// ── Chart Trigger ────────────────────────────────────────────

const CRYPTO_MAP = {
  "BTC": "BTC/USD", "ETH": "ETH/USD", "SOL": "SOL/USD", "DOGE": "DOGE/USD",
  "ADA": "ADA/USD", "XRP": "XRP/USD", "DOT": "DOT/USD", "AVAX": "AVAX/USD",
  "LINK": "LINK/USD", "MATIC": "MATIC/USD", "UNI": "UNI/USD", "SHIB": "SHIB/USD",
  "LTC": "LTC/USD", "BCH": "BCH/USD", "AAVE": "AAVE/USD", "SUSHI": "SUSHI/USD",
};

function triggerLoad() {
  let symbol = document.getElementById("symbol-input").value.trim().toUpperCase();
  const tf = document.getElementById("timeframe-select").value;
  if (!symbol) return;
  if (CRYPTO_MAP[symbol]) symbol = CRYPTO_MAP[symbol];
  document.getElementById("symbol-input").value = symbol;
  document.getElementById("symbol-autocomplete").classList.add("hidden");
  loadChart(symbol, tf);
}

// ── Button Handlers ─────────────────────────────────────────

function setupButtons() {
  // Auto-load on timeframe or bar count change
  document.getElementById("timeframe-select").addEventListener("change", triggerLoad);
  document.getElementById("bar-count").addEventListener("change", triggerLoad);

  // Buy Lines: SL = lowest visible, TP = highest visible
  document.getElementById("btn-buy-lines").addEventListener("click", () => {
    const data = candleSeries.data();
    if (!data || data.length === 0) return;
    const recent = data.slice(-50);
    createSLLine(Math.min(...recent.map((d) => d.low)));
    createTPLine(Math.max(...recent.map((d) => d.high)));
  });

  // Sell Lines: SL = highest, TP = lowest
  document.getElementById("btn-sell-lines").addEventListener("click", () => {
    const data = candleSeries.data();
    if (!data || data.length === 0) return;
    const recent = data.slice(-50);
    createSLLine(Math.max(...recent.map((d) => d.high)));
    createTPLine(Math.min(...recent.map((d) => d.low)));
  });

  document.getElementById("btn-destroy-lines").addEventListener("click", () => {
    removeSLLine();
    removeTPLine();
  });

  // ── Open Trade — calculates lots via backend, confirms, places ──
  let orderInFlight = false;
  document.getElementById("btn-trade").addEventListener("click", async () => {
    if (orderInFlight) return; // prevent double-fire
    const sl = getSLPrice();
    const tp = getTPPrice();
    if (!sl || !tp || !currentSymbol) {
      alert("Set SL and TP lines first, and load a chart.");
      return;
    }
    try {
      const calcJson = await invoke("calculate_lots", {
        symbol: currentSymbol, slPrice: sl, tpPrice: tp, currentPrice: lastPrice,
      });
      const calc = JSON.parse(calcJson);

      if (calc.lots <= 0) {
        alert(`Cannot place order: calculated lots = 0\nMode: ${calc.mode}\nSL distance: ${calc.sl_distance}`);
        return;
      }

      const msg = `${calc.side.toUpperCase()} ${currentSymbol}\n` +
        `Lots: ${calc.lots} (×${calc.count})\nMode: ${calc.mode}\n` +
        `SL: ${sl} | TP: ${tp}\nSL distance: ${calc.sl_distance.toFixed(4)}\n` +
        (calc.risk_money > 0 ? `Risk: $${calc.risk_money.toFixed(2)}\n` : "") +
        `\nPlace order?`;

      if (!confirm(msg)) return;

      orderInFlight = true;
      try {
        const orderType = document.getElementById("order-type").value;
        for (let i = 0; i < calc.count; i++) {
          if (orderType === "bracket") {
            await invoke("place_bracket_order", { symbol: currentSymbol, qty: calc.lots, side: calc.side, tpPrice: tp, slPrice: sl });
          } else if (orderType === "limit") {
            await invoke("place_limit_order", { symbol: currentSymbol, qty: calc.lots, side: calc.side, limitPrice: lastPrice, tif: "gtc" });
          } else if (orderType === "stop") {
            await invoke("place_stop_order", { symbol: currentSymbol, qty: calc.lots, side: calc.side, stopPrice: sl, tif: "gtc" });
          } else if (orderType === "stop_limit") {
            await invoke("place_stop_limit_order", { symbol: currentSymbol, qty: calc.lots, side: calc.side, stopPrice: sl, limitPrice: tp, tif: "gtc" });
          } else if (orderType === "trailing_stop") {
            const trail = Math.abs(lastPrice - sl);
            await invoke("place_trailing_stop", { symbol: currentSymbol, qty: calc.lots, side: calc.side, trailPrice: trail, trailPercent: null });
          } else {
            await invoke("place_order", { symbol: currentSymbol, qty: calc.lots, side: calc.side });
          }
        }
        await invoke("set_sl_level", { symbol: currentSymbol, price: sl });
        await invoke("set_tp_level", { symbol: currentSymbol, price: tp });
        updateDashboard();
      } finally {
        orderInFlight = false;
      }
    } catch (e) {
      orderInFlight = false;
      alert(`Order failed: ${e}`);
    }
  });

  // ── Close All ──
  let closeAllInFlight = false;
  document.getElementById("btn-close-all").addEventListener("click", async () => {
    if (closeAllInFlight) return;
    if (!currentSymbol || !confirm(`Close ALL positions on ${currentSymbol}?`)) return;
    closeAllInFlight = true;
    try {
      await invoke("close_position", { symbol: currentSymbol, qty: null });
      updateDashboard();
    } catch (e) { alert(`Close failed: ${e}`); }
    finally { closeAllInFlight = false; }
  });

  // ── Close Partial (smart) ──
  let closePartialInFlight = false;
  document.getElementById("btn-close-partial").addEventListener("click", async () => {
    if (closePartialInFlight || !currentSymbol) return;
    // Smart close: show floating window with fraction buttons
    const win = createWindow({ title: `Close Partial — ${currentSymbol}`, width: 320, height: 250 });
    const posJson = await invoke("get_positions").catch(() => "[]");
    const positions = JSON.parse(posJson);
    const pos = positions.find(p => p.symbol === currentSymbol || p.symbol === currentSymbol.replace("/", ""));
    if (!pos) { win.setContent("No position found"); return; }

    const totalQty = Math.abs(pos.qty);
    const container = document.createElement("div");
    container.style.cssText = "display:flex;flex-direction:column;gap:8px;padding:4px;";

    const info = document.createElement("div");
    info.textContent = `${pos.side === "long" ? "Long" : "Short"} ${totalQty} lots | P/L: $${pos.unrealized_pl.toFixed(2)}`;
    info.style.color = "#8cf";
    container.appendChild(info);

    const input = document.createElement("input");
    input.type = "number";
    input.step = "0.01";
    input.value = (totalQty / 2).toFixed(2);
    input.style.cssText = "background:#111;color:#fff;border:1px solid #555;padding:6px;font-family:inherit;";
    container.appendChild(input);

    const btnRow = document.createElement("div");
    btnRow.style.cssText = "display:flex;gap:4px;";
    for (const [label, frac] of [["25%", 0.25], ["50%", 0.5], ["75%", 0.75], ["100%", 1.0]]) {
      const btn = document.createElement("button");
      btn.textContent = label;
      btn.style.cssText = "flex:1;padding:6px;background:#1a3a5a;color:#8ff;border:1px solid #555;cursor:pointer;font-family:inherit;";
      btn.addEventListener("click", () => { input.value = (totalQty * frac).toFixed(2); });
      btnRow.appendChild(btn);
    }
    container.appendChild(btnRow);

    const closeBtn = document.createElement("button");
    closeBtn.textContent = "Close Position";
    closeBtn.style.cssText = "padding:8px;background:#5a1a1a;color:#f88;border:1px solid #f44;cursor:pointer;font-family:inherit;font-weight:bold;";
    closeBtn.addEventListener("click", async () => {
      const qty = parseFloat(input.value);
      if (isNaN(qty) || qty <= 0) return;
      closePartialInFlight = true;
      try {
        await invoke("close_position", { symbol: currentSymbol, qty });
        updateDashboard();
        win.close();
      } catch (e) { alert(`Close failed: ${e}`); }
      finally { closePartialInFlight = false; }
    });
    container.appendChild(closeBtn);

    win.contentElement.textContent = "";
    win.appendElement(container);
  });

  // ── Set SL/TP — sync dragged lines to backend ──
  document.getElementById("btn-set-sl").addEventListener("click", async () => {
    const sl = getSLPrice();
    if (!sl || !currentSymbol) return;
    await invoke("set_sl_level", { symbol: currentSymbol, price: sl });
    updateDashboard();
  });

  document.getElementById("btn-set-tp").addEventListener("click", async () => {
    const tp = getTPPrice();
    if (!tp || !currentSymbol) return;
    await invoke("set_tp_level", { symbol: currentSymbol, price: tp });
    updateDashboard();
  });

  // ── Martingale Toggle — syncs to backend ──
  document.getElementById("btn-martingale").addEventListener("click", async () => {
    try {
      const resultJson = await invoke("toggle_martingale");
      const result = JSON.parse(resultJson);
      const btn = document.getElementById("btn-martingale");
      btn.textContent = result.label;
      btn.style.backgroundColor = { Off: "#3a3a00", Long: "#0a5f38", Short: "#5a1a1a", Unwind: "#5a3a00" }[result.mode] || "#3a3a00";
    } catch (e) { alert(`MG toggle failed: ${e}`); }
  });

  // ── Open MG — calculates sizing and places hedge/bias ──
  let mgInFlight = false;
  document.getElementById("btn-open-mg").addEventListener("click", async () => {
    if (mgInFlight || !currentSymbol) return;
    try {
      const sizeJson = await invoke("calc_open_mg_size");
      const size = JSON.parse(sizeJson);

      const sl = getSLPrice();
      const tp = getTPPrice();
      let direction = "Long";
      if (sl && tp) {
        direction = tp > sl ? "Long" : "Short";
      } else {
        const pick = prompt("No SL/TP lines. Enter direction (Long/Short):");
        if (!pick) return;
        direction = pick;
      }

      const msg = `Open MG ${direction} on ${currentSymbol}\n\n` +
        `Equity: $${fmt(size.equity)}\nSpread tolerance: $${size.spread_tolerance}/lot\n` +
        `Safe gross: ${fmt(size.safe_gross)} lots\nPer side: ${fmt(size.per_side)} lots\n\n` +
        `Place ${fmt(size.per_side)} ${direction === "Long" ? "BUY" : "SELL"} (bias) +\n` +
        `      ${fmt(size.per_side)} ${direction === "Long" ? "SELL" : "BUY"} (hedge)?\n`;

      if (!confirm(msg)) return;

      mgInFlight = true;
      try {
        await invoke("open_martingale_hedge", { symbol: currentSymbol, direction });
        await invoke("set_martingale_mode", { mode: direction });
        document.getElementById("btn-martingale").textContent = `MG: ${direction.toUpperCase()}`;
        updateDashboard();
      } finally { mgInFlight = false; }
    } catch (e) { mgInFlight = false; alert(`Open MG failed: ${e}`); }
  });

  // ── Order Mode Selector ──
  document.getElementById("order-mode").addEventListener("change", async (e) => {
    try {
      await invoke("set_order_mode", { mode: e.target.value });
    } catch (err) { alert(`Failed to set order mode: ${err}`); }
  });
}

// ── Keyboard Shortcuts ──────────────────────────────────────

function setupKeyboard() {
  document.addEventListener("keydown", (e) => {
    if (e.target.tagName === "INPUT" || e.target.tagName === "SELECT") return;
    switch (e.key) {
      case "b": document.getElementById("btn-buy-lines").click(); break;
      case "s": document.getElementById("btn-sell-lines").click(); break;
      case "d": document.getElementById("btn-destroy-lines").click(); break;
      case "t": document.getElementById("btn-trade").click(); break;
      case "m": document.getElementById("btn-martingale").click(); break;
      case "o": document.getElementById("btn-open-mg").click(); break;
      case "c": document.getElementById("btn-close-all").click(); break;
      case "p": document.getElementById("btn-close-partial").click(); break;
      case "Escape": removeSLLine(); removeTPLine(); break;
      case "a":
        if (currentSymbol && lastPrice > 0) {
          const dir = prompt("Alert direction (above/below):", "above");
          if (dir === "above" || dir === "below") addPriceAlert(currentSymbol, lastPrice, dir);
        }
        break;
      case "h": updateOrdersPanel(); break;
      case "l": // trend Line
        drawingMode = "trendline"; drawingAnchor = null;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing mode: trend line — click two points", "info");
        break;
      case "f": // Fibonacci
        drawingMode = "fibonacci"; drawingAnchor = null;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing mode: Fibonacci — click high and low points", "info");
        break;
      case "x": // delete last drawing
        if (drawings.length > 0) {
          drawings.pop(); saveDrawings(); renderDrawings(); renderDrawingsExtended();
          log("Deleted last drawing", "info");
        }
        break;
      case "n": // horizontal line
        drawingMode = "horizontal"; drawingAnchor = null; channelThirdClick = false;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing mode: horizontal line — click to place", "info");
        break;
      case "r": // rectangle
        drawingMode = "rectangle"; drawingAnchor = null; channelThirdClick = false;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing mode: rectangle — click two corners", "info");
        break;
      case "y": // channel (parallel)
        drawingMode = "channel"; drawingAnchor = null; channelThirdClick = false;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing mode: channel — click two points + offset", "info");
        break;
      case "w":
        if (e.altKey) { closeAllWindows(); e.preventDefault(); }
        break;
      case "g":
        if (e.altKey) { tileWindows(); e.preventDefault(); }
        break;
    }
  });
}

// ── Credential Storage (OS Keychain + localStorage metadata) ──

const STORAGE_KEY = "typhoon_accounts";

// localStorage stores ONLY { name, type } — no keys/secrets.
// Actual credentials stored in OS keychain (gnome-keyring / KWallet / macOS Keychain).
function loadSavedAccounts() {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) || "[]");
  } catch { return []; }
}

function saveAccountMetadata(accounts) {
  // Strip any leftover apiKey/secretKey from metadata (migration safety)
  const clean = accounts.map(a => ({ name: a.name, type: a.type }));
  localStorage.setItem(STORAGE_KEY, JSON.stringify(clean));
}

async function saveCredentials(accountName, apiKey, secretKey, accountType) {
  // Save keys to OS keychain
  try {
    await invoke("keychain_save", { accountName, apiKey, secretKey });
    log(`Credentials saved to OS keychain for "${accountName}"`, "ok");
  } catch (e) {
    log(`Keychain save failed (${e}), falling back to localStorage`, "warn");
    // Fallback: save in localStorage (legacy behavior)
    const accounts = loadSavedAccounts();
    const existing = accounts.findIndex(a => a.name === accountName);
    const entry = { name: accountName, apiKey, secretKey, type: accountType };
    if (existing >= 0) accounts[existing] = entry;
    else accounts.push(entry);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(accounts));
    return;
  }
  // Save metadata (no keys) in localStorage
  const accounts = loadSavedAccounts();
  const existing = accounts.findIndex(a => a.name === accountName);
  const entry = { name: accountName, type: accountType };
  if (existing >= 0) accounts[existing] = entry;
  else accounts.push(entry);
  saveAccountMetadata(accounts);
}

async function loadCredentials(accountName) {
  // Try OS keychain first
  try {
    const json = await invoke("keychain_load", { accountName });
    return JSON.parse(json); // { apiKey, secretKey }
  } catch (_) {
    // Fallback: check localStorage for legacy entries with keys
    const accounts = loadSavedAccounts();
    const acct = accounts.find(a => a.name === accountName);
    if (acct && acct.apiKey) return { apiKey: acct.apiKey, secretKey: acct.secretKey };
    return null;
  }
}

async function deleteCredentials(accountName) {
  try { await invoke("keychain_delete", { accountName }); } catch (_) {}
  const accounts = loadSavedAccounts().filter(a => a.name !== accountName);
  saveAccountMetadata(accounts);
}

function populateAccountDropdown() {
  const select = document.getElementById("saved-accounts");
  const accounts = loadSavedAccounts();
  while (select.options.length > 1) select.remove(1);
  for (const acct of accounts) {
    const opt = document.createElement("option");
    opt.value = acct.name;
    opt.textContent = `${acct.name} (${acct.type})`;
    select.appendChild(opt);
  }
}

async function fillFormFromAccount(name) {
  const accounts = loadSavedAccounts();
  const acct = accounts.find(a => a.name === name);
  if (acct) {
    document.getElementById("account-name").value = acct.name;
    document.getElementById("account-type").value = acct.type;
    // Load keys from keychain asynchronously
    const creds = await loadCredentials(name);
    if (creds) {
      document.getElementById("api-key").value = creds.apiKey || "";
      document.getElementById("secret-key").value = creds.secretKey || "";
    } else {
      document.getElementById("api-key").value = "";
      document.getElementById("secret-key").value = "";
    }
  } else {
    document.getElementById("account-name").value = "";
    document.getElementById("api-key").value = "";
    document.getElementById("secret-key").value = "";
    document.getElementById("account-type").value = "paper";
  }
}

// ── Connection ──────────────────────────────────────────────

let dashboardInterval = null;

function setupConnect() {
  const modal = document.getElementById("connect-modal");
  const status = document.getElementById("connect-status");

  // Load saved accounts into dropdown
  populateAccountDropdown();

  // When saved account selected, fill form (async — loads from keychain)
  document.getElementById("saved-accounts").addEventListener("change", (e) => {
    fillFormFromAccount(e.target.value);
  });

  // Delete saved account (from keychain + localStorage)
  document.getElementById("btn-delete-account").addEventListener("click", async () => {
    const select = document.getElementById("saved-accounts");
    const name = select.value;
    if (!name) return;
    if (!confirm(`Delete saved account "${name}"?`)) return;
    await deleteCredentials(name);
    populateAccountDropdown();
    await fillFormFromAccount("");
    status.textContent = `Deleted "${name}"`;
    status.style.color = "#ff8";
  });

  // Warn on live account selection
  document.getElementById("account-type").addEventListener("change", (e) => {
    if (e.target.value === "live") {
      e.target.classList.add("live-warning");
      status.textContent = "WARNING: Live trading uses real money!";
      status.style.color = "#f44";
    } else {
      e.target.classList.remove("live-warning");
      status.textContent = "";
    }
  });

  // Connect button
  document.getElementById("btn-connect").addEventListener("click", async () => {
    const apiKey = document.getElementById("api-key").value.trim();
    const secretKey = document.getElementById("secret-key").value.trim();
    const accountType = document.getElementById("account-type").value;
    const accountName = document.getElementById("account-name").value.trim();
    const saveCredentials = document.getElementById("save-credentials").checked;
    const paper = accountType === "paper";

    if (!apiKey || !secretKey) {
      status.textContent = "API Key and Secret Key are required";
      return;
    }

    status.textContent = "Connecting...";
    status.style.color = "#ff8";

    try {
      const result = await invoke("connect", { apiKey, secretKey, paper });
      const acct = JSON.parse(result);

      // Save credentials to OS keychain if requested
      if (saveCredentials && accountName) {
        await saveCredentials(accountName, apiKey, secretKey, accountType);
        populateAccountDropdown();
      }

      const typeLabel = paper ? "Paper" : "LIVE";
      status.textContent = `Connected! [${typeLabel}] Equity: $${Number(acct.equity).toFixed(2)} — Loading symbols...`;
      status.style.color = "#8f8";

      // Load symbol list for autocomplete (async, don't block connect)
      loadSymbolList().then(() => {
        status.textContent = `Connected! [${typeLabel}] Equity: $${Number(acct.equity).toFixed(2)}`;
      });

      setTimeout(() => modal.classList.add("hidden"), 1200);

      // Start dashboard updates (only once)
      if (!dashboardInterval) {
        dashboardInterval = setInterval(updateDashboard, 2000);
      }
    } catch (e) {
      status.textContent = `Failed: ${e}`;
      status.style.color = "#f88";
    }
  });

  // Enter to connect
  document.getElementById("secret-key").addEventListener("keydown", (e) => {
    if (e.key === "Enter") document.getElementById("btn-connect").click();
  });

  // Auto-connect if saved account exists (load keys from OS keychain)
  const accounts = loadSavedAccounts();
  if (accounts.length >= 1) {
    const acctMeta = accounts[0];
    fillFormFromAccount(acctMeta.name);
    document.getElementById("saved-accounts").value = acctMeta.name;

    status.textContent = "Auto-connecting...";
    status.style.color = "#ff8";

    loadCredentials(acctMeta.name).then(async (creds) => {
      if (!creds || !creds.apiKey || !creds.secretKey) {
        status.textContent = "No credentials found — enter keys manually";
        status.style.color = "#f88";
        return;
      }
      const paper = acctMeta.type === "paper";
      try {
        const result = await invoke("connect", { apiKey: creds.apiKey, secretKey: creds.secretKey, paper });
        const parsed = JSON.parse(result);
        const typeLabel = paper ? "Paper" : "LIVE";
        status.textContent = `Connected [${typeLabel}] $${Number(parsed.equity).toFixed(0)}`;
        status.style.color = "#8f8";
        modal.classList.add("hidden");
        if (!dashboardInterval) {
          dashboardInterval = setInterval(updateDashboard, 2000);
        }
        loadSymbolList();
        log(`Auto-connected to ${acctMeta.name} (${typeLabel}) [keychain]`, "ok");
      } catch (e) {
        status.textContent = `Auto-connect failed: ${e}`;
        status.style.color = "#f88";
        log(`Auto-connect failed: ${e}`, "error");
      }
    });
  }
}

// ── Log Panel ───────────────────────────────────────────────

// ── News & Fundamentals ─────────────────────────────────────

async function loadNewsAndFundamentals(symbol) {
  const panel = document.getElementById("news-content");
  if (!panel) return;
  panel.textContent = "";

  // Load fundamentals (SEC EDGAR — cached, show summary + click to expand)
  try {
    const cacheKey = `fundamentals:${symbol}`;
    let data = await coldLoad(cacheKey);
    if (!data) {
      const json = await invoke("get_company_fundamentals", { symbol });
      data = JSON.parse(json);
      if (data && data.entity) coldSave(cacheKey, data);
    }
    if (data && data.entity) {
      // Compact summary in sidebar
      const btn = document.createElement("div");
      btn.className = "news-item";
      btn.style.cursor = "pointer";
      btn.style.borderBottom = "1px solid #333";
      const label = document.createElement("div");
      label.className = "news-headline";
      label.textContent = `${data.entity} — Fundamentals`;
      label.style.color = "#8cf";
      const sub = document.createElement("div");
      sub.className = "news-source";
      sub.textContent = "Click to open detailed view";
      btn.appendChild(label);
      btn.appendChild(sub);
      btn.addEventListener("click", () => openFundamentalsWindow(symbol, data));
      panel.appendChild(btn);
      log(`Fundamentals loaded for ${symbol}`, "ok");
    }
  } catch (e) {
    log(`Fundamentals failed for ${symbol}: ${e}`, "warn");
  }

  // Load news (Alpaca News API — cached)
  try {
    const cacheKey = `news:${symbol}`;
    let articles = await coldLoad(cacheKey);
    const cacheAge = articles ? 0 : Infinity; // cold cache doesn't track age, always refresh
    if (!articles || cacheAge > 15 * 60 * 1000) {
      const json = await invoke("get_news", { symbol, limit: 20 });
      articles = JSON.parse(json);
      if (articles && articles.length > 0) coldSave(cacheKey, articles);
    }
    if (articles && articles.length > 0) {
      for (const article of articles.slice(0, 15)) {
        const item = document.createElement("div");
        item.className = "news-item";

        const date = document.createElement("div");
        date.className = "news-date";
        const ts = article.created_at || article.updated_at || "";
        date.textContent = ts.substring(0, 16).replace("T", " ");

        const headline = document.createElement("div");
        headline.className = "news-headline";
        headline.textContent = article.headline || article.title || "";

        const source = document.createElement("div");
        source.className = "news-source";
        source.textContent = article.source || "";

        item.appendChild(date);
        item.appendChild(headline);
        item.appendChild(source);

        if (article.url) {
          item.addEventListener("click", () => {
            openArticleInline(article.url, article.headline || article.title || "Article");
          });
        }

        panel.appendChild(item);
      }
      log(`${articles.length} news articles loaded for ${symbol}`, "ok");
    }
  } catch (e) {
    log(`News failed for ${symbol}: ${e}`, "warn");
  }
}

async function openArticleInline(url, title) {
  // Open article in a floating window (Godel Terminal style)
  const win = createWindow({
    title: title.substring(0, 60),
    type: "article",
    width: 550,
    height: 500,
  });
  win.setContent("Loading...");

  // Check cold cache first
  const cacheKey = `article:${url}`;
  let html = await coldLoad(cacheKey);

  if (!html) {
    try {
      html = await invoke("fetch_article", { url });
      if (html) coldSave(cacheKey, html);
    } catch (e) {
      win.setContent(`Failed to load: ${e}`);
      return;
    }
  }

  // Extract readable content (XSS-safe via textContent)
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, "text/html");
  doc.querySelectorAll("script, style, nav, header, footer, iframe, .ad, .ads, .sidebar").forEach(el => el.remove());

  const main = doc.querySelector("article, main, .article-body, .post-content, .entry-content, .story-body");
  const source = main || doc.body;
  const paragraphs = source ? source.querySelectorAll("p, h1, h2, h3, h4, li") : doc.querySelectorAll("p");

  win.contentElement.textContent = ""; // Clear loading text
  let found = 0;
  for (const p of paragraphs) {
    if (p.textContent.trim().length > 15) {
      const el = document.createElement("p");
      el.textContent = p.textContent;
      win.appendElement(el);
      found++;
    }
  }

  if (found === 0) {
    win.setContent("Could not extract article content. Source may require JavaScript or authentication.");
  }

  log(`Article opened: ${title.substring(0, 50)}`, "ok");
}

function setupNewsPanel() {
  const panel = document.getElementById("news-panel");
  const header = document.getElementById("news-header");

  header.addEventListener("click", () => {
    panel.classList.toggle("collapsed");
    header.textContent = panel.classList.contains("collapsed") ? "News & Events ▶" : "News & Events ▼";
  });

}

function setupIndicatorPanel() {
  const panel = document.getElementById("indicator-panel");
  const header = document.getElementById("indicator-header");

  // Start collapsed by default
  panel.classList.add("collapsed");
  header.textContent = "Indicators ▶";

  header.addEventListener("click", () => {
    panel.classList.toggle("collapsed");
    header.textContent = panel.classList.contains("collapsed") ? "Indicators ▶" : "Indicators ▼";
  });

  // Re-apply indicators when checkboxes change
  document.querySelectorAll("#indicator-list input[type=checkbox]").forEach(cb => {
    cb.addEventListener("change", () => {
      const data = candleSeries.data();
      if (data && data.length > 0) applyIndicators(data);
    });
  });
}

function setupLogPanel() {
  const panel = document.getElementById("log-panel");
  const toggle = document.getElementById("btn-log-toggle");
  const clear = document.getElementById("btn-log-clear");

  // Start collapsed
  panel.classList.add("collapsed");

  toggle.addEventListener("click", () => {
    panel.classList.toggle("collapsed");
    toggle.textContent = panel.classList.contains("collapsed") ? "▼" : "▲";
  });

  document.getElementById("log-header").addEventListener("click", (e) => {
    if (e.target === toggle || e.target === clear) return;
    toggle.click();
  });

  clear.addEventListener("click", (e) => {
    e.stopPropagation();
    document.getElementById("log-content").textContent = "";
  });

  log("TyphooN Terminal initialized", "info");
}

// ── Pane Resizers ───────────────────────────────────────────

function setupPaneResizers() {
  const resizers = document.querySelectorAll(".pane-resizer");

  for (const resizer of resizers) {
    const aboveId = resizer.dataset.above;
    const belowId = resizer.dataset.below;
    const aboveEl = document.getElementById(aboveId);
    const belowEl = document.getElementById(belowId);

    let startY = 0;
    let startAboveH = 0;
    let startBelowH = 0;

    const onMouseDown = (e) => {
      e.preventDefault();
      startY = e.clientY;
      startAboveH = aboveEl.getBoundingClientRect().height;
      startBelowH = belowEl.getBoundingClientRect().height;
      resizer.classList.add("active");
      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
    };

    const onMouseMove = (e) => {
      const dy = e.clientY - startY;
      const newAbove = Math.max(60, startAboveH + dy);
      const newBelow = Math.max(40, startBelowH - dy);

      // For the main chart (flex:1), set flex to none and use explicit height
      if (aboveId === "chart-container") {
        aboveEl.style.flex = "none";
        aboveEl.style.height = newAbove + "px";
      } else {
        aboveEl.style.height = newAbove + "px";
      }
      belowEl.style.height = newBelow + "px";

      // Trigger chart resizes
      chart.resize(document.getElementById("chart-container").clientWidth, document.getElementById("chart-container").clientHeight);
      fisherChart.resize(document.getElementById("fisher-pane").clientWidth, document.getElementById("fisher-pane").clientHeight);
      volumeChart.resize(document.getElementById("volume-pane").clientWidth, document.getElementById("volume-pane").clientHeight);
    };

    const onMouseUp = () => {
      resizer.classList.remove("active");
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };

    resizer.addEventListener("mousedown", onMouseDown);
  }
}

// ── Init ────────────────────────────────────────────────────

// ── Session State Persistence ────────────────────────────────

const SESSION_KEY = "typhoon_session";

function saveSession() {
  try {
    // Save current tab state first
    if (activeTabId !== null) {
      const cur = tabs.find(t => t.id === activeTabId);
      if (cur) {
        cur.symbol = currentSymbol;
        cur.timeframe = currentTimeframe;
        cur.barCount = document.getElementById("bar-count").value;
        cur.lastPrice = lastPrice;
      }
    }

    // Gather indicator checkbox states
    const indicators = {};
    document.querySelectorAll("#indicator-list input[type=checkbox]").forEach(cb => {
      const key = `${cb.dataset.ind}_${cb.dataset.period || ""}`;
      indicators[key] = cb.checked;
    });

    // Pane heights
    const fisherH = document.getElementById("fisher-pane")?.offsetHeight || 120;
    const volumeH = document.getElementById("volume-pane")?.offsetHeight || 100;

    const session = {
      tabs: tabs.map(t => ({ symbol: t.symbol, timeframe: t.timeframe, barCount: t.barCount })),
      activeTabIndex: tabs.findIndex(t => t.id === activeTabId),
      indicators,
      orderMode: document.getElementById("order-mode")?.value || "VaR",
      fisherPaneHeight: fisherH,
      volumePaneHeight: volumeH,
      timestamp: Date.now(),
    };

    localStorage.setItem(SESSION_KEY, JSON.stringify(session));
  } catch (_) {}
}

function restoreSession() {
  try {
    const json = localStorage.getItem(SESSION_KEY);
    if (!json) return false;
    const session = JSON.parse(json);
    if (!session || !session.tabs || session.tabs.length === 0) return false;

    // Restore indicator checkboxes
    if (session.indicators) {
      document.querySelectorAll("#indicator-list input[type=checkbox]").forEach(cb => {
        const key = `${cb.dataset.ind}_${cb.dataset.period || ""}`;
        if (key in session.indicators) cb.checked = session.indicators[key];
      });
    }

    // Restore order mode
    if (session.orderMode) {
      const modeEl = document.getElementById("order-mode");
      if (modeEl) modeEl.value = session.orderMode;
    }

    // Restore pane heights
    if (session.fisherPaneHeight) {
      const fp = document.getElementById("fisher-pane");
      if (fp) fp.style.height = session.fisherPaneHeight + "px";
    }
    if (session.volumePaneHeight) {
      const vp = document.getElementById("volume-pane");
      if (vp) vp.style.height = session.volumePaneHeight + "px";
    }

    // Restore tabs
    for (const t of session.tabs) {
      createTab(t.symbol, t.timeframe);
      const tab = tabs[tabs.length - 1];
      tab.barCount = t.barCount || "1000";
    }

    // Switch to previously active tab
    const idx = session.activeTabIndex >= 0 ? session.activeTabIndex : 0;
    if (tabs[idx]) switchTab(tabs[idx].id);

    log(`Session restored: ${session.tabs.length} tabs`, "ok");
    return true;
  } catch (e) {
    log(`Session restore failed: ${e}`, "warn");
    return false;
  }
}

// ── Positions Panel ──────────────────────────────────────────

function setupPositionsPanel() {
  const panel = document.getElementById("positions-panel");
  const header = document.getElementById("positions-header");
  const content = document.getElementById("positions-content");

  header.addEventListener("click", () => {
    panel.classList.toggle("collapsed");
    header.textContent = panel.classList.contains("collapsed") ? "Positions ▶" : "Positions ▼";
  });
}

async function updatePositionsPanel() {
  const content = document.getElementById("positions-content");
  if (!content) return;
  try {
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);
    content.textContent = "";
    if (positions.length === 0) {
      content.textContent = "No positions";
      return;
    }
    for (const p of positions) {
      const row = document.createElement("div");
      row.className = "pos-row";
      row.style.cssText = "display:flex;justify-content:space-between;align-items:center;padding:3px 0;border-bottom:1px solid #1a1a2e;font-size:11px;";

      const info = document.createElement("span");
      const plColor = p.unrealized_pl >= 0 ? "#4caf50" : "#f44336";
      info.style.color = "#ccc";
      info.textContent = `${p.symbol} ${p.side === "long" ? "L" : "S"} ${Math.abs(p.qty)}`;

      const pl = document.createElement("span");
      pl.style.cssText = `color:${plColor};font-family:Consolas,monospace;`;
      pl.textContent = `$${p.unrealized_pl.toFixed(2)}`;

      const closeBtn = document.createElement("button");
      closeBtn.textContent = "×";
      closeBtn.title = "Close position";
      closeBtn.style.cssText = "background:none;border:1px solid #f44;color:#f44;cursor:pointer;font-size:10px;padding:1px 5px;border-radius:2px;";
      closeBtn.addEventListener("click", async (e) => {
        e.stopPropagation();
        if (!confirm(`Close ${p.symbol} (${Math.abs(p.qty)} ${p.side})?`)) return;
        try {
          await invoke("close_position", { symbol: p.symbol, qty: null });
          updateDashboard();
        } catch (err) { alert(`Close failed: ${err}`); }
      });

      row.appendChild(info);
      row.appendChild(pl);
      row.appendChild(closeBtn);
      content.appendChild(row);

      // Click row to switch chart to that symbol
      row.style.cursor = "pointer";
      row.addEventListener("click", () => {
        document.getElementById("symbol-input").value = p.symbol;
        triggerLoad();
      });
    }
  } catch (_) {}
}

// ── Orders Panel (Trade History) ─────────────────────────────

function setupOrdersPanel() {
  const panel = document.getElementById("orders-panel");
  const header = document.getElementById("orders-header");

  panel.classList.add("collapsed");
  header.textContent = "Orders ▶";

  header.addEventListener("click", () => {
    panel.classList.toggle("collapsed");
    header.textContent = panel.classList.contains("collapsed") ? "Orders ▶" : "Orders ▼";
    if (!panel.classList.contains("collapsed")) updateOrdersPanel();
  });
}

async function updateOrdersPanel() {
  const content = document.getElementById("orders-content");
  if (!content) return;
  content.textContent = "";
  try {
    // Open orders first
    const openJson = await invoke("get_open_orders");
    const openOrders = JSON.parse(openJson);
    if (openOrders.length > 0) {
      const hdr = document.createElement("div");
      hdr.textContent = "Open Orders";
      hdr.style.cssText = "color:#ff8;font-size:10px;font-weight:bold;padding:4px 0 2px;";
      content.appendChild(hdr);
      for (const o of openOrders) {
        content.appendChild(renderOrderRow(o, true));
      }
    }

    // Recent closed orders
    const histJson = await invoke("get_order_history", { limit: 20 });
    const history = JSON.parse(histJson);
    if (history.length > 0) {
      const hdr = document.createElement("div");
      hdr.textContent = "Recent Fills";
      hdr.style.cssText = "color:#888;font-size:10px;font-weight:bold;padding:4px 0 2px;";
      content.appendChild(hdr);
      for (const o of history.slice(0, 15)) {
        content.appendChild(renderOrderRow(o, false));
      }
    }

    if (openOrders.length === 0 && history.length === 0) {
      content.textContent = "No orders";
    }
  } catch (_) {}
}

function renderOrderRow(o, canCancel) {
  const row = document.createElement("div");
  row.style.cssText = "display:flex;justify-content:space-between;align-items:center;padding:2px 0;border-bottom:1px solid #111;font-size:10px;color:#aaa;";

  const left = document.createElement("span");
  const typeLabel = o.order_type === "market" ? "" : ` ${o.order_type}`;
  left.textContent = `${o.symbol} ${o.side}${typeLabel} ${o.qty}`;

  const mid = document.createElement("span");
  mid.style.color = "#666";
  const price = o.filled_avg_price || o.limit_price || o.stop_price || "";
  mid.textContent = price ? `@${price}` : o.status;

  const right = document.createElement("span");
  right.style.color = "#555";
  right.textContent = o.created_at.substring(0, 16).replace("T", " ");

  row.appendChild(left);
  row.appendChild(mid);
  row.appendChild(right);

  if (canCancel) {
    const cancelBtn = document.createElement("button");
    cancelBtn.textContent = "×";
    cancelBtn.style.cssText = "background:none;border:1px solid #f44;color:#f44;cursor:pointer;font-size:9px;padding:0 4px;margin-left:4px;border-radius:2px;";
    cancelBtn.addEventListener("click", async (e) => {
      e.stopPropagation();
      try {
        await invoke("cancel_order", { orderId: o.id });
        updateOrdersPanel();
      } catch (err) { alert(`Cancel failed: ${err}`); }
    });
    row.appendChild(cancelBtn);
  }

  return row;
}

// ── Price Alerts ─────────────────────────────────────────────

let priceAlerts = []; // [{ symbol, price, direction: "above"|"below", triggered }]
const ALERTS_KEY = "typhoon_alerts";

function loadAlerts() {
  try { priceAlerts = JSON.parse(localStorage.getItem(ALERTS_KEY) || "[]"); } catch { priceAlerts = []; }
}

function saveAlerts() {
  localStorage.setItem(ALERTS_KEY, JSON.stringify(priceAlerts));
}

function addPriceAlert(symbol, price, direction) {
  priceAlerts.push({ symbol, price, direction, triggered: false });
  saveAlerts();
  log(`Alert set: ${symbol} ${direction} $${price.toFixed(4)}`, "ok");
}

function checkAlerts() {
  if (priceAlerts.length === 0 || !currentSymbol || lastPrice <= 0) return;
  for (const alert of priceAlerts) {
    if (alert.triggered || alert.symbol !== currentSymbol) continue;
    if (alert.direction === "above" && lastPrice >= alert.price) {
      alert.triggered = true;
      fireAlert(alert);
    } else if (alert.direction === "below" && lastPrice <= alert.price) {
      alert.triggered = true;
      fireAlert(alert);
    }
  }
  saveAlerts();
}

function fireAlert(alert) {
  log(`ALERT: ${alert.symbol} ${alert.direction} $${alert.price.toFixed(4)} — price: $${lastPrice.toFixed(4)}`, "warn");
  try { new Notification(`${alert.symbol} Alert`, { body: `Price ${alert.direction} $${alert.price.toFixed(4)}` }); } catch (_) {}
}

// ══════════════════════════════════════════════════════════════
// FEATURE 1: Chart Templates — save/load indicator + order mode
// ══════════════════════════════════════════════════════════════

const TEMPLATES_KEY = "typhoon_templates";

function saveChartTemplate(name) {
  const templates = listChartTemplatesRaw();
  const indicators = {};
  document.querySelectorAll("#indicator-list input[type=checkbox]").forEach(cb => {
    const key = `${cb.dataset.ind}_${cb.dataset.period || ""}`;
    indicators[key] = cb.checked;
  });
  templates[name] = {
    indicators,
    orderMode: document.getElementById("order-mode")?.value || "VaR",
    timestamp: Date.now(),
  };
  localStorage.setItem(TEMPLATES_KEY, JSON.stringify(templates));
  log(`Template "${name}" saved`, "ok");
}

function loadChartTemplate(name) {
  const templates = listChartTemplatesRaw();
  const tpl = templates[name];
  if (!tpl) { log(`Template "${name}" not found`, "warn"); return; }
  if (tpl.indicators) {
    document.querySelectorAll("#indicator-list input[type=checkbox]").forEach(cb => {
      const key = `${cb.dataset.ind}_${cb.dataset.period || ""}`;
      if (key in tpl.indicators) cb.checked = tpl.indicators[key];
    });
  }
  if (tpl.orderMode) {
    const modeEl = document.getElementById("order-mode");
    if (modeEl) modeEl.value = tpl.orderMode;
  }
  // Re-apply indicators
  const data = candleSeries.data();
  if (data && data.length > 0) applyIndicators(data);
  log(`Template "${name}" loaded`, "ok");
}

function listChartTemplates() {
  return Object.keys(listChartTemplatesRaw());
}

function listChartTemplatesRaw() {
  try { return JSON.parse(localStorage.getItem(TEMPLATES_KEY) || "{}"); }
  catch { return {}; }
}

function deleteChartTemplate(name) {
  const templates = listChartTemplatesRaw();
  delete templates[name];
  localStorage.setItem(TEMPLATES_KEY, JSON.stringify(templates));
}

function populateTemplateDropdown() {
  const sel = document.getElementById("template-select");
  if (!sel) return;
  while (sel.options.length > 1) sel.remove(1);
  for (const name of listChartTemplates()) {
    const opt = document.createElement("option");
    opt.value = name;
    opt.textContent = name;
    sel.appendChild(opt);
  }
}

function setupTemplates() {
  populateTemplateDropdown();

  document.getElementById("btn-save-template")?.addEventListener("click", () => {
    const name = prompt("Template name:");
    if (!name) return;
    saveChartTemplate(name);
    populateTemplateDropdown();
    document.getElementById("template-select").value = name;
  });

  document.getElementById("template-select")?.addEventListener("change", (e) => {
    if (e.target.value) loadChartTemplate(e.target.value);
  });

  document.getElementById("btn-delete-template")?.addEventListener("click", () => {
    const sel = document.getElementById("template-select");
    const name = sel?.value;
    if (!name) return;
    deleteChartTemplate(name);
    populateTemplateDropdown();
    log(`Template "${name}" deleted`, "info");
  });
}

// ══════════════════════════════════════════════════════════════
// FEATURE 2: Workspace Profiles — save/load full layout
// ══════════════════════════════════════════════════════════════

const PROFILES_KEY = "typhoon_profiles";

function listProfilesRaw() {
  try { return JSON.parse(localStorage.getItem(PROFILES_KEY) || "{}"); }
  catch { return {}; }
}

function saveWorkspaceProfile(name) {
  const profiles = listProfilesRaw();
  // Capture current tab state
  if (activeTabId !== null) {
    const cur = tabs.find(t => t.id === activeTabId);
    if (cur) {
      cur.symbol = currentSymbol;
      cur.timeframe = currentTimeframe;
      cur.barCount = document.getElementById("bar-count").value;
      cur.lastPrice = lastPrice;
    }
  }
  const indicators = {};
  document.querySelectorAll("#indicator-list input[type=checkbox]").forEach(cb => {
    const key = `${cb.dataset.ind}_${cb.dataset.period || ""}`;
    indicators[key] = cb.checked;
  });

  profiles[name] = {
    tabs: tabs.map(t => ({ symbol: t.symbol, timeframe: t.timeframe, barCount: t.barCount })),
    activeTabIndex: tabs.findIndex(t => t.id === activeTabId),
    indicators,
    orderMode: document.getElementById("order-mode")?.value || "VaR",
    fisherPaneHeight: document.getElementById("fisher-pane")?.offsetHeight || 120,
    volumePaneHeight: document.getElementById("volume-pane")?.offsetHeight || 100,
    splitActive: splitActive,
    splitSymbol: splitSymbol,
    timestamp: Date.now(),
  };
  localStorage.setItem(PROFILES_KEY, JSON.stringify(profiles));
  log(`Profile "${name}" saved`, "ok");
}

function loadWorkspaceProfile(name) {
  const profiles = listProfilesRaw();
  const profile = profiles[name];
  if (!profile) { log(`Profile "${name}" not found`, "warn"); return; }

  // Restore indicator checkboxes
  if (profile.indicators) {
    document.querySelectorAll("#indicator-list input[type=checkbox]").forEach(cb => {
      const key = `${cb.dataset.ind}_${cb.dataset.period || ""}`;
      if (key in profile.indicators) cb.checked = profile.indicators[key];
    });
  }
  if (profile.orderMode) {
    const modeEl = document.getElementById("order-mode");
    if (modeEl) modeEl.value = profile.orderMode;
  }
  if (profile.fisherPaneHeight) {
    const fp = document.getElementById("fisher-pane");
    if (fp) fp.style.height = profile.fisherPaneHeight + "px";
  }
  if (profile.volumePaneHeight) {
    const vp = document.getElementById("volume-pane");
    if (vp) vp.style.height = profile.volumePaneHeight + "px";
  }

  // Close existing tabs and recreate
  tabs.length = 0;
  activeTabId = null;
  nextTabId = 1;
  if (profile.tabs && profile.tabs.length > 0) {
    for (const t of profile.tabs) {
      createTab(t.symbol, t.timeframe);
      const tab = tabs[tabs.length - 1];
      tab.barCount = t.barCount || "50000";
    }
    const idx = profile.activeTabIndex >= 0 ? profile.activeTabIndex : 0;
    if (tabs[idx]) switchTab(tabs[idx].id);
  } else {
    createTab();
  }

  // Restore split if applicable
  if (profile.splitActive && profile.splitSymbol) {
    activateSplit(profile.splitSymbol);
  } else {
    deactivateSplit();
  }

  log(`Profile "${name}" loaded`, "ok");
}

function populateProfileDropdown() {
  const sel = document.getElementById("profile-select");
  if (!sel) return;
  while (sel.options.length > 1) sel.remove(1);
  for (const name of Object.keys(listProfilesRaw())) {
    const opt = document.createElement("option");
    opt.value = name;
    opt.textContent = name;
    sel.appendChild(opt);
  }
}

function setupProfiles() {
  populateProfileDropdown();

  document.getElementById("btn-save-profile")?.addEventListener("click", () => {
    const name = prompt("Profile name:");
    if (!name) return;
    saveWorkspaceProfile(name);
    populateProfileDropdown();
    document.getElementById("profile-select").value = name;
  });

  document.getElementById("btn-load-profile")?.addEventListener("click", () => {
    const sel = document.getElementById("profile-select");
    const name = sel?.value;
    if (!name) return;
    loadWorkspaceProfile(name);
  });
}

// ══════════════════════════════════════════════════════════════
// FEATURE 3: Command Palette (Bloomberg/Godel-style)
// ══════════════════════════════════════════════════════════════

const CMD_PALETTE_COMMANDS = [
  { name: "DES", desc: "Description / Fundamentals", action: cmdDescription },
  { name: "NEWS", desc: "News headlines", action: cmdNews },
  { name: "FA", desc: "Financial Analysis (income, balance, cash flow)", action: cmdFinancialAnalysis },
  { name: "OPT", desc: "Options chain (coming soon)", action: cmdOptions },
  { name: "SCAN", desc: "Screener / Scanner", action: cmdScreener },
  { name: "HDS", desc: "Institutional Holders", action: cmdInstitutionalHolders },
  { name: "MOST", desc: "Most Active stocks", action: cmdMostActive },
  { name: "DOM", desc: "DOM / Level 2 Order Book", action: cmdOrderBook },
  { name: "BACKTEST", desc: "Visual Backtester", action: openVisualBacktester },
  { name: "OPTIMIZE", desc: "Genetic Optimizer", action: openOptimizer },
  { name: "HIST", desc: "Trade History / Orders", action: cmdHistory },
  { name: "QM", desc: "Quote Monitor / Watchlist", action: cmdWatchlist },
  { name: "CAL", desc: "Economic Calendar", action: cmdCalendar },
  { name: "TILE", desc: "Tile all floating windows", action: () => tileWindows() },
  { name: "CLOSE", desc: "Close all floating windows", action: () => closeAllWindows() },
];

function fuzzyMatch(query, target) {
  query = query.toLowerCase();
  target = target.toLowerCase();
  if (target.startsWith(query)) return 100;
  if (target.includes(query)) return 50;
  let qi = 0;
  for (let ti = 0; ti < target.length && qi < query.length; ti++) {
    if (target[ti] === query[qi]) qi++;
  }
  return qi === query.length ? 30 : 0;
}

let cmdPaletteIndex = -1;

function setupCommandPalette() {
  const overlay = document.getElementById("command-palette");
  const input = document.getElementById("cmd-palette-input");
  const results = document.getElementById("cmd-palette-results");
  if (!overlay || !input || !results) return;

  function openPalette() {
    overlay.classList.remove("hidden");
    input.value = "";
    input.focus();
    cmdPaletteIndex = -1;
    renderCmdResults("");
  }

  function closePalette() {
    overlay.classList.add("hidden");
    input.value = "";
  }

  function renderCmdResults(query) {
    results.textContent = "";
    let items;
    if (!query) {
      items = CMD_PALETTE_COMMANDS.map(c => ({ ...c, score: 100 }));
    } else {
      items = CMD_PALETTE_COMMANDS
        .map(c => ({ ...c, score: Math.max(fuzzyMatch(query, c.name), fuzzyMatch(query, c.desc)) }))
        .filter(c => c.score > 0)
        .sort((a, b) => b.score - a.score);
    }

    for (let i = 0; i < items.length; i++) {
      const c = items[i];
      const div = document.createElement("div");
      div.className = `cmd-result-item${i === cmdPaletteIndex ? " selected" : ""}`;
      const nameSpan = document.createElement("span");
      nameSpan.className = "cmd-name";
      nameSpan.textContent = c.name;
      const descSpan = document.createElement("span");
      descSpan.className = "cmd-desc";
      descSpan.textContent = c.desc;
      div.appendChild(nameSpan);
      div.appendChild(descSpan);
      div.addEventListener("click", () => {
        closePalette();
        c.action();
      });
      results.appendChild(div);
    }

    // If no command matches, treat query as symbol
    if (items.length === 0 && query.length >= 1) {
      const div = document.createElement("div");
      div.className = "cmd-result-item";
      const nameSpan = document.createElement("span");
      nameSpan.className = "cmd-name";
      nameSpan.textContent = query.toUpperCase();
      const descSpan = document.createElement("span");
      descSpan.className = "cmd-desc";
      descSpan.textContent = "Switch chart to this symbol";
      div.appendChild(nameSpan);
      div.appendChild(descSpan);
      div.addEventListener("click", () => {
        closePalette();
        document.getElementById("symbol-input").value = query.toUpperCase();
        triggerLoad();
      });
      results.appendChild(div);
    }
  }

  input.addEventListener("input", () => {
    cmdPaletteIndex = -1;
    renderCmdResults(input.value.trim());
  });

  input.addEventListener("keydown", (e) => {
    const items = results.querySelectorAll(".cmd-result-item");
    if (e.key === "ArrowDown") {
      e.preventDefault();
      cmdPaletteIndex = Math.min(cmdPaletteIndex + 1, items.length - 1);
      items.forEach((el, i) => el.classList.toggle("selected", i === cmdPaletteIndex));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      cmdPaletteIndex = Math.max(cmdPaletteIndex - 1, 0);
      items.forEach((el, i) => el.classList.toggle("selected", i === cmdPaletteIndex));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (cmdPaletteIndex >= 0 && items[cmdPaletteIndex]) {
        items[cmdPaletteIndex].click();
      } else if (items.length > 0) {
        items[0].click();
      } else {
        // Treat as symbol
        const sym = input.value.trim().toUpperCase();
        if (sym) {
          closePalette();
          document.getElementById("symbol-input").value = sym;
          triggerLoad();
        }
      }
    } else if (e.key === "Escape") {
      closePalette();
    }
  });

  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) closePalette();
  });

  // Global shortcut: Ctrl+K or / to open
  document.addEventListener("keydown", (e) => {
    if ((e.ctrlKey && e.key === "k") || (e.key === "/" && e.target.tagName !== "INPUT" && e.target.tagName !== "SELECT")) {
      e.preventDefault();
      if (overlay.classList.contains("hidden")) openPalette();
      else closePalette();
    }
  });
}

// Command palette actions
async function cmdDescription() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  try {
    const json = await invoke("get_company_fundamentals", { symbol: currentSymbol });
    const data = JSON.parse(json);
    if (data && data.entity) openFundamentalsWindow(currentSymbol, data);
    else {
      const win = createWindow({ title: `${currentSymbol} — Description`, width: 400, height: 300 });
      win.setContent("No fundamental data available for this symbol.");
    }
  } catch (e) {
    log(`DES command failed: ${e}`, "error");
  }
}

async function cmdNews() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  try {
    const json = await invoke("get_news", { symbol: currentSymbol, limit: 30 });
    const articles = JSON.parse(json);
    const win = createWindow({ title: `${currentSymbol} — News`, width: 550, height: 500 });
    if (!articles || articles.length === 0) {
      win.setContent("No news available.");
      return;
    }
    win.contentElement.textContent = "";
    for (const article of articles) {
      const item = document.createElement("div");
      item.style.cssText = "padding:6px 0;border-bottom:1px solid #1a1a2e;cursor:pointer;";
      const ts = (article.created_at || "").substring(0, 16).replace("T", " ");
      const dateEl = document.createElement("div");
      dateEl.style.cssText = "color:#666;font-size:10px;";
      dateEl.textContent = `${ts} | ${article.source || ""}`;
      const headEl = document.createElement("div");
      headEl.style.cssText = "color:#ccc;font-size:11px;margin-top:2px;";
      headEl.textContent = article.headline || article.title || "";
      item.appendChild(dateEl);
      item.appendChild(headEl);
      if (article.url) {
        item.addEventListener("click", () => openArticleInline(article.url, article.headline || "Article"));
      }
      win.appendElement(item);
    }
  } catch (e) { log(`NEWS command failed: ${e}`, "error"); }
}

async function cmdFinancialAnalysis() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Financial Analysis`, width: 700, height: 550 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Loading financial data...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);
  try {
    const json = await invoke("get_financial_analysis", { symbol: currentSymbol });
    const data = typeof json === "string" ? JSON.parse(json) : json;
    win.contentElement.textContent = "";

    const sections = [
      { title: "Income Statement", key: "income_statement" },
      { title: "Balance Sheet", key: "balance_sheet" },
      { title: "Cash Flow", key: "cash_flow" },
    ];

    for (const sec of sections) {
      const rows = data[sec.key];
      if (!rows || (Array.isArray(rows) && rows.length === 0)) continue;

      const heading = document.createElement("h3");
      heading.textContent = sec.title;
      heading.style.cssText = "color:#8cf;margin:12px 0 6px;font-size:13px;border-bottom:1px solid #333;padding-bottom:4px;";
      win.appendElement(heading);

      const table = document.createElement("table");
      table.className = "fw-table";

      if (Array.isArray(rows)) {
        for (const row of rows) {
          const tr = document.createElement("tr");
          for (const [k, v] of Object.entries(row)) {
            const td = document.createElement("td");
            td.className = "fw-value";
            td.style.textAlign = "left";
            td.textContent = typeof v === "number" ? v.toLocaleString() : String(v ?? "—");
            tr.appendChild(td);
          }
          table.appendChild(tr);
        }
      } else if (typeof rows === "object") {
        for (const [label, value] of Object.entries(rows)) {
          const tr = document.createElement("tr");
          const td1 = document.createElement("td");
          td1.className = "fw-label";
          td1.textContent = label;
          const td2 = document.createElement("td");
          td2.className = "fw-value";
          td2.textContent = typeof value === "number" ? value.toLocaleString() : String(value ?? "—");
          tr.appendChild(td1);
          tr.appendChild(td2);
          table.appendChild(tr);
        }
      }
      win.appendElement(table);
    }

    if (!data.income_statement && !data.balance_sheet && !data.cash_flow) {
      win.setContent("No financial analysis data available for this symbol.");
    }
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Could not load financial analysis: ${e}`);
  }
}

function cmdOptions() {
  const win = createWindow({ title: `${currentSymbol || ""} — Options`, width: 400, height: 200 });
  win.setContent("Options chain viewer is not yet implemented. Coming in a future release.");
}

function cmdScreener() {
  const win = createWindow({ title: "Screener", width: 500, height: 400 });
  win.setContent("Stock screener is not yet implemented. Coming in a future release.");
}

async function cmdInstitutionalHolders() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Institutional Holders`, width: 600, height: 450 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Loading holder data...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);
  try {
    const json = await invoke("get_institutional_holders", { symbol: currentSymbol });
    const holders = typeof json === "string" ? JSON.parse(json) : json;
    win.contentElement.textContent = "";
    if (!holders || (Array.isArray(holders) && holders.length === 0)) {
      win.setContent("No institutional holder data available.");
      return;
    }
    const list = Array.isArray(holders) ? holders : (holders.holders || []);
    const table = document.createElement("table");
    table.className = "fw-table";
    const thead = document.createElement("tr");
    for (const h of ["Holder", "Shares", "Value", "% Out", "Change"]) {
      const th = document.createElement("td");
      th.style.cssText = "color:#666;font-weight:bold;font-size:10px;text-transform:uppercase;";
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);
    for (const h of list) {
      const tr = document.createElement("tr");
      const vals = [
        h.holder || h.name || "—",
        h.shares ? Number(h.shares).toLocaleString() : "—",
        h.value ? `$${Number(h.value).toLocaleString()}` : "—",
        h.percent_out ? `${Number(h.percent_out).toFixed(2)}%` : (h.pct ? `${Number(h.pct).toFixed(2)}%` : "—"),
        h.change ? Number(h.change).toLocaleString() : "—",
      ];
      for (const v of vals) {
        const td = document.createElement("td");
        td.className = "fw-value";
        td.style.textAlign = "left";
        td.textContent = v;
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }
    win.appendElement(table);
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Could not load holders: ${e}`);
  }
}

async function cmdMostActive() {
  const win = createWindow({ title: "Most Active Stocks", width: 600, height: 500 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Loading most active...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);
  try {
    const json = await invoke("get_most_active");
    const stocks = typeof json === "string" ? JSON.parse(json) : json;
    win.contentElement.textContent = "";
    const list = Array.isArray(stocks) ? stocks : (stocks.most_active || stocks.stocks || []);
    if (list.length === 0) {
      win.setContent("No most active data available.");
      return;
    }
    const table = document.createElement("table");
    table.className = "fw-table most-active-table";
    const thead = document.createElement("tr");
    for (const h of ["Symbol", "Last", "Change", "% Chg", "Volume"]) {
      const th = document.createElement("td");
      th.style.cssText = "color:#666;font-weight:bold;font-size:10px;text-transform:uppercase;";
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);
    for (const s of list) {
      const tr = document.createElement("tr");
      tr.style.cursor = "pointer";
      const sym = s.symbol || s.ticker || "—";
      const chg = s.change ?? s.price_change ?? 0;
      const chgPct = s.change_percent ?? s.pct_change ?? 0;
      const vals = [
        sym,
        s.last ?? s.price ?? "—",
        typeof chg === "number" ? (chg >= 0 ? `+${chg.toFixed(2)}` : chg.toFixed(2)) : String(chg),
        typeof chgPct === "number" ? `${chgPct >= 0 ? "+" : ""}${chgPct.toFixed(2)}%` : String(chgPct),
        s.volume ? (s.volume >= 1e6 ? `${(s.volume / 1e6).toFixed(1)}M` : Number(s.volume).toLocaleString()) : "—",
      ];
      for (let i = 0; i < vals.length; i++) {
        const td = document.createElement("td");
        td.className = "fw-value";
        td.style.textAlign = "left";
        td.textContent = vals[i];
        if (i === 0) td.style.cssText = "color:#8ff;font-weight:bold;text-align:left;padding:6px 8px;";
        if (i === 2 || i === 3) {
          const n = parseFloat(vals[i]);
          if (!isNaN(n)) td.style.color = n >= 0 ? "#4caf50" : "#f44336";
        }
        tr.appendChild(td);
      }
      tr.addEventListener("click", () => {
        document.getElementById("symbol-input").value = sym;
        triggerLoad();
      });
      table.appendChild(tr);
    }
    win.appendElement(table);
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Could not load most active: ${e}`);
  }
}

async function cmdOrderBook() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — DOM / Level 2`, width: 400, height: 500 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Loading order book...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);
  try {
    const json = await invoke("get_orderbook", { symbol: currentSymbol });
    const book = typeof json === "string" ? JSON.parse(json) : json;
    win.contentElement.textContent = "";

    const bids = Array.isArray(book.bids) ? book.bids : [];
    const asks = Array.isArray(book.asks) ? book.asks : [];
    const maxSize = Math.max(
      ...bids.map(b => b.size || b.qty || 0),
      ...asks.map(a => a.size || a.qty || 0),
      1
    );

    const container = document.createElement("div");
    container.className = "dom-container";

    // Asks (reversed so highest ask on top)
    const asksReversed = [...asks].reverse();
    for (const a of asksReversed) {
      const price = a.price ?? 0;
      const size = a.size ?? a.qty ?? 0;
      const pct = (size / maxSize) * 100;
      const row = document.createElement("div");
      row.className = "dom-row dom-ask";
      row.innerHTML = `<div class="dom-bar dom-bar-ask" style="width:${pct}%"></div>`
        + `<span class="dom-size">${size.toLocaleString()}</span>`
        + `<span class="dom-price">${Number(price).toFixed(2)}</span>`;
      container.appendChild(row);
    }

    // Spread line
    if (asks.length > 0 && bids.length > 0) {
      const bestAsk = Math.min(...asks.map(a => a.price ?? Infinity));
      const bestBid = Math.max(...bids.map(b => b.price ?? 0));
      const spread = (bestAsk - bestBid).toFixed(2);
      const spreadRow = document.createElement("div");
      spreadRow.className = "dom-spread";
      spreadRow.textContent = `Spread: $${spread}`;
      container.appendChild(spreadRow);
    }

    // Bids
    for (const b of bids) {
      const price = b.price ?? 0;
      const size = b.size ?? b.qty ?? 0;
      const pct = (size / maxSize) * 100;
      const row = document.createElement("div");
      row.className = "dom-row dom-bid";
      row.innerHTML = `<div class="dom-bar dom-bar-bid" style="width:${pct}%"></div>`
        + `<span class="dom-size">${size.toLocaleString()}</span>`
        + `<span class="dom-price">${Number(price).toFixed(2)}</span>`;
      container.appendChild(row);
    }

    if (bids.length === 0 && asks.length === 0) {
      const msg = document.createElement("div");
      msg.style.cssText = "color:#888;padding:20px;text-align:center;";
      msg.textContent = "No order book data available.";
      container.appendChild(msg);
    }

    win.appendElement(container);
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Could not load order book: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// Visual Backtester
// ══════════════════════════════════════════════════════════════

function openVisualBacktester() {
  const sym = currentSymbol || "SPY";
  const tf = currentTimeframe || "1Day";

  const win = createWindow({ title: "Visual Backtester", width: 750, height: 600 });
  win.contentElement.textContent = "";

  // Controls row
  const controls = document.createElement("div");
  controls.className = "bt-controls";

  const makeInput = (label, id, value, type = "number") => {
    const wrap = document.createElement("label");
    wrap.className = "bt-field";
    wrap.textContent = label;
    const inp = document.createElement("input");
    inp.type = type;
    inp.id = id;
    inp.value = value;
    inp.className = "bt-input";
    wrap.appendChild(inp);
    return wrap;
  };

  const symInput = makeInput("Symbol:", "bt-symbol", sym, "text");
  const tfSelect = document.createElement("label");
  tfSelect.className = "bt-field";
  tfSelect.textContent = "TF:";
  const tfSel = document.createElement("select");
  tfSel.className = "bt-input";
  for (const [v, l] of [["1Min","1m"],["5Min","5m"],["15Min","15m"],["1Hour","1H"],["4Hour","4H"],["1Day","D1"],["1Week","W1"]]) {
    const opt = document.createElement("option");
    opt.value = v; opt.textContent = l;
    if (v === tf) opt.selected = true;
    tfSel.appendChild(opt);
  }
  tfSelect.appendChild(tfSel);

  const stratSelect = document.createElement("label");
  stratSelect.className = "bt-field";
  stratSelect.textContent = "Strategy:";
  const stratSel = document.createElement("select");
  stratSel.className = "bt-input";
  for (const s of ["SMA Cross"]) {
    const opt = document.createElement("option");
    opt.value = s; opt.textContent = s;
    stratSel.appendChild(opt);
  }
  stratSelect.appendChild(stratSel);

  const fastInput = makeInput("Fast:", "bt-fast", "10");
  const slowInput = makeInput("Slow:", "bt-slow", "50");

  const runBtn = document.createElement("button");
  runBtn.textContent = "Run Backtest";
  runBtn.className = "bt-run-btn";

  controls.appendChild(symInput);
  controls.appendChild(tfSelect);
  controls.appendChild(stratSelect);
  controls.appendChild(fastInput);
  controls.appendChild(slowInput);
  controls.appendChild(runBtn);
  win.appendElement(controls);

  // Results area
  const resultsArea = document.createElement("div");
  resultsArea.className = "bt-results";
  win.appendElement(resultsArea);

  // Equity chart container
  const eqContainer = document.createElement("div");
  eqContainer.className = "bt-equity-chart";
  eqContainer.style.cssText = "width:100%;height:200px;background:#000;border:1px solid #333;margin-bottom:8px;";
  resultsArea.appendChild(eqContainer);

  // Stats table container
  const statsDiv = document.createElement("div");
  statsDiv.className = "bt-stats";
  resultsArea.appendChild(statsDiv);

  // Trade list container
  const tradesDiv = document.createElement("div");
  tradesDiv.className = "bt-trades";
  tradesDiv.style.cssText = "max-height:150px;overflow-y:auto;";
  resultsArea.appendChild(tradesDiv);

  let eqChart = null;

  runBtn.addEventListener("click", async () => {
    const btSym = symInput.querySelector("input").value.trim().toUpperCase() || sym;
    const btTf = tfSel.value;
    const btStrat = stratSel.value;
    const btFast = parseInt(fastInput.querySelector("input").value) || 10;
    const btSlow = parseInt(slowInput.querySelector("input").value) || 50;

    runBtn.disabled = true;
    runBtn.textContent = "Running...";
    statsDiv.textContent = "";
    tradesDiv.textContent = "";

    try {
      const json = await invoke("run_backtest", {
        symbol: btSym,
        timeframe: btTf,
        strategy: btStrat,
        fast_period: btFast,
        slow_period: btSlow,
      });
      const result = typeof json === "string" ? JSON.parse(json) : json;

      // Equity curve
      const equityCurve = result.equity_curve || [];
      if (eqChart) { eqChart.remove(); eqChart = null; }
      eqChart = createChart(eqContainer, {
        width: eqContainer.clientWidth,
        height: 200,
        layout: { background: { color: "#000" }, textColor: "#888", fontFamily: "Consolas, monospace", attributionLogo: false },
        grid: { vertLines: { color: "#1a1a2e" }, horzLines: { color: "#1a1a2e" } },
        rightPriceScale: { borderColor: "#333" },
        timeScale: { borderColor: "#333", timeVisible: true },
      });
      const eqSeries = eqChart.addLineSeries({ color: "#4caf50", lineWidth: 2, title: "Equity" });
      const eqData = equityCurve.map(p => ({
        time: typeof p.time === "number" ? p.time : Math.floor(new Date(p.time).getTime() / 1000),
        value: p.value ?? p.equity ?? 0,
      }));
      if (eqData.length > 0) {
        eqSeries.setData(eqData);
        eqChart.timeScale().fitContent();
      }

      // Stats table
      const stats = result.stats || {};
      const statsTable = document.createElement("table");
      statsTable.className = "fw-table";
      const statRows = [
        ["Total P/L", stats.total_pnl != null ? `$${Number(stats.total_pnl).toFixed(2)}` : "—"],
        ["Sharpe Ratio", stats.sharpe != null ? Number(stats.sharpe).toFixed(3) : "—"],
        ["Win Rate", stats.win_rate != null ? `${(Number(stats.win_rate) * 100).toFixed(1)}%` : "—"],
        ["Max Drawdown", stats.max_drawdown != null ? `$${Number(stats.max_drawdown).toFixed(2)}` : "—"],
        ["Trades", stats.total_trades ?? stats.num_trades ?? "—"],
        ["Profit Factor", stats.profit_factor != null ? Number(stats.profit_factor).toFixed(2) : "—"],
      ];
      for (const [label, value] of statRows) {
        const tr = document.createElement("tr");
        const td1 = document.createElement("td");
        td1.className = "fw-label"; td1.textContent = label;
        const td2 = document.createElement("td");
        td2.className = "fw-value"; td2.textContent = value;
        tr.appendChild(td1); tr.appendChild(td2);
        statsTable.appendChild(tr);
      }
      statsDiv.appendChild(statsTable);

      // Trade list
      const trades = result.trades || [];
      if (trades.length > 0) {
        const heading = document.createElement("div");
        heading.style.cssText = "color:#666;font-size:10px;margin:8px 0 4px;text-transform:uppercase;";
        heading.textContent = `Trades (${trades.length})`;
        tradesDiv.appendChild(heading);
        const tbl = document.createElement("table");
        tbl.className = "fw-table";
        const th = document.createElement("tr");
        for (const h of ["Entry", "Exit", "Side", "P/L"]) {
          const td = document.createElement("td");
          td.style.cssText = "color:#666;font-weight:bold;font-size:10px;";
          td.textContent = h; th.appendChild(td);
        }
        tbl.appendChild(th);
        for (const t of trades) {
          const tr = document.createElement("tr");
          const entryTime = t.entry_time ? String(t.entry_time).substring(0, 16) : "—";
          const exitTime = t.exit_time ? String(t.exit_time).substring(0, 16) : "—";
          const pnl = t.pnl ?? t.profit ?? 0;
          for (const val of [entryTime, exitTime, t.side || "—", `$${Number(pnl).toFixed(2)}`]) {
            const td = document.createElement("td");
            td.className = "fw-value"; td.style.textAlign = "left";
            td.textContent = val;
            if (val.startsWith("$")) td.style.color = pnl >= 0 ? "#4caf50" : "#f44336";
            tr.appendChild(td);
          }
          tbl.appendChild(tr);
        }
        tradesDiv.appendChild(tbl);
      }
    } catch (e) {
      statsDiv.textContent = `Backtest failed: ${e}`;
      statsDiv.style.color = "#f44";
    }

    runBtn.disabled = false;
    runBtn.textContent = "Run Backtest";
  });
}

// ══════════════════════════════════════════════════════════════
// Genetic Optimizer
// ══════════════════════════════════════════════════════════════

function openOptimizer() {
  const sym = currentSymbol || "SPY";
  const tf = currentTimeframe || "1Day";

  const win = createWindow({ title: "Genetic Optimizer", width: 700, height: 550 });
  win.contentElement.textContent = "";

  // Controls
  const controls = document.createElement("div");
  controls.className = "bt-controls";

  const makeField = (label, id, value) => {
    const wrap = document.createElement("label");
    wrap.className = "bt-field";
    wrap.textContent = label;
    const inp = document.createElement("input");
    inp.type = "number"; inp.id = id; inp.value = value;
    inp.className = "bt-input";
    wrap.appendChild(inp);
    return { wrap, inp };
  };

  const symWrap = document.createElement("label");
  symWrap.className = "bt-field";
  symWrap.textContent = "Symbol:";
  const symInp = document.createElement("input");
  symInp.type = "text"; symInp.value = sym; symInp.className = "bt-input";
  symWrap.appendChild(symInp);
  controls.appendChild(symWrap);

  const fMin = makeField("Fast Min:", "opt-fmin", "5");
  const fMax = makeField("Fast Max:", "opt-fmax", "50");
  const sMin = makeField("Slow Min:", "opt-smin", "20");
  const sMax = makeField("Slow Max:", "opt-smax", "200");
  controls.appendChild(fMin.wrap);
  controls.appendChild(fMax.wrap);
  controls.appendChild(sMin.wrap);
  controls.appendChild(sMax.wrap);

  const runBtn = document.createElement("button");
  runBtn.textContent = "Optimize";
  runBtn.className = "bt-run-btn";
  controls.appendChild(runBtn);
  win.appendElement(controls);

  // Results
  const resultsDiv = document.createElement("div");
  resultsDiv.style.cssText = "max-height:400px;overflow-y:auto;";
  win.appendElement(resultsDiv);

  let sortCol = null;
  let sortAsc = true;
  let lastResults = [];

  function renderOptResults(results) {
    resultsDiv.textContent = "";
    if (!results || results.length === 0) {
      resultsDiv.textContent = "No results.";
      return;
    }
    const table = document.createElement("table");
    table.className = "fw-table opt-results-table";
    const thead = document.createElement("tr");
    const headers = ["fast", "slow", "pnl", "sharpe", "win_rate", "max_drawdown"];
    const labels = ["Fast", "Slow", "P/L", "Sharpe", "Win %", "Max DD"];
    for (let i = 0; i < headers.length; i++) {
      const th = document.createElement("td");
      th.style.cssText = "color:#8cf;font-weight:bold;font-size:10px;cursor:pointer;user-select:none;text-transform:uppercase;padding:6px 8px;";
      th.textContent = labels[i] + (sortCol === headers[i] ? (sortAsc ? " ▲" : " ▼") : "");
      const col = headers[i];
      th.addEventListener("click", () => {
        if (sortCol === col) { sortAsc = !sortAsc; }
        else { sortCol = col; sortAsc = false; }
        const sorted = [...lastResults].sort((a, b) => {
          const av = a[col] ?? 0;
          const bv = b[col] ?? 0;
          return sortAsc ? av - bv : bv - av;
        });
        renderOptResults(sorted);
      });
      thead.appendChild(th);
    }
    table.appendChild(thead);

    for (const r of results) {
      const tr = document.createElement("tr");
      const vals = [
        r.fast ?? "—",
        r.slow ?? "—",
        r.pnl != null ? `$${Number(r.pnl).toFixed(2)}` : "—",
        r.sharpe != null ? Number(r.sharpe).toFixed(3) : "—",
        r.win_rate != null ? `${(Number(r.win_rate) * 100).toFixed(1)}%` : "—",
        r.max_drawdown != null ? `$${Number(r.max_drawdown).toFixed(2)}` : "—",
      ];
      for (let i = 0; i < vals.length; i++) {
        const td = document.createElement("td");
        td.className = "fw-value"; td.style.textAlign = "right";
        td.textContent = vals[i];
        if (i === 2) {
          const n = r.pnl ?? 0;
          td.style.color = n >= 0 ? "#4caf50" : "#f44336";
        }
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }
    resultsDiv.appendChild(table);
  }

  runBtn.addEventListener("click", async () => {
    runBtn.disabled = true;
    runBtn.textContent = "Optimizing...";
    resultsDiv.textContent = "Running optimization...";
    resultsDiv.style.color = "#888";

    try {
      const json = await invoke("run_optimization", {
        symbol: symInp.value.trim().toUpperCase() || sym,
        timeframe: tf,
        fast_min: parseInt(fMin.inp.value) || 5,
        fast_max: parseInt(fMax.inp.value) || 50,
        slow_min: parseInt(sMin.inp.value) || 20,
        slow_max: parseInt(sMax.inp.value) || 200,
      });
      const result = typeof json === "string" ? JSON.parse(json) : json;
      lastResults = Array.isArray(result) ? result : (result.results || []);
      sortCol = "pnl";
      sortAsc = false;
      const sorted = [...lastResults].sort((a, b) => (b.pnl ?? 0) - (a.pnl ?? 0));
      resultsDiv.style.color = "";
      renderOptResults(sorted);
    } catch (e) {
      resultsDiv.textContent = `Optimization failed: ${e}`;
      resultsDiv.style.color = "#f44";
    }

    runBtn.disabled = false;
    runBtn.textContent = "Optimize";
  });
}

// ══════════════════════════════════════════════════════════════
// Custom Indicator Plugin System
// ══════════════════════════════════════════════════════════════

let customPlugins = {}; // { name: { name, params, calculate, seriesKeys } }
let customPluginSeries = {}; // { pluginName: [series] }

async function loadCustomIndicatorPlugins() {
  try {
    const json = await invoke("list_custom_indicators");
    const plugins = typeof json === "string" ? JSON.parse(json) : json;
    return Array.isArray(plugins) ? plugins : (plugins.indicators || []);
  } catch (e) {
    log(`Failed to list custom indicators: ${e}`, "error");
    return [];
  }
}

async function activateCustomPlugin(pluginInfo) {
  try {
    const name = pluginInfo.name || pluginInfo;
    const json = await invoke("get_custom_indicator_source", { name });
    const source = typeof json === "string" ? json : JSON.stringify(json);
    // Evaluate plugin in a sandboxed function scope
    const pluginFn = new Function("return (" + source + ")")();
    if (!pluginFn || !pluginFn.calculate) {
      log(`Plugin ${name} has no calculate() function`, "error");
      return;
    }
    customPlugins[name] = pluginFn;
    applyCustomPlugin(name, pluginFn);
    log(`Custom indicator loaded: ${name}`, "ok");
  } catch (e) {
    log(`Failed to load plugin: ${e}`, "error");
  }
}

function applyCustomPlugin(name, plugin) {
  // Remove existing series for this plugin
  removeCustomPlugin(name);
  if (!chart || !candleSeries) return;
  const data = candleSeries.data();
  if (!data || data.length === 0) return;

  const params = plugin.params || {};
  const result = plugin.calculate(data, params);
  if (!result || !Array.isArray(result)) return;

  const colors = ["#e040fb", "#40c4ff", "#ffab40", "#69f0ae", "#ff5252"];
  const seriesList = [];
  // If result is array of {time, value}, single series
  if (result.length > 0 && result[0].time !== undefined && result[0].value !== undefined) {
    const s = chart.addLineSeries({
      color: colors[0],
      lineWidth: 1.5,
      title: name,
      lastValueVisible: true,
    });
    s.setData(result);
    seriesList.push(s);
  }
  customPluginSeries[name] = seriesList;
}

function removeCustomPlugin(name) {
  if (customPluginSeries[name]) {
    for (const s of customPluginSeries[name]) {
      try { chart.removeSeries(s); } catch (_) {}
    }
    delete customPluginSeries[name];
  }
  delete customPlugins[name];
}

function setupCustomPluginUI() {
  const indicatorList = document.getElementById("indicator-list");
  if (!indicatorList) return;

  // Add plugin section
  const section = document.createElement("div");
  section.className = "ind-section";
  section.textContent = "Custom Plugins";
  indicatorList.appendChild(section);

  const pluginContainer = document.createElement("div");
  pluginContainer.id = "custom-plugin-list";
  indicatorList.appendChild(pluginContainer);

  const loadBtn = document.createElement("button");
  loadBtn.textContent = "Load Plugins";
  loadBtn.className = "bt-run-btn";
  loadBtn.style.cssText = "font-size:10px;padding:3px 8px;margin-top:4px;width:100%;";
  indicatorList.appendChild(loadBtn);

  loadBtn.addEventListener("click", async () => {
    loadBtn.textContent = "Loading...";
    loadBtn.disabled = true;
    const plugins = await loadCustomIndicatorPlugins();
    pluginContainer.textContent = "";
    if (plugins.length === 0) {
      const msg = document.createElement("div");
      msg.style.cssText = "color:#666;font-size:10px;padding:2px 0;";
      msg.textContent = "No custom plugins found.";
      pluginContainer.appendChild(msg);
    } else {
      for (const p of plugins) {
        const pName = p.name || p;
        const label = document.createElement("label");
        label.className = "ind-row";
        const cb = document.createElement("input");
        cb.type = "checkbox";
        cb.dataset.plugin = pName;
        cb.addEventListener("change", () => {
          if (cb.checked) activateCustomPlugin(p);
          else removeCustomPlugin(pName);
        });
        label.appendChild(cb);
        label.appendChild(document.createTextNode(` ${pName}`));
        pluginContainer.appendChild(label);
      }
    }
    loadBtn.textContent = "Load Plugins";
    loadBtn.disabled = false;
  });
}

async function cmdHistory() {
  const win = createWindow({ title: "Trade History", width: 550, height: 400 });
  try {
    const histJson = await invoke("get_order_history", { limit: 50 });
    const history = JSON.parse(histJson);
    if (!history || history.length === 0) { win.setContent("No order history found."); return; }
    win.contentElement.textContent = "";
    const table = document.createElement("table");
    table.className = "fw-table";
    const thead = document.createElement("tr");
    for (const h of ["Time", "Symbol", "Side", "Qty", "Price", "Status"]) {
      const th = document.createElement("td");
      th.style.cssText = "color:#666;font-weight:bold;font-size:10px;";
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);
    for (const o of history) {
      const tr = document.createElement("tr");
      const vals = [
        (o.created_at || "").substring(0, 16).replace("T", " "),
        o.symbol,
        o.side,
        o.qty,
        o.filled_avg_price || o.limit_price || "—",
        o.status,
      ];
      for (const v of vals) {
        const td = document.createElement("td");
        td.className = "fw-value";
        td.style.textAlign = "left";
        td.textContent = v;
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }
    win.appendElement(table);
  } catch (e) { win.setContent(`Failed to load history: ${e}`); }
}

// ══════════════════════════════════════════════════════════════
// FEATURE 4: Watchlist / Quote Monitor
// ══════════════════════════════════════════════════════════════

const WATCHLIST_KEY = "typhoon_watchlist";
let watchlistInterval = null;
let watchlistWindow = null;

function getWatchlist() {
  try { return JSON.parse(localStorage.getItem(WATCHLIST_KEY) || "[]"); }
  catch { return []; }
}

function saveWatchlist(list) {
  localStorage.setItem(WATCHLIST_KEY, JSON.stringify(list));
}

function cmdWatchlist() {
  if (watchlistWindow) {
    try { watchlistWindow.close(); } catch (_) {}
  }
  watchlistWindow = createWindow({
    title: "Quote Monitor",
    width: 450,
    height: 400,
    onClose: () => {
      if (watchlistInterval) { clearInterval(watchlistInterval); watchlistInterval = null; }
      watchlistWindow = null;
    },
  });

  const container = document.createElement("div");
  container.style.cssText = "display:flex;flex-direction:column;height:100%;";

  // Add symbol row
  const addRow = document.createElement("div");
  addRow.style.cssText = "display:flex;gap:4px;padding:4px;border-bottom:1px solid #333;";
  const addInput = document.createElement("input");
  addInput.type = "text";
  addInput.placeholder = "Add symbol...";
  addInput.style.cssText = "flex:1;background:#111;color:#fff;border:1px solid #555;padding:4px 8px;font-family:inherit;font-size:11px;";
  const addBtn = document.createElement("button");
  addBtn.textContent = "+";
  addBtn.style.cssText = "background:#0a5f38;color:#8f8;border:1px solid #555;padding:4px 10px;cursor:pointer;font-family:inherit;font-weight:bold;";
  addBtn.addEventListener("click", () => {
    const sym = addInput.value.trim().toUpperCase();
    if (!sym) return;
    const wl = getWatchlist();
    if (!wl.includes(sym)) { wl.push(sym); saveWatchlist(wl); }
    addInput.value = "";
    refreshWatchlist(tableBody);
  });
  addInput.addEventListener("keydown", (e) => { if (e.key === "Enter") addBtn.click(); });
  addRow.appendChild(addInput);
  addRow.appendChild(addBtn);
  container.appendChild(addRow);

  // Table
  const table = document.createElement("table");
  table.className = "watchlist-table";
  const thead = document.createElement("thead");
  const hdr = document.createElement("tr");
  for (const h of ["Symbol", "Last", "Chg %", "Vol", ""]) {
    const th = document.createElement("th");
    th.textContent = h;
    hdr.appendChild(th);
  }
  thead.appendChild(hdr);
  table.appendChild(thead);
  const tableBody = document.createElement("tbody");
  table.appendChild(tableBody);
  container.appendChild(table);

  watchlistWindow.contentElement.textContent = "";
  watchlistWindow.appendElement(container);

  refreshWatchlist(tableBody);
  if (watchlistInterval) clearInterval(watchlistInterval);
  watchlistInterval = setInterval(() => refreshWatchlist(tableBody), 30000);
}

async function refreshWatchlist(tableBody) {
  const wl = getWatchlist();
  tableBody.textContent = "";
  for (const sym of wl) {
    const tr = document.createElement("tr");

    const tdSym = document.createElement("td");
    tdSym.textContent = sym;
    tdSym.style.color = "#8ff";
    tdSym.style.fontWeight = "bold";

    const tdLast = document.createElement("td");
    tdLast.textContent = "...";
    const tdChg = document.createElement("td");
    tdChg.textContent = "...";
    const tdVol = document.createElement("td");
    tdVol.textContent = "...";

    const tdDel = document.createElement("td");
    const delBtn = document.createElement("button");
    delBtn.textContent = "×";
    delBtn.style.cssText = "background:none;border:1px solid #f44;color:#f44;cursor:pointer;font-size:10px;padding:0 4px;border-radius:2px;";
    delBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      const list = getWatchlist().filter(s => s !== sym);
      saveWatchlist(list);
      tr.remove();
    });
    tdDel.appendChild(delBtn);

    tr.appendChild(tdSym);
    tr.appendChild(tdLast);
    tr.appendChild(tdChg);
    tr.appendChild(tdVol);
    tr.appendChild(tdDel);

    // Click row to switch chart
    tr.addEventListener("click", () => {
      document.getElementById("symbol-input").value = sym;
      triggerLoad();
    });

    tableBody.appendChild(tr);

    // Fetch data async
    (async () => {
      try {
        const barsJson = await invoke("get_bars", { symbol: sym, timeframe: "1Day", limit: 2 });
        const bars = JSON.parse(barsJson);
        if (bars.length >= 2) {
          const last = bars[bars.length - 1];
          const prev = bars[bars.length - 2];
          const chgPct = ((last.close - prev.close) / prev.close * 100);
          tdLast.textContent = last.close.toFixed(2);
          tdChg.textContent = `${chgPct >= 0 ? "+" : ""}${chgPct.toFixed(2)}%`;
          tdChg.className = chgPct >= 0 ? "wl-positive" : "wl-negative";
          tdVol.textContent = last.volume ? (last.volume >= 1e6 ? `${(last.volume / 1e6).toFixed(1)}M` : last.volume.toLocaleString()) : "—";
        } else if (bars.length === 1) {
          tdLast.textContent = bars[0].close.toFixed(2);
          tdChg.textContent = "—";
          tdVol.textContent = bars[0].volume ? bars[0].volume.toLocaleString() : "—";
        }
      } catch (_) {
        tdLast.textContent = "err";
      }
    })();
  }
}

// ══════════════════════════════════════════════════════════════
// FEATURE 5: Multi-chart layouts (Split View)
// ══════════════════════════════════════════════════════════════

let splitActive = false;
let splitChart = null;
let splitCandleSeries = null;
let splitSymbol = "";
let splitContainer = null;

function activateSplit(symbol) {
  if (splitActive) deactivateSplit();
  const chartStack = document.getElementById("chart-stack");

  // Create split container
  splitContainer = document.createElement("div");
  splitContainer.id = "split-chart-container";
  chartStack.appendChild(splitContainer);
  chartStack.classList.add("split-mode");

  splitChart = createChart(splitContainer, {
    width: splitContainer.clientWidth,
    height: splitContainer.clientHeight,
    layout: {
      background: { color: "#000000" },
      textColor: "#d1d4dc",
      fontFamily: "Consolas, Courier New, monospace",
      attributionLogo: false,
    },
    grid: {
      vertLines: { color: "#333333", style: 3 },
      horzLines: { color: "#333333", style: 3 },
    },
    crosshair: { mode: CrosshairMode.Normal },
    rightPriceScale: { borderColor: "#333" },
    timeScale: { borderColor: "#333", timeVisible: true },
  });

  splitCandleSeries = splitChart.addCandlestickSeries({
    upColor: "#00ff00",
    downColor: "#ff0000",
    borderDownColor: "#ff0000",
    borderUpColor: "#00ff00",
    wickDownColor: "#ff0000",
    wickUpColor: "#00ff00",
  });

  const ro = new ResizeObserver(() => {
    if (splitChart && splitContainer) {
      splitChart.resize(splitContainer.clientWidth, splitContainer.clientHeight);
    }
  });
  ro.observe(splitContainer);

  splitActive = true;
  splitSymbol = symbol;

  // Load data for split chart
  loadSplitChart(symbol);
  log(`Split view activated: ${symbol}`, "ok");
}

async function loadSplitChart(symbol) {
  if (!splitChart || !splitCandleSeries) return;
  try {
    const tf = document.getElementById("timeframe-select").value;
    const limit = parseInt(document.getElementById("bar-count").value) || 1000;
    const cacheKey = getCacheKey(symbol, tf);
    let bars;
    const cached = barCache[cacheKey];
    if (cached && cached.data) {
      bars = cached.data;
    } else {
      const barsJson = await invoke("get_bars", { symbol, timeframe: tf, limit });
      bars = JSON.parse(barsJson);
    }
    const chartData = bars.map(b => ({
      time: Math.floor(new Date(b.timestamp).getTime() / 1000),
      open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume,
    }));
    splitCandleSeries.setData(chartData);
    splitChart.timeScale().fitContent();
  } catch (e) {
    log(`Split chart load failed: ${e}`, "error");
  }
}

function deactivateSplit() {
  if (!splitActive) return;
  const chartStack = document.getElementById("chart-stack");
  chartStack.classList.remove("split-mode");
  if (splitChart) { splitChart.remove(); splitChart = null; splitCandleSeries = null; }
  if (splitContainer) { splitContainer.remove(); splitContainer = null; }
  splitActive = false;
  splitSymbol = "";
  // Resize main chart
  const container = document.getElementById("chart-container");
  chart.resize(container.clientWidth, container.clientHeight);
  log("Split view deactivated", "info");
}

function setupSplitButton() {
  const btn = document.getElementById("btn-split");
  if (!btn) return;
  btn.addEventListener("click", () => {
    if (splitActive) {
      deactivateSplit();
      btn.textContent = "Split";
      btn.style.color = "#8cf";
    } else {
      const sym = prompt("Symbol for split panel:", currentSymbol || "SPY");
      if (!sym) return;
      activateSplit(sym.toUpperCase());
      btn.textContent = "Unsplit";
      btn.style.color = "#f88";
    }
  });
}

// ══════════════════════════════════════════════════════════════
// FEATURE 6: Chart Screenshot Export (Ctrl+Shift+S)
// ══════════════════════════════════════════════════════════════

function showToast(msg) {
  const toast = document.createElement("div");
  toast.className = "screenshot-toast";
  toast.textContent = msg;
  document.body.appendChild(toast);
  setTimeout(() => toast.remove(), 2100);
}

async function captureChartScreenshot() {
  const container = document.getElementById("chart-container");
  if (!container) return;

  // Find all canvas elements within the chart container
  const canvases = container.querySelectorAll("canvas");
  if (canvases.length === 0) { log("No chart canvas found", "warn"); return; }

  // Create a composite canvas
  const compositeCanvas = document.createElement("canvas");
  compositeCanvas.width = container.clientWidth;
  compositeCanvas.height = container.clientHeight;
  const ctx = compositeCanvas.getContext("2d");

  // Fill background
  ctx.fillStyle = "#000000";
  ctx.fillRect(0, 0, compositeCanvas.width, compositeCanvas.height);

  // Draw each canvas at its position
  for (const canvas of canvases) {
    const rect = canvas.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    const x = rect.left - containerRect.left;
    const y = rect.top - containerRect.top;
    try {
      ctx.drawImage(canvas, x, y, rect.width, rect.height);
    } catch (_) {}
  }

  // Also draw the drawing overlay canvas
  if (drawCanvas) {
    try { ctx.drawImage(drawCanvas, 0, 0); } catch (_) {}
  }

  // Add watermark
  ctx.fillStyle = "#ffffff44";
  ctx.font = "10px Consolas";
  ctx.fillText(`${currentSymbol} ${currentTimeframe} — TyphooN Terminal`, 8, compositeCanvas.height - 8);

  try {
    // Copy to clipboard
    compositeCanvas.toBlob(async (blob) => {
      if (!blob) return;
      try {
        await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
        showToast("Screenshot copied to clipboard");
        log("Chart screenshot copied to clipboard", "ok");
      } catch (e) {
        // Fallback: download
        downloadBlob(blob);
      }
    }, "image/png");
  } catch (e) {
    log(`Screenshot failed: ${e}`, "error");
  }
}

function downloadChartScreenshot() {
  const container = document.getElementById("chart-container");
  if (!container) return;
  const canvases = container.querySelectorAll("canvas");
  if (canvases.length === 0) return;

  const compositeCanvas = document.createElement("canvas");
  compositeCanvas.width = container.clientWidth;
  compositeCanvas.height = container.clientHeight;
  const ctx = compositeCanvas.getContext("2d");
  ctx.fillStyle = "#000000";
  ctx.fillRect(0, 0, compositeCanvas.width, compositeCanvas.height);

  for (const canvas of canvases) {
    const rect = canvas.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    try { ctx.drawImage(canvas, rect.left - containerRect.left, rect.top - containerRect.top, rect.width, rect.height); } catch (_) {}
  }
  if (drawCanvas) try { ctx.drawImage(drawCanvas, 0, 0); } catch (_) {}

  compositeCanvas.toBlob((blob) => {
    if (blob) downloadBlob(blob);
  }, "image/png");
}

function downloadBlob(blob) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${currentSymbol || "chart"}_${currentTimeframe}_${new Date().toISOString().slice(0, 10)}.png`;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
  showToast("Screenshot saved to file");
  log("Chart screenshot saved to file", "ok");
}

function setupScreenshotShortcut() {
  document.addEventListener("keydown", (e) => {
    if (e.ctrlKey && e.shiftKey && e.key === "S") {
      e.preventDefault();
      captureChartScreenshot();
    }
  });
}

// ══════════════════════════════════════════════════════════════
// FEATURE 7: Economic Calendar
// ══════════════════════════════════════════════════════════════

async function cmdCalendar() {
  const win = createWindow({ title: "Economic Calendar", width: 550, height: 400 });
  win.contentElement.textContent = "";

  // Try Alpaca calendar API first
  let calendarData = null;
  try {
    const json = await invoke("get_calendar", { start: new Date().toISOString().slice(0, 10), end: new Date(Date.now() + 14 * 86400000).toISOString().slice(0, 10) });
    calendarData = JSON.parse(json);
  } catch (_) {}

  if (calendarData && calendarData.length > 0) {
    const table = document.createElement("table");
    table.className = "calendar-table";
    const thead = document.createElement("tr");
    for (const h of ["Date", "Open", "Close", "Status"]) {
      const th = document.createElement("th");
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);
    for (const day of calendarData) {
      const tr = document.createElement("tr");
      const isToday = day.date === new Date().toISOString().slice(0, 10);
      for (const val of [day.date, day.open || "09:30", day.close || "16:00", isToday ? "TODAY" : ""]) {
        const td = document.createElement("td");
        td.textContent = val;
        if (isToday) td.style.color = "#4caf50";
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }
    win.appendElement(table);
  } else {
    // Fallback: hardcoded major events
    const events = [
      { date: "2026-03-17", time: "08:30", event: "Retail Sales", impact: "High", prev: "0.2%", forecast: "0.5%" },
      { date: "2026-03-18", time: "14:00", event: "FOMC Meeting", impact: "High", prev: "5.25%", forecast: "5.25%" },
      { date: "2026-03-19", time: "14:00", event: "FOMC Decision", impact: "High", prev: "5.25%", forecast: "5.25%" },
      { date: "2026-03-20", time: "08:30", event: "Initial Claims", impact: "Medium", prev: "220K", forecast: "218K" },
      { date: "2026-03-21", time: "09:45", event: "PMI Flash", impact: "Medium", prev: "52.2", forecast: "52.5" },
      { date: "2026-03-25", time: "10:00", event: "Consumer Confidence", impact: "High", prev: "104.7", forecast: "105.0" },
      { date: "2026-03-26", time: "08:30", event: "Durable Goods", impact: "Medium", prev: "-4.5%", forecast: "1.0%" },
      { date: "2026-03-28", time: "08:30", event: "GDP (Q4 Final)", impact: "High", prev: "3.2%", forecast: "3.2%" },
    ];
    const table = document.createElement("table");
    table.className = "calendar-table";
    const thead = document.createElement("tr");
    for (const h of ["Date", "Time", "Event", "Impact", "Prev", "Forecast"]) {
      const th = document.createElement("th");
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);
    for (const ev of events) {
      const tr = document.createElement("tr");
      for (const val of [ev.date, ev.time, ev.event, ev.impact, ev.prev, ev.forecast]) {
        const td = document.createElement("td");
        td.textContent = val;
        if (val === "High") td.style.color = "#f44336";
        else if (val === "Medium") td.style.color = "#ff8";
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }
    win.appendElement(table);
    const note = document.createElement("div");
    note.style.cssText = "color:#555;font-size:9px;padding:8px;";
    note.textContent = "Note: Showing hardcoded calendar data. Connect to Alpaca for live market calendar.";
    win.appendElement(note);
  }
}

// ══════════════════════════════════════════════════════════════
// FEATURE 8: Extended Drawing Tools
// ══════════════════════════════════════════════════════════════

// Extended drawing types: "horizontal", "rectangle", "channel"
// These augment the existing "trendline" and "fibonacci" system.
// drawingMode can now be: "trendline", "fibonacci", "horizontal", "rectangle", "channel"
// "horizontal" needs one click.
// "rectangle" needs two clicks (two corners).
// "channel" needs three clicks (trendline + offset point).

let channelThirdClick = false; // track channel state

// Extended drawing renderer: handles trendline, fibonacci, horizontal, rectangle, channel
// Extended drawing renderer: handles all types including horizontal, rectangle, channel.
// The original renderDrawings() delegates here via the check at its top.

function renderDrawingsExtended() {
  if (!drawCanvas || !chart || !candleSeries) return;
  const ctx = drawCanvas.getContext("2d");
  ctx.clearRect(0, 0, drawCanvas.width, drawCanvas.height);

  for (const d of drawings) {
    if (d.type === "trendline") {
      const x1 = chart.timeScale().timeToCoordinate(d.p1.time);
      const y1 = candleSeries.priceToCoordinate(d.p1.price);
      const x2 = chart.timeScale().timeToCoordinate(d.p2.time);
      const y2 = candleSeries.priceToCoordinate(d.p2.price);
      if (x1 === null || y1 === null || x2 === null || y2 === null) continue;
      ctx.beginPath();
      ctx.strokeStyle = "#00bcd4";
      ctx.lineWidth = 1.5;
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();

    } else if (d.type === "fibonacci") {
      const x1 = chart.timeScale().timeToCoordinate(d.p1.time);
      const y1 = candleSeries.priceToCoordinate(d.p1.price);
      const x2 = chart.timeScale().timeToCoordinate(d.p2.time);
      const y2 = candleSeries.priceToCoordinate(d.p2.price);
      if (x1 === null || y1 === null || x2 === null || y2 === null) continue;
      const high = Math.max(d.p1.price, d.p2.price);
      const low = Math.min(d.p1.price, d.p2.price);
      const range = high - low;
      const levels = [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
      const colors = ["#f44336", "#ff9800", "#ffeb3b", "#8bc34a", "#00bcd4", "#3f51b5", "#9c27b0"];
      const xLeft = Math.min(x1, x2);
      const xRight = drawCanvas.width;
      for (let i = 0; i < levels.length; i++) {
        const price = high - range * levels[i];
        const y = candleSeries.priceToCoordinate(price);
        if (y === null) continue;
        ctx.beginPath();
        ctx.strokeStyle = colors[i];
        ctx.lineWidth = 0.8;
        ctx.setLineDash([4, 4]);
        ctx.moveTo(xLeft, y);
        ctx.lineTo(xRight, y);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.fillStyle = colors[i];
        ctx.font = "10px Consolas";
        ctx.fillText(`${(levels[i] * 100).toFixed(1)}% $${price.toFixed(2)}`, xLeft + 4, y - 3);
      }

    } else if (d.type === "horizontal") {
      const y = candleSeries.priceToCoordinate(d.p1.price);
      if (y === null) continue;
      ctx.beginPath();
      ctx.strokeStyle = "#ff9800";
      ctx.lineWidth = 1;
      ctx.setLineDash([6, 3]);
      ctx.moveTo(0, y);
      ctx.lineTo(drawCanvas.width, y);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.fillStyle = "#ff9800";
      ctx.font = "10px Consolas";
      ctx.fillText(`$${d.p1.price.toFixed(2)}`, 4, y - 4);

    } else if (d.type === "rectangle") {
      const x1 = chart.timeScale().timeToCoordinate(d.p1.time);
      const y1 = candleSeries.priceToCoordinate(d.p1.price);
      const x2 = chart.timeScale().timeToCoordinate(d.p2.time);
      const y2 = candleSeries.priceToCoordinate(d.p2.price);
      if (x1 === null || y1 === null || x2 === null || y2 === null) continue;
      const rx = Math.min(x1, x2);
      const ry = Math.min(y1, y2);
      const rw = Math.abs(x2 - x1);
      const rh = Math.abs(y2 - y1);
      ctx.fillStyle = "#00bcd420";
      ctx.fillRect(rx, ry, rw, rh);
      ctx.strokeStyle = "#00bcd4";
      ctx.lineWidth = 1;
      ctx.strokeRect(rx, ry, rw, rh);

    } else if (d.type === "channel") {
      const x1 = chart.timeScale().timeToCoordinate(d.p1.time);
      const y1 = candleSeries.priceToCoordinate(d.p1.price);
      const x2 = chart.timeScale().timeToCoordinate(d.p2.time);
      const y2 = candleSeries.priceToCoordinate(d.p2.price);
      if (x1 === null || y1 === null || x2 === null || y2 === null) continue;
      const offset = d.offset || 0;
      const oY = candleSeries.priceToCoordinate(d.p1.price + offset);
      const oY2 = candleSeries.priceToCoordinate(d.p2.price + offset);
      if (oY === null || oY2 === null) continue;
      // Main line
      ctx.beginPath();
      ctx.strokeStyle = "#00bcd4";
      ctx.lineWidth = 1.5;
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
      // Parallel line
      ctx.beginPath();
      ctx.strokeStyle = "#00bcd4";
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 2]);
      ctx.moveTo(x1, oY);
      ctx.lineTo(x2, oY2);
      ctx.stroke();
      ctx.setLineDash([]);
    }
  }
}

// Override the existing click handler for drawing to support new types
function setupExtendedDrawings() {
  const container = document.getElementById("chart-container");

  // We need to intercept clicks for the extended drawing modes
  // The existing click handler in setupDrawingCanvas already handles trendline/fibonacci
  // We add a new capture-phase listener for our extended types
  container.addEventListener("click", (e) => {
    if (!drawingMode || drawingMode === "trendline" || drawingMode === "fibonacci") return;

    const rect = container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const time = chart.timeScale().coordinateToTime(x);
    const price = candleSeries.coordinateToPrice(y);
    if (time === null || price === null) return;

    if (drawingMode === "horizontal") {
      drawings.push({ type: "horizontal", p1: { time, price }, p2: { time, price } });
      saveDrawings();
      log(`Horizontal line at $${price.toFixed(4)}`, "ok");
      drawingMode = null;
      drawingAnchor = null;
      container.style.cursor = "";
      renderDrawings();
      e.stopImmediatePropagation(); // prevent the original handler

    } else if (drawingMode === "rectangle") {
      if (!drawingAnchor) {
        drawingAnchor = { time, price };
        log("Rectangle: first corner set — click second corner", "info");
        e.stopImmediatePropagation();
      } else {
        drawings.push({ type: "rectangle", p1: drawingAnchor, p2: { time, price } });
        saveDrawings();
        log("Rectangle drawn", "ok");
        drawingAnchor = null;
        drawingMode = null;
        container.style.cursor = "";
        renderDrawings();
        e.stopImmediatePropagation();
      }

    } else if (drawingMode === "channel") {
      if (!drawingAnchor) {
        drawingAnchor = { time, price };
        log("Channel: set first point — click second point", "info");
        e.stopImmediatePropagation();
      } else if (!channelThirdClick) {
        // Second click: save the trendline endpoints, wait for offset
        channelThirdClick = true;
        drawingAnchor._p2 = { time, price };
        log("Channel: set line — click to set parallel offset", "info");
        e.stopImmediatePropagation();
      } else {
        // Third click: compute offset from first point
        const offset = price - drawingAnchor.price;
        drawings.push({
          type: "channel",
          p1: { time: drawingAnchor.time, price: drawingAnchor.price },
          p2: drawingAnchor._p2,
          offset,
        });
        saveDrawings();
        log("Channel drawn", "ok");
        drawingAnchor = null;
        drawingMode = null;
        channelThirdClick = false;
        container.style.cursor = "";
        renderDrawings();
        e.stopImmediatePropagation();
      }
    }
  }, true); // capture phase to run before existing handler
}

// patchRenderDrawings is no longer needed - renderDrawings delegates to renderDrawingsExtended
function patchRenderDrawings() {
  // No-op: delegation handled in renderDrawings() directly
}

// ── Init ────────────────────────────────────────────────────

document.addEventListener("DOMContentLoaded", () => {
  loadBarCacheFromDisk().then(() => migrateLocalStorageCache());
  initChart();
  setupDrawingCanvas();
  loadDrawings();
  setupLineDrag();
  setupExtendedDrawings();
  patchRenderDrawings();
  setupPaneResizers();
  setupLogPanel();
  setupNewsPanel();
  setupIndicatorPanel();
  setupPositionsPanel();
  setupOrdersPanel();
  loadAlerts();
  setupAutocomplete();
  setupButtons();
  setupKeyboard();
  setupConnect();
  setupTabs();
  setupTemplates();
  setupProfiles();
  setupCommandPalette();
  setupSplitButton();
  setupScreenshotShortcut();
  setupCustomPluginUI();

  // Auto-save session periodically and on shutdown
  setInterval(saveSession, 30000); // every 30s
  window.addEventListener("beforeunload", saveSession);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") saveSession();
  });
});

function setupTabs() {
  // Restore previous session or create a fresh tab
  if (!restoreSession()) {
    createTab();
  }

  // "+" button creates new tab
  document.getElementById("btn-new-tab").addEventListener("click", () => {
    createTab();
    document.getElementById("symbol-input").focus();
  });

  // Keyboard: Ctrl+T new tab, Ctrl+W close tab
  document.addEventListener("keydown", (e) => {
    if (e.ctrlKey && e.key === "t") {
      e.preventDefault();
      document.getElementById("btn-new-tab").click();
    }
    if (e.ctrlKey && e.key === "w") {
      e.preventDefault();
      if (activeTabId !== null) closeTab(activeTabId);
    }
  });
}
