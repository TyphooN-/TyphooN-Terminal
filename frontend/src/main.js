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

function createTab(symbol = "", timeframe = "1Hour") {
  const id = nextTabId++;
  const tab = { id, symbol, timeframe, barCount: "1000", lastPrice: 0, chartData: null };
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
  list.innerHTML = "";
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
    draggable: true,
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
    draggable: true,
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
  // Applied to OPEN price (matching MQL5 PRICE_OPEN)
  let kama = data[period].open;
  for (let i = period; i < data.length; i++) {
    const signal = Math.abs(data[i].open - data[i - period].open);
    let noise = 0;
    for (let j = i - period + 1; j <= i; j++) {
      noise += Math.abs(data[j].open - data[j - 1].open);
    }
    const er = noise !== 0 ? signal / noise : 0;
    const ssc = er * (fastSC - slowSC) + slowSC;
    const ssc2 = ssc * ssc;
    kama = ssc2 * (data[i].open - kama) + kama;
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
// Volume bars colored by price action characteristics
function calcBetterVolume(data) {
  const result = [];
  if (data.length < 2) return result;
  // Calculate average volume over 20 bars
  for (let i = 1; i < data.length; i++) {
    const vol = data[i].volume || 0;
    const range = data[i].high - data[i].low;
    const body = Math.abs(data[i].close - data[i].open);
    const isUp = data[i].close >= data[i].open;
    // Average volume (lookback 20)
    let avgVol = 0, count = 0;
    for (let j = Math.max(0, i - 20); j < i; j++) { avgVol += (data[j].volume || 0); count++; }
    avgVol = count > 0 ? avgVol / count : vol;
    // Climax: very high volume + large range
    const isClimax = vol > avgVol * 2 && range > 0;
    // High volume
    const isHigh = vol > avgVol * 1.5;
    // Low volume
    const isLow = vol < avgVol * 0.5;
    // Churn: high volume but small body relative to range
    const isChurn = isHigh && range > 0 && (body / range) < 0.3;

    let color;
    if (isChurn) color = "#FFFF00";           // Yellow — churn (indecision)
    else if (isClimax && isUp) color = "#00FF00";  // Green — climax up
    else if (isClimax && !isUp) color = "#FF0000"; // Red — climax down
    else if (isHigh && isUp) color = "#00BFFF";    // Cyan — high vol up
    else if (isHigh && !isUp) color = "#FF00FF";   // Magenta — high vol down
    else if (isLow) color = "#FFFFFF44";           // Dim white — low volume
    else color = isUp ? "#2196f380" : "#ef535080";  // Normal — muted green/red

    result.push({ time: data[i].time, value: vol, color });
  }
  return result;
}

// ── Supply/Demand Zones ─────────────────────────────────────
// Detect strong move-away candles and project their origin as zones
function calcSupplyDemandZones(data, lookback = 200) {
  const zones = [];
  if (data.length < 10) return zones;

  // Calculate average range for significance filtering
  let avgRange = 0;
  for (let i = Math.max(0, data.length - lookback); i < data.length; i++) {
    avgRange += data[i].high - data[i].low;
  }
  avgRange /= Math.min(lookback, data.length);
  const minMoveSize = avgRange * 1.5; // impulse candle must be significantly larger than average

  const start = Math.max(0, data.length - lookback);
  for (let i = start + 2; i < data.length - 1; i++) {
    const prev = data[i - 1];
    const cur = data[i];
    const range = cur.high - cur.low;
    const body = Math.abs(cur.close - cur.open);

    if (range === 0) continue;
    const bodyRatio = body / range;

    // Impulse candle: large body (>70%), range > 1.5x average
    if (bodyRatio < 0.7 || range < minMoveSize) continue;

    const prevRange = prev.high - prev.low;
    if (prevRange <= 0) continue;
    const prevBodyRatio = Math.abs(prev.close - prev.open) / prevRange;

    // Base candle must be small body (<40%) — indecision before the move
    if (prevBodyRatio >= 0.4) continue;

    if (cur.close > cur.open) {
      // Demand: strong bullish impulse leaving a base
      zones.push({ type: "demand", high: prev.high, low: prev.low, startTime: prev.time });
    } else {
      // Supply: strong bearish impulse leaving a base
      zones.push({ type: "supply", high: prev.high, low: prev.low, startTime: prev.time });
    }
  }

  // Filter out broken zones (price traded through the zone body)
  const lastPrice = data[data.length - 1].close;
  const valid = zones.filter(z => {
    // Check if price has traded through the zone since it formed
    const startIdx = data.findIndex(d => d.time >= z.startTime);
    if (startIdx < 0) return false;
    for (let k = startIdx + 2; k < data.length; k++) {
      if (z.type === "demand" && data[k].close < z.low) return false; // broken demand
      if (z.type === "supply" && data[k].close > z.high) return false; // broken supply
    }
    return true;
  });
  return valid.slice(-6); // keep max 6 most recent valid zones
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
      // Supply/Demand zones — filled rectangles (like a trader would draw)
      // Two area series per zone: one fills DOWN from top, one fills UP from bottom
      // Where they overlap creates a solid filled rectangle
      const zones = calcSupplyDemandZones(chartData);
      for (let zi = 0; zi < zones.length; zi++) {
        const z = zones[zi];
        const isDemand = z.type === "demand";
        const fillColor = isDemand ? "#00FF0025" : "#FF000025";
        const lineColor = isDemand ? "#00FF0077" : "#FF000077";
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

// ── Bar Cache (localStorage persistent) ─────────────────────

const barCache = {}; // "SYMBOL:TF" → { data: [], timestamp: Date }
const CACHE_TTL_MS = 60 * 1000; // 1 minute — only refetch if older than this
const BAR_CACHE_PREFIX = "typhoon_bars_";

function getCacheKey(symbol, tf) { return `${symbol}:${tf}`; }

// Load cache from localStorage on startup
function loadBarCacheFromDisk() {
  try {
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key && key.startsWith(BAR_CACHE_PREFIX)) {
        const cacheKey = key.substring(BAR_CACHE_PREFIX.length);
        const stored = JSON.parse(localStorage.getItem(key));
        if (stored && stored.data && stored.timestamp) {
          barCache[cacheKey] = { data: stored.data, timestamp: stored.timestamp };
        }
      }
    }
    const count = Object.keys(barCache).length;
    if (count > 0) log(`Loaded ${count} cached bar sets from disk`, "info");
  } catch (_) {}
}

// Save a cache entry to localStorage
function saveBarCacheToDisk(cacheKey, data) {
  try {
    const entry = { data, timestamp: Date.now() };
    localStorage.setItem(BAR_CACHE_PREFIX + cacheKey, JSON.stringify(entry));
  } catch (e) {
    // localStorage full — clear old entries
    log(`Cache storage full, clearing old entries`, "warn");
    for (let i = localStorage.length - 1; i >= 0; i--) {
      const key = localStorage.key(i);
      if (key && key.startsWith(BAR_CACHE_PREFIX)) {
        localStorage.removeItem(key);
      }
    }
  }
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

    // Check memory cache, then disk cache, then fetch
    const cached = barCache[cacheKey];
    if (cached && (Date.now() - cached.timestamp) < CACHE_TTL_MS && cached.data.length >= limit * 0.9) {
      bars = cached.data;
      log(`${symbol} @ ${timeframe}: ${bars.length} bars from memory cache`, "info");
    } else {
      const barsJson = await invoke("get_bars", { symbol, timeframe, limit });
      bars = JSON.parse(barsJson);
      barCache[cacheKey] = { data: bars, timestamp: Date.now() };
      saveBarCacheToDisk(cacheKey, bars);
      // Show date range progress
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

async function updateLatestBar(symbol, timeframe) {
  if (symbol !== currentSymbol || timeframe !== currentTimeframe) return;
  try {
    const barsJson = await invoke("get_bars", { symbol, timeframe, limit: 2 });
    const bars = JSON.parse(barsJson);
    if (bars.length === 0) return;
    const latest = bars[bars.length - 1];
    const bar = {
      time: Math.floor(new Date(latest.timestamp).getTime() / 1000),
      open: latest.open,
      high: latest.high,
      low: latest.low,
      close: latest.close,
    };
    candleSeries.update(bar);
    lastPrice = bar.close;
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
        list.innerHTML = "";
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
            document.getElementById("btn-load-chart").click();
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
      document.getElementById("btn-load-chart").click();
    } else if (e.key === "Escape") {
      list.classList.add("hidden");
    }
  });

  input.addEventListener("blur", () => {
    setTimeout(() => list.classList.add("hidden"), 200);
  });
}

// ── Button Handlers ─────────────────────────────────────────

function setupButtons() {
  document.getElementById("btn-load-chart").addEventListener("click", () => {
    let symbol = document.getElementById("symbol-input").value.trim().toUpperCase();
    const tf = document.getElementById("timeframe-select").value;
    if (!symbol) return;

    // Auto-detect common crypto tickers → Alpaca format (BTC → BTC/USD)
    const cryptoMap = {
      "BTC": "BTC/USD", "ETH": "ETH/USD", "SOL": "SOL/USD", "DOGE": "DOGE/USD",
      "ADA": "ADA/USD", "XRP": "XRP/USD", "DOT": "DOT/USD", "AVAX": "AVAX/USD",
      "LINK": "LINK/USD", "MATIC": "MATIC/USD", "UNI": "UNI/USD", "SHIB": "SHIB/USD",
      "LTC": "LTC/USD", "BCH": "BCH/USD", "AAVE": "AAVE/USD", "SUSHI": "SUSHI/USD",
    };
    if (cryptoMap[symbol]) symbol = cryptoMap[symbol];

    document.getElementById("symbol-input").value = symbol;
    document.getElementById("symbol-autocomplete").classList.add("hidden");
    loadChart(symbol, tf);
  });

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
  document.getElementById("btn-trade").addEventListener("click", async () => {
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

      for (let i = 0; i < calc.count; i++) {
        await invoke("place_order", { symbol: currentSymbol, qty: calc.lots, side: calc.side });
      }
      await invoke("set_sl_level", { symbol: currentSymbol, price: sl });
      await invoke("set_tp_level", { symbol: currentSymbol, price: tp });
      updateDashboard();
    } catch (e) {
      alert(`Order failed: ${e}`);
    }
  });

  // ── Close All ──
  document.getElementById("btn-close-all").addEventListener("click", async () => {
    if (!currentSymbol || !confirm(`Close ALL positions on ${currentSymbol}?`)) return;
    try {
      await invoke("close_position", { symbol: currentSymbol, qty: null });
      updateDashboard();
    } catch (e) { alert(`Close failed: ${e}`); }
  });

  // ── Close Partial ──
  document.getElementById("btn-close-partial").addEventListener("click", async () => {
    if (!currentSymbol) return;
    const qty = prompt(`Qty to close on ${currentSymbol}:`);
    if (!qty || isNaN(qty)) return;
    try {
      await invoke("close_position", { symbol: currentSymbol, qty: parseFloat(qty) });
      updateDashboard();
    } catch (e) { alert(`Close partial failed: ${e}`); }
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
  document.getElementById("btn-open-mg").addEventListener("click", async () => {
    if (!currentSymbol) return;
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

      await invoke("open_martingale_hedge", { symbol: currentSymbol, direction });
      await invoke("set_martingale_mode", { mode: direction });
      document.getElementById("btn-martingale").textContent = `MG: ${direction.toUpperCase()}`;
      updateDashboard();
    } catch (e) { alert(`Open MG failed: ${e}`); }
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
    }
  });
}

// ── Credential Storage ──────────────────────────────────────

const STORAGE_KEY = "typhoon_accounts";

function loadSavedAccounts() {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) || "[]");
  } catch { return []; }
}

function saveAccounts(accounts) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(accounts));
}

function populateAccountDropdown() {
  const select = document.getElementById("saved-accounts");
  const accounts = loadSavedAccounts();
  // Keep the "New Account" option, remove others
  while (select.options.length > 1) select.remove(1);
  for (const acct of accounts) {
    const opt = document.createElement("option");
    opt.value = acct.name;
    opt.textContent = `${acct.name} (${acct.type})`;
    select.appendChild(opt);
  }
}

function fillFormFromAccount(name) {
  const accounts = loadSavedAccounts();
  const acct = accounts.find(a => a.name === name);
  if (acct) {
    document.getElementById("account-name").value = acct.name;
    document.getElementById("api-key").value = acct.apiKey;
    document.getElementById("secret-key").value = acct.secretKey;
    document.getElementById("account-type").value = acct.type;
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

  // When saved account selected, fill form
  document.getElementById("saved-accounts").addEventListener("change", (e) => {
    fillFormFromAccount(e.target.value);
  });

  // Delete saved account
  document.getElementById("btn-delete-account").addEventListener("click", () => {
    const select = document.getElementById("saved-accounts");
    const name = select.value;
    if (!name) return;
    if (!confirm(`Delete saved account "${name}"?`)) return;
    const accounts = loadSavedAccounts().filter(a => a.name !== name);
    saveAccounts(accounts);
    populateAccountDropdown();
    fillFormFromAccount("");
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

      // Save credentials if requested
      if (saveCredentials && accountName) {
        const accounts = loadSavedAccounts();
        const existing = accounts.findIndex(a => a.name === accountName);
        const entry = { name: accountName, apiKey, secretKey, type: accountType };
        if (existing >= 0) accounts[existing] = entry;
        else accounts.push(entry);
        saveAccounts(accounts);
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

  // Auto-connect if only one saved account
  const accounts = loadSavedAccounts();
  if (accounts.length === 1) {
    fillFormFromAccount(accounts[0].name);
    document.getElementById("saved-accounts").value = accounts[0].name;
  }
}

// ── Log Panel ───────────────────────────────────────────────

function setupIndicatorPanel() {
  const panel = document.getElementById("indicator-panel");
  const header = document.getElementById("indicator-header");

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
    document.getElementById("log-content").innerHTML = "";
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

document.addEventListener("DOMContentLoaded", () => {
  loadBarCacheFromDisk();
  initChart();
  setupPaneResizers();
  setupLogPanel();
  setupIndicatorPanel();
  setupAutocomplete();
  setupButtons();
  setupKeyboard();
  setupConnect();
  setupTabs();
});

function setupTabs() {
  // Create initial tab
  createTab();

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
