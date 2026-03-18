/**
 * TyphooN Terminal — Frontend
 *
 * Lightweight-charts candlestick rendering with draggable SL/TP lines.
 * Communicates with Rust backend via Tauri invoke().
 * Wasm indicator engine for high-performance optimization (32KB).
 *
 * Matches TyphooN EA workflow:
 * 1. Drag SL/TP lines to desired levels
 * 2. Click "Open Trade" — system calculates lots and places order
 */

// ── Wasm Indicator Engine (lazy-loaded, 32KB) ───────────────
let wasmReady = false;
let wasmModule = null;

async function loadWasm() {
  if (wasmReady) return wasmModule;
  try {
    const mod = await import("./wasm_indicators.js");
    await mod.default(); // init wasm
    wasmModule = mod;
    wasmReady = true;
    console.log("Wasm indicator engine loaded (32KB)");
    return mod;
  } catch (e) {
    console.warn("Wasm load failed, falling back to JS:", e);
    return null;
  }
}

// ── GPU Chart Engine (WebGL2, lazy-loaded, 45KB) ────────────
let gpuChartReady = false;
let gpuChartModule = null;
let gpuChartInstance = null;
let gpuAnimFrame = null;

async function loadGpuChart() {
  if (gpuChartReady) return gpuChartModule;
  try {
    const mod = await import("./gpu_charts.js");
    await mod.default(); // init wasm
    gpuChartModule = mod;
    gpuChartReady = true;
    console.log("GPU chart engine loaded (45KB WebGL2)");
    return mod;
  } catch (e) {
    console.warn("GPU chart load failed:", e);
    return null;
  }
}

// Map frontend chart type names to GPU ChartType enum values
const GPU_CHART_TYPES = {
  "gpu": 0,           // Candles (default GPU)
  "gpu-heikin": 1,    // Heikin-Ashi
  "gpu-line": 2,      // Line
  "gpu-bars": 3,      // OHLC Bars
  "gpu-renko": 4,     // Renko
};

function activateGpuChart(chartData, gpuType) {
  const canvas = document.getElementById("gpu-chart-canvas");
  const container = document.getElementById("chart-container");
  if (!canvas || !container || !gpuChartModule) return;

  canvas.style.display = "block";
  canvas.width = container.clientWidth;
  canvas.height = container.clientHeight;

  if (!gpuChartInstance) {
    gpuChartInstance = new gpuChartModule.GpuChart("gpu-chart-canvas");
  }
  gpuChartInstance.resize(canvas.width, canvas.height);

  // Set chart type before loading data
  const typeId = GPU_CHART_TYPES[gpuType] ?? 0;
  gpuChartInstance.set_chart_type(typeId);

  const flat = packBarsForWasm(chartData);
  gpuChartInstance.set_data(flat);

  // Add SMA 200 as indicator line (if enough data)
  gpuChartInstance.clear_lines();
  if (chartData.length > 200) {
    const sma = [];
    for (let i = 199; i < chartData.length; i++) {
      let sum = 0;
      for (let j = i - 199; j <= i; j++) sum += chartData[j].close;
      sma.push(sum / 200);
    }
    gpuChartInstance.add_line(new Float64Array(sma), 1.0, 0.84, 0.0, 1.0); // gold
  }

  // Render loop
  if (gpuAnimFrame) cancelAnimationFrame(gpuAnimFrame);
  function renderLoop() {
    gpuChartInstance.render();
    gpuAnimFrame = requestAnimationFrame(renderLoop);
  }
  renderLoop();

  // Mouse interaction: scroll to pan, wheel to zoom
  canvas.onwheel = (e) => {
    e.preventDefault();
    if (e.ctrlKey) {
      gpuChartInstance.zoom(e.deltaY > 0 ? 0.9 : 1.1, e.offsetX / canvas.width);
    } else {
      gpuChartInstance.scroll(e.deltaY > 0 ? 5 : -5);
    }
  };
  let dragging = false, dragStartX = 0;
  canvas.onmousedown = (e) => { dragging = true; dragStartX = e.offsetX; };
  canvas.onmousemove = (e) => {
    if (dragging) {
      const dx = e.offsetX - dragStartX;
      const barDelta = -dx / (canvas.width / gpuChartInstance.visible_bars());
      gpuChartInstance.scroll(barDelta);
      dragStartX = e.offsetX;
    }
  };
  canvas.onmouseup = () => { dragging = false; };
  canvas.onmouseleave = () => { dragging = false; };
}

function deactivateGpuChart() {
  const canvas = document.getElementById("gpu-chart-canvas");
  if (canvas) canvas.style.display = "none";
  if (gpuAnimFrame) { cancelAnimationFrame(gpuAnimFrame); gpuAnimFrame = null; }
}

/// Pack chart bars into flat f64 array for Wasm interop.
function packBarsForWasm(bars) {
  const flat = new Float64Array(bars.length * 5);
  for (let i = 0; i < bars.length; i++) {
    flat[i * 5] = bars[i].open;
    flat[i * 5 + 1] = bars[i].high;
    flat[i * 5 + 2] = bars[i].low;
    flat[i * 5 + 3] = bars[i].close;
    flat[i * 5 + 4] = bars[i].volume || 0;
  }
  return flat;
}

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

/// Silent invoke — no log on success or failure. Used for optional/expected-fail calls.
function invokeQuiet(cmd, args) {
  if (!window.__TAURI__ || !window.__TAURI__.core) return Promise.reject("Tauri not loaded");
  return window.__TAURI__.core.invoke(cmd, args);
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
let currentChartData = []; // full chartData with volume — candleSeries.data() drops volume
let chartLoadGeneration = 0; // increments on each loadChart call — stale intervals check this
let activeBrokerId = "default"; // per-broker data isolation — set on connect
let currentChartType = "gpu"; // default GPU candles, "candles" for CPU fallback
let orderPriceLines = []; // pending order visualization lines on chart
let mtfGridOrderLines = []; // position SL/TP/entry lines on MTF grid cells
let lastTradePrice = 0; // for T&S uptick/downtick detection

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
  removeSLLine(); removeTPLine(); // Clear SL/TP lines from previous symbol
  mtfData = {}; // Clear stale MTF data from previous symbol
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

let dragTabId = null;
let dragOverTabId = null;
let tabJustDragged = false;

function renderTabs() {
  const list = document.getElementById("tab-list");
  list.textContent = "";
  for (const tab of tabs) {
    const el = document.createElement("div");
    el.className = `chart-tab${tab.id === activeTabId ? " active" : ""}`;
    el.draggable = true;
    el.dataset.tabId = tab.id;

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

    el.addEventListener("click", (e) => {
      // Don't switch tab if we just finished a drag
      if (tabJustDragged) { tabJustDragged = false; return; }
      switchTab(tab.id);
    });

    // Drag to reorder
    el.addEventListener("dragstart", (e) => {
      dragTabId = tab.id;
      el.style.opacity = "0.4";
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", tab.id.toString()); // required for Firefox
    });
    el.addEventListener("dragend", () => {
      el.style.opacity = "";
      tabJustDragged = dragTabId !== null;
      dragTabId = null;
      dragOverTabId = null;
      list.querySelectorAll(".chart-tab").forEach(t => {
        t.classList.remove("drag-over-left", "drag-over-right");
      });
    });
    el.addEventListener("dragover", (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      if (dragTabId === null || dragTabId === tab.id) return;
      // Show drop indicator
      const rect = el.getBoundingClientRect();
      const midX = rect.left + rect.width / 2;
      list.querySelectorAll(".chart-tab").forEach(t => {
        t.classList.remove("drag-over-left", "drag-over-right");
      });
      if (e.clientX < midX) {
        el.classList.add("drag-over-left");
      } else {
        el.classList.add("drag-over-right");
      }
      dragOverTabId = tab.id;
    });
    el.addEventListener("drop", (e) => {
      e.preventDefault();
      if (dragTabId === null || dragTabId === tab.id) return;
      // Reorder tabs array
      const fromIdx = tabs.findIndex(t => t.id === dragTabId);
      const toIdx = tabs.findIndex(t => t.id === tab.id);
      if (fromIdx < 0 || toIdx < 0) return;
      const [moved] = tabs.splice(fromIdx, 1);
      // Insert before or after based on cursor position
      const rect = el.getBoundingClientRect();
      const midX = rect.left + rect.width / 2;
      const insertIdx = e.clientX < midX ? toIdx : toIdx + (fromIdx < toIdx ? 0 : 1);
      tabs.splice(Math.min(insertIdx, tabs.length), 0, moved);
      renderTabs();
      log(`Tab reordered: ${moved.symbol || "New"}`, "info");
    });

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

  // Lock Fisher/Volume sub-panes — no independent scrolling/zooming
  fisherChart.applyOptions({ handleScroll: false, handleScale: false });
  volumeChart.applyOptions({ handleScroll: false, handleScale: false });

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
  // Cursor-following tooltip — shows OHLCV + indicator values near the crosshair
  const dataWindow = document.createElement("div");
  dataWindow.id = "data-window";
  dataWindow.className = "data-window";
  document.getElementById("chart-container").appendChild(dataWindow);

  chart.subscribeCrosshairMove((param) => {
    if (!param.time || !param.point || param.point.x < 0) {
      dataWindow.style.display = "none";
      return;
    }
    dataWindow.style.display = "";
    // Follow cursor — offset to avoid overlapping crosshair
    const x = param.point.x + 16;
    const y = param.point.y + 16;
    const container = document.getElementById("chart-container");
    const cw = container?.clientWidth || 800;
    const ch = container?.clientHeight || 400;
    // Flip to left/above if near right/bottom edge
    dataWindow.style.left = (x + 220 > cw ? Math.max(0, param.point.x - 230) : x) + "px";
    dataWindow.style.top = (y + 150 > ch ? Math.max(0, param.point.y - 160) : y) + "px";

    const lines = [];

    // OHLCV from candle series
    const ohlc = param.seriesData.get(candleSeries);
    if (ohlc) {
      const dp = lastPrice > 100 ? 2 : lastPrice > 1 ? 4 : 6; // auto decimal places
      if (ohlc.open !== undefined) {
        lines.push(`O: ${ohlc.open.toFixed(dp)}  H: ${ohlc.high.toFixed(dp)}`);
        lines.push(`L: ${ohlc.low.toFixed(dp)}  C: ${ohlc.close.toFixed(dp)}`);
      } else if (ohlc.value !== undefined) {
        lines.push(`Price: ${ohlc.value.toFixed(dp)}`);
      }
      const bar = currentChartData.find(d => d.time === param.time);
      if (bar && bar.volume) lines.push(`Vol: ${bar.volume.toLocaleString()}`);
    }

    // Active indicator values (limit to 8 to avoid overflow)
    let indCount = 0;
    for (const [key, series] of Object.entries(indicatorSeries)) {
      if (indCount >= 8) break;
      const data = param.seriesData.get(series);
      if (data && data.value !== undefined) {
        const label = key.replace(/_\d+$/,"").replace(/_/g," ").substring(0,12);
        lines.push(`${label}: ${data.value.toFixed(2)}`);
        indCount++;
      }
    }

    // Fisher pane values
    for (const [key, series] of Object.entries(fisherSeries)) {
      const data = param.seriesData.get(series);
      if (data && data.value !== undefined) {
        lines.push(`Fisher: ${data.value.toFixed(2)}`);
        break; // just show one fisher value
      }
    }

    dataWindow.textContent = lines.join("\n");
  });
}

// ── Chart Type Switching ─────────────────────────────────────

function rebuildMainSeries(chartType) {
  // Save existing SL/TP line prices
  const slPrice = slLine ? slLine.options().price : null;
  const tpPrice = tpLine ? tpLine.options().price : null;

  // Remove old series (this also removes its price lines)
  if (candleSeries) {
    slLine = null;
    tpLine = null;
    chart.removeSeries(candleSeries);
  }

  // Create new series based on type
  if (chartType === "line") {
    candleSeries = chart.addLineSeries({
      color: "#2196f3",
      lineWidth: 2,
    });
  } else if (chartType === "bars") {
    candleSeries = chart.addBarSeries({
      upColor: "#00ff00",
      downColor: "#ff0000",
    });
  } else if (chartType === "heikin-ashi") {
    candleSeries = chart.addCandlestickSeries({
      upColor: "#00e676",
      downColor: "#ff1744",
      borderDownColor: "#ff1744",
      borderUpColor: "#00e676",
      wickDownColor: "#ff1744",
      wickUpColor: "#00e676",
    });
  } else if (chartType === "renko") {
    candleSeries = chart.addCandlestickSeries({
      upColor: "#00e676",
      downColor: "#ff1744",
      borderDownColor: "#00e676",
      borderUpColor: "#00e676",
      wickDownColor: "#ff1744",
      wickUpColor: "#00e676",
    });
  } else {
    // Default: candlestick
    candleSeries = chart.addCandlestickSeries({
      upColor: "#00ff00",
      downColor: "#ff0000",
      borderDownColor: "#ff0000",
      borderUpColor: "#00ff00",
      wickDownColor: "#ff0000",
      wickUpColor: "#00ff00",
    });
  }

  currentChartType = chartType;

  // Re-set data if available
  if (currentChartData && currentChartData.length > 0) {
    if (chartType === "line") {
      candleSeries.setData(currentChartData.map(d => ({ time: d.time, value: d.close })));
    } else if (chartType === "heikin-ashi") {
      candleSeries.setData(calcHeikinAshi(currentChartData));
    } else if (chartType === "renko") {
      const brickSize = getRenkoBrickSize(currentChartData);
      const renkoBricks = calcRenko(currentChartData, brickSize);
      if (renkoBricks.length > 0) {
        candleSeries.setData(renkoBricks);
      }
    } else if (chartType.startsWith("gpu")) {
      // GPU chart: use WebGL2 renderer (all chart types)
      deactivateGpuChart();
      loadGpuChart().then(() => {
        if (gpuChartModule && currentChartData.length > 0) {
          activateGpuChart(currentChartData, chartType);
        }
      });
    } else {
      candleSeries.setData(currentChartData);
    }
  }

  // Deactivate GPU chart if switching away from it
  if (!chartType.startsWith("gpu")) deactivateGpuChart();

  // Restore SL/TP lines
  if (slPrice) createSLLine(slPrice);
  if (tpPrice) createTPLine(tpPrice);

  // Re-apply order price lines
  updateOrderPriceLines();
}

// ── SL/TP Lines ─────────────────────────────────────────────

// Get the active candle series (main chart or MTF grid cell)
function getActiveCandleSeries() {
  if (mtfGridActive && mtfActiveCell) return mtfActiveCell.candleSeries;
  return candleSeries;
}

function createSLLine(price) {
  removeSLLine();
  slLine = getActiveCandleSeries().createPriceLine({
    price,
    color: "#f44336",
    lineWidth: 2,
    lineStyle: 0,
    axisLabelVisible: true,
    title: "SL",
  });
  if (currentSymbol) invoke("set_sl_level", { symbol: currentSymbol, price }).catch(() => {});
  // Sync input field
  const slInp = document.getElementById("sl-input");
  if (slInp) slInp.value = price.toFixed(2);
  updateRiskRewardOverlay();
  updateRiskCalcPanel();
}

function createTPLine(price) {
  removeTPLine();
  tpLine = getActiveCandleSeries().createPriceLine({
    price,
    color: "#4caf50",
    lineWidth: 2,
    lineStyle: 0,
    axisLabelVisible: true,
    title: "TP",
  });
  if (currentSymbol) invoke("set_tp_level", { symbol: currentSymbol, price }).catch(() => {});
  // Sync input field
  const tpInp = document.getElementById("tp-input");
  if (tpInp) tpInp.value = price.toFixed(2);
  updateRiskRewardOverlay();
}

function removeSLLine() {
  if (slLine) { try { getActiveCandleSeries().removePriceLine(slLine); } catch (_) { try { candleSeries.removePriceLine(slLine); } catch (_) {} } slLine = null; }
}
function removeTPLine() {
  if (tpLine) { try { getActiveCandleSeries().removePriceLine(tpLine); } catch (_) { try { candleSeries.removePriceLine(tpLine); } catch (_) {} } tpLine = null; }
}
function getSLPrice() { return slLine ? slLine.options().price : null; }
function getTPPrice() { return tpLine ? tpLine.options().price : null; }

/// Place a protective stop or limit order on an existing position.
/// Called when "Set SL" / "Set TP" is clicked or from the warning banner.
async function placeProtectiveOrder(symbol, type, price) {
  try {
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);
    const symNoSlash = symbol.replace("/", "");
    const pos = positions.find(p => p.symbol === symbol || p.symbol === symNoSlash);
    if (!pos) return; // no position to protect

    const qty = Math.abs(pos.qty);
    const oppSide = pos.side === "long" ? "sell" : "buy";

    if (type === "sl") {
      await invoke("place_stop_order", {
        symbol, qty, side: oppSide, stopPrice: price, tif: "gtc",
      });
      log(`Protective SL placed: ${oppSide} ${qty} @ ${price}`, "ok");
    } else if (type === "tp") {
      await invoke("place_limit_order", {
        symbol, qty, side: oppSide, limitPrice: price, tif: "gtc",
      });
      log(`Protective TP placed: ${oppSide} ${qty} @ ${price}`, "ok");
    }
  } catch (e) {
    log(`Protective order failed: ${e}`, "error");
    alert(`Failed to place protective ${type.toUpperCase()}: ${e}`);
  }
}

// ── Draggable SL/TP Lines (MT5-style) ──────────────────────
// Double-click near a line to grab it, drag to new price, release to set.
// Uses chart price scale coordinateToPrice/priceToCoordinate for pixel↔price.

let draggingLine = null; // "sl" | "tp" | null

function setupLineDrag() {
  const HIT_TOLERANCE = 14; // pixels — generous for easy grabbing

  // Drag info tooltip
  const dragInfo = document.createElement("div");
  dragInfo.id = "drag-info";
  dragInfo.style.cssText = "position:fixed;z-index:100;background:#000e;border:1px solid #4caf50;padding:4px 8px;font-size:10px;font-family:Consolas,monospace;color:#ccc;border-radius:3px;pointer-events:none;display:none;white-space:pre;";
  document.body.appendChild(dragInfo);

  function getActiveContainer() {
    if (mtfGridActive && mtfActiveCell) return mtfActiveCell.chartDiv;
    return document.getElementById("chart-container");
  }

  function getActiveSeries() {
    return getActiveCandleSeries();
  }

  function getLineYCoord(line) {
    if (!line) return null;
    try { return getActiveSeries().priceToCoordinate(line.options().price); } catch (_) { return null; }
  }

  function hitTestLine(clientY) {
    const container = getActiveContainer();
    if (!container) return null;
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

  function calcDragInfo(sl, tp) {
    if (!sl || !tp || !lastPrice || lastPrice <= 0) return "";
    const isBuy = tp > sl;
    const slDist = isBuy ? lastPrice - sl : sl - lastPrice;
    const tpDist = isBuy ? tp - lastPrice : lastPrice - tp;
    const rr = slDist > 0 ? (tpDist / slDist).toFixed(2) : "—";
    const riskPct = lastPrice > 0 ? ((slDist / lastPrice) * 100).toFixed(2) : "0";
    const lines = [
      `${isBuy ? "BUY" : "SELL"} @ $${lastPrice.toFixed(2)}`,
      `SL: $${sl.toFixed(2)} (${riskPct}%)`,
      `TP: $${tp.toFixed(2)}`,
      `R:R = ${rr}`,
      `SL dist: $${slDist.toFixed(2)}`,
      `TP dist: $${tpDist.toFixed(2)}`,
    ];
    return lines.join("\n");
  }

  // Single mousedown near line starts drag — capture phase to beat lightweight-charts
  // Must stopPropagation to prevent chart pan/scroll when dragging SL/TP
  window._onDragMouseDown = function onDragMouseDown(e) {
    if (e.button !== 0) return;
    const hit = hitTestLine(e.clientY);
    if (!hit) return;
    draggingLine = hit;
    const container = getActiveContainer();
    if (container) container.style.cursor = "ns-resize";
    e.preventDefault();
    e.stopPropagation(); // prevent chart from panning
  }
  // Attach to chart container in capture phase (fires before lightweight-charts)
  document.getElementById("chart-container").addEventListener("mousedown", window._onDragMouseDown, true);

  document.addEventListener("mousemove", (e) => {
    const container = getActiveContainer();
    if (!draggingLine) {
      // Show resize cursor when hovering near a line
      const hit = hitTestLine(e.clientY);
      if (container) container.style.cursor = hit ? "ns-resize" : "";
      return;
    }
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const y = e.clientY - rect.top;
    try {
      const newPrice = getActiveSeries().coordinateToPrice(y);
      if (newPrice === null || newPrice <= 0) return;
      const line = draggingLine === "sl" ? slLine : tpLine;
      if (line) line.applyOptions({ price: newPrice });

      // Show live risk info tooltip near cursor
      const sl = getSLPrice();
      const tp = getTPPrice();
      const info = calcDragInfo(sl, tp);
      if (info) {
        dragInfo.textContent = info;
        dragInfo.style.display = "";
        dragInfo.style.left = (e.clientX + 20) + "px";
        dragInfo.style.top = (e.clientY - 40) + "px";
      }
    } catch (_) {}
  });

  document.addEventListener("mouseup", () => {
    if (!draggingLine) return;
    const line = draggingLine === "sl" ? slLine : tpLine;
    if (line && currentSymbol) {
      const finalPrice = line.options().price;
      if (draggingLine === "sl") {
        invoke("set_sl_level", { symbol: currentSymbol, price: finalPrice }).catch(() => {});
        const slInp = document.getElementById("sl-input");
        if (slInp) slInp.value = finalPrice.toFixed(2);
        log(`SL → $${finalPrice.toFixed(4)}`, "info");
      } else {
        invoke("set_tp_level", { symbol: currentSymbol, price: finalPrice }).catch(() => {});
        const tpInp = document.getElementById("tp-input");
        if (tpInp) tpInp.value = finalPrice.toFixed(2);
        log(`TP → $${finalPrice.toFixed(4)}`, "info");
      }
      updateRiskRewardOverlay();
      updateRiskCalcPanel();
    }
    draggingLine = null;
    dragInfo.style.display = "none";
    const container = getActiveContainer();
    if (container) container.style.cursor = "";
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
// Per-broker localStorage key — drawings are symbol-specific, so broker-scoped
function getDrawingsKey() { return `typhoon_drawings_${activeBrokerId}`; }

function loadDrawings() {
  try { drawings = JSON.parse(localStorage.getItem(getDrawingsKey()) || "[]"); } catch { drawings = []; }
}
function saveDrawings() { localStorage.setItem(getDrawingsKey(), JSON.stringify(drawings)); }

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

// ── Stochastic Oscillator ────────────────────────────────────
function calcStochastic(data, kPeriod = 14, dPeriod = 3, smooth = 3) {
  const result = { k: [], d: [] };
  if (data.length < kPeriod + smooth + dPeriod) return result;
  const rawK = [];
  for (let i = kPeriod - 1; i < data.length; i++) {
    let hi = -Infinity, lo = Infinity;
    for (let j = i - kPeriod + 1; j <= i; j++) { hi = Math.max(hi, data[j].high); lo = Math.min(lo, data[j].low); }
    rawK.push({ time: data[i].time, value: hi !== lo ? ((data[i].close - lo) / (hi - lo)) * 100 : 50 });
  }
  // Smooth %K
  const kSmooth = [];
  for (let i = smooth - 1; i < rawK.length; i++) {
    let sum = 0;
    for (let j = 0; j < smooth; j++) sum += rawK[i - j].value;
    kSmooth.push({ time: rawK[i].time, value: sum / smooth });
  }
  // %D = SMA of %K
  for (let i = dPeriod - 1; i < kSmooth.length; i++) {
    let sum = 0;
    for (let j = 0; j < dPeriod; j++) sum += kSmooth[i - j].value;
    result.d.push({ time: kSmooth[i].time, value: sum / dPeriod });
  }
  result.k = kSmooth;
  return result;
}

// ── CCI (Commodity Channel Index) ────────────────────────────
function calcCCI(data, period = 20) {
  const result = [];
  if (data.length < period) return result;
  for (let i = period - 1; i < data.length; i++) {
    let sum = 0;
    for (let j = i - period + 1; j <= i; j++) sum += (data[j].high + data[j].low + data[j].close) / 3;
    const mean = sum / period;
    let devSum = 0;
    for (let j = i - period + 1; j <= i; j++) devSum += Math.abs((data[j].high + data[j].low + data[j].close) / 3 - mean);
    const meanDev = devSum / period;
    const tp = (data[i].high + data[i].low + data[i].close) / 3;
    result.push({ time: data[i].time, value: meanDev !== 0 ? (tp - mean) / (0.015 * meanDev) : 0 });
  }
  return result;
}

// ── ADX (Average Directional Index) ──────────────────────────
function calcADX(data, period = 14) {
  const result = { adx: [], diPlus: [], diMinus: [] };
  if (data.length < period * 2 + 1) return result;
  const tr = [], dmPlus = [], dmMinus = [];
  for (let i = 1; i < data.length; i++) {
    tr.push(Math.max(data[i].high - data[i].low, Math.abs(data[i].high - data[i-1].close), Math.abs(data[i].low - data[i-1].close)));
    const upMove = data[i].high - data[i-1].high;
    const downMove = data[i-1].low - data[i].low;
    dmPlus.push(upMove > downMove && upMove > 0 ? upMove : 0);
    dmMinus.push(downMove > upMove && downMove > 0 ? downMove : 0);
  }
  let atr = tr.slice(0, period).reduce((a,b) => a+b, 0) / period;
  let sdmp = dmPlus.slice(0, period).reduce((a,b) => a+b, 0) / period;
  let sdmm = dmMinus.slice(0, period).reduce((a,b) => a+b, 0) / period;
  const dxArr = [];
  for (let i = period; i < tr.length; i++) {
    atr = (atr * (period-1) + tr[i]) / period;
    sdmp = (sdmp * (period-1) + dmPlus[i]) / period;
    sdmm = (sdmm * (period-1) + dmMinus[i]) / period;
    const dip = atr > 0 ? (sdmp / atr) * 100 : 0;
    const dim = atr > 0 ? (sdmm / atr) * 100 : 0;
    const dx = (dip + dim) > 0 ? Math.abs(dip - dim) / (dip + dim) * 100 : 0;
    dxArr.push(dx);
    result.diPlus.push({ time: data[i+1].time, value: dip });
    result.diMinus.push({ time: data[i+1].time, value: dim });
  }
  // ADX = smoothed DX
  if (dxArr.length >= period) {
    let adx = dxArr.slice(0, period).reduce((a,b) => a+b, 0) / period;
    for (let i = period; i < dxArr.length; i++) {
      adx = (adx * (period-1) + dxArr[i]) / period;
      result.adx.push({ time: result.diPlus[i].time, value: adx });
    }
  }
  return result;
}

// ── Williams %R ──────────────────────────────────────────────
function calcWilliamsR(data, period = 14) {
  const result = [];
  if (data.length < period) return result;
  for (let i = period - 1; i < data.length; i++) {
    let hi = -Infinity, lo = Infinity;
    for (let j = i - period + 1; j <= i; j++) { hi = Math.max(hi, data[j].high); lo = Math.min(lo, data[j].low); }
    result.push({ time: data[i].time, value: hi !== lo ? ((hi - data[i].close) / (hi - lo)) * -100 : -50 });
  }
  return result;
}

// ── Ichimoku Cloud ───────────────────────────────────────────
function calcIchimoku(data, tenkan = 9, kijun = 26, senkou = 52) {
  const result = { tenkanSen: [], kijunSen: [], senkouA: [], senkouB: [], chikou: [] };
  if (data.length < senkou) return result;
  const midHL = (start, len) => {
    let hi = -Infinity, lo = Infinity;
    for (let i = start; i < start + len && i < data.length; i++) { hi = Math.max(hi, data[i].high); lo = Math.min(lo, data[i].low); }
    return (hi + lo) / 2;
  };
  for (let i = senkou - 1; i < data.length; i++) {
    const ts = midHL(i - tenkan + 1, tenkan);
    const ks = midHL(i - kijun + 1, kijun);
    const sb = midHL(i - senkou + 1, senkou);
    result.tenkanSen.push({ time: data[i].time, value: ts });
    result.kijunSen.push({ time: data[i].time, value: ks });
    result.senkouA.push({ time: data[i].time, value: (ts + ks) / 2 });
    result.senkouB.push({ time: data[i].time, value: sb });
    if (i >= kijun) result.chikou.push({ time: data[i].time, value: data[i - kijun].close });
  }
  return result;
}

// ── Parabolic SAR ────────────────────────────────────────────
function calcParabolicSAR(data, step = 0.02, maxStep = 0.2) {
  const result = [];
  if (data.length < 2) return result;
  let isLong = data[1].close > data[0].close;
  let sar = isLong ? data[0].low : data[0].high;
  let ep = isLong ? data[1].high : data[1].low;
  let af = step;
  for (let i = 1; i < data.length; i++) {
    sar = sar + af * (ep - sar);
    if (isLong) {
      if (i >= 2) sar = Math.min(sar, data[i-1].low, data[i-2] ? data[i-2].low : data[i-1].low);
      if (data[i].low < sar) { isLong = false; sar = ep; ep = data[i].low; af = step; }
      else if (data[i].high > ep) { ep = data[i].high; af = Math.min(af + step, maxStep); }
    } else {
      if (i >= 2) sar = Math.max(sar, data[i-1].high, data[i-2] ? data[i-2].high : data[i-1].high);
      if (data[i].high > sar) { isLong = true; sar = ep; ep = data[i].high; af = step; }
      else if (data[i].low < ep) { ep = data[i].low; af = Math.min(af + step, maxStep); }
    }
    result.push({ time: data[i].time, value: sar, color: isLong ? "#4caf50" : "#f44336" });
  }
  return result;
}

// ── OBV (On-Balance Volume) ──────────────────────────────────
function calcOBV(data) {
  const result = [];
  let obv = 0;
  for (let i = 0; i < data.length; i++) {
    if (i > 0) {
      if (data[i].close > data[i-1].close) obv += (data[i].volume || 0);
      else if (data[i].close < data[i-1].close) obv -= (data[i].volume || 0);
    }
    result.push({ time: data[i].time, value: obv });
  }
  return result;
}

// ── Momentum ─────────────────────────────────────────────────
function calcMomentum(data, period = 10) {
  const result = [];
  for (let i = period; i < data.length; i++) {
    result.push({ time: data[i].time, value: data[i].close - data[i - period].close });
  }
  return result;
}

// ── WMA (Weighted Moving Average) ────────────────────────────
function calcWMA(data, period = 20) {
  const result = [];
  if (data.length < period) return result;
  const denom = (period * (period + 1)) / 2;
  for (let i = period - 1; i < data.length; i++) {
    let sum = 0;
    for (let j = 0; j < period; j++) sum += data[i - period + 1 + j].close * (j + 1);
    result.push({ time: data[i].time, value: sum / denom });
  }
  return result;
}

// ── HMA (Hull Moving Average) ────────────────────────────────
function calcHMA(data, period = 20) {
  if (data.length < period) return [];
  const half = Math.floor(period / 2);
  const sqrtP = Math.floor(Math.sqrt(period));
  const wmaHalf = calcWMA(data, half);
  const wmaFull = calcWMA(data, period);
  if (wmaHalf.length === 0 || wmaFull.length === 0) return [];
  // 2×WMA(half) - WMA(full)
  const diff = [];
  const offset = wmaHalf.length - wmaFull.length;
  for (let i = 0; i < wmaFull.length; i++) {
    diff.push({ time: wmaFull[i].time, close: 2 * wmaHalf[i + offset].value - wmaFull[i].value });
  }
  const hma = calcWMA(diff.map(d => ({ ...d, high: d.close, low: d.close, open: d.close })), sqrtP);
  return hma;
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

// ── Auto Fibonacci (fractal-based swing detection) ──────────
// Finds the most significant swing high/low and draws retracement + extension levels.
// Bull scenario: swing low → swing high. Bear scenario: swing high → swing low.
function calcAutoFibonacci(data, fractalLookback = 10) {
  if (data.length < fractalLookback * 2 + 10) return null;

  // Find all fractal highs and lows
  const swingHighs = [];
  const swingLows = [];

  for (let i = fractalLookback; i < data.length - fractalLookback; i++) {
    let isHigh = true, isLow = true;
    for (let j = 1; j <= fractalLookback; j++) {
      if (data[i - j].high >= data[i].high || data[i + j].high >= data[i].high) isHigh = false;
      if (data[i - j].low <= data[i].low || data[i + j].low <= data[i].low) isLow = false;
    }
    if (isHigh) swingHighs.push({ idx: i, price: data[i].high, time: data[i].time });
    if (isLow) swingLows.push({ idx: i, price: data[i].low, time: data[i].time });
  }

  if (swingHighs.length === 0 || swingLows.length === 0) return null;

  // Find the most significant recent swing pair:
  // Use the highest high and lowest low from the recent portion of the chart
  const recentStart = Math.max(0, data.length - Math.floor(data.length * 0.6));
  const recentHighs = swingHighs.filter(h => h.idx >= recentStart);
  const recentLows = swingLows.filter(l => l.idx >= recentStart);

  if (recentHighs.length === 0 || recentLows.length === 0) return null;

  const swingHigh = recentHighs.reduce((a, b) => b.price > a.price ? b : a);
  const swingLow = recentLows.reduce((a, b) => b.price < a.price ? b : a);

  // Determine bull or bear based on which came first
  const isBull = swingLow.idx < swingHigh.idx; // low first = uptrend = bull fib

  const high = swingHigh.price;
  const low = swingLow.price;
  const range = high - low;
  if (range <= 0) return null;

  // Retracement levels (from the end of the move back toward start)
  // Extension levels (beyond the move)
  const levels = [
    { ratio: 0.0,   label: "0%",       type: "retrace" },
    { ratio: 0.236, label: "23.6%",    type: "retrace" },
    { ratio: 0.382, label: "38.2%",    type: "retrace" },
    { ratio: 0.5,   label: "50%",      type: "retrace" },
    { ratio: 0.618, label: "61.8%",    type: "retrace" },
    { ratio: 0.786, label: "78.6%",    type: "retrace" },
    { ratio: 1.0,   label: "100%",     type: "retrace" },
    { ratio: 1.272, label: "127.2%",   type: "extension" },
    { ratio: 1.618, label: "161.8%",   type: "extension" },
    { ratio: 2.0,   label: "200%",     type: "extension" },
    { ratio: 2.618, label: "261.8%",   type: "extension" },
    { ratio: 3.618, label: "361.8%",   type: "extension" },
    { ratio: 4.236, label: "423.6%",   type: "extension" },
  ];

  const result = [];
  for (const level of levels) {
    let price;
    if (isBull) {
      // Bull: retrace from high toward low; extend above high
      price = high - range * level.ratio;
      if (level.type === "extension" && level.ratio > 1.0) {
        price = low + range * level.ratio; // extension above swing high
      }
    } else {
      // Bear: retrace from low toward high; extend below low
      price = low + range * level.ratio;
      if (level.type === "extension" && level.ratio > 1.0) {
        price = high - range * level.ratio; // extension below swing low
      }
    }
    result.push({
      price,
      label: level.label,
      type: level.type,
      ratio: level.ratio,
    });
  }

  const startTime = Math.min(swingHigh.time, swingLow.time);

  return {
    isBull,
    swingHigh: { price: high, time: swingHigh.time },
    swingLow: { price: low, time: swingLow.time },
    levels: result,
    startTime,
  };
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

// Resolve rank for custom timeframes (e.g., "2Day" → 1Day rank + 0.5, "3Hour" → 1Hour rank + 0.5)
function getTFRank(tf) {
  if (TF_RANK[tf] !== undefined) return TF_RANK[tf];
  // Parse custom TF string to find base: "2Day" → "1Day", "3Hour" → "1Hour", "10Min" → "1Min"
  const m = tf.match(/^(\d+)(Min|Hour|Day|Week|Month)$/);
  if (m) {
    const baseKey = "1" + m[2];
    const baseRank = TF_RANK[baseKey];
    if (baseRank !== undefined) return baseRank + 0.5; // between base and next standard TF
  }
  return 3; // fallback
}

function getRelevantMTFs(allTFs) {
  const currentRank = getTFRank(currentTimeframe);
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

// Auto-generate TF labels for all supported timeframes
const MTF_LABELS = (() => {
  const m = {};
  for (let i of [1,5,10,15,20,30,40,45,50,55]) m[`${i}Min`] = `M${i}`;
  for (let i = 1; i <= 23; i++) m[`${i}Hour`] = `H${i}`;
  for (let i = 1; i <= 13; i++) m[`${i}Day`] = `D${i}`;
  for (let i = 1; i <= 3; i++) m[`${i}Week`] = `W${i}`;
  for (let i = 1; i <= 11; i++) m[`${i}Month`] = `MN${i}`;
  for (let i = 1; i <= 33; i++) m[`${i}Year`] = `Y${i}`;
  return m;
})();

async function loadMTFData(symbol) {
  try {
    const json = await invoke("get_multi_tf_bars", {
      symbol,
      timeframes: MTF_TIMEFRAMES,
      limit: 500,
    });
    // Guard: discard if symbol changed during async fetch
    if (currentSymbol !== symbol) {
      log(`MTF data discarded (symbol changed: ${symbol} → ${currentSymbol})`, "warn");
      return;
    }
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
  // Helper: add a simple line series overlay
  const addLine = (color, width, data, seriesKey) => {
    if (!data || data.length < 2) return;
    const s = chart.addLineSeries({ color, lineWidth: width, lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false });
    s.setData(data);
    if (seriesKey) indicatorSeries[seriesKey] = s;
  };

  for (const cb of checkboxes) {
    const ind = cb.dataset.ind;
    const period = parseInt(cb.dataset.period) || 14;
    const key = `${ind}_${period}`;

    // Isolate each indicator — one failure must not break others
    try {

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
      // PreviousCandleLevels.mqh: multi-TF previous high/low (horizontal lines only, no current-TF per-bar)
      // MQL5 TF filtering: chart < H1 shows all HTF; H1-H4 shows D1+ only; D1 shows W1+ only; W1+ nothing
      const currentRank = getTFRank(currentTimeframe);
      const pclTFs = ALL_MTF_PCL_TFS.filter(tf => {
        const tfRank = TF_RANK[tf] ?? 0;
        if (currentRank < TF_RANK["1Hour"]) return tfRank > currentRank; // chart < H1: show all higher
        if (currentRank <= TF_RANK["4Hour"]) return tfRank >= TF_RANK["1Day"]; // chart H1-H4: D1+ only
        if (currentRank >= TF_RANK["1Day"] && currentRank < TF_RANK["1Week"]) return tfRank >= TF_RANK["1Week"]; // chart D1-D6: W1+ only
        return false; // W1+ chart: nothing in our set
      });
      // HTF previous candle levels — solid lines from HTF bar start to last candle
      for (const tf of pclTFs) {
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
      // Current chart's ATR projection — horizontal lines at last bar's open ± ATR (matches MQL5)
      if (chartData.length > period + 1) {
        const atrp = calcATRProjection(chartData, period);
        if (atrp.atrValues.length > 0) {
          const lastATR = atrp.atrValues[atrp.atrValues.length - 1].value;
          const lastOpen = chartData[chartData.length - 1].open;
          const upper = lastOpen + lastATR;
          const lower = lastOpen - lastATR;
          // Span from ~30 bars ago to last bar (matching MQL5 lookback)
          const startIdx = Math.max(0, chartData.length - 30);
          const levelBars = clip(chartData.slice(startIdx));
          if (levelBars.length >= 2) {
            const su = chart.addLineSeries({ color: "#FFFF00", lineWidth: 2, lineStyle: 1, title: "", lastValueVisible: false, priceLineVisible: false });
            const sl3 = chart.addLineSeries({ color: "#FFFF00", lineWidth: 2, lineStyle: 1, title: "", lastValueVisible: false, priceLineVisible: false });
            su.setData(levelBars.map(d => ({ time: d.time, value: upper })));
            sl3.setData(levelBars.map(d => ({ time: d.time, value: lower })));
            indicatorSeries[key + "_u"] = su;
            indicatorSeries[key + "_l"] = sl3;
          }
        }
      }
      // HTF ATR projections — dotted yellow lines, span from several HTF bars back (matches MQL5 lookbacks)
      const HTF_ATR_LOOKBACK = { "1Hour": 12, "4Hour": 11, "1Day": 7, "1Week": 4 };
      for (const tf of getRelevantMTFs(ALL_MTF_ATR_TFS)) {
        const tfBars = mtfData[tf];
        const proj = calcHTFATRProjection(tfBars, period);
        if (!proj) continue;
        // Start from several HTF bars back to match MQL5 g_startTime lookbacks
        const lookbackN = HTF_ATR_LOOKBACK[tf] || 7;
        const startIdx = Math.max(0, tfBars.length - lookbackN);
        const htfStart = tfBars[startIdx].time;
        const projBars = clip(chartData.filter(d => d.time >= htfStart));
        if (projBars.length < 2) continue;
        const sU = chart.addLineSeries({ color: "#FFFF00", lineWidth: 2, lineStyle: 1, title: "", lastValueVisible: false, priceLineVisible: false });
        const sL = chart.addLineSeries({ color: "#FFFF00", lineWidth: 2, lineStyle: 1, title: "", lastValueVisible: false, priceLineVisible: false });
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

      // Ensure last segment always has >= 2 points for rendering (fixes last bar color change)
      if (segments.length >= 2) {
        const lastSeg = segments[segments.length - 1];
        if (lastSeg.data.length < 2) {
          const prevSeg = segments[segments.length - 2];
          lastSeg.data.unshift(prevSeg.data[prevSeg.data.length - 1]);
        }
      }

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
      // Sync Fisher time scale — use fitContent then match main chart range
      fisherChart.timeScale().fitContent();
      try { fisherChart.timeScale().setVisibleLogicalRange(chart.timeScale().getVisibleLogicalRange()); } catch (_) {}

    } else if (ind === "better-vol" && chartData.length > 2) {
      // BetterVolume — rendered in dedicated volumeChart pane
      const bvData = calcBetterVolume(chartData);
      const s = volumeChart.addHistogramSeries({
        priceFormat: { type: "volume" },
      });
      s.setData(bvData);
      volumeSeries.hist = s;
      volumeChart.timeScale().fitContent();
      try { volumeChart.timeScale().setVisibleLogicalRange(chart.timeScale().getVisibleLogicalRange()); } catch (_) {}

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

    } else if (ind === "auto-fib" && chartData.length > 30) {
      // Auto Fibonacci: fractal-based swing detection with retracement + extension levels
      const fib = calcAutoFibonacci(chartData);
      if (fib) {
        const fibBars = clip(chartData.filter(d => d.time >= fib.startTime));
        if (fibBars.length >= 2) {
          // Retracement colors (warm → cool gradient)
          const retraceColors = {
            "0%": "#f44336", "23.6%": "#ff9800", "38.2%": "#ffeb3b",
            "50%": "#8bc34a", "61.8%": "#00bcd4", "78.6%": "#3f51b5", "100%": "#9c27b0",
          };
          // Extension colors (distinct from retracements)
          const extColors = {
            "127.2%": "#e91e63", "161.8%": "#ff5722", "200%": "#ff9800",
            "261.8%": "#ffc107", "361.8%": "#cddc39", "423.6%": "#8bc34a",
          };

          for (let li = 0; li < fib.levels.length; li++) {
            const level = fib.levels[li];
            const color = level.type === "extension"
              ? (extColors[level.label] || "#888")
              : (retraceColors[level.label] || "#888");
            const lineStyle = level.type === "extension" ? 2 : 0; // dashed for extensions
            const lineWidth = level.label === "50%" || level.label === "61.8%" ? 1.5 : 0.8;

            const s = chart.addLineSeries({
              color,
              lineWidth,
              lineStyle,
              lastValueVisible: false,
              priceLineVisible: false,
              crosshairMarkerVisible: false,
              title: "",
            });
            s.setData(fibBars.map(d => ({ time: d.time, value: level.price })));
            indicatorSeries[`afib_${li}`] = s;
          }

          // Draw the swing line (connecting swing low to swing high)
          const swingLine = chart.addLineSeries({
            color: fib.isBull ? "#4caf50" : "#f44336",
            lineWidth: 1,
            lineStyle: 2,
            lastValueVisible: false,
            priceLineVisible: false,
            crosshairMarkerVisible: false,
          });
          swingLine.setData([
            { time: Math.min(fib.swingLow.time, fib.swingHigh.time), value: fib.isBull ? fib.swingLow.price : fib.swingHigh.price },
            { time: Math.max(fib.swingLow.time, fib.swingHigh.time), value: fib.isBull ? fib.swingHigh.price : fib.swingLow.price },
          ]);
          indicatorSeries.afib_swing = swingLine;

          const direction = fib.isBull ? "BULL" : "BEAR";
          log(`Auto-Fib: ${direction} ${fib.swingLow.price.toFixed(2)} → ${fib.swingHigh.price.toFixed(2)} (${fib.levels.length} levels)`, "ok");
        }
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

    } else if (ind === "stochastic" && chartData.length > period + 10) {
      const st = calcStochastic(chartData, period, 3, 3);
      if (st.k.length > 0) {
        const sk = chart.addLineSeries({ color: "#2196f3", lineWidth: 1, priceScaleId: "stoch", lastValueVisible: false, priceLineVisible: false });
        const sd = chart.addLineSeries({ color: "#ff9800", lineWidth: 1, priceScaleId: "stoch", lastValueVisible: false, priceLineVisible: false });
        chart.priceScale("stoch").applyOptions({ scaleMargins: { top: 0.8, bottom: 0 } });
        sk.setData(clip(st.k)); sd.setData(clip(st.d));
        indicatorSeries[key + "_k"] = sk; indicatorSeries[key + "_d"] = sd;
      }

    } else if (ind === "cci" && chartData.length > period) {
      const cci = calcCCI(chartData, period);
      const s = chart.addLineSeries({ color: "#9c27b0", lineWidth: 1, priceScaleId: "cci", lastValueVisible: false, priceLineVisible: false });
      chart.priceScale("cci").applyOptions({ scaleMargins: { top: 0.8, bottom: 0 } });
      s.setData(clip(cci)); indicatorSeries[key] = s;

    } else if (ind === "adx" && chartData.length > period * 3) {
      const adx = calcADX(chartData, period);
      if (adx.adx.length > 0) {
        const sa = chart.addLineSeries({ color: "#ff9800", lineWidth: 1.5, priceScaleId: "adx", lastValueVisible: false, priceLineVisible: false });
        const sp = chart.addLineSeries({ color: "#4caf50", lineWidth: 1, priceScaleId: "adx", lastValueVisible: false, priceLineVisible: false });
        const sm = chart.addLineSeries({ color: "#f44336", lineWidth: 1, priceScaleId: "adx", lastValueVisible: false, priceLineVisible: false });
        chart.priceScale("adx").applyOptions({ scaleMargins: { top: 0.8, bottom: 0 } });
        sa.setData(clip(adx.adx)); sp.setData(clip(adx.diPlus)); sm.setData(clip(adx.diMinus));
        indicatorSeries[key + "_adx"] = sa; indicatorSeries[key + "_dip"] = sp; indicatorSeries[key + "_dim"] = sm;
      }

    } else if (ind === "williams" && chartData.length > period) {
      const wr = calcWilliamsR(chartData, period);
      const s = chart.addLineSeries({ color: "#e91e63", lineWidth: 1, priceScaleId: "wr", lastValueVisible: false, priceLineVisible: false });
      chart.priceScale("wr").applyOptions({ scaleMargins: { top: 0.8, bottom: 0 } });
      s.setData(clip(wr)); indicatorSeries[key] = s;

    } else if (ind === "ichimoku" && chartData.length > 52) {
      const ich = calcIchimoku(chartData);
      addLine("#2196f3", 1, clip(ich.tenkanSen), "ich_tenkan");
      addLine("#f44336", 1, clip(ich.kijunSen), "ich_kijun");
      addLine("#4caf50", 1, clip(ich.senkouA), "ich_sa");
      addLine("#ff9800", 1, clip(ich.senkouB), "ich_sb");
      if (ich.chikou.length > 0) addLine("#9c27b0", 1, clip(ich.chikou), "ich_chikou");
      // Cloud fill
      if (ich.senkouA.length > 1 && ich.senkouB.length > 1) {
        const fill = chart.addBaselineSeries({
          topFillColor1: "#4caf5020", topFillColor2: "#4caf5020",
          bottomFillColor1: "#ff980020", bottomFillColor2: "#ff980020",
          topLineColor: "transparent", bottomLineColor: "transparent",
          lineWidth: 0, baseValue: { type: "price", price: 0 },
          lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false,
        });
        // Use senkouB as baseline, senkouA as value
        fill.setData(clip(ich.senkouA));
        indicatorSeries.ich_cloud = fill;
      }

    } else if (ind === "psar" && chartData.length > 2) {
      const psar = calcParabolicSAR(chartData);
      const s = chart.addLineSeries({ color: "#ffeb3b", lineWidth: 0, lineStyle: 0, lastValueVisible: false, priceLineVisible: false, pointMarkersVisible: true, pointMarkersRadius: 1.5 });
      s.setData(clip(psar)); indicatorSeries[key] = s;

    } else if (ind === "obv") {
      const obv = calcOBV(chartData);
      const s = chart.addLineSeries({ color: "#00bcd4", lineWidth: 1, priceScaleId: "obv", lastValueVisible: false, priceLineVisible: false });
      chart.priceScale("obv").applyOptions({ scaleMargins: { top: 0.85, bottom: 0 } });
      s.setData(clip(obv)); indicatorSeries[key] = s;

    } else if (ind === "momentum" && chartData.length > period) {
      const mom = calcMomentum(chartData, period);
      const s = chart.addLineSeries({ color: "#ff5722", lineWidth: 1, priceScaleId: "mom", lastValueVisible: false, priceLineVisible: false });
      chart.priceScale("mom").applyOptions({ scaleMargins: { top: 0.85, bottom: 0 } });
      s.setData(clip(mom)); indicatorSeries[key] = s;

    } else if (ind === "wma" && chartData.length > period) {
      const wma = calcWMA(chartData, period);
      addLine("#ff4081", 1, clip(wma), key);

    } else if (ind === "hma" && chartData.length > period) {
      const hma = calcHMA(chartData, period);
      addLine("#00e5ff", 1.5, clip(hma), key);

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

    } catch (e) { log(`Indicator ${ind} failed: ${e.message || e}`, "warn"); }
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
// FOUR-TIER BAR CACHE (with LRU eviction)
//   Hot:  In-memory LRU (instant, 1-min TTL, max 200 entries)
//   Warm: IndexedDB (50MB+, survives restarts)
//   SQL:  SQLite via Rust (unlimited, zstd-compressed, WAL mode)
//   Cold: zstd-compressed files via Rust (legacy, persistent)
// ══════════════════════════════════════════════════════════════

const barCache = {}; // Hot: "SYMBOL:TF" → { data: [], timestamp: Date, lastAccess: Date }
const CACHE_TTL_MS = 60 * 1000; // 1 minute — fresh threshold
const CACHE_MAX_ENTRIES = 200; // LRU eviction threshold

// LRU eviction: remove least-recently-accessed entries when cache exceeds max
function evictLRU() {
  const keys = Object.keys(barCache);
  if (keys.length <= CACHE_MAX_ENTRIES) return;
  // Sort by lastAccess ascending (oldest first)
  keys.sort((a, b) => (barCache[a].lastAccess || 0) - (barCache[b].lastAccess || 0));
  const toRemove = keys.length - CACHE_MAX_ENTRIES;
  for (let i = 0; i < toRemove; i++) {
    delete barCache[keys[i]];
  }
  log(`LRU evicted ${toRemove} cache entries (${keys.length} → ${CACHE_MAX_ENTRIES})`, "info");
}
let idb = null; // Warm: IndexedDB handle

function getCacheKey(symbol, tf) { return `${symbol}:${tf}`; }

// ── IndexedDB (Warm Cache) — per-broker database ────────────

function openIndexedDB() {
  return new Promise((resolve, reject) => {
    const dbName = `typhoon_bars_${activeBrokerId}`;
    const req = indexedDB.open(dbName, 1);
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

// ── SQLite KV Cache (replaces legacy IndexedDB + cold zstd files) ────

/// Save to SQLite KV cache (replaces legacy coldSave/zstd files).
async function coldSave(key, data) {
  try {
    const brokerKey = `${activeBrokerId}/${key}`;
    await invokeQuiet("db_cache_put", { key: brokerKey, data: JSON.stringify(data), kind: "kv" });
  } catch (_) {}
}

/// Load from SQLite KV cache. Falls back to legacy zstd files for migration.
async function coldLoad(key) {
  const brokerKey = `${activeBrokerId}/${key}`;
  // Try SQLite KV first (current format)
  try {
    const json = await invokeQuiet("db_cache_get", { key: brokerKey, kind: "kv" });
    return JSON.parse(json);
  } catch (_) {}
  // Fallback: try legacy zstd files and migrate to SQLite
  try {
    const json = await invokeQuiet("load_cold_cache", { key: brokerKey });
    const data = JSON.parse(json);
    // Auto-migrate to SQLite KV
    await invokeQuiet("db_cache_put", { key: brokerKey, data: JSON.stringify(data), kind: "kv" });
    return data;
  } catch (_) {}
  // Fallback: try legacy key without broker prefix
  try {
    const json = await invokeQuiet("load_cold_cache", { key });
    const data = JSON.parse(json);
    await invokeQuiet("db_cache_put", { key: brokerKey, data: JSON.stringify(data), kind: "kv" });
    return data;
  } catch (_) {}
  return null;
}

// ── Unified Cache Operations ────────────────────────────────

// Load bar cache from SQLite on startup. Migrates legacy IndexedDB entries.
async function loadBarCacheFromDisk() {
  try {
    // Migrate legacy IndexedDB entries to SQLite
    try {
      await openIndexedDB();
      if (idb) {
        const tx = idb.transaction("bars", "readonly");
        const store = tx.objectStore("bars");
        const req = store.getAll();
        await new Promise((resolve) => {
          req.onsuccess = async () => {
            let migrated = 0;
            for (const entry of req.result || []) {
              if (entry.key && entry.data) {
                barCache[entry.key] = { data: entry.data, timestamp: entry.timestamp || 0, lastAccess: Date.now() };
                // Migrate to SQLite
                try {
                  await invokeQuiet("db_cache_put", {
                    key: `${activeBrokerId}:${entry.key}`,
                    data: JSON.stringify(entry.data), kind: "bars",
                  });
                  migrated++;
                } catch (_) {}
              }
            }
            if (migrated > 0) log(`Migrated ${migrated} bar sets from IndexedDB to SQLite`, "ok");
            resolve();
          };
          req.onerror = () => resolve();
        });
      }
    } catch (_) {}

    // Show SQLite cache stats
    try {
      const stats = JSON.parse(await invoke("db_cache_stats"));
      log(`SQLite cache: ${stats.bar_entries} bar sets, ${stats.kv_entries} KV entries, ${stats.total_compressed_mb.toFixed(1)}MB compressed`, "info");
    } catch (_) {}

    evictLRU();
  } catch (e) {
    log(`Cache init: ${e}`, "warn");
  }
}

// Save bar data: hot (memory) + SQLite (binary+zstd, persistent).
// IndexedDB and cold zstd files are legacy — reads still supported for migration.
function saveBarCacheToDisk(cacheKey, data) {
  const ts = Date.now();
  barCache[cacheKey] = { data, timestamp: ts, lastAccess: ts };
  evictLRU();
  // SQLite (binary packed + zstd compressed) — single persistent store
  invokeQuiet("db_cache_put", { key: `${activeBrokerId}:${cacheKey}`, data: JSON.stringify(data), kind: "bars" }).catch(() => {});
}

// Migrate old localStorage bar cache to SQLite on first run
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
          barCache[cacheKey] = { data: stored.data, timestamp: stored.timestamp || 0 };
          // Migrate directly to SQLite (skip IndexedDB)
          await invokeQuiet("db_cache_put", {
            key: `${activeBrokerId}:${cacheKey}`,
            data: JSON.stringify(stored.data), kind: "bars",
          }).catch(() => {});
          migrated++;
        }
        localStorage.removeItem(key); // clean up old format
      } catch (_) {}
    }
  }
  if (migrated > 0) log(`Migrated ${migrated} cache entries from localStorage to SQLite`, "ok");
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

// ── Custom Timeframe Aggregation ─────────────────────────────

// Aggregate bars by a factor (e.g., factor=2 combines every 2 bars into 1)
function aggregateBars(bars, factor) {
  if (factor <= 1 || !bars || bars.length === 0) return bars;
  const result = [];
  for (let i = 0; i < bars.length; i += factor) {
    const chunk = bars.slice(i, i + factor);
    if (chunk.length === 0) break;
    const agg = {
      time: chunk[0].time,
      open: chunk[0].open,
      high: Math.max(...chunk.map(b => b.high)),
      low: Math.min(...chunk.map(b => b.low)),
      close: chunk[chunk.length - 1].close,
      volume: chunk.reduce((s, b) => s + (b.volume || 0), 0),
    };
    result.push(agg);
  }
  return result;
}

// Custom timeframe map: custom TF -> { base TF, aggregation factor }
const CUSTOM_TIMEFRAME_MAP = {
  // Minutes (base: 5Min or 15Min)
  "10Min":  { base: "5Min",  factor: 2 },
  "20Min":  { base: "5Min",  factor: 4 },
  "40Min":  { base: "5Min",  factor: 8 },
  "45Min":  { base: "15Min", factor: 3 },
  "50Min":  { base: "5Min",  factor: 10 },
  "55Min":  { base: "5Min",  factor: 11 },
  // Hours (base: 1Hour or 4Hour)
  "2Hour":  { base: "1Hour", factor: 2 },
  "3Hour":  { base: "1Hour", factor: 3 },
  "5Hour":  { base: "1Hour", factor: 5 },
  "6Hour":  { base: "1Hour", factor: 6 },
  "7Hour":  { base: "1Hour", factor: 7 },
  "8Hour":  { base: "4Hour", factor: 2 },
  "9Hour":  { base: "1Hour", factor: 9 },
  "10Hour": { base: "1Hour", factor: 10 },
  "11Hour": { base: "1Hour", factor: 11 },
  "12Hour": { base: "4Hour", factor: 3 },
  "13Hour": { base: "1Hour", factor: 13 },
  "14Hour": { base: "1Hour", factor: 14 },
  "15Hour": { base: "1Hour", factor: 15 },
  "16Hour": { base: "4Hour", factor: 4 },
  "17Hour": { base: "1Hour", factor: 17 },
  "18Hour": { base: "1Hour", factor: 18 },
  "19Hour": { base: "1Hour", factor: 19 },
  "20Hour": { base: "4Hour", factor: 5 },
  "21Hour": { base: "1Hour", factor: 21 },
  "22Hour": { base: "1Hour", factor: 22 },
  "23Hour": { base: "1Hour", factor: 23 },
  // Days (base: 1Day)
  "2Day":   { base: "1Day",  factor: 2 },
  "3Day":   { base: "1Day",  factor: 3 },
  "4Day":   { base: "1Day",  factor: 4 },
  "5Day":   { base: "1Day",  factor: 5 },
  "6Day":   { base: "1Day",  factor: 6 },
  "7Day":   { base: "1Day",  factor: 7 },
  "8Day":   { base: "1Day",  factor: 8 },
  "9Day":   { base: "1Day",  factor: 9 },
  "10Day":  { base: "1Day",  factor: 10 },
  "11Day":  { base: "1Day",  factor: 11 },
  "12Day":  { base: "1Day",  factor: 12 },
  "13Day":  { base: "1Day",  factor: 13 },
  // Weeks (base: 1Week)
  "2Week":  { base: "1Week", factor: 2 },
  "3Week":  { base: "1Week", factor: 3 },
  // Months (base: 1Month)
  "2Month": { base: "1Month", factor: 2 },
  "3Month": { base: "1Month", factor: 3 },
  "4Month": { base: "1Month", factor: 4 },
  "5Month": { base: "1Month", factor: 5 },
  "6Month": { base: "1Month", factor: 6 },
  "7Month": { base: "1Month", factor: 7 },
  "8Month": { base: "1Month", factor: 8 },
  "9Month": { base: "1Month", factor: 9 },
  "10Month": { base: "1Month", factor: 10 },
  "11Month": { base: "1Month", factor: 11 },
  // Years (base: 1Month aggregated)
  "1Year":  { base: "1Month", factor: 12 },
  "2Year":  { base: "1Month", factor: 24 },
  "3Year":  { base: "1Month", factor: 36 },
  "4Year":  { base: "1Month", factor: 48 },
  "5Year":  { base: "1Month", factor: 60 },
  "6Year":  { base: "1Month", factor: 72 },
  "7Year":  { base: "1Month", factor: 84 },
  "8Year":  { base: "1Month", factor: 96 },
  "9Year":  { base: "1Month", factor: 108 },
  "10Year": { base: "1Month", factor: 120 },
  "11Year": { base: "1Month", factor: 132 },
  "12Year": { base: "1Month", factor: 144 },
  "13Year": { base: "1Month", factor: 156 },
  "14Year": { base: "1Month", factor: 168 },
  "15Year": { base: "1Month", factor: 180 },
  "16Year": { base: "1Month", factor: 192 },
  "17Year": { base: "1Month", factor: 204 },
  "18Year": { base: "1Month", factor: 216 },
  "19Year": { base: "1Month", factor: 228 },
  "20Year": { base: "1Month", factor: 240 },
  "21Year": { base: "1Month", factor: 252 },
  "22Year": { base: "1Month", factor: 264 },
  "23Year": { base: "1Month", factor: 276 },
  "24Year": { base: "1Month", factor: 288 },
  "25Year": { base: "1Month", factor: 300 },
  "26Year": { base: "1Month", factor: 312 },
  "27Year": { base: "1Month", factor: 324 },
  "28Year": { base: "1Month", factor: 336 },
  "29Year": { base: "1Month", factor: 348 },
  "30Year": { base: "1Month", factor: 360 },
  "31Year": { base: "1Month", factor: 372 },
  "32Year": { base: "1Month", factor: 384 },
  "33Year": { base: "1Month", factor: 396 },
};

// ── Renko Calculation ────────────────────────────────────────

function calcRenko(data, brickSize) {
  if (!data || data.length === 0 || brickSize <= 0) return [];
  const bricks = [];
  let lastClose = data[0].close;
  let direction = 0;
  for (let i = 1; i < data.length; i++) {
    const price = data[i].close;
    const diff = price - lastClose;
    if (Math.abs(diff) >= brickSize) {
      const numBricks = Math.floor(Math.abs(diff) / brickSize);
      const newDir = diff > 0 ? 1 : -1;
      if (direction !== 0 && newDir !== direction && Math.abs(diff) < brickSize * 2) continue;
      for (let b = 0; b < numBricks; b++) {
        const brickOpen = lastClose + (newDir > 0 ? b * brickSize : -b * brickSize);
        const brickClose = brickOpen + newDir * brickSize;
        bricks.push({
          time: data[i].time,
          open: Math.min(brickOpen, brickClose),
          high: Math.max(brickOpen, brickClose),
          low: Math.min(brickOpen, brickClose),
          close: newDir > 0 ? Math.max(brickOpen, brickClose) : Math.min(brickOpen, brickClose),
        });
        lastClose = brickClose;
      }
      direction = newDir;
    }
  }
  const seen = {};
  for (const brick of bricks) {
    while (seen[brick.time]) brick.time += 1;
    seen[brick.time] = true;
  }
  return bricks;
}

function getRenkoBrickSize(data) {
  const atrValues = calcATR(data, 14);
  if (atrValues.length > 0) return atrValues[atrValues.length - 1].value;
  if (data.length > 0) return (data.reduce((s, d) => s + d.close, 0) / data.length) * 0.01;
  return 1;
}

// ── Load Chart Data ─────────────────────────────────────────

let liveBarInterval = null;

async function loadChart(symbol, timeframe) {
  setLoadingStatus(symbol, "loading...");

  // Set symbol immediately so tab identity is correct
  currentSymbol = symbol;
  currentTimeframe = timeframe;
  const loadTabId = activeTabId; // capture which tab initiated this load

  // Custom timeframe support: resolve to base TF + aggregation factor
  const customTF = CUSTOM_TIMEFRAME_MAP[timeframe];
  const fetchTimeframe = customTF ? customTF.base : timeframe;
  const aggFactor = customTF ? customTF.factor : 1;

  try {
    const limit = parseInt(document.getElementById("bar-count").value) || 1000;
    // For custom TFs, fetch more bars to compensate for aggregation
    const fetchLimit = aggFactor > 1 ? limit * aggFactor : limit;
    const cacheKey = getCacheKey(symbol, fetchTimeframe);
    let bars;

    // Strategy: show cached data IMMEDIATELY, then refresh in background
    const cached = barCache[cacheKey];
    const isFresh = cached && (Date.now() - cached.timestamp) < CACHE_TTL_MS;
    const hasEnough = cached && cached.data && cached.data.length >= fetchLimit * 0.5;

    if (hasEnough) {
      // Display cached data instantly
      bars = cached.data;
      log(`${symbol} @ ${timeframe}: ${bars.length} bars from cache (${isFresh ? "fresh" : "stale, refreshing..."})`, "info");

      if (!isFresh) {
        // Refresh in background — will update chart when done
        (async () => {
          try {
            const freshJson = await invoke("get_bars", { symbol, timeframe: fetchTimeframe, limit: fetchLimit });
            const freshBars = JSON.parse(freshJson);
            if (freshBars.length > 0) {
              barCache[cacheKey] = { data: freshBars, timestamp: Date.now() };
              saveBarCacheToDisk(cacheKey, freshBars);
              // If still on same tab/symbol, update chart (activeTabId prevents cross-tab writes)
              if (currentSymbol === symbol && currentTimeframe === timeframe && activeTabId === loadTabId) {
                let freshChartData = freshBars.map(b => ({
                  time: Math.floor(new Date(b.timestamp).getTime() / 1000),
                  open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume,
                }));
                if (aggFactor > 1) freshChartData = aggregateBars(freshChartData, aggFactor);
                currentChartData = freshChartData;
                if (currentChartType === "line") {
                  candleSeries.setData(freshChartData.map(d => ({ time: d.time, value: d.close })));
                } else if (currentChartType === "renko") {
                  const brickSize = getRenkoBrickSize(freshChartData);
                  candleSeries.setData(calcRenko(freshChartData, brickSize));
                } else if (currentChartType === "heikin-ashi") {
                  candleSeries.setData(calcHeikinAshi(freshChartData));
                } else {
                  candleSeries.setData(freshChartData);
                }
                lastPrice = freshChartData[freshChartData.length - 1].close;
                log(`${symbol} @ ${timeframe}: refreshed to ${freshChartData.length} bars`, "ok");
              }
            }
          } catch (_) {}
        })();
      }
    } else {
      // No usable cache — fetch synchronously
      const barsJson = await invoke("get_bars", { symbol, timeframe: fetchTimeframe, limit: fetchLimit });
      bars = JSON.parse(barsJson);
      barCache[cacheKey] = { data: bars, timestamp: Date.now() };
      saveBarCacheToDisk(cacheKey, bars);
      if (bars.length > 0) {
        const first = bars[0].timestamp.substring(0, 10);
        const last = bars[bars.length - 1].timestamp.substring(0, 10);
        setLoadingStatus(symbol, `${first} → ${last} · ${bars.length} bars`);
      }
    }

    let chartData = bars.map((b) => ({
      time: Math.floor(new Date(b.timestamp).getTime() / 1000),
      open: b.open,
      high: b.high,
      low: b.low,
      close: b.close,
      volume: b.volume,
    }));

    // Apply custom timeframe aggregation
    if (aggFactor > 1) {
      chartData = aggregateBars(chartData, aggFactor);
      log(`${symbol} @ ${timeframe}: aggregated ${aggFactor}x from ${fetchTimeframe} → ${chartData.length} bars`, "info");
    }

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

    currentChartData = chartData; // preserve volume for indicators
    if (currentChartType === "line") {
      candleSeries.setData(chartData.map(d => ({ time: d.time, value: d.close })));
    } else if (currentChartType === "renko") {
      const brickSize = getRenkoBrickSize(chartData);
      const renkoBricks = calcRenko(chartData, brickSize);
      if (renkoBricks.length > 0) candleSeries.setData(renkoBricks);
      else candleSeries.setData(chartData);
    } else if (currentChartType === "heikin-ashi") {
      candleSeries.setData(calcHeikinAshi(chartData));
    } else {
      candleSeries.setData(chartData);
    }
    chart.timeScale().fitContent();
    currentSymbol = symbol;
    currentTimeframe = timeframe;
    if (chartData.length > 0) lastPrice = chartData[chartData.length - 1].close;

    // Load MTF data for multi-timeframe indicators, then apply all + update grid
    // Guard: discard stale MTF data if user switched symbols during fetch
    loadMTFData(symbol).then(() => {
      if (currentSymbol !== symbol) return;
      applyIndicators(chartData);
      renderAnnotations();
      updateMTFGrid();
    }).catch(() => {
      if (currentSymbol !== symbol) return;
      applyIndicators(chartData);
    });

    log(`${symbol} @ ${timeframe}: ${chartData.length} bars, last=$${lastPrice}`, "ok");
    setText("connect-status-bar", `${symbol} — ${chartData.length} bars`);
    setLoadingStatus(symbol, null);
    updateTabLabel();
    if (typeof updateNotesIndicator === "function") updateNotesIndicator();

    // Restore SL/TP lines from backend state (persists across tab switches)
    try {
      const stJson = await invoke("get_sl_tp_pl", {
        symbol, qty: 1, side: "long", entryPrice: lastPrice,
      });
      const st = JSON.parse(stJson);
      if (st.sl_price && st.sl_price > 0 && !getSLPrice()) createSLLine(st.sl_price);
      if (st.tp_price && st.tp_price > 0 && !getTPPrice()) createTPLine(st.tp_price);
    } catch (_) {}

    // Start live bar polling (update latest bar every 10s)
    // Generation counter prevents stale intervals from updating wrong charts
    if (liveBarInterval) clearInterval(liveBarInterval);
    const gen = ++chartLoadGeneration;
    liveBarInterval = setInterval(() => {
      if (chartLoadGeneration !== gen) { clearInterval(liveBarInterval); return; }
      updateLatestBar(symbol, timeframe);
    }, 10000);

    // If MTF grid is active, reload cells with new symbol (defer to let DOM settle)
    if (mtfGridActive && mtfGridCells.length > 0) {
      const selectedTFs = mtfGridCells.map(c => c.tf);
      closeMTFGrid();
      // Wait for DOM cleanup, then reopen grid with new symbol
      requestAnimationFrame(() => {
        openMTFGrid(symbol, selectedTFs).then(() => {
          // Extra resize after data loads to fix 0-dimension cells
          setTimeout(() => resizeMTFGrid(), 100);
        }).catch(e => {
          log(`MTF grid reload failed: ${e}`, "warn");
          document.getElementById("chart-stack").style.display = "";
        });
      });
    } else {
      prefetchAllTimeframes(symbol, timeframe, limit);
    }

    // Load news and fundamentals for this symbol (background)
    loadNewsAndFundamentals(symbol);
  } catch (e) {
    log(`Chart load failed for ${symbol} @ ${timeframe}: ${e}`, "error");
    setText("connect-status-bar", `Chart error: ${e}`);
    // Ensure main chart is visible if MTF grid was closed during failed load
    document.getElementById("chart-stack").style.display = "";
    setLoadingStatus(symbol, null);
  }
}

const ALL_NATIVE_TIMEFRAMES = ["1Min", "5Min", "15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week", "1Month"];

const prefetchInProgress = new Set();
// Track which symbol:tf combos are fully loaded (persisted across restarts)
const fullyLoadedTFs = new Set((() => {
  try { return JSON.parse(localStorage.getItem("fullyLoadedTFs") || "[]"); } catch { return []; }
})());
function saveFullyLoaded() {
  try { localStorage.setItem("fullyLoadedTFs", JSON.stringify([...fullyLoadedTFs])); } catch (_) {}
}

async function prefetchAllTimeframes(symbol, currentTF) {
  if (prefetchInProgress.has(symbol)) return;
  prefetchInProgress.add(symbol);

  let fetched = 0;
  for (const tf of ALL_NATIVE_TIMEFRAMES) {
    if (symbol !== currentSymbol) break; // user switched tabs
    const cacheKey = getCacheKey(symbol, tf);
    const fullyLoadedKey = `${symbol}:${tf}`;

    // Skip if already fully loaded in this session
    if (fullyLoadedTFs.has(fullyLoadedKey)) continue;

    const cached = barCache[cacheKey];
    if (cached && cached.data && cached.data.length > 0) {
      fullyLoadedTFs.add(fullyLoadedKey);
      continue;
    }

    // No cached data — initial fetch (max bars for this timeframe)
    try {
      const barsJson = await invokeQuiet("get_bars", { symbol, timeframe: tf, limit: 2000 });
      const bars = JSON.parse(barsJson);
      if (bars.length > 0) {
        barCache[cacheKey] = { data: bars, timestamp: Date.now() };
        saveBarCacheToDisk(cacheKey, bars);
        fetched++;
        log(`${symbol} ${tf}: ${bars.length} bars`, "info");
      }
      fullyLoadedTFs.add(fullyLoadedKey);
    } catch (_) {
      // Don't mark as loaded on error — retry next time
    }
  }

  if (fetched > 0) log(`Pre-cached ${fetched} timeframes for ${symbol}`, "ok");
  saveFullyLoaded();
  prefetchInProgress.delete(symbol);
}

let lastBarTime = 0;

async function updateLatestBar(symbol, timeframe) {
  if (symbol !== currentSymbol) return;
  // Use base timeframe for custom TFs
  const customTF = CUSTOM_TIMEFRAME_MAP[timeframe];
  const fetchTF = customTF ? customTF.base : timeframe;
  try {
    const barsJson = await invoke("get_bars", { symbol, timeframe: fetchTF, limit: 5 });
    // Re-check symbol after async — user may have switched tabs during fetch
    if (symbol !== currentSymbol) return;
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
    lastPrice = bar.close;
    // Only update main chart series if it's visible (not in MTF grid mode)
    if (!mtfGridActive) {
      try {
        if (currentChartType === "line") {
          candleSeries.update({ time: bar.time, value: bar.close });
        } else {
          candleSeries.update(bar);
        }
      } catch (_) {}
    }

    // If a NEW bar has printed (different timestamp), refresh all indicators
    if (barTime !== lastBarTime && lastBarTime !== 0) {
      // Final symbol guard before mutating global chart state
      if (symbol !== currentSymbol) return;
      log(`New bar on ${symbol} @ ${timeframe}`, "info");
      // Update currentChartData with the new bar (preserve volume)
      if (currentChartData.length > 0 && currentChartData[currentChartData.length - 1].time === barTime) {
        currentChartData[currentChartData.length - 1] = bar;
      } else {
        currentChartData.push(bar);
      }
      applyIndicators(currentChartData);
    }
    lastBarTime = barTime;
    // Sync MTF grid cells with latest price
    syncMTFGridLivePrice();
  } catch (_) {}
}

// ── Dashboard Update (all 11 labels) ────────────────────────

async function updateDashboard() {
  if (window._dashboardInFlight) return;
  window._dashboardInFlight = true;
  try {
    // Parallel: fetch margin + positions simultaneously (2 API calls → 1 round trip)
    const [marginJson, posJson] = await Promise.all([
      invoke("get_margin_info"),
      invoke("get_positions"),
    ]);
    const mi = JSON.parse(marginJson);
    const positions = JSON.parse(posJson);

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

    // Positions (already fetched in parallel above)
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
    const warnEl = document.getElementById("no-sl-warning");
    if (posQty > 0 && posEntry > 0) {
      try {
        const stJson = await invoke("get_sl_tp_pl", {
          symbol: currentSymbol, qty: posQty, side: posSide, entryPrice: posEntry,
        });
        const st = JSON.parse(stJson);

        const hasSL = st.sl_pl !== null;
        const hasTP = st.tp_pl !== null;

        if (hasSL) {
          setTextClass("info-sl-pl", `SL P/L: $${st.sl_pl.toFixed(2)}`, st.sl_pl >= 0 ? "positive" : "negative");
          if (mi.balance > 0) {
            setText("info-risk", `Risk: $${Math.abs(st.sl_pl).toFixed(2)} (${(Math.abs(st.sl_pl) / mi.balance * 100).toFixed(2)}%)`);
          }
        } else {
          setText("info-sl-pl", "SL P/L: —");
          setText("info-risk", "Risk: —");
        }
        if (hasTP) setTextClass("info-tp-pl", `TP P/L: $${st.tp_pl.toFixed(2)}`, "positive");
        else setText("info-tp-pl", "TP P/L: —");
        if (st.rr !== null) setText("info-rr", `RR: ${st.rr.toFixed(2)}`);
        else setText("info-rr", "RR: —");

        // Show/hide unprotected position warning
        if (!hasSL && !hasTP) {
          warnEl.classList.remove("hidden");
        } else {
          warnEl.classList.add("hidden");
        }
      } catch (_) {}

      // VaR — throttled to avoid fetching 1Day bars every 2s dashboard cycle
      if (lastPrice > 0) {
        const varKey = `${currentSymbol}:${posQty}`;
        const now = Date.now();
        if (!window._varCache || window._varCache.key !== varKey || (now - window._varCache.ts) > 60000) {
          try {
            const varJson = await invoke("calculate_position_var", {
              symbol: currentSymbol, positionSize: posQty, currentPrice: lastPrice,
            });
            const v = JSON.parse(varJson);
            window._varCache = { key: varKey, ts: now, val: v.var_dollars };
            setText("info-var", `VaR: $${v.var_dollars.toFixed(2)}`);
          } catch (_) { setText("info-var", "VaR: —"); }
        } else if (window._varCache) {
          setText("info-var", `VaR: $${window._varCache.val.toFixed(2)}`);
        }
      }
    } else {
      setText("info-sl-pl", "SL P/L: —");
      setText("info-tp-pl", "TP P/L: —");
      setText("info-rr", "RR: —");
      setText("info-var", "VaR: —");
      setText("info-risk", "Risk: —");
      warnEl.classList.add("hidden");
    }

    updateNextBarTime();
    updatePositionsPanel();
    updateOrdersPanel();
    updateOrderPriceLines();
    checkAlerts();
    checkMultiConditionAlerts();
    checkEquityProtection();
    checkDividendAlerts();
    syncMTFGridLivePrice();
    updateRiskCalcPanel();

    // Bid/Ask spread (non-blocking — don't fail dashboard if quote fails)
    if (currentSymbol) {
      invoke("get_latest_quote", { symbol: currentSymbol }).then(json => {
        const q = JSON.parse(json);
        const spreadEl = document.getElementById("bid-ask-spread");
        if (spreadEl && q.bid > 0 && q.ask > 0) {
          spreadEl.textContent = `Bid: ${q.bid.toFixed(4)} | Ask: ${q.ask.toFixed(4)} | Spread: ${q.spread.toFixed(4)}`;
        }
      }).catch(() => {});
    }
  } catch (_) {
  } finally {
    window._dashboardInFlight = false;
  }
}

// ── Equity TP/SL Protection (port of MQL5 EnableEquityTP/SL) ──
async function checkEquityProtection() {
  try {
    const json = await invoke("check_equity_protection");
    const result = JSON.parse(json);
    if (result.triggered) {
      log(`EQUITY PROTECTION TRIGGERED: ${result.triggered}`, "warn");
      if (confirm(`${result.triggered}\n\nClose ALL positions now?`)) {
        await invoke("close_all");
        log("All positions closed by equity protection", "warn");
      }
    }
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
  const regime = currentChartData.length > 40 ? detectRegime(currentChartData) : "";
  const regimeLabel = regime ? ` [${regime.toUpperCase()}]` : "";
  setText("info-time", `Next bar: ${h > 0 ? `${h}H ${m}M ${s}s` : m > 0 ? `${m}M ${s}s` : `${s}s`}${regimeLabel}`);
}

function setText(id, text) {
  const el = document.getElementById(id);
  if (el && el.textContent !== text) el.textContent = text;
}
function setTextClass(id, text, cls) {
  const el = document.getElementById(id);
  if (!el) return;
  if (el.textContent !== text) el.textContent = text;
  const full = "dash-row " + cls;
  if (el.className !== full) el.className = full;
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

  // Chart type selector
  document.getElementById("chart-type-select").addEventListener("change", (e) => {
    const newType = e.target.value;
    if (mtfGridActive && mtfGridCells.length > 0) {
      // MTF Grid active: rebuild grid with new chart type
      const selectedTFs = mtfGridCells.map(c => c.tf);
      const sym = mtfGridSymbol || currentSymbol;
      closeMTFGrid();
      currentChartType = newType;
      openMTFGrid(sym, selectedTFs);
    } else {
      // Single chart mode: rebuild main series
      rebuildMainSeries(newType);
    }
  });

  // Buy Lines: SL = lowest visible, TP = highest visible
  document.getElementById("btn-buy-lines").addEventListener("click", () => {
    const data = getActiveCandleSeries().data();
    if (!data || data.length === 0) return;
    const recent = data.slice(-50);
    createSLLine(Math.min(...recent.map((d) => d.low)));
    createTPLine(Math.max(...recent.map((d) => d.high)));
  });

  // Sell Lines: SL = highest, TP = lowest
  document.getElementById("btn-sell-lines").addEventListener("click", () => {
    const data = getActiveCandleSeries().data();
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
        let orderType = document.getElementById("order-type").value;
        // MT5 behavior: market order with SL/TP lines → auto-upgrade to bracket
        if (orderType === "market" && sl && tp) {
          orderType = "bracket";
        }
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
    // Place a broker-side stop order if position exists (protective SL)
    await placeProtectiveOrder(currentSymbol, "sl", sl);
    updateDashboard();
  });

  document.getElementById("btn-set-tp").addEventListener("click", async () => {
    const tp = getTPPrice();
    if (!tp || !currentSymbol) return;
    await invoke("set_tp_level", { symbol: currentSymbol, price: tp });
    // Place a broker-side limit order if position exists (protective TP)
    await placeProtectiveOrder(currentSymbol, "tp", tp);
    updateDashboard();
  });

  // Warning banner click: prompt for SL/TP and place protective orders
  document.getElementById("no-sl-warning").addEventListener("click", async () => {
    if (!currentSymbol) return;
    const sl = prompt(`Set SL price for ${currentSymbol}:`);
    const tp = prompt(`Set TP price for ${currentSymbol}:`);
    if (sl) {
      const slNum = parseFloat(sl);
      if (slNum > 0 && isFinite(slNum)) {
        createSLLine(slNum);
        await invoke("set_sl_level", { symbol: currentSymbol, price: slNum });
        await placeProtectiveOrder(currentSymbol, "sl", slNum);
      }
    }
    if (tp) {
      const tpNum = parseFloat(tp);
      if (tpNum > 0 && isFinite(tpNum)) {
        createTPLine(tpNum);
        await invoke("set_tp_level", { symbol: currentSymbol, price: tpNum });
        await placeProtectiveOrder(currentSymbol, "tp", tpNum);
      }
    }
    updateDashboard();
  });

  // ── SL/TP Manual Input ──
  const slInput = document.getElementById("sl-input");
  const tpInput = document.getElementById("tp-input");

  // SL/TP inputs: Enter key sets line + saves to backend + places protective order
  if (slInput) {
    slInput.addEventListener("keydown", async (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        const val = parseFloat(slInput.value);
        if (!isNaN(val) && val > 0 && currentSymbol) {
          createSLLine(val);
          await invoke("set_sl_level", { symbol: currentSymbol, price: val });
          await placeProtectiveOrder(currentSymbol, "sl", val);
          updateDashboard();
        }
        slInput.blur();
      }
    });
  }
  if (tpInput) {
    tpInput.addEventListener("keydown", async (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        const val = parseFloat(tpInput.value);
        if (!isNaN(val) && val > 0 && currentSymbol) {
          createTPLine(val);
          await invoke("set_tp_level", { symbol: currentSymbol, price: val });
          await placeProtectiveOrder(currentSymbol, "tp", val);
          updateDashboard();
        }
        tpInput.blur();
      }
    });
  }

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
      // ── F-keys (primary actions) ──
      case "F1": e.preventDefault(); showHelpOverlay(); break;
      case "F2": document.getElementById("btn-buy-lines").click(); break;
      case "F3": document.getElementById("btn-sell-lines").click(); break;
      case "F4": document.getElementById("btn-trade").click(); break;
      case "F5": e.preventDefault(); document.getElementById("btn-destroy-lines").click(); break;
      case "F6": document.getElementById("btn-martingale").click(); break;
      case "F7": document.getElementById("btn-close-all").click(); break;
      case "F8": document.getElementById("btn-close-partial").click(); break;

      // ── Single-key shortcuts ──
      case "Escape": removeSLLine(); removeTPLine(); break;
      case "?": showHelpOverlay(); break;
      case "Delete":
        if (drawings.length > 0) {
          drawings.pop(); saveDrawings(); renderDrawings(); renderDrawingsExtended();
          log("Deleted last drawing", "info");
        }
        break;

      // ── Drawing tools (lowercase letters) ──
      case "l": // trend Line
        drawingMode = "trendline"; drawingAnchor = null;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing: trend line — click two points", "info");
        break;
      case "f": // Fibonacci
        drawingMode = "fibonacci"; drawingAnchor = null;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing: Fibonacci — click high and low", "info");
        break;
      case "h": // Horizontal line
        drawingMode = "horizontal"; drawingAnchor = null; channelThirdClick = false;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing: horizontal line — click to place", "info");
        break;
      case "r": // Rectangle
        drawingMode = "rectangle"; drawingAnchor = null; channelThirdClick = false;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing: rectangle — click two corners", "info");
        break;
      case "e": // ray
        drawingMode = "ray"; drawingAnchor = null;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing: ray — click two points (extends right)", "info");
        break;
      case "c": // channel
        drawingMode = "channel"; drawingAnchor = null; channelThirdClick = false;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing: channel — click two points + offset", "info");
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
// Actual credentials stored in encrypted storage (AES-256-GCM encrypted SQLite).
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
  // Save keys to encrypted storage — no localStorage fallback (security)
  try {
    await invoke("keychain_save", { accountName, apiKey, secretKey });
    log(`Credentials saved to encrypted storage for "${accountName}"`, "ok");
  } catch (e) {
    log(`Keychain save failed: ${e}`, "error");
    alert(`Failed to save credentials to encrypted storage: ${e}\nSQLite cache may not be available.`);
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
  // Try encrypted storage first
  try {
    const json = await invokeQuiet("keychain_load", { accountName });
    const parsed = JSON.parse(json);
    if (parsed && parsed.apiKey) return parsed;
  } catch (e) {
    log(`Keychain load for "${accountName}": ${e}`, "info");
    // Fallback: check localStorage for legacy entries with keys
    const accounts = loadSavedAccounts();
    const acct = accounts.find(a => a.name === accountName);
    if (acct && acct.apiKey) {
      // Auto-migrate legacy credentials to encrypted storage
      try {
        await invokeQuiet("keychain_save", { accountName, apiKey: acct.apiKey, secretKey: acct.secretKey });
        log(`Migrated "${accountName}" credentials to encrypted storage`, "ok");
        // Strip keys from localStorage (keep metadata only)
        saveAccountMetadata(accounts);
      } catch (_) {}
      return { apiKey: acct.apiKey, secretKey: acct.secretKey };
    }
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
  // Update placeholders when broker changes
  const brokerSel = document.getElementById("broker-select");
  if (brokerSel) {
    brokerSel.addEventListener("change", () => {
      const isTasty = brokerSel.value === "tastytrade";
      document.getElementById("api-key").placeholder = isTasty ? "Username / Email" : "API Key";
      document.getElementById("secret-key").placeholder = isTasty ? "Password" : "Secret Key";
    });
  }

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
    const brokerType = document.getElementById("broker-select")?.value || "alpaca";
    const apiKey = document.getElementById("api-key").value.trim();
    const secretKey = document.getElementById("secret-key").value.trim();
    const accountType = document.getElementById("account-type").value;
    const accountName = document.getElementById("account-name").value.trim();
    const shouldSaveCreds = document.getElementById("save-credentials").checked;
    const paper = accountType === "paper";

    if (!apiKey || !secretKey) {
      status.textContent = brokerType === "tastytrade" ? "Username and Password required" : "API Key and Secret Key required";
      return;
    }

    status.textContent = `Connecting to ${brokerType}...`;
    status.style.color = "#ff8";

    try {
      let result;
      if (brokerType === "tastytrade") {
        result = await invoke("connect_tastytrade", { username: apiKey, password: secretKey, isSandbox: paper });
      } else {
        result = await invoke("connect", { apiKey, secretKey, paper });
      }
      const acct = JSON.parse(result);

      // Set broker ID for per-broker data isolation
      activeBrokerId = `${brokerType}_${(accountName || "default").replace(/[^a-zA-Z0-9_-]/g, "_")}`;

      // Save credentials to encrypted storage if requested
      if (shouldSaveCreds && accountName) {
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

  // Auto-connect if saved account exists (load keys from encrypted storage)
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
        activeBrokerId = (acctMeta.name || "default").replace(/[^a-zA-Z0-9_-]/g, "_");
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

  // Extract readable content (XSS-safe: textContent for text, sanitized src for images)
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, "text/html");
  doc.querySelectorAll("script, style, nav, header, footer, iframe, .ad, .ads, .sidebar").forEach(el => el.remove());

  const main = doc.querySelector("article, main, .article-body, .post-content, .entry-content, .story-body");
  const source = main || doc.body;
  const elements = source ? source.querySelectorAll("p, h1, h2, h3, h4, li, img, figure") : doc.querySelectorAll("p, img");

  win.contentElement.textContent = ""; // Clear loading text
  let found = 0;
  for (const node of elements) {
    if (node.tagName === "IMG" || (node.tagName === "FIGURE" && node.querySelector("img"))) {
      // Extract image — only allow HTTPS src (XSS-safe)
      const imgNode = node.tagName === "IMG" ? node : node.querySelector("img");
      const src = imgNode?.getAttribute("src") || "";
      if (src.startsWith("https://")) {
        const img = document.createElement("img");
        img.src = src; // safe: HTTPS only, CSP restricts to https:
        img.alt = imgNode?.getAttribute("alt") || "";
        img.style.cssText = "max-width:200px;max-height:150px;height:auto;border-radius:3px;margin:6px 0;display:block;cursor:pointer;";
        img.title = "Click to enlarge";
        img.addEventListener("click", () => {
          img.style.maxWidth = img.style.maxWidth === "200px" ? "100%" : "200px";
          img.style.maxHeight = img.style.maxHeight === "150px" ? "none" : "150px";
        });
        img.loading = "lazy";
        win.appendElement(img);
        found++;
      }
    } else if (node.textContent.trim().length > 15) {
      const el = document.createElement("p");
      el.textContent = node.textContent;
      el.style.margin = "6px 0";
      el.style.lineHeight = "1.6";
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

  // Re-apply indicators when checkboxes change (use currentChartData for volume)
  document.querySelectorAll("#indicator-list input[type=checkbox]").forEach(cb => {
    cb.addEventListener("change", () => {
      if (currentChartData && currentChartData.length > 0) applyIndicators(currentChartData);
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

let positionsChartOnly = false;

function setupPositionsPanel() {
  const panel = document.getElementById("positions-panel");
  const header = document.getElementById("positions-header");

  header.addEventListener("click", () => {
    panel.classList.toggle("collapsed");
    updatePositionsHeader(panel);
  });
}

function updatePositionsHeader(panel) {
  const header = document.getElementById("positions-header");
  header.textContent = panel.classList.contains("collapsed") ? "Positions ▶" : "Positions ▼";
}

async function updatePositionsPanel() {
  const content = document.getElementById("positions-content");
  const panel = document.getElementById("positions-panel");
  if (!content) return;
  try {
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);

    // Build new content in a fragment to avoid layout thrash (atomic swap)
    const frag = document.createDocumentFragment();

    if (positions.length === 0) {
      const msg = document.createTextNode("No positions");
      frag.appendChild(msg);
      content.replaceChildren(frag);
      return;
    }

    // Auto-expand when positions exist (never auto-collapse)
    if (panel.classList.contains("collapsed")) {
      panel.classList.remove("collapsed");
      updatePositionsHeader(panel);
    }

    // Filter controls
    const controls = document.createElement("div");
    controls.style.cssText = "display:flex;align-items:center;gap:6px;padding:2px 0 4px;border-bottom:1px solid #1a1a2e;";
    const filterLabel = document.createElement("label");
    filterLabel.style.cssText = "display:flex;align-items:center;gap:3px;font-size:10px;color:#888;cursor:pointer;user-select:none;";
    const filterCb = document.createElement("input");
    filterCb.type = "checkbox";
    filterCb.checked = positionsChartOnly;
    filterCb.style.cssText = "margin:0;cursor:pointer;";
    filterCb.addEventListener("change", () => {
      positionsChartOnly = filterCb.checked;
      updatePositionsPanel();
    });
    const filterText = document.createElement("span");
    filterText.textContent = "Chart only";
    filterLabel.appendChild(filterCb);
    filterLabel.appendChild(filterText);
    controls.appendChild(filterLabel);
    frag.appendChild(controls);

    // Filter positions if checkbox is ticked
    const sym = currentSymbol.replace("/", "");
    const filtered = positionsChartOnly
      ? positions.filter(p => p.symbol === currentSymbol || p.symbol === sym)
      : positions;

    if (filtered.length === 0) {
      const msg = document.createElement("div");
      msg.style.cssText = "color:#888;font-size:10px;padding:4px 0;";
      msg.textContent = `No positions for ${currentSymbol}`;
      frag.appendChild(msg);
      content.replaceChildren(frag);
      return;
    }

    for (const p of filtered) {
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
      frag.appendChild(row);

      // Click row to switch chart to that symbol
      row.style.cursor = "pointer";
      row.addEventListener("click", () => {
        document.getElementById("symbol-input").value = p.symbol;
        triggerLoad();
      });
    }

    // Atomic swap — old content replaced in one operation, no flicker
    content.replaceChildren(frag);
  } catch (_) {}
}

// ── Orders Panel (Trade History) ─────────────────────────────

let ordersChartOnly = false;

function setupOrdersPanel() {
  const panel = document.getElementById("orders-panel");
  const header = document.getElementById("orders-header");

  header.addEventListener("click", () => {
    panel.classList.toggle("collapsed");
    updateOrdersHeader(panel);
    if (!panel.classList.contains("collapsed")) updateOrdersPanel();
  });
}

function updateOrdersHeader(panel) {
  const header = document.getElementById("orders-header");
  header.textContent = panel.classList.contains("collapsed") ? "Orders ▶" : "Orders ▼";
}

async function updateOrdersPanel() {
  const content = document.getElementById("orders-content");
  const panel = document.getElementById("orders-panel");
  if (!content) return;
  try {
    // Fetch data BEFORE touching the DOM — keeps old content visible during network round-trip
    const [openJson, histJson] = await Promise.all([
      invoke("get_open_orders"),
      invoke("get_order_history", { limit: 20 }),
    ]);
    const openOrders = JSON.parse(openJson);
    const history = JSON.parse(histJson);

    // Build new content in a fragment (atomic swap, no flicker)
    const frag = document.createDocumentFragment();
    const hasOrders = openOrders.length > 0 || history.length > 0;

    if (!hasOrders) {
      frag.appendChild(document.createTextNode("No orders"));
      content.replaceChildren(frag);
      return;
    }

    // Auto-expand when open orders exist (never auto-collapse)
    if (openOrders.length > 0 && panel.classList.contains("collapsed")) {
      panel.classList.remove("collapsed");
      updateOrdersHeader(panel);
    }

    // Filter controls
    const controls = document.createElement("div");
    controls.style.cssText = "display:flex;align-items:center;gap:6px;padding:2px 0 4px;border-bottom:1px solid #1a1a2e;";
    const filterLabel = document.createElement("label");
    filterLabel.style.cssText = "display:flex;align-items:center;gap:3px;font-size:10px;color:#888;cursor:pointer;user-select:none;";
    const filterCb = document.createElement("input");
    filterCb.type = "checkbox";
    filterCb.checked = ordersChartOnly;
    filterCb.style.cssText = "margin:0;cursor:pointer;";
    filterCb.addEventListener("change", () => {
      ordersChartOnly = filterCb.checked;
      updateOrdersPanel();
    });
    const filterText = document.createElement("span");
    filterText.textContent = "Chart only";
    filterLabel.appendChild(filterCb);
    filterLabel.appendChild(filterText);
    controls.appendChild(filterLabel);
    frag.appendChild(controls);

    // Filter by current chart symbol if checkbox ticked
    const sym = currentSymbol.replace("/", "");
    const matchSymbol = (o) => !ordersChartOnly || o.symbol === currentSymbol || o.symbol === sym;

    const filteredOpen = openOrders.filter(matchSymbol);
    const filteredHist = history.filter(matchSymbol);

    if (filteredOpen.length > 0) {
      const hdr = document.createElement("div");
      hdr.textContent = "Open Orders";
      hdr.style.cssText = "color:#ff8;font-size:10px;font-weight:bold;padding:4px 0 2px;";
      frag.appendChild(hdr);
      for (const o of filteredOpen) {
        frag.appendChild(renderOrderRow(o, true));
      }
    }

    if (filteredHist.length > 0) {
      const hdr = document.createElement("div");
      hdr.textContent = "Recent Fills";
      hdr.style.cssText = "color:#888;font-size:10px;font-weight:bold;padding:4px 0 2px;";
      frag.appendChild(hdr);
      for (const o of filteredHist.slice(0, 15)) {
        frag.appendChild(renderOrderRow(o, false));
      }
    }

    if (filteredOpen.length === 0 && filteredHist.length === 0) {
      const msg = document.createElement("div");
      msg.style.cssText = "color:#888;font-size:10px;padding:4px 0;";
      msg.textContent = `No orders for ${currentSymbol}`;
      frag.appendChild(msg);
    }

    // Atomic swap — old content replaced in one operation, no flicker
    content.replaceChildren(frag);
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
  // Fire webhooks with price_alert event
  if (typeof fireWebhooks === "function") {
    fireWebhooks("price_alert", {
      symbol: alert.symbol,
      price: lastPrice,
      condition: `${alert.direction} ${alert.price.toFixed(4)}`,
    });
  }
}

// ── Dividend/Corporate Action Alerts ─────────────────────────
const DIVIDEND_ALERTS_KEY = "typhoon_dividend_alerts_seen";
let lastDividendCheck = 0;

function getSeenDividendAlerts() {
  try { return JSON.parse(localStorage.getItem(DIVIDEND_ALERTS_KEY) || "{}"); } catch { return {}; }
}
function markDividendAlertSeen(key) {
  const seen = getSeenDividendAlerts();
  seen[key] = Date.now();
  localStorage.setItem(DIVIDEND_ALERTS_KEY, JSON.stringify(seen));
}

async function checkDividendAlerts() {
  // Only check every 60s to avoid hammering the API
  const now = Date.now();
  if (now - lastDividendCheck < 60000) return;
  lastDividendCheck = now;

  if (!currentSymbol) return;
  try {
    const json = await invoke("get_corporate_actions", { symbol: currentSymbol, types: "dividend" });
    const actions = JSON.parse(json);
    if (!Array.isArray(actions) || actions.length === 0) return;

    const seen = getSeenDividendAlerts();
    const today = new Date();
    const fiveDaysMs = 5 * 24 * 60 * 60 * 1000;

    for (const action of actions) {
      const exDate = action.ex_date || action.ex_dividend_date || action.date;
      if (!exDate) continue;
      const exDateObj = new Date(exDate);
      const diff = exDateObj.getTime() - today.getTime();
      if (diff >= 0 && diff <= fiveDaysMs) {
        const alertKey = `${currentSymbol}_${exDate}`;
        if (seen[alertKey]) continue;
        const daysUntil = Math.ceil(diff / (24 * 60 * 60 * 1000));
        const amount = action.cash_amount || action.amount || "?";
        const msg = `${currentSymbol}: Ex-dividend in ${daysUntil} day(s) — $${amount}/share on ${exDate}`;
        log(`DIVIDEND ALERT: ${msg}`, "warn");
        try { new Notification(`${currentSymbol} Dividend Alert`, { body: msg }); } catch (_) {}
        markDividendAlertSeen(alertKey);
      }
    }
  } catch (_) {
    // Corporate actions API may not be available — fail silently
  }
}

// ── Multi-Condition Alerts ──────────────────────────────────
const MULTI_ALERTS_KEY = "typhoon_multi_alerts";
let multiConditionAlerts = [];

function loadMultiAlerts() {
  try { multiConditionAlerts = JSON.parse(localStorage.getItem(MULTI_ALERTS_KEY) || "[]"); } catch { multiConditionAlerts = []; }
}
function saveMultiAlerts() {
  localStorage.setItem(MULTI_ALERTS_KEY, JSON.stringify(multiConditionAlerts));
}

function addMultiConditionAlert(symbol, condition) {
  multiConditionAlerts.push({ symbol, condition, triggered: false, createdAt: Date.now() });
  saveMultiAlerts();
  log(`Multi-alert set: ${symbol} ${condition}`, "ok");
}

function evaluateCondition(condition, chartData) {
  if (!chartData || chartData.length < 200) return false;
  const lastBar = chartData[chartData.length - 1];

  // RSI conditions
  if (condition === "RSI > 70" || condition === "RSI < 30") {
    const rsi = calcRSI(chartData, 14);
    if (rsi.length === 0) return false;
    const lastRSI = rsi[rsi.length - 1].value;
    if (condition === "RSI > 70") return lastRSI > 70;
    if (condition === "RSI < 30") return lastRSI < 30;
  }

  // KAMA vs SMA200 conditions
  if (condition === "KAMA > SMA200" || condition === "KAMA < SMA200") {
    const kama = calcKAMA(chartData, 10);
    const sma = calcSMA(chartData, 200);
    if (kama.length === 0 || sma.length === 0) return false;
    const lastKAMA = kama[kama.length - 1].value;
    const lastSMA = sma[sma.length - 1].value;
    if (condition === "KAMA > SMA200") return lastKAMA > lastSMA;
    if (condition === "KAMA < SMA200") return lastKAMA < lastSMA;
  }

  // Fisher conditions
  if (condition === "Fisher > 0" || condition === "Fisher < 0") {
    const ef = calcEhlersFisher(chartData, 32);
    if (ef.fisher.length === 0) return false;
    const lastFisher = ef.fisher[ef.fisher.length - 1].value;
    if (condition === "Fisher > 0") return lastFisher > 0;
    if (condition === "Fisher < 0") return lastFisher < 0;
  }

  return false;
}

function checkMultiConditionAlerts() {
  if (multiConditionAlerts.length === 0 || !currentSymbol || currentChartData.length === 0) return;
  for (const alert of multiConditionAlerts) {
    if (alert.triggered || alert.symbol !== currentSymbol) continue;
    if (evaluateCondition(alert.condition, currentChartData)) {
      alert.triggered = true;
      log(`MULTI-ALERT: ${alert.symbol} ${alert.condition} triggered`, "warn");
      try { new Notification(`${alert.symbol} Alert`, { body: `Condition met: ${alert.condition}` }); } catch (_) {}
    }
  }
  saveMultiAlerts();
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

// ── Time & Sales Panel ──────────────────────────────────────

let tsWindow = null;
let tsTradeList = []; // capped at 200
let tsPollInterval = null;

function cmdTimeSales() {
  if (tsWindow) {
    try { tsWindow.close(); } catch (_) {}
  }
  tsTradeList = [];
  lastTradePrice = 0;

  tsWindow = createWindow({
    title: `${currentSymbol || "?"} — Time & Sales`,
    type: "custom",
    width: 380,
    height: 500,
    onClose: () => {
      if (tsPollInterval) { clearInterval(tsPollInterval); tsPollInterval = null; }
      tsWindow = null;
    },
  });

  // Header row
  const header = document.createElement("div");
  header.style.cssText = "display:flex;justify-content:space-between;padding:4px 8px;color:#888;font-size:10px;border-bottom:1px solid #333;font-family:Consolas,monospace;";
  const hTime = document.createElement("span"); hTime.textContent = "TIME";
  const hPrice = document.createElement("span"); hPrice.textContent = "PRICE";
  const hSize = document.createElement("span"); hSize.textContent = "SIZE";
  header.appendChild(hTime);
  header.appendChild(hPrice);
  header.appendChild(hSize);
  tsWindow.appendElement(header);

  const listEl = document.createElement("div");
  listEl.id = "ts-trade-list";
  listEl.style.cssText = "overflow-y:auto;max-height:calc(100% - 30px);font-family:Consolas,monospace;font-size:11px;";
  tsWindow.appendElement(listEl);

  // Poll stream for trades
  if (tsPollInterval) clearInterval(tsPollInterval);
  tsPollInterval = setInterval(async () => {
    try {
      const json = await invoke("poll_stream");
      const messages = JSON.parse(json);
      for (const msg of messages) {
        if (msg.Trade) {
          const t = msg.Trade;
          const isUp = t.price >= lastTradePrice;
          lastTradePrice = t.price;

          tsTradeList.unshift({
            time: t.timestamp ? t.timestamp.substring(11, 19) : "",
            price: t.price,
            size: t.size,
            up: isUp,
          });
          if (tsTradeList.length > 200) tsTradeList.length = 200;
        }
      }
      renderTSList(listEl);
    } catch (_) {}
  }, 250);
}

function renderTSList(listEl) {
  // Only re-render if there are new trades (check child count)
  while (listEl.children.length > tsTradeList.length) {
    listEl.removeChild(listEl.lastChild);
  }
  for (let i = 0; i < tsTradeList.length; i++) {
    const t = tsTradeList[i];
    let row = listEl.children[i];
    if (!row) {
      row = document.createElement("div");
      row.style.cssText = "display:flex;justify-content:space-between;padding:1px 8px;";
      const sTime = document.createElement("span"); sTime.className = "ts-time";
      sTime.style.cssText = "color:#888;min-width:65px;";
      const sPrice = document.createElement("span"); sPrice.className = "ts-price";
      sPrice.style.cssText = "min-width:80px;text-align:right;";
      const sSize = document.createElement("span"); sSize.className = "ts-size";
      sSize.style.cssText = "color:#ccc;min-width:60px;text-align:right;";
      row.appendChild(sTime);
      row.appendChild(sPrice);
      row.appendChild(sSize);
      listEl.appendChild(row);
    }
    row.children[0].textContent = t.time;
    row.children[1].textContent = t.price.toFixed(4);
    row.children[1].style.color = t.up ? "#4caf50" : "#f44336";
    row.children[2].textContent = t.size.toFixed(2);
  }
}

// ── Account Activities Window ───────────────────────────────

async function cmdActivities() {
  const win = createWindow({
    title: "Account Activities",
    type: "custom",
    width: 550,
    height: 500,
  });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Loading activities...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  try {
    const json = await invoke("get_account_activities", { activityTypes: "", limit: 100 });
    const activities = JSON.parse(json);
    win.contentElement.textContent = "";

    if (!activities || activities.length === 0) {
      win.setContent("No activities found.");
      return;
    }

    // Category filter buttons
    const filterBar = document.createElement("div");
    filterBar.style.cssText = "display:flex;gap:4px;padding:6px 0;border-bottom:1px solid #333;flex-wrap:wrap;";
    const categories = ["ALL", "FILL", "DIV", "CSD", "CSW"];
    let activeFilter = "ALL";
    const filterBtns = [];

    for (const cat of categories) {
      const btn = document.createElement("button");
      btn.textContent = cat;
      btn.style.cssText = "background:#1a1a2e;border:1px solid #333;color:#aaa;font-size:10px;padding:2px 8px;cursor:pointer;font-family:Consolas,monospace;border-radius:2px;";
      if (cat === "ALL") btn.style.color = "#fff";
      btn.addEventListener("click", () => {
        activeFilter = cat;
        filterBtns.forEach(b => b.style.color = "#aaa");
        btn.style.color = "#fff";
        renderActivities();
      });
      filterBtns.push(btn);
      filterBar.appendChild(btn);
    }
    win.appendElement(filterBar);

    const listEl = document.createElement("div");
    listEl.style.cssText = "overflow-y:auto;max-height:calc(100% - 50px);";
    win.appendElement(listEl);

    function renderActivities() {
      listEl.textContent = "";
      const filtered = activeFilter === "ALL" ? activities : activities.filter(a => a.activity_type === activeFilter);
      for (const a of filtered) {
        const row = document.createElement("div");
        row.style.cssText = "display:flex;justify-content:space-between;padding:4px 0;border-bottom:1px solid #111;font-size:11px;";

        const left = document.createElement("div");
        left.style.cssText = "display:flex;flex-direction:column;";
        const desc = document.createElement("span");
        desc.style.color = "#ccc";
        desc.textContent = a.description;
        const date = document.createElement("span");
        date.style.cssText = "color:#666;font-size:9px;";
        date.textContent = (a.date || "").substring(0, 19).replace("T", " ");
        left.appendChild(desc);
        left.appendChild(date);

        const badge = document.createElement("span");
        badge.style.cssText = "font-size:9px;padding:2px 6px;border-radius:2px;align-self:center;";
        const typeColors = { FILL: "#2196f3", DIV: "#4caf50", CSD: "#ff9800", CSW: "#f44336" };
        badge.style.background = typeColors[a.activity_type] || "#555";
        badge.style.color = "#fff";
        badge.textContent = a.activity_type;

        row.appendChild(left);
        row.appendChild(badge);
        listEl.appendChild(row);
      }
    }
    renderActivities();
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to load activities: ${e}`);
  }
}

// ── Insider Trading Window ──────────────────────────────────

async function cmdInsider() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({
    title: `${currentSymbol} — Insider Trading (Form 4)`,
    type: "custom",
    width: 600,
    height: 450,
  });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Fetching SEC Form 4 filings...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  try {
    const json = await invoke("get_insider_trades", { symbol: currentSymbol });
    const trades = JSON.parse(json);
    win.contentElement.textContent = "";

    if (!trades || trades.length === 0) {
      win.setContent("No insider trading filings found for this symbol.");
      return;
    }

    // Table header
    const header = document.createElement("div");
    header.style.cssText = "display:flex;gap:8px;padding:6px 0;border-bottom:1px solid #333;color:#888;font-size:10px;font-family:Consolas,monospace;";
    const cols = ["Date", "Owner", "Type", "Link"];
    for (const c of cols) {
      const span = document.createElement("span");
      span.textContent = c;
      span.style.flex = c === "Owner" ? "2" : "1";
      header.appendChild(span);
    }
    win.appendElement(header);

    const listEl = document.createElement("div");
    listEl.style.cssText = "overflow-y:auto;max-height:calc(100% - 40px);";
    win.appendElement(listEl);

    for (const t of trades) {
      const row = document.createElement("div");
      row.style.cssText = "display:flex;gap:8px;padding:3px 0;border-bottom:1px solid #111;font-size:11px;color:#ccc;";

      const date = document.createElement("span");
      date.style.flex = "1";
      date.textContent = t.filing_date;

      const owner = document.createElement("span");
      owner.style.flex = "2";
      owner.textContent = t.owner_name || "—";
      owner.style.cssText += "overflow:hidden;text-overflow:ellipsis;white-space:nowrap;";

      const type = document.createElement("span");
      type.style.flex = "1";
      type.textContent = t.transaction_type;

      const link = document.createElement("span");
      link.style.flex = "1";
      if (t.form_url) {
        const a = document.createElement("a");
        a.textContent = "View";
        a.style.cssText = "color:#2196f3;cursor:pointer;text-decoration:underline;font-size:10px;";
        a.addEventListener("click", () => {
          openArticleInline(t.form_url, `Form 4 — ${t.owner_name}`);
        });
        link.appendChild(a);
      }

      row.appendChild(date);
      row.appendChild(owner);
      row.appendChild(type);
      row.appendChild(link);
      listEl.appendChild(row);
    }
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to load insider trades: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// BREADTH — Market Breadth Dashboard
// ══════════════════════════════════════════════════════════════
async function cmdBreadth() {
  const win = createWindow({ title: "Market Breadth Dashboard", width: 500, height: 450 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Scanning watchlist for market breadth...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);
  try {
    const watchlist = getWatchlist();
    if (watchlist.length === 0) { win.contentElement.textContent = ""; win.setContent("No watchlist symbols. Add symbols via QM (Quote Monitor) first."); return; }
    let aboveSMA200 = 0, aboveSMA50 = 0, advances = 0, declines = 0, totalChange = 0, validCount = 0;
    const details = [];
    for (const sym of watchlist) {
      let data = null;
      const cKey = `${sym}:1Day`;
      const cached = barCache[cKey];
      if (cached && cached.data && cached.data.length > 200) data = cached.data;
      if (!data) { try { const barsJson = await invoke("get_bars", { symbol: sym, timeframe: "1Day", limit: 300 }); const bars = JSON.parse(barsJson); if (bars.length > 50) { data = bars; barCache[cKey] = { data: bars, timestamp: Date.now(), lastAccess: Date.now() }; } } catch (_) {} }
      if (!data || data.length < 2) continue;
      validCount++;
      const close = data[data.length - 1].close, prevClose = data[data.length - 2].close;
      const dayChange = ((close - prevClose) / prevClose) * 100;
      totalChange += dayChange;
      if (dayChange >= 0) advances++; else declines++;
      let sma200Val = null;
      if (data.length >= 200) { let s = 0; for (let i = data.length - 200; i < data.length; i++) s += data[i].close; sma200Val = s / 200; if (close > sma200Val) aboveSMA200++; }
      let sma50Val = null;
      if (data.length >= 50) { let s = 0; for (let i = data.length - 50; i < data.length; i++) s += data[i].close; sma50Val = s / 50; if (close > sma50Val) aboveSMA50++; }
      details.push({ sym, close, dayChange, abv200: sma200Val ? close > sma200Val : null, abv50: sma50Val ? close > sma50Val : null });
    }
    if (validCount === 0) { win.contentElement.textContent = ""; win.setContent("No bar data available for watchlist symbols. Load some charts first."); return; }
    const pctAbove200 = (aboveSMA200 / validCount) * 100, pctAbove50 = (aboveSMA50 / validCount) * 100;
    const avgChange = totalChange / validCount;
    const assessment = pctAbove200 > 60 ? "BULLISH" : pctAbove200 < 40 ? "BEARISH" : "MIXED";
    const assessColor = assessment === "BULLISH" ? "#4caf50" : assessment === "BEARISH" ? "#f44336" : "#ff9800";
    win.contentElement.textContent = "";
    const container = document.createElement("div");
    container.style.cssText = "padding:12px;font-family:monospace;font-size:13px;color:#ddd;overflow-y:auto;height:100%;";
    const header = document.createElement("div");
    header.style.cssText = `text-align:center;font-size:20px;font-weight:bold;color:${assessColor};margin-bottom:16px;`;
    header.textContent = `Market: ${assessment}`;
    container.appendChild(header);
    function makeProgressBar(label, pct, color) {
      const row = document.createElement("div"); row.style.cssText = "margin-bottom:12px;";
      const lbl = document.createElement("div"); lbl.style.cssText = "margin-bottom:4px;";
      lbl.appendChild(document.createTextNode(label + ": "));
      const lblVal = document.createElement("span"); lblVal.style.cssText = `color:${color};font-weight:bold`; lblVal.textContent = pct.toFixed(1) + "%"; lbl.appendChild(lblVal);
      row.appendChild(lbl);
      const track = document.createElement("div"); track.style.cssText = "background:#333;border-radius:4px;height:16px;overflow:hidden;";
      const fill = document.createElement("div"); fill.style.cssText = `background:${color};height:100%;width:${Math.min(pct, 100)}%;border-radius:4px;transition:width 0.3s;`;
      track.appendChild(fill); row.appendChild(track); return row;
    }
    const bc200 = pctAbove200 > 60 ? "#4caf50" : pctAbove200 < 40 ? "#f44336" : "#ff9800";
    const bc50 = pctAbove50 > 60 ? "#4caf50" : pctAbove50 < 40 ? "#f44336" : "#ff9800";
    container.appendChild(makeProgressBar("Above SMA 200 (long-term health)", pctAbove200, bc200));
    container.appendChild(makeProgressBar("Above SMA 50 (medium-term momentum)", pctAbove50, bc50));
    const adLine = document.createElement("div"); adLine.style.cssText = "margin-bottom:12px;display:flex;justify-content:space-between;";
    const adLabel = document.createElement("span"); adLabel.textContent = "Advance/Decline:"; adLine.appendChild(adLabel);
    adLine.appendChild(document.createTextNode(" "));
    const adAdv = document.createElement("span"); adAdv.style.color = "#4caf50"; adAdv.textContent = advances + " Adv"; adLine.appendChild(adAdv);
    adLine.appendChild(document.createTextNode(" / "));
    const adDec = document.createElement("span"); adDec.style.color = "#f44336"; adDec.textContent = declines + " Dec"; adLine.appendChild(adDec);
    adLine.appendChild(document.createTextNode(" "));
    const adRatio = document.createElement("span"); adRatio.style.color = "#888"; adRatio.textContent = "(Ratio: " + (declines > 0 ? (advances / declines).toFixed(2) : "N/A") + ")"; adLine.appendChild(adRatio);
    container.appendChild(adLine);
    const avgLine = document.createElement("div"); avgLine.style.cssText = "margin-bottom:16px;";
    const avgColor = avgChange >= 0 ? "#4caf50" : "#f44336";
    avgLine.appendChild(document.createTextNode("Avg 1-Day Change: "));
    const avgVal = document.createElement("span"); avgVal.style.cssText = `color:${avgColor};font-weight:bold`; avgVal.textContent = (avgChange >= 0 ? "+" : "") + avgChange.toFixed(2) + "%"; avgLine.appendChild(avgVal);
    container.appendChild(avgLine);
    const table = document.createElement("table"); table.style.cssText = "width:100%;border-collapse:collapse;font-size:12px;";
    const thead = document.createElement("thead");
    const headTr = document.createElement("tr"); headTr.style.cssText = "color:#888;border-bottom:1px solid #444;";
    for (const [text, align] of [["Symbol","left"],["Close","right"],["Chg%","right"],["SMA200","center"],["SMA50","center"]]) { const th = document.createElement("th"); th.style.cssText = `text-align:${align};padding:4px;`; th.textContent = text; headTr.appendChild(th); }
    thead.appendChild(headTr); table.appendChild(thead);
    const tbody = document.createElement("tbody");
    for (const d of details) {
      const tr = document.createElement("tr"); tr.style.cssText = "border-bottom:1px solid #333;";
      const chgC = d.dayChange >= 0 ? "#4caf50" : "#f44336";
      const tdSym = document.createElement("td"); tdSym.style.padding = "4px"; tdSym.textContent = d.sym; tr.appendChild(tdSym);
      const tdClose = document.createElement("td"); tdClose.style.cssText = "text-align:right;padding:4px;"; tdClose.textContent = "$" + d.close.toFixed(2); tr.appendChild(tdClose);
      const tdChg = document.createElement("td"); tdChg.style.cssText = `text-align:right;padding:4px;color:${chgC}`; tdChg.textContent = (d.dayChange >= 0 ? "+" : "") + d.dayChange.toFixed(2) + "%"; tr.appendChild(tdChg);
      const tdS200 = document.createElement("td"); tdS200.style.cssText = "text-align:center;padding:4px;";
      if (d.abv200 === null) { tdS200.textContent = "-"; } else { const arrow = document.createElement("span"); arrow.style.color = d.abv200 ? "#4caf50" : "#f44336"; arrow.textContent = d.abv200 ? "\u25B2" : "\u25BC"; tdS200.appendChild(arrow); }
      tr.appendChild(tdS200);
      const tdS50 = document.createElement("td"); tdS50.style.cssText = "text-align:center;padding:4px;";
      if (d.abv50 === null) { tdS50.textContent = "-"; } else { const arrow = document.createElement("span"); arrow.style.color = d.abv50 ? "#4caf50" : "#f44336"; arrow.textContent = d.abv50 ? "\u25B2" : "\u25BC"; tdS50.appendChild(arrow); }
      tr.appendChild(tdS50);
      tbody.appendChild(tr);
    }
    table.appendChild(tbody); container.appendChild(table);
    win.appendElement(container);
  } catch (e) { win.contentElement.textContent = ""; win.setContent(`Failed to compute breadth: ${e}`); }
}

// ══════════════════════════════════════════════════════════════
// DIVERGENCE — Indicator Divergence Scanner
// ══════════════════════════════════════════════════════════════
function cmdDivergence() {
  if (!currentChartData || currentChartData.length < 55) { log("Need at least 55 bars loaded for divergence scan", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Divergence Scanner`, width: 600, height: 450 });
  win.contentElement.textContent = "";
  const fisherResult = calcEhlersFisher(currentChartData, 32);
  const rsiResult = calcRSI(currentChartData, 14);
  const totalBars = currentChartData.length;
  const scanStart = Math.max(0, totalBars - 50), scanEnd = totalBars;
  const fisherByTime = {}; for (const f of fisherResult.fisher) fisherByTime[f.time] = f.value;
  const rsiByTime = {}; for (const r of rsiResult) rsiByTime[r.time] = r.value;
  function findSwingHighs(si, ei) {
    const sw = [];
    for (let i = si + 2; i < ei - 2; i++) { const b = currentChartData[i]; if (b.high > currentChartData[i-1].high && b.high > currentChartData[i-2].high && b.high > currentChartData[i+1].high && b.high > currentChartData[i+2].high) sw.push({ idx: i, price: b.high, time: b.time }); }
    return sw;
  }
  function findSwingLows(si, ei) {
    const sw = [];
    for (let i = si + 2; i < ei - 2; i++) { const b = currentChartData[i]; if (b.low < currentChartData[i-1].low && b.low < currentChartData[i-2].low && b.low < currentChartData[i+1].low && b.low < currentChartData[i+2].low) sw.push({ idx: i, price: b.low, time: b.time }); }
    return sw;
  }
  const swingHighs = findSwingHighs(scanStart, scanEnd);
  const swingLows = findSwingLows(scanStart, scanEnd);
  const divergences = [];
  for (let i = 1; i < swingHighs.length; i++) {
    const prev = swingHighs[i-1], curr = swingHighs[i];
    if (curr.price > prev.price) {
      const pF = fisherByTime[prev.time], cF = fisherByTime[curr.time];
      if (pF !== undefined && cF !== undefined && cF < pF) divergences.push({ barIdx: curr.idx, type: "Bearish", indicator: "Fisher", priceLevel: curr.price, indLevel: cF, time: curr.time });
      const pR = rsiByTime[prev.time], cR = rsiByTime[curr.time];
      if (pR !== undefined && cR !== undefined && cR < pR) divergences.push({ barIdx: curr.idx, type: "Bearish", indicator: "RSI", priceLevel: curr.price, indLevel: cR, time: curr.time });
    }
  }
  for (let i = 1; i < swingLows.length; i++) {
    const prev = swingLows[i-1], curr = swingLows[i];
    if (curr.price < prev.price) {
      const pF = fisherByTime[prev.time], cF = fisherByTime[curr.time];
      if (pF !== undefined && cF !== undefined && cF > pF) divergences.push({ barIdx: curr.idx, type: "Bullish", indicator: "Fisher", priceLevel: curr.price, indLevel: cF, time: curr.time });
      const pR = rsiByTime[prev.time], cR = rsiByTime[curr.time];
      if (pR !== undefined && cR !== undefined && cR > pR) divergences.push({ barIdx: curr.idx, type: "Bullish", indicator: "RSI", priceLevel: curr.price, indLevel: cR, time: curr.time });
    }
  }
  divergences.sort((a, b) => a.barIdx - b.barIdx);
  const divMarkers = divergences.map(d => ({ time: d.time, position: d.type === "Bullish" ? "belowBar" : "aboveBar", color: d.type === "Bullish" ? "#4caf50" : "#f44336", shape: d.type === "Bullish" ? "arrowUp" : "arrowDown", text: `${d.type[0]}-${d.indicator}` }));
  const seenKeys = new Set(); const uniqueMarkers = [];
  for (const m of divMarkers) { const key = `${m.time}-${m.text}`; if (!seenKeys.has(key)) { seenKeys.add(key); uniqueMarkers.push(m); } }
  uniqueMarkers.sort((a, b) => a.time - b.time);
  try { candleSeries.setMarkers(uniqueMarkers); } catch (_) {}
  const container = document.createElement("div"); container.style.cssText = "padding:12px;font-family:monospace;font-size:13px;color:#ddd;overflow-y:auto;height:100%;";
  const summary = document.createElement("div"); summary.style.cssText = "margin-bottom:12px;";
  const bullCount = divergences.filter(d => d.type === "Bullish").length;
  const bearCount = divergences.filter(d => d.type === "Bearish").length;
  const summaryB = document.createElement("b"); summaryB.textContent = divergences.length; summary.appendChild(document.createTextNode("Found ")); summary.appendChild(summaryB); summary.appendChild(document.createTextNode(" divergences (last 50 bars): "));
  const summaryBull = document.createElement("span"); summaryBull.style.color = "#4caf50"; summaryBull.textContent = bullCount + " Bullish"; summary.appendChild(summaryBull);
  summary.appendChild(document.createTextNode(" | "));
  const summaryBear = document.createElement("span"); summaryBear.style.color = "#f44336"; summaryBear.textContent = bearCount + " Bearish"; summary.appendChild(summaryBear);
  container.appendChild(summary);
  if (divergences.length === 0) {
    const none = document.createElement("div"); none.style.cssText = "color:#888;padding:20px;text-align:center;"; none.textContent = "No divergences detected in the scan window."; container.appendChild(none);
  } else {
    const table = document.createElement("table"); table.style.cssText = "width:100%;border-collapse:collapse;font-size:12px;";
    const dThead = document.createElement("thead");
    const dHeadTr = document.createElement("tr"); dHeadTr.style.cssText = "color:#888;border-bottom:1px solid #444;";
    for (const [text, align] of [["Bar#","left"],["Type","left"],["Indicator","left"],["Price","right"],["Ind Value","right"]]) { const th = document.createElement("th"); th.style.cssText = `text-align:${align};padding:4px;`; th.textContent = text; dHeadTr.appendChild(th); }
    dThead.appendChild(dHeadTr); table.appendChild(dThead);
    const tbody = document.createElement("tbody");
    for (const d of divergences) {
      const tr = document.createElement("tr"); tr.style.cssText = "border-bottom:1px solid #333;";
      const tc = d.type === "Bullish" ? "#4caf50" : "#f44336";
      const tdBar = document.createElement("td"); tdBar.style.padding = "4px"; tdBar.textContent = d.barIdx; tr.appendChild(tdBar);
      const tdType = document.createElement("td"); tdType.style.cssText = `padding:4px;color:${tc};font-weight:bold`; tdType.textContent = d.type; tr.appendChild(tdType);
      const tdInd = document.createElement("td"); tdInd.style.padding = "4px"; tdInd.textContent = d.indicator; tr.appendChild(tdInd);
      const tdPrice = document.createElement("td"); tdPrice.style.cssText = "text-align:right;padding:4px;"; tdPrice.textContent = "$" + d.priceLevel.toFixed(2); tr.appendChild(tdPrice);
      const tdVal = document.createElement("td"); tdVal.style.cssText = "text-align:right;padding:4px;"; tdVal.textContent = d.indLevel.toFixed(2); tr.appendChild(tdVal);
      tbody.appendChild(tr);
    }
    table.appendChild(tbody); container.appendChild(table);
  }
  const clearBtn = document.createElement("button"); clearBtn.textContent = "Clear Markers";
  clearBtn.style.cssText = "margin-top:12px;padding:6px 16px;background:#333;color:#ddd;border:1px solid #555;border-radius:4px;cursor:pointer;";
  clearBtn.addEventListener("click", () => { candleSeries.setMarkers([]); log("Divergence markers cleared", "info"); });
  container.appendChild(clearBtn);
  win.appendElement(container);
}

// ── REPLAY — Bar-by-Bar Practice Trading ────────────────────

let replayIndex = 100;
let replaySpeed = 1;
let replayRunning = false;
let replayTrades = [];
let replayInterval = null;
let replayOpenTrade = null;
let replaySavedData = null;

function cmdReplay() {
  if (!currentChartData || currentChartData.length < 120) {
    log("REPLAY: Need at least 120 bars loaded. Load a chart first.", "warn");
    return;
  }

  replaySavedData = currentChartData.slice();
  replayIndex = 100;
  replaySpeed = 1;
  replayRunning = false;
  replayTrades = [];
  replayOpenTrade = null;

  const dp = lastPrice > 100 ? 2 : lastPrice > 1 ? 4 : 6;

  function cleanup() {
    replayRunning = false;
    if (replayInterval) { clearInterval(replayInterval); replayInterval = null; }
    if (replaySavedData) {
      currentChartData = replaySavedData;
      candleSeries.setData(currentChartData);
      applyIndicators(currentChartData);
      candleSeries.setMarkers([]);
      replaySavedData = null;
    }
    replayOpenTrade = null;
    log("REPLAY: Session ended, chart restored.", "info");
  }

  const win = createWindow({
    title: `REPLAY — ${currentSymbol} Practice`,
    width: 380,
    height: 520,
    onClose: cleanup,
  });
  win.contentElement.textContent = "";
  win.contentElement.style.cssText = "padding:10px;font-family:Consolas,monospace;font-size:11px;color:#ccc;display:flex;flex-direction:column;gap:8px;";

  const statusEl = document.createElement("div");
  statusEl.style.cssText = "background:#111;border:1px solid #333;padding:8px;border-radius:4px;";
  statusEl.innerHTML = '<div style="color:#888;font-size:10px;margin-bottom:4px;">STATUS</div>';
  const barLabel = document.createElement("div");
  barLabel.textContent = `Bar: ${replayIndex} / ${currentChartData.length}`;
  const priceLabel = document.createElement("div");
  priceLabel.style.cssText = "font-size:14px;font-weight:bold;color:#ffeb3b;margin:4px 0;";
  const stateLabel = document.createElement("div");
  stateLabel.style.color = "#888";
  stateLabel.textContent = "PAUSED";
  statusEl.appendChild(barLabel);
  statusEl.appendChild(priceLabel);
  statusEl.appendChild(stateLabel);
  win.appendElement(statusEl);

  const transport = document.createElement("div");
  transport.style.cssText = "display:flex;gap:6px;align-items:center;";

  const makeBtn = (text, color, onClick) => {
    const btn = document.createElement("button");
    btn.textContent = text;
    btn.style.cssText = `padding:6px 12px;background:${color};border:1px solid #555;color:#fff;cursor:pointer;border-radius:3px;font-family:Consolas,monospace;font-size:11px;`;
    btn.addEventListener("click", onClick);
    return btn;
  };

  const playBtn = makeBtn("Play", "#1b5e20", () => {
    replayRunning = !replayRunning;
    playBtn.textContent = replayRunning ? "Pause" : "Play";
    playBtn.style.background = replayRunning ? "#b71c1c" : "#1b5e20";
    stateLabel.textContent = replayRunning ? `PLAYING ${replaySpeed}x` : "PAUSED";
    stateLabel.style.color = replayRunning ? "#4caf50" : "#888";
    if (replayRunning) startReplayInterval();
    else stopReplayInterval();
  });

  const stepBtn = makeBtn("Step >", "#333", () => {
    if (!replayRunning) advanceReplay();
  });

  const speedSel = document.createElement("select");
  speedSel.style.cssText = "padding:4px;background:#222;border:1px solid #555;color:#ccc;font-family:Consolas,monospace;font-size:11px;border-radius:3px;";
  for (const spd of [1, 5, 25, 100]) {
    const opt = document.createElement("option");
    opt.value = spd;
    opt.textContent = `${spd}x`;
    if (spd === 1) opt.selected = true;
    speedSel.appendChild(opt);
  }
  speedSel.addEventListener("change", () => {
    replaySpeed = parseInt(speedSel.value);
    stateLabel.textContent = replayRunning ? `PLAYING ${replaySpeed}x` : "PAUSED";
    if (replayRunning) { stopReplayInterval(); startReplayInterval(); }
  });

  transport.appendChild(playBtn);
  transport.appendChild(stepBtn);
  transport.appendChild(speedSel);
  win.appendElement(transport);

  const tradingDiv = document.createElement("div");
  tradingDiv.style.cssText = "display:flex;gap:6px;flex-wrap:wrap;";

  const buyBtn = makeBtn("BUY", "#1b5e20", () => {
    if (replayOpenTrade) { log("REPLAY: Already in a trade. Close first.", "warn"); return; }
    const price = currentChartData[replayIndex - 1].close;
    replayOpenTrade = { side: "buy", entryPrice: price, entryBar: replayIndex - 1 };
    posLabel.textContent = `LONG @ ${price.toFixed(dp)}`;
    posLabel.style.color = "#4caf50";
    log(`REPLAY: BUY @ ${price.toFixed(dp)}`, "ok");
    updateReplayMarkers();
  });

  const sellBtn = makeBtn("SELL", "#b71c1c", () => {
    if (replayOpenTrade) { log("REPLAY: Already in a trade. Close first.", "warn"); return; }
    const price = currentChartData[replayIndex - 1].close;
    replayOpenTrade = { side: "sell", entryPrice: price, entryBar: replayIndex - 1 };
    posLabel.textContent = `SHORT @ ${price.toFixed(dp)}`;
    posLabel.style.color = "#f44336";
    log(`REPLAY: SELL @ ${price.toFixed(dp)}`, "ok");
    updateReplayMarkers();
  });

  const closeBtn = makeBtn("Close Position", "#4a148c", () => {
    if (!replayOpenTrade) { log("REPLAY: No open trade.", "warn"); return; }
    const exitPrice = currentChartData[replayIndex - 1].close;
    const pnl = replayOpenTrade.side === "buy"
      ? exitPrice - replayOpenTrade.entryPrice
      : replayOpenTrade.entryPrice - exitPrice;
    const trade = {
      side: replayOpenTrade.side,
      entryPrice: replayOpenTrade.entryPrice,
      entryBar: replayOpenTrade.entryBar,
      exitPrice,
      exitBar: replayIndex - 1,
      pnl,
    };
    replayTrades.push(trade);
    log(`REPLAY: Closed ${trade.side.toUpperCase()} — P&L: ${pnl >= 0 ? "+" : ""}${pnl.toFixed(dp)}`, pnl >= 0 ? "ok" : "warn");
    replayOpenTrade = null;
    posLabel.textContent = "No position";
    posLabel.style.color = "#888";
    updatePnL();
    updateReplayMarkers();
  });

  const resetBtn = makeBtn("Reset", "#333", () => {
    replayRunning = false;
    stopReplayInterval();
    replayIndex = 100;
    replayTrades = [];
    replayOpenTrade = null;
    playBtn.textContent = "Play";
    playBtn.style.background = "#1b5e20";
    stateLabel.textContent = "PAUSED";
    stateLabel.style.color = "#888";
    posLabel.textContent = "No position";
    posLabel.style.color = "#888";
    updatePnL();
    advanceReplay();
    log("REPLAY: Reset to bar 100.", "info");
  });

  tradingDiv.appendChild(buyBtn);
  tradingDiv.appendChild(sellBtn);
  tradingDiv.appendChild(closeBtn);
  tradingDiv.appendChild(resetBtn);
  win.appendElement(tradingDiv);

  const posDiv = document.createElement("div");
  posDiv.style.cssText = "background:#111;border:1px solid #333;padding:8px;border-radius:4px;";
  posDiv.innerHTML = '<div style="color:#888;font-size:10px;margin-bottom:4px;">POSITION</div>';
  const posLabel = document.createElement("div");
  posLabel.textContent = "No position";
  posLabel.style.color = "#888";
  const unrealizedLabel = document.createElement("div");
  unrealizedLabel.style.cssText = "font-size:10px;margin-top:4px;";
  posDiv.appendChild(posLabel);
  posDiv.appendChild(unrealizedLabel);
  win.appendElement(posDiv);

  const pnlDiv = document.createElement("div");
  pnlDiv.style.cssText = "background:#111;border:1px solid #333;padding:8px;border-radius:4px;";
  pnlDiv.innerHTML = '<div style="color:#888;font-size:10px;margin-bottom:4px;">PERFORMANCE</div>';
  const pnlLabel = document.createElement("div");
  pnlLabel.style.cssText = "font-size:14px;font-weight:bold;";
  pnlLabel.textContent = "P&L: 0.00";
  const statsLabel = document.createElement("div");
  statsLabel.style.cssText = "font-size:10px;color:#888;margin-top:4px;";
  statsLabel.textContent = "Trades: 0 | Win: 0 | Loss: 0 | Win%: —";
  pnlDiv.appendChild(pnlLabel);
  pnlDiv.appendChild(statsLabel);
  win.appendElement(pnlDiv);

  const logDiv = document.createElement("div");
  logDiv.style.cssText = "background:#111;border:1px solid #333;padding:8px;border-radius:4px;flex:1;overflow-y:auto;max-height:120px;";
  logDiv.innerHTML = '<div style="color:#888;font-size:10px;margin-bottom:4px;">TRADE LOG</div>';
  const logList = document.createElement("div");
  logList.style.cssText = "font-size:10px;";
  logDiv.appendChild(logList);
  win.appendElement(logDiv);

  function updatePnL() {
    const totalPnL = replayTrades.reduce((sum, t) => sum + t.pnl, 0);
    const wins = replayTrades.filter(t => t.pnl > 0).length;
    const losses = replayTrades.filter(t => t.pnl <= 0).length;
    const winRate = replayTrades.length > 0 ? ((wins / replayTrades.length) * 100).toFixed(1) : "—";
    pnlLabel.textContent = `P&L: ${totalPnL >= 0 ? "+" : ""}${totalPnL.toFixed(dp)}`;
    pnlLabel.style.color = totalPnL >= 0 ? "#4caf50" : "#f44336";
    statsLabel.textContent = `Trades: ${replayTrades.length} | Win: ${wins} | Loss: ${losses} | Win%: ${winRate}`;
    logList.textContent = "";
    for (const t of replayTrades.slice().reverse()) {
      const entry = document.createElement("div");
      entry.style.cssText = `color:${t.pnl >= 0 ? "#4caf50" : "#f44336"};margin:2px 0;`;
      entry.textContent = `${t.side.toUpperCase()} ${t.entryPrice.toFixed(dp)} -> ${t.exitPrice.toFixed(dp)} = ${t.pnl >= 0 ? "+" : ""}${t.pnl.toFixed(dp)}`;
      logList.appendChild(entry);
    }
  }

  function updateReplayMarkers() {
    const markers = [];
    for (const t of replayTrades) {
      markers.push({
        time: currentChartData[t.entryBar].time,
        position: t.side === "buy" ? "belowBar" : "aboveBar",
        color: t.side === "buy" ? "#4caf50" : "#f44336",
        shape: t.side === "buy" ? "arrowUp" : "arrowDown",
        text: `${t.side === "buy" ? "B" : "S"} ${t.entryPrice.toFixed(dp)}`,
      });
      markers.push({
        time: currentChartData[t.exitBar].time,
        position: "aboveBar",
        color: t.pnl >= 0 ? "#4caf50" : "#f44336",
        shape: "circle",
        text: `X ${t.pnl >= 0 ? "+" : ""}${t.pnl.toFixed(dp)}`,
      });
    }
    if (replayOpenTrade) {
      markers.push({
        time: currentChartData[replayOpenTrade.entryBar].time,
        position: replayOpenTrade.side === "buy" ? "belowBar" : "aboveBar",
        color: replayOpenTrade.side === "buy" ? "#4caf50" : "#f44336",
        shape: replayOpenTrade.side === "buy" ? "arrowUp" : "arrowDown",
        text: `${replayOpenTrade.side === "buy" ? "B" : "S"} ${replayOpenTrade.entryPrice.toFixed(dp)}`,
      });
    }
    markers.sort((a, b) => a.time - b.time);
    try { candleSeries.setMarkers(markers); } catch (_) {}
  }

  function advanceReplay() {
    if (replayIndex >= replaySavedData.length) {
      replayRunning = false;
      stopReplayInterval();
      playBtn.textContent = "Play";
      playBtn.style.background = "#1b5e20";
      stateLabel.textContent = "FINISHED";
      stateLabel.style.color = "#ffeb3b";
      log("REPLAY: Reached end of data.", "info");
      return;
    }
    const slice = replaySavedData.slice(0, replayIndex);
    currentChartData = slice;
    candleSeries.setData(slice);
    applyIndicators(slice);
    const bar = slice[slice.length - 1];
    barLabel.textContent = `Bar: ${replayIndex} / ${replaySavedData.length}`;
    priceLabel.textContent = `${bar.close.toFixed(dp)}`;
    if (replayOpenTrade) {
      const unrealized = replayOpenTrade.side === "buy"
        ? bar.close - replayOpenTrade.entryPrice
        : replayOpenTrade.entryPrice - bar.close;
      unrealizedLabel.textContent = `Unrealized: ${unrealized >= 0 ? "+" : ""}${unrealized.toFixed(dp)}`;
      unrealizedLabel.style.color = unrealized >= 0 ? "#4caf50" : "#f44336";
    } else {
      unrealizedLabel.textContent = "";
    }
    updateReplayMarkers();
    replayIndex++;
  }

  function startReplayInterval() {
    stopReplayInterval();
    const baseMs = 500;
    const interval = Math.max(10, Math.round(baseMs / replaySpeed));
    replayInterval = setInterval(() => {
      if (!replayRunning) { stopReplayInterval(); return; }
      advanceReplay();
    }, interval);
  }

  function stopReplayInterval() {
    if (replayInterval) { clearInterval(replayInterval); replayInterval = null; }
  }

  advanceReplay();
  log(`REPLAY: Started on ${currentSymbol} — ${replaySavedData.length} bars available.`, "ok");
}

// ══════════════════════════════════════════════════════════════
// RISKMAP — Portfolio Risk Heatmap (treemap-style)
// ══════════════════════════════════════════════════════════════

async function cmdRiskMap() {
  const win = createWindow({ title: "Portfolio Risk Heatmap", width: 700, height: 500 });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Loading positions and calculating VaR...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);

  try {
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);

    if (!positions || positions.length === 0) {
      win.contentElement.textContent = "";
      win.setContent("No open positions for risk heatmap.");
      return;
    }

    // Compute market value and weight for each position
    let totalValue = 0;
    const items = [];
    for (const p of positions) {
      const qty = Math.abs(parseFloat(p.qty) || 0);
      const price = parseFloat(p.current_price) || 0;
      const mv = Math.abs(parseFloat(p.market_value) || qty * price);
      totalValue += mv;
      items.push({ symbol: p.symbol, qty, price, mv, var_dollars: null });
    }

    // Fetch VaR for each position (best-effort, non-blocking)
    const varPromises = items.map(item =>
      invokeQuiet("calculate_position_var", {
        symbol: item.symbol, positionSize: item.qty, currentPrice: item.price,
      }).then(json => {
        const v = JSON.parse(json);
        item.var_dollars = v.var_dollars || 0;
      }).catch(() => { item.var_dollars = null; })
    );
    await Promise.all(varPromises);

    const totalVaR = items.reduce((s, i) => s + (i.var_dollars || 0), 0);
    let maxWeight = 0;
    let maxWeightSymbol = "";
    for (const item of items) {
      item.weight = totalValue > 0 ? item.mv / totalValue : 1 / items.length;
      if (item.weight > maxWeight) { maxWeight = item.weight; maxWeightSymbol = item.symbol; }
    }

    // Find max VaR for color scaling
    const maxVaR = Math.max(...items.map(i => Math.abs(i.var_dollars || 0)), 1);

    win.contentElement.textContent = "";

    // Build treemap-style heatmap grid
    const grid = document.createElement("div");
    grid.style.cssText = "display:flex;flex-wrap:wrap;gap:3px;padding:8px;min-height:200px;";

    for (const item of items) {
      // Size proportional to weight (min 70px, max 200px)
      const boxWidth = Math.max(70, Math.min(200, Math.round(item.weight * 600 + 70)));
      const boxHeight = Math.max(50, Math.min(120, Math.round(item.weight * 400 + 50)));

      const box = document.createElement("div");

      // Color: red for high VaR/weight concentration, green for low
      let bgColor;
      if (item.var_dollars !== null) {
        const intensity = Math.min(Math.abs(item.var_dollars) / maxVaR, 1);
        if (intensity > 0.5) {
          const r = Math.round(120 + intensity * 135);
          bgColor = `rgba(${r}, ${Math.round(40 * (1 - intensity))}, 0, ${0.35 + intensity * 0.45})`;
        } else {
          const g = Math.round(100 + (1 - intensity) * 155);
          bgColor = `rgba(${Math.round(60 * intensity)}, ${g}, ${Math.round(40 * intensity)}, ${0.3 + intensity * 0.3})`;
        }
      } else {
        // No VaR data — use weight-based coloring
        const intensity = Math.min(item.weight * 3, 1);
        bgColor = `rgba(100, 100, 100, ${0.2 + intensity * 0.3})`;
      }

      box.style.cssText = `width:${boxWidth}px;height:${boxHeight}px;background:${bgColor};border:1px solid #444;border-radius:4px;display:flex;flex-direction:column;justify-content:center;align-items:center;cursor:pointer;padding:4px;`;
      box.addEventListener("click", () => {
        document.getElementById("symbol-input").value = item.symbol;
        triggerLoad();
      });

      const symEl = document.createElement("div");
      symEl.textContent = item.symbol;
      symEl.style.cssText = "font-size:12px;font-weight:bold;color:#fff;";

      const weightEl = document.createElement("div");
      weightEl.textContent = `${(item.weight * 100).toFixed(1)}%`;
      weightEl.style.cssText = "font-size:13px;font-weight:bold;color:#ccc;";

      const varEl = document.createElement("div");
      varEl.textContent = item.var_dollars !== null ? `VaR $${item.var_dollars.toFixed(2)}` : "VaR —";
      varEl.style.cssText = "font-size:9px;color:#aaa;";

      box.appendChild(symEl);
      box.appendChild(weightEl);
      box.appendChild(varEl);
      grid.appendChild(box);
    }

    win.appendElement(grid);

    // Summary row
    const summary = document.createElement("div");
    summary.style.cssText = "padding:8px 12px;border-top:1px solid #333;font-size:11px;display:flex;justify-content:space-between;flex-wrap:wrap;gap:8px;";

    const sp1 = document.createElement("span");
    sp1.style.color = "#888";
    sp1.textContent = `Portfolio: $${totalValue.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;

    const sp2 = document.createElement("span");
    sp2.style.color = totalVaR > 0 ? "#f44336" : "#888";
    sp2.textContent = `Total VaR: $${totalVaR.toFixed(2)}`;

    const sp3 = document.createElement("span");
    sp3.style.color = maxWeight > 0.3 ? "#ff8" : "#888";
    sp3.textContent = `Largest: ${maxWeightSymbol} (${(maxWeight * 100).toFixed(1)}%)`;

    summary.appendChild(sp1);
    summary.appendChild(sp2);
    summary.appendChild(sp3);
    win.appendElement(summary);
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to load risk heatmap: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// ECALENDAR — Unified Earnings + Ex-Div Calendar
// ══════════════════════════════════════════════════════════════

async function cmdEarningsCalendar() {
  const win = createWindow({ title: "Earnings & Dividend Calendar", width: 700, height: 500 });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Loading earnings and dividend events...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);

  try {
    // Get symbols from watchlist or fall back to current symbol
    let symbols = getWatchlist();
    if (!symbols || symbols.length === 0) {
      if (currentSymbol) symbols = [currentSymbol];
      else { win.contentElement.textContent = ""; win.setContent("No watchlist or symbol loaded. Add symbols via QM first."); return; }
    }

    // Fetch dividends and earnings in parallel
    const events = []; // { date, symbol, type, details }

    const promises = symbols.map(async (sym) => {
      // Fetch dividends
      try {
        const divJson = await invokeQuiet("get_corporate_actions", { symbol: sym, types: "dividend" });
        const divActions = JSON.parse(divJson);
        if (Array.isArray(divActions)) {
          for (const a of divActions) {
            const date = a.ex_date || a.effective_date || a.date;
            if (date) {
              events.push({
                date,
                symbol: a.symbol || sym,
                type: "dividend",
                details: a.cash_amount ? `$${a.cash_amount}/share` : "ex-div",
              });
            }
          }
        }
      } catch (_) {}

      // Fetch earnings (corporate actions with type earnings, or fallback)
      try {
        const earnJson = await invokeQuiet("get_corporate_actions", { symbol: sym, types: "earnings" });
        const earnActions = JSON.parse(earnJson);
        if (Array.isArray(earnActions)) {
          for (const a of earnActions) {
            const date = a.date || a.report_date || a.effective_date;
            if (date) {
              events.push({
                date,
                symbol: a.symbol || sym,
                type: "earnings",
                details: a.description || "earnings report",
              });
            }
          }
        }
      } catch (_) {}
    });

    await Promise.all(promises);

    win.contentElement.textContent = "";

    // Build 5-week calendar grid (current week + next 4)
    const today = new Date();
    const dayOfWeek = today.getDay(); // 0=Sun
    const mondayOffset = dayOfWeek === 0 ? -6 : 1 - dayOfWeek;
    const startDate = new Date(today);
    startDate.setDate(today.getDate() + mondayOffset);
    startDate.setHours(0, 0, 0, 0);

    // Build event lookup by date string
    const eventsByDate = {};
    for (const ev of events) {
      const key = ev.date.substring(0, 10); // YYYY-MM-DD
      if (!eventsByDate[key]) eventsByDate[key] = [];
      eventsByDate[key].push(ev);
    }

    // Find next event for countdown
    const futureEvents = events
      .filter(e => new Date(e.date) >= today)
      .sort((a, b) => new Date(a.date) - new Date(b.date));

    if (futureEvents.length > 0) {
      const nextEv = futureEvents[0];
      const daysUntil = Math.ceil((new Date(nextEv.date) - today) / 86400000);
      const countdown = document.createElement("div");
      countdown.style.cssText = "padding:8px 12px;font-size:12px;color:#4caf50;border-bottom:1px solid #333;";
      countdown.textContent = `Next event: ${nextEv.symbol} ${nextEv.type} in ${daysUntil} day${daysUntil !== 1 ? "s" : ""} (${nextEv.date.substring(0, 10)})`;
      win.appendElement(countdown);
    }

    // Calendar table: 5 weeks x 5 days (Mon-Fri)
    const table = document.createElement("table");
    table.style.cssText = "width:100%;border-collapse:collapse;font-size:10px;";

    // Header row
    const thead = document.createElement("tr");
    for (const day of ["Mon", "Tue", "Wed", "Thu", "Fri"]) {
      const th = document.createElement("th");
      th.style.cssText = "color:#888;font-size:10px;padding:4px;border-bottom:1px solid #333;text-align:center;width:20%;";
      th.textContent = day;
      thead.appendChild(th);
    }
    table.appendChild(thead);

    // 5 weeks
    for (let week = 0; week < 5; week++) {
      const tr = document.createElement("tr");
      for (let day = 0; day < 5; day++) {
        const cellDate = new Date(startDate);
        cellDate.setDate(startDate.getDate() + week * 7 + day);
        const dateStr = cellDate.toISOString().slice(0, 10);
        const isToday = dateStr === today.toISOString().slice(0, 10);

        const td = document.createElement("td");
        td.style.cssText = `padding:4px;border:1px solid #333;vertical-align:top;height:60px;${isToday ? "background:#1a2a1a;" : ""}`;

        const dateLabel = document.createElement("div");
        dateLabel.textContent = `${cellDate.getMonth() + 1}/${cellDate.getDate()}`;
        dateLabel.style.cssText = `font-size:9px;color:${isToday ? "#4caf50" : "#666"};margin-bottom:2px;`;
        td.appendChild(dateLabel);

        // Events for this date
        const dayEvents = eventsByDate[dateStr] || [];
        for (const ev of dayEvents) {
          const evEl = document.createElement("div");
          evEl.style.cssText = `font-size:9px;padding:1px 3px;margin:1px 0;border-radius:2px;cursor:pointer;background:${ev.type === "dividend" ? "rgba(33,150,243,0.25)" : "rgba(255,152,0,0.25)"};color:${ev.type === "dividend" ? "#64b5f6" : "#ffb74d"};white-space:nowrap;overflow:hidden;text-overflow:ellipsis;`;
          evEl.textContent = `${ev.symbol} ${ev.type === "dividend" ? "div" : "earn"}`;
          evEl.title = `${ev.symbol} — ${ev.type}: ${ev.details}`;
          evEl.addEventListener("click", () => {
            document.getElementById("symbol-input").value = ev.symbol;
            triggerLoad();
          });
          td.appendChild(evEl);
        }

        tr.appendChild(td);
      }
      table.appendChild(tr);
    }

    win.appendElement(table);

    // Legend
    const legend = document.createElement("div");
    legend.style.cssText = "padding:6px 12px;font-size:9px;color:#666;border-top:1px solid #333;";
    legend.innerHTML = `<span style="color:#64b5f6;">&#9632;</span> Dividend &nbsp; <span style="color:#ffb74d;">&#9632;</span> Earnings &nbsp; | &nbsp; ${events.length} events across ${symbols.length} symbol${symbols.length !== 1 ? "s" : ""}`;
    win.appendElement(legend);

  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to load calendar: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// GREEKS — Aggregate Portfolio Greeks
// ══════════════════════════════════════════════════════════════

async function cmdGreeks() {
  const win = createWindow({ title: "Portfolio Greeks", width: 650, height: 500 });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Loading positions and computing Greeks...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);

  try {
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);

    if (!positions || positions.length === 0) {
      win.contentElement.textContent = "";
      win.setContent("No open positions.");
      return;
    }

    // Detect options: symbol typically contains date/strike pattern like AAPL250321C00150000
    // or has asset_class = "option" / "us_option"
    const optionPattern = /^([A-Z]+)\d{6}[CP]\d{8}$/;

    const rows = []; // { symbol, qty, delta, gamma, theta, vega, isOption }

    for (const p of positions) {
      const qty = parseFloat(p.qty) || 0;
      const symbol = p.symbol;
      const isOption = optionPattern.test(symbol) ||
                       (p.asset_class && p.asset_class.toLowerCase().includes("option"));

      if (isOption) {
        // Extract underlying and expiry from option symbol for chain lookup
        const match = symbol.match(/^([A-Z]+)(\d{6})([CP])(\d{8})$/);
        let delta = 0, gamma = 0, theta = 0, vega = 0;

        if (match) {
          const underlying = match[1];
          const dateStr = match[2]; // YYMMDD
          const expiry = `20${dateStr.substring(0, 2)}-${dateStr.substring(2, 4)}-${dateStr.substring(4, 6)}`;
          const putCall = match[3];
          const strikeRaw = parseInt(match[4], 10) / 1000;

          try {
            const json = await invokeQuiet("get_options", { symbol: underlying, expiry });
            const chain = JSON.parse(json);
            // Search for matching contract in chain
            const contracts = Array.isArray(chain) ? chain :
                              (chain.options || chain.snapshots || chain.contracts || []);
            let found = null;
            for (const c of contracts) {
              const cSymbol = c.symbol || c.contract_symbol || "";
              const cStrike = parseFloat(c.strike_price || c.strike) || 0;
              const cType = (c.type || c.option_type || c.put_call || "").toUpperCase();
              if (Math.abs(cStrike - strikeRaw) < 0.01 && cType.startsWith(putCall)) {
                found = c;
                break;
              }
              if (cSymbol === symbol) { found = c; break; }
            }

            if (found) {
              const greeks = found.greeks || found;
              delta = (parseFloat(greeks.delta) || 0) * qty * 100; // options = 100 shares per contract
              gamma = (parseFloat(greeks.gamma) || 0) * qty * 100;
              theta = (parseFloat(greeks.theta) || 0) * qty * 100;
              vega = (parseFloat(greeks.vega) || 0) * qty * 100;
            }
          } catch (_) {
            // Chain fetch failed — greeks remain 0
          }
        }

        rows.push({ symbol, qty, delta, gamma, theta, vega, isOption: true });
      } else {
        // Stock: delta = qty (delta 1.0 per share), no gamma/theta/vega
        rows.push({ symbol, qty, delta: qty, gamma: 0, theta: 0, vega: 0, isOption: false });
      }
    }

    // Aggregate totals
    const totals = { delta: 0, gamma: 0, theta: 0, vega: 0 };
    for (const r of rows) {
      totals.delta += r.delta;
      totals.gamma += r.gamma;
      totals.theta += r.theta;
      totals.vega += r.vega;
    }

    win.contentElement.textContent = "";

    // Summary cards
    const summaryGrid = document.createElement("div");
    summaryGrid.style.cssText = "display:flex;gap:8px;padding:10px;flex-wrap:wrap;";

    const greekCards = [
      { label: "Net Delta", value: totals.delta, desc: "Directional exposure" },
      { label: "Net Gamma", value: totals.gamma, desc: "Delta sensitivity" },
      { label: "Net Theta/day", value: totals.theta, desc: "Daily time decay" },
      { label: "Net Vega", value: totals.vega, desc: "IV sensitivity" },
    ];

    for (const gc of greekCards) {
      const card = document.createElement("div");
      card.style.cssText = "flex:1;min-width:120px;background:#1a1a2e;border:1px solid #333;border-radius:6px;padding:10px;text-align:center;";

      const labelEl = document.createElement("div");
      labelEl.textContent = gc.label;
      labelEl.style.cssText = "font-size:10px;color:#888;margin-bottom:4px;";

      const valueEl = document.createElement("div");
      const sign = gc.value >= 0 ? "+" : "";
      valueEl.textContent = `${sign}${gc.value.toFixed(2)}`;
      const isPositive = gc.value >= 0;
      valueEl.style.cssText = `font-size:18px;font-weight:bold;color:${isPositive ? "#4caf50" : "#f44336"};`;

      const descEl = document.createElement("div");
      descEl.textContent = gc.desc;
      descEl.style.cssText = "font-size:8px;color:#555;margin-top:2px;";

      card.appendChild(labelEl);
      card.appendChild(valueEl);
      card.appendChild(descEl);
      summaryGrid.appendChild(card);
    }

    win.appendElement(summaryGrid);

    // Per-position breakdown table
    const table = document.createElement("table");
    table.className = "fw-table";
    table.style.cssText = "width:100%;font-size:10px;";

    const thead = document.createElement("tr");
    for (const h of ["Symbol", "Type", "Qty", "Delta", "Gamma", "Theta", "Vega"]) {
      const th = document.createElement("td");
      th.style.cssText = "color:#888;font-weight:bold;font-size:10px;border-bottom:1px solid #333;padding:4px 6px;";
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);

    // Sort: options first, then stocks
    rows.sort((a, b) => (b.isOption ? 1 : 0) - (a.isOption ? 1 : 0) || a.symbol.localeCompare(b.symbol));

    for (const row of rows) {
      const tr = document.createElement("tr");

      const vals = [
        { text: row.symbol, color: "#ccc" },
        { text: row.isOption ? "OPT" : "STK", color: row.isOption ? "#ffb74d" : "#64b5f6" },
        { text: row.qty.toString(), color: "#ccc" },
        { text: row.delta.toFixed(2), color: row.delta >= 0 ? "#4caf50" : "#f44336" },
        { text: row.gamma.toFixed(4), color: "#aaa" },
        { text: row.theta.toFixed(2), color: row.theta >= 0 ? "#4caf50" : "#f44336" },
        { text: row.vega.toFixed(2), color: "#aaa" },
      ];

      for (const v of vals) {
        const td = document.createElement("td");
        td.style.cssText = `padding:3px 6px;color:${v.color};`;
        td.textContent = v.text;
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }

    win.appendElement(table);

    // Footer note
    const note = document.createElement("div");
    note.style.cssText = "padding:6px 12px;font-size:9px;color:#555;border-top:1px solid #333;";
    note.textContent = `${rows.length} positions | Stocks: delta=qty, gamma/theta/vega=0 | Options: greeks x qty x 100 multiplier`;
    win.appendElement(note);

  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to load portfolio Greeks: ${e}`);
  }
}

// ── EQUITY — Account Equity Curve ────────────────────────────
async function cmdEquity() {
  const win = createWindow({ title: "Equity Curve", type: "custom", width: 650, height: 500 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Loading equity data...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);
  try {
    const [actJson, miJson] = await Promise.all([
      invoke("get_account_activities", { activityTypes: "", limit: 500 }),
      invoke("get_margin_info"),
    ]);
    const activities = JSON.parse(actJson);
    const mi = JSON.parse(miJson);
    win.contentElement.textContent = "";
    const EQ_STORAGE_KEY = "typhoon_equity_snapshots";
    let snapshots = [];
    try { snapshots = JSON.parse(localStorage.getItem(EQ_STORAGE_KEY) || "[]"); } catch (_) {}
    const today = new Date().toISOString().slice(0, 10);
    const currentEquity = mi.equity || 0;
    if (currentEquity > 0) {
      const existing = snapshots.findIndex(s => s.date === today);
      if (existing >= 0) snapshots[existing].equity = currentEquity;
      else snapshots.push({ date: today, equity: currentEquity });
    }
    if (activities && activities.length > 0) {
      const sorted = [...activities].sort((a, b) => (a.date || "").localeCompare(b.date || ""));
      let running = 0;
      const dailyMap = {};
      for (const act of sorted) {
        const d = (act.date || "").slice(0, 10);
        if (!d) continue;
        const amt = parseFloat(act.net_amount || act.amount || act.qty || 0);
        if (act.activity_type === "CSD") running += Math.abs(amt);
        else if (act.activity_type === "CSW") running -= Math.abs(amt);
        else if (act.activity_type === "FILL" && act.net_amount) running += amt;
        else if (act.activity_type === "DIV") running += Math.abs(amt);
        if (running > 0) dailyMap[d] = running;
      }
      const existingDates = new Set(snapshots.map(s => s.date));
      for (const [date, equity] of Object.entries(dailyMap)) {
        if (!existingDates.has(date)) snapshots.push({ date, equity });
      }
    }
    snapshots.sort((a, b) => a.date.localeCompare(b.date));
    const seen = new Set();
    snapshots = snapshots.filter(s => { if (seen.has(s.date)) return false; seen.add(s.date); return s.equity > 0; });
    localStorage.setItem(EQ_STORAGE_KEY, JSON.stringify(snapshots.slice(-500)));
    if (snapshots.length < 2) { win.setContent("Not enough equity data yet. Equity snapshots are recorded each time you open this command. Check back after a few days."); return; }
    const chartDiv = document.createElement("div");
    chartDiv.style.cssText = "width:100%;height:300px;";
    win.appendElement(chartDiv);
    const eqChart = createChart(chartDiv, {
      width: chartDiv.clientWidth || 600, height: 300,
      layout: { background: { color: "#000" }, textColor: "#888", fontFamily: "Consolas, monospace", attributionLogo: false },
      grid: { vertLines: { color: "#1a1a2e" }, horzLines: { color: "#1a1a2e" } },
      rightPriceScale: { borderColor: "#333" }, timeScale: { borderColor: "#333" },
    });
    const startEq = snapshots[0].equity;
    const eqData = snapshots.map(s => ({ time: s.date, value: s.equity }));
    const isAbove = currentEquity >= startEq;
    const eqSeries = eqChart.addLineSeries({ color: isAbove ? "#4caf50" : "#f44336", lineWidth: 2, title: "Equity" });
    eqSeries.setData(eqData);
    let peak = 0;
    const ddData = snapshots.map(s => { if (s.equity > peak) peak = s.equity; return { time: s.date, value: peak > 0 ? ((s.equity - peak) / peak) * 100 : 0 }; });
    const ddSeries = eqChart.addAreaSeries({ topColor: "rgba(244,67,54,0.0)", bottomColor: "rgba(244,67,54,0.3)", lineColor: "rgba(244,67,54,0.6)", lineWidth: 1, title: "Drawdown %", priceScaleId: "dd" });
    ddSeries.setData(ddData);
    eqChart.priceScale("dd").applyOptions({ scaleMargins: { top: 0.7, bottom: 0 } });
    eqChart.timeScale().fitContent();
    const totalReturn = ((currentEquity - startEq) / startEq * 100);
    peak = 0; let maxDD = 0;
    for (const s of snapshots) { if (s.equity > peak) peak = s.equity; const dd = (s.equity - peak) / peak * 100; if (dd < maxDD) maxDD = dd; }
    let sharpe = "\u2014";
    if (snapshots.length > 5) {
      const rets = []; for (let i = 1; i < snapshots.length; i++) rets.push((snapshots[i].equity - snapshots[i - 1].equity) / snapshots[i - 1].equity);
      const mean = rets.reduce((a, b) => a + b, 0) / rets.length;
      const variance = rets.reduce((a, b) => a + (b - mean) ** 2, 0) / rets.length;
      const std = Math.sqrt(variance);
      if (std > 0) sharpe = ((mean / std) * Math.sqrt(252)).toFixed(3);
    }
    const statsDiv = document.createElement("div");
    statsDiv.style.cssText = "padding:8px;display:grid;grid-template-columns:1fr 1fr;gap:4px 16px;font-size:11px;";
    for (const [label, value] of [["Total Return", `${totalReturn >= 0 ? "+" : ""}${totalReturn.toFixed(2)}%`], ["Max Drawdown", `${maxDD.toFixed(2)}%`], ["Sharpe Ratio", sharpe], ["Current Equity", `$${currentEquity.toFixed(2)}`]]) {
      const row = document.createElement("div"); row.style.cssText = "display:flex;justify-content:space-between;";
      const lbl = document.createElement("span"); lbl.style.color = "#666"; lbl.textContent = label;
      const val = document.createElement("span"); val.style.color = label === "Total Return" ? (totalReturn >= 0 ? "#4caf50" : "#f44336") : label === "Max Drawdown" ? "#f44336" : "#ccc"; val.textContent = value;
      row.appendChild(lbl); row.appendChild(val); statsDiv.appendChild(row);
    }
    win.appendElement(statsDiv);
    const ro = new ResizeObserver(() => { if (chartDiv.clientWidth > 0) eqChart.applyOptions({ width: chartDiv.clientWidth }); });
    ro.observe(chartDiv);
  } catch (e) { win.contentElement.textContent = ""; win.setContent(`Failed to load equity data: ${e}`); }
}

// ── HEATCAL — Calendar Heatmap of Daily Returns ─────────────
async function cmdHeatCal() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} \u2014 Calendar Heatmap`, type: "custom", width: 700, height: 480 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Building calendar heatmap...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);
  try {
    let bars = null;
    const cKey = `${currentSymbol}:1Day`;
    const cached = barCache[cKey];
    if (cached && cached.data && cached.data.length > 20) bars = cached.data;
    if (!bars) {
      const barsJson = await invoke("get_bars", { symbol: currentSymbol, timeframe: "1Day", limit: 500 });
      bars = JSON.parse(barsJson);
      if (bars.length > 0) barCache[cKey] = { data: bars, timestamp: Date.now(), lastAccess: Date.now() };
    }
    if (!bars || bars.length < 2) { win.contentElement.textContent = ""; win.setContent("Not enough daily bar data for heatmap."); return; }
    win.contentElement.textContent = "";
    const returns = [];
    for (let i = 1; i < bars.length; i++) {
      const prevClose = bars[i - 1].close || bars[i - 1].c || 0;
      const close = bars[i].close || bars[i].c || 0;
      if (prevClose <= 0) continue;
      const ret = ((close - prevClose) / prevClose) * 100;
      let dateStr = bars[i].t || bars[i].time || bars[i].date || "";
      if (typeof dateStr === "number") dateStr = new Date(dateStr * 1000).toISOString().slice(0, 10);
      else dateStr = String(dateStr).slice(0, 10);
      if (dateStr) returns.push({ date: dateStr, ret });
    }
    if (returns.length < 2) { win.setContent("Insufficient return data for heatmap."); return; }
    const dayOfWeekCounts = [0, 0, 0, 0, 0, 0, 0];
    for (const r of returns) { const d = new Date(r.date + "T12:00:00Z"); dayOfWeekCounts[d.getUTCDay()]++; }
    const isCrypto = dayOfWeekCounts[0] > 2 || dayOfWeekCounts[6] > 2;
    const daysOfWeek = isCrypto ? ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] : ["Mon", "Tue", "Wed", "Thu", "Fri"];
    const rowCount = daysOfWeek.length;
    const retMap = {}; for (const r of returns) retMap[r.date] = r.ret;
    const startDate = new Date(returns[0].date + "T12:00:00Z");
    const endDate = new Date(returns[returns.length - 1].date + "T12:00:00Z");
    const firstDay = new Date(startDate);
    while (firstDay.getUTCDay() !== 1) firstDay.setUTCDate(firstDay.getUTCDate() - 1);
    const weeks = []; let cursor = new Date(firstDay);
    while (cursor <= endDate) {
      const week = [];
      for (let d = 0; d < 7; d++) {
        const ds = cursor.toISOString().slice(0, 10); const dow = cursor.getUTCDay();
        const rowIdx = dow === 0 ? 6 : dow - 1;
        if (isCrypto || (dow >= 1 && dow <= 5)) week.push({ date: ds, rowIdx, ret: retMap[ds] !== undefined ? retMap[ds] : null });
        cursor.setUTCDate(cursor.getUTCDate() + 1);
      }
      weeks.push(week);
    }
    function retColor(ret) {
      if (ret === null) return "#1a1a2e";
      if (ret > 3) return "#1b5e20"; if (ret > 1) return "#388e3c"; if (ret > 0) return "#66bb6a";
      if (ret > -1) return "#ef9a9a"; if (ret > -3) return "#e53935"; return "#b71c1c";
    }
    const container = document.createElement("div"); container.style.cssText = "overflow-x:auto;padding:8px;";
    const monthBar = document.createElement("div");
    monthBar.style.cssText = "display:flex;margin-left:32px;margin-bottom:2px;font-size:9px;color:#666;";
    let lastMonth = -1;
    for (let w = 0; w < weeks.length; w++) {
      const firstDateInWeek = weeks[w].find(d => d.ret !== null);
      const span = document.createElement("span"); span.style.cssText = "width:14px;text-align:center;flex-shrink:0;";
      if (firstDateInWeek) { const m = new Date(firstDateInWeek.date + "T12:00:00Z").getUTCMonth(); if (m !== lastMonth) { span.textContent = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"][m]; lastMonth = m; } }
      monthBar.appendChild(span);
    }
    container.appendChild(monthBar);
    for (let r = 0; r < rowCount; r++) {
      const row = document.createElement("div"); row.style.cssText = "display:flex;align-items:center;height:14px;";
      const label = document.createElement("span"); label.style.cssText = "width:28px;font-size:9px;color:#666;text-align:right;padding-right:4px;flex-shrink:0;"; label.textContent = daysOfWeek[r];
      row.appendChild(label);
      for (let w = 0; w < weeks.length; w++) {
        const cell = document.createElement("div"); cell.style.cssText = "width:12px;height:12px;border-radius:2px;margin:1px;flex-shrink:0;cursor:pointer;";
        const dayData = weeks[w].find(d => d.rowIdx === r);
        if (dayData && dayData.ret !== null) { cell.style.background = retColor(dayData.ret); cell.title = `${dayData.date}: ${dayData.ret >= 0 ? "+" : ""}${dayData.ret.toFixed(2)}%`; }
        else { cell.style.background = "#111"; }
        row.appendChild(cell);
      }
      container.appendChild(row);
    }
    const legend = document.createElement("div"); legend.style.cssText = "display:flex;align-items:center;gap:4px;margin-top:8px;margin-left:32px;font-size:9px;color:#666;";
    for (const item of [{ label: "< -3%", color: "#b71c1c" }, { label: "-3 to -1%", color: "#e53935" }, { label: "-1 to 0%", color: "#ef9a9a" }, { label: "0 to +1%", color: "#66bb6a" }, { label: "+1 to +3%", color: "#388e3c" }, { label: "> +3%", color: "#1b5e20" }]) {
      const box = document.createElement("span"); box.style.cssText = `display:inline-block;width:10px;height:10px;border-radius:2px;background:${item.color};`;
      legend.appendChild(box); const txt = document.createElement("span"); txt.textContent = item.label; txt.style.marginRight = "6px"; legend.appendChild(txt);
    }
    container.appendChild(legend);
    win.appendElement(container);
    const upDays = returns.filter(r => r.ret > 0); const downDays = returns.filter(r => r.ret < 0);
    const avgRet = returns.reduce((s, r) => s + r.ret, 0) / returns.length;
    const bestDay = returns.reduce((best, r) => r.ret > best.ret ? r : best, returns[0]);
    const worstDay = returns.reduce((worst, r) => r.ret < worst.ret ? r : worst, returns[0]);
    const summaryDiv = document.createElement("div");
    summaryDiv.style.cssText = "padding:8px;display:grid;grid-template-columns:1fr 1fr;gap:4px 16px;font-size:11px;border-top:1px solid #222;margin-top:4px;";
    for (const [label, value, color] of [["Up Days", `${upDays.length}`, "#4caf50"], ["Down Days", `${downDays.length}`, "#f44336"], ["Avg Daily Return", `${avgRet >= 0 ? "+" : ""}${avgRet.toFixed(3)}%`, avgRet >= 0 ? "#4caf50" : "#f44336"], ["Best Day", `${bestDay.date} (${bestDay.ret >= 0 ? "+" : ""}${bestDay.ret.toFixed(2)}%)`, "#4caf50"], ["Worst Day", `${worstDay.date} (${worstDay.ret.toFixed(2)}%)`, "#f44336"], ["Total Days", `${returns.length}`, "#888"]]) {
      const row = document.createElement("div"); row.style.cssText = "display:flex;justify-content:space-between;";
      const lbl = document.createElement("span"); lbl.style.color = "#666"; lbl.textContent = label;
      const val = document.createElement("span"); val.style.color = color; val.textContent = value;
      row.appendChild(lbl); row.appendChild(val); summaryDiv.appendChild(row);
    }
    win.appendElement(summaryDiv);
  } catch (e) { win.contentElement.textContent = ""; win.setContent(`Failed to build heatmap: ${e}`); }
}

// ── CORRWATCH — Correlation Breakdown Alerts ─────────────────
async function cmdCorrWatch() {
  const win = createWindow({ title: "Correlation Breakdown Alerts", type: "custom", width: 700, height: 500 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Calculating correlation breakdowns...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);
  try {
    const watchlist = getWatchlist();
    if (watchlist.length < 2) { win.contentElement.textContent = ""; win.setContent("Need at least 2 watchlist symbols. Add symbols via QM (Quote Monitor) first."); return; }
    const closePrices = {};
    for (const sym of watchlist) {
      let data = null; const cKey = `${sym}:1Day`; const cached = barCache[cKey];
      if (cached && cached.data && cached.data.length > 60) data = cached.data;
      if (!data) { try { const barsJson = await invoke("get_bars", { symbol: sym, timeframe: "1Day", limit: 300 }); const bars = JSON.parse(barsJson); if (bars.length > 60) { data = bars; barCache[cKey] = { data: bars, timestamp: Date.now(), lastAccess: Date.now() }; } } catch (_) {} }
      if (data) closePrices[sym] = data.slice(-300).map(b => b.close || b.c || 0);
    }
    const validSymbols = Object.keys(closePrices).filter(s => closePrices[s].length > 60);
    if (validSymbols.length < 2) { win.contentElement.textContent = ""; win.setContent("Insufficient cached bar data (need >60 daily bars per symbol). Load some daily charts first."); return; }
    const returns = {};
    for (const sym of validSymbols) { const prices = closePrices[sym]; returns[sym] = []; for (let i = 1; i < prices.length; i++) returns[sym].push(prices[i] > 0 ? (prices[i] - prices[i - 1]) / prices[i - 1] : 0); }
    function pearson(a, b, start, len) {
      const n = Math.min(len, a.length - start, b.length - start);
      if (n < 10) return null;
      let sumA = 0, sumB = 0, sumAB = 0, sumA2 = 0, sumB2 = 0;
      for (let i = 0; i < n; i++) { const ai = a[start + i], bi = b[start + i]; sumA += ai; sumB += bi; sumAB += ai * bi; sumA2 += ai * ai; sumB2 += bi * bi; }
      const num = n * sumAB - sumA * sumB; const den = Math.sqrt((n * sumA2 - sumA * sumA) * (n * sumB2 - sumB * sumB));
      return den > 0 ? num / den : 0;
    }
    const pairs = [];
    for (let i = 0; i < validSymbols.length; i++) {
      for (let j = i + 1; j < validSymbols.length; j++) {
        const symA = validSymbols[i], symB = validSymbols[j];
        const retA = returns[symA], retB = returns[symB];
        const minLen = Math.min(retA.length, retB.length);
        if (minLen < 60) continue;
        const tailA = retA.slice(-minLen), tailB = retB.slice(-minLen);
        const corr60 = pearson(tailA, tailB, tailA.length - 60, 60);
        const longLen = Math.min(252, tailA.length);
        const corr252 = pearson(tailA, tailB, tailA.length - longLen, longLen);
        if (corr60 === null || corr252 === null) continue;
        const delta = Math.abs(corr60 - corr252);
        let status, statusColor;
        if (delta > 0.5) { status = "BREAKDOWN"; statusColor = "#f44336"; }
        else if (delta > 0.3) { status = "DIVERGING"; statusColor = "#ff9800"; }
        else { status = "NORMAL"; statusColor = "#4caf50"; }
        pairs.push({ symA, symB, corr60, corr252, delta, status, statusColor });
      }
    }
    pairs.sort((a, b) => b.delta - a.delta);
    win.contentElement.textContent = "";
    if (pairs.length === 0) { win.setContent("No valid pairs found. Need overlapping daily bar data for at least 2 symbols."); return; }
    const header = document.createElement("div");
    header.style.cssText = "padding:6px 8px;font-size:10px;color:#666;border-bottom:1px solid #333;";
    const breakdowns = pairs.filter(p => p.status === "BREAKDOWN").length;
    const diverging = pairs.filter(p => p.status === "DIVERGING").length;
    header.textContent = `${pairs.length} pairs analyzed | ${breakdowns} breakdowns | ${diverging} diverging`;
    win.appendElement(header);
    const table = document.createElement("table"); table.className = "fw-table"; table.style.cssText = "width:100%;font-size:11px;";
    const thead = document.createElement("tr");
    for (const h of ["Pair", "60D Corr", "1Y Corr", "Delta", "Status"]) { const th = document.createElement("td"); th.style.cssText = "color:#666;font-weight:bold;font-size:10px;padding:4px 6px;border-bottom:1px solid #333;"; th.textContent = h; thead.appendChild(th); }
    table.appendChild(thead);
    for (const p of pairs) {
      const tr = document.createElement("tr");
      const tdPair = document.createElement("td"); tdPair.style.cssText = "color:#ccc;padding:3px 6px;"; tdPair.textContent = `${p.symA} / ${p.symB}`; tr.appendChild(tdPair);
      const td60 = document.createElement("td"); td60.style.cssText = "color:#aaa;padding:3px 6px;text-align:right;"; td60.textContent = p.corr60.toFixed(3); tr.appendChild(td60);
      const td252 = document.createElement("td"); td252.style.cssText = "color:#aaa;padding:3px 6px;text-align:right;"; td252.textContent = p.corr252.toFixed(3); tr.appendChild(td252);
      const tdDelta = document.createElement("td"); tdDelta.style.cssText = `color:${p.statusColor};padding:3px 6px;text-align:right;font-weight:bold;`; tdDelta.textContent = p.delta.toFixed(3); tr.appendChild(tdDelta);
      const tdStatus = document.createElement("td"); tdStatus.style.cssText = "padding:3px 6px;";
      const badge = document.createElement("span"); badge.style.cssText = `font-size:9px;padding:2px 6px;border-radius:2px;background:${p.statusColor};color:#fff;`; badge.textContent = p.status;
      tdStatus.appendChild(badge); tr.appendChild(tdStatus); table.appendChild(tr);
    }
    const tableWrap = document.createElement("div"); tableWrap.style.cssText = "overflow-y:auto;max-height:calc(100% - 50px);";
    tableWrap.appendChild(table); win.appendElement(tableWrap);
  } catch (e) { win.contentElement.textContent = ""; win.setContent(`Failed to calculate correlations: ${e}`); }
}

// ══════════════════════════════════════════════════════════════
// SIGNAL — Composite Trading Signal Generator
// ══════════════════════════════════════════════════════════════
function cmdSignal() {
  if (!currentChartData || currentChartData.length < 201) { log("SIGNAL: Need at least 201 bars loaded", "warn"); return; }
  const chartData = currentChartData;
  const win = createWindow({ title: `${currentSymbol} — Composite Signal`, width: 420, height: 520 });
  win.contentElement.textContent = "";

  const ef = calcEhlersFisher(chartData, 32);
  const rsiData = calcRSI(chartData, 14);
  const kamaData = calcKAMA(chartData, 10);
  const sma200 = calcSMA(chartData, 200);
  const atrData = calcATR(chartData, 14);

  let score = 0;
  const components = [];

  // 1. Fisher Transform
  let fisherScore = 0;
  if (ef.fisher.length > 1 && ef.signal.length > 0) {
    const lastFisher = ef.fisher[ef.fisher.length - 1].value;
    const lastSignal = ef.signal[ef.signal.length - 1].value;
    fisherScore = lastFisher > lastSignal ? 20 : -20;
  }
  score += fisherScore;
  components.push({ name: "Fisher Transform", value: fisherScore, max: 20, detail: fisherScore > 0 ? "Bullish (fisher > signal)" : "Bearish (fisher < signal)" });

  // 2. RSI (14)
  let rsiScore = 0;
  if (rsiData.length > 0) {
    const rsiVal = rsiData[rsiData.length - 1].value;
    if (rsiVal < 30) rsiScore = 15;
    else if (rsiVal > 70) rsiScore = -15;
    else rsiScore = Math.round(15 * (50 - rsiVal) / 20);
    components.push({ name: `RSI (${rsiVal.toFixed(1)})`, value: rsiScore, max: 15, detail: rsiVal < 30 ? "Oversold" : rsiVal > 70 ? "Overbought" : "Neutral zone" });
  } else {
    components.push({ name: "RSI", value: 0, max: 15, detail: "Insufficient data" });
  }
  score += rsiScore;

  // 3. Price vs SMA200
  let smaScore = 0;
  if (sma200.length > 0) {
    const lastClose = chartData[chartData.length - 1].close;
    const lastSMA = sma200[sma200.length - 1].value;
    smaScore = lastClose > lastSMA ? 20 : -20;
    components.push({ name: "Price vs SMA200", value: smaScore, max: 20, detail: smaScore > 0 ? `Above ($${lastSMA.toFixed(2)})` : `Below ($${lastSMA.toFixed(2)})` });
  } else {
    components.push({ name: "Price vs SMA200", value: 0, max: 20, detail: "Insufficient data" });
  }
  score += smaScore;

  // 4. KAMA slope
  let kamaScore = 0;
  if (kamaData.length >= 3) {
    const k1 = kamaData[kamaData.length - 1].value;
    const k2 = kamaData[kamaData.length - 3].value;
    kamaScore = k1 > k2 ? 15 : -15;
    components.push({ name: "KAMA Slope", value: kamaScore, max: 15, detail: kamaScore > 0 ? "Positive (bullish)" : "Negative (bearish)" });
  } else {
    components.push({ name: "KAMA Slope", value: 0, max: 15, detail: "Insufficient data" });
  }
  score += kamaScore;

  // 5. Volume confirmation: current > 1.5x 20-day avg
  let volScore = 0;
  const volLookback = 20;
  if (chartData.length > volLookback + 1) {
    const recentVols = chartData.slice(-volLookback - 1, -1).map(d => d.volume || 0);
    const avgVol = recentVols.reduce((a, b) => a + b, 0) / recentVols.length;
    const curVol = chartData[chartData.length - 1].volume || 0;
    volScore = (avgVol > 0 && curVol > avgVol * 1.5) ? 15 : 0;
    components.push({ name: "Volume Confirm", value: volScore, max: 15, detail: avgVol > 0 ? `${(curVol / avgVol).toFixed(1)}x avg` : "No volume data" });
  } else {
    components.push({ name: "Volume Confirm", value: 0, max: 15, detail: "Insufficient data" });
  }
  score += volScore;

  // 6. ATR trend
  let atrScore = 0;
  if (atrData.length >= 10) {
    const atrNow = atrData[atrData.length - 1].value;
    const atr10Ago = atrData[atrData.length - 10].value;
    atrScore = atrNow < atr10Ago ? 15 : -5;
    components.push({ name: "ATR Trend", value: atrScore, max: 15, detail: atrScore > 0 ? "Contracting (calm)" : "Expanding (volatile)" });
  } else {
    components.push({ name: "ATR Trend", value: 0, max: 15, detail: "Insufficient data" });
  }
  score += atrScore;

  const composite = Math.max(0, Math.min(100, score + 50));

  let label, labelColor;
  if (composite <= 30) { label = "SELL"; labelColor = "#f44336"; }
  else if (composite <= 45) { label = "WEAK"; labelColor = "#ff9800"; }
  else if (composite <= 55) { label = "NEUTRAL"; labelColor = "#888"; }
  else if (composite <= 70) { label = "BUILDING"; labelColor = "#ffeb3b"; }
  else { label = "BUY"; labelColor = "#4caf50"; }

  const container = document.createElement("div");
  container.style.cssText = "padding:16px;font-family:monospace;font-size:13px;color:#ddd;overflow-y:auto;height:100%;";

  const scoreDiv = document.createElement("div");
  scoreDiv.style.cssText = `text-align:center;padding:18px;background:#111;border:2px solid ${labelColor};border-radius:10px;margin-bottom:16px;`;
  const scoreNum = document.createElement("div"); scoreNum.style.cssText = `font-size:48px;font-weight:bold;color:${labelColor};`; scoreNum.textContent = composite; scoreDiv.appendChild(scoreNum);
  const scoreLbl = document.createElement("div"); scoreLbl.style.cssText = `font-size:18px;color:${labelColor};font-weight:bold;`; scoreLbl.textContent = label; scoreDiv.appendChild(scoreLbl);
  const scoreStr = document.createElement("div"); scoreStr.style.cssText = "color:#888;font-size:12px;margin-top:4px;"; scoreStr.textContent = "Signal strength: " + composite + "/100"; scoreDiv.appendChild(scoreStr);
  container.appendChild(scoreDiv);

  const breakdownTitle = document.createElement("div");
  breakdownTitle.style.cssText = "color:#888;font-size:11px;margin-bottom:8px;text-transform:uppercase;letter-spacing:1px;";
  breakdownTitle.textContent = "Component Breakdown";
  container.appendChild(breakdownTitle);

  for (const c of components) {
    const row = document.createElement("div");
    row.style.cssText = "margin-bottom:8px;";
    const nameRow = document.createElement("div");
    nameRow.style.cssText = "display:flex;justify-content:space-between;margin-bottom:2px;";
    const nameSpan = document.createElement("span");
    nameSpan.style.cssText = "font-size:12px;color:#ccc;";
    nameSpan.textContent = c.name;
    const valSpan = document.createElement("span");
    const valColor = c.value > 0 ? "#4caf50" : c.value < 0 ? "#f44336" : "#888";
    valSpan.style.cssText = `font-size:12px;color:${valColor};font-weight:bold;`;
    valSpan.textContent = `${c.value > 0 ? "+" : ""}${c.value}`;
    nameRow.appendChild(nameSpan);
    nameRow.appendChild(valSpan);
    row.appendChild(nameRow);

    const barOuter = document.createElement("div");
    barOuter.style.cssText = "width:100%;height:6px;background:#222;border-radius:3px;overflow:hidden;";
    const barInner = document.createElement("div");
    const pct = c.max > 0 ? Math.abs(c.value) / c.max * 100 : 0;
    barInner.style.cssText = `width:${pct}%;height:100%;background:${valColor};border-radius:3px;transition:width 0.3s;`;
    barOuter.appendChild(barInner);
    row.appendChild(barOuter);

    const detailDiv = document.createElement("div");
    detailDiv.style.cssText = "font-size:10px;color:#666;margin-top:1px;";
    detailDiv.textContent = c.detail;
    row.appendChild(detailDiv);

    container.appendChild(row);
  }

  const recDiv = document.createElement("div");
  recDiv.style.cssText = `margin-top:14px;padding:10px;background:#111;border-radius:6px;border:1px solid #333;text-align:center;`;
  let recText;
  if (composite >= 70) recText = "Strong bullish confluence. Consider long entries with tight SL.";
  else if (composite >= 55) recText = "Building bullish bias. Wait for confirmation or reduce size.";
  else if (composite >= 45) recText = "No clear directional bias. Stay flat or reduce exposure.";
  else if (composite >= 30) recText = "Weak bearish bias. Caution on longs, watch for breakdown.";
  else recText = "Strong bearish confluence. Consider short entries or hedge longs.";
  const recSpan = document.createElement("span"); recSpan.style.cssText = "color:#888;font-size:11px;"; recSpan.textContent = recText; recDiv.appendChild(recSpan);
  container.appendChild(recDiv);

  win.appendElement(container);
  log(`SIGNAL: ${currentSymbol} composite score ${composite}/100 (${label})`, composite >= 55 ? "ok" : composite <= 30 ? "warn" : "info");
}

// ══════════════════════════════════════════════════════════════
// PROFILE — Trading Profile Analytics
// ══════════════════════════════════════════════════════════════
async function cmdProfile() {
  const win = createWindow({ title: "Trading Profile Analytics", width: 650, height: 550 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.style.cssText = "padding:20px;color:#888;text-align:center;";
  loading.textContent = "Loading order history...";
  win.appendElement(loading);

  try {
    const histJson = await invoke("get_order_history", { limit: 500 });
    const orders = JSON.parse(histJson);
    const filled = orders.filter(o => o.status === "filled" && o.filled_avg_price);
    win.contentElement.textContent = "";

    if (filled.length === 0) { win.setContent("No filled orders found."); return; }

    const container = document.createElement("div");
    container.style.cssText = "padding:14px;font-family:monospace;font-size:12px;color:#ddd;overflow-y:auto;height:100%;";

    // Group by symbol
    const bySymbol = {};
    for (const o of filled) {
      if (!bySymbol[o.symbol]) bySymbol[o.symbol] = { trades: 0, buyCost: 0, sellProceeds: 0 };
      const s = bySymbol[o.symbol];
      s.trades++;
      const qty = parseFloat(o.filled_qty || o.qty || 0);
      const price = parseFloat(o.filled_avg_price || 0);
      if (o.side === "buy") s.buyCost += qty * price;
      else s.sellProceeds += qty * price;
    }
    const symbolPnL = Object.entries(bySymbol).map(([sym, s]) => ({
      symbol: sym, pnl: s.sellProceeds - s.buyCost, trades: s.trades
    })).sort((a, b) => b.pnl - a.pnl);

    // Best Symbols
    const bestTitle = document.createElement("div");
    bestTitle.style.cssText = "color:#4caf50;font-weight:bold;font-size:14px;margin-bottom:6px;";
    bestTitle.textContent = "Best Symbols (by estimated P&L)";
    container.appendChild(bestTitle);
    const bestTable = document.createElement("table");
    bestTable.style.cssText = "width:100%;border-collapse:collapse;margin-bottom:16px;font-size:12px;";
    bestTable.innerHTML = `<thead><tr style="color:#888;border-bottom:1px solid #444;"><th style="text-align:left;padding:4px;">Symbol</th><th style="text-align:right;padding:4px;">Est. P&L</th><th style="text-align:right;padding:4px;">Trades</th></tr></thead>`;
    const bestBody = document.createElement("tbody");
    for (const s of symbolPnL.slice(0, 5)) {
      const tr = document.createElement("tr"); tr.style.borderBottom = "1px solid #333";
      const pc = s.pnl >= 0 ? "#4caf50" : "#f44336";
      tr.innerHTML = `<td style="padding:4px;">${s.symbol}</td><td style="text-align:right;padding:4px;color:${pc};">$${s.pnl.toFixed(2)}</td><td style="text-align:right;padding:4px;">${s.trades}</td>`;
      bestBody.appendChild(tr);
    }
    bestTable.appendChild(bestBody); container.appendChild(bestTable);

    // Worst Symbols
    const worstTitle = document.createElement("div");
    worstTitle.style.cssText = "color:#f44336;font-weight:bold;font-size:14px;margin-bottom:6px;";
    worstTitle.textContent = "Worst Symbols";
    container.appendChild(worstTitle);
    const worstTable = document.createElement("table");
    worstTable.style.cssText = "width:100%;border-collapse:collapse;margin-bottom:16px;font-size:12px;";
    worstTable.innerHTML = `<thead><tr style="color:#888;border-bottom:1px solid #444;"><th style="text-align:left;padding:4px;">Symbol</th><th style="text-align:right;padding:4px;">Est. P&L</th><th style="text-align:right;padding:4px;">Trades</th></tr></thead>`;
    const worstBody = document.createElement("tbody");
    for (const s of symbolPnL.slice(-5).reverse()) {
      const tr = document.createElement("tr"); tr.style.borderBottom = "1px solid #333";
      const pc = s.pnl >= 0 ? "#4caf50" : "#f44336";
      tr.innerHTML = `<td style="padding:4px;">${s.symbol}</td><td style="text-align:right;padding:4px;color:${pc};">$${s.pnl.toFixed(2)}</td><td style="text-align:right;padding:4px;">${s.trades}</td>`;
      worstBody.appendChild(tr);
    }
    worstTable.appendChild(worstBody); container.appendChild(worstTable);

    // Day of Week
    const dowTitle = document.createElement("div");
    dowTitle.style.cssText = "color:#64b5f6;font-weight:bold;font-size:14px;margin-bottom:6px;";
    dowTitle.textContent = "Day of Week Activity";
    container.appendChild(dowTitle);
    const dayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const dowCounts = [0, 0, 0, 0, 0, 0, 0];
    const dowBuys = [0, 0, 0, 0, 0, 0, 0];
    const dowSells = [0, 0, 0, 0, 0, 0, 0];
    for (const o of filled) {
      const d = new Date(o.submitted_at || o.created_at);
      const day = d.getDay();
      dowCounts[day]++;
      if (o.side === "buy") dowBuys[day]++; else dowSells[day]++;
    }
    const maxDow = Math.max(...dowCounts, 1);
    const dowChart = document.createElement("div");
    dowChart.style.cssText = "display:flex;gap:6px;align-items:flex-end;height:80px;margin-bottom:16px;";
    for (let i = 1; i <= 5; i++) {
      const col = document.createElement("div");
      col.style.cssText = "flex:1;display:flex;flex-direction:column;align-items:center;";
      const bar = document.createElement("div");
      const pct = (dowCounts[i] / maxDow) * 100;
      const barColor = dowBuys[i] > dowSells[i] ? "#4caf50" : dowSells[i] > dowBuys[i] ? "#f44336" : "#888";
      bar.style.cssText = `width:100%;height:${Math.max(pct, 2)}%;background:${barColor};border-radius:3px 3px 0 0;min-height:2px;`;
      const lbl = document.createElement("div");
      lbl.style.cssText = "font-size:10px;color:#888;margin-top:4px;";
      lbl.textContent = dayNames[i];
      const cnt = document.createElement("div");
      cnt.style.cssText = "font-size:10px;color:#ccc;";
      cnt.textContent = dowCounts[i];
      col.appendChild(cnt); col.appendChild(bar); col.appendChild(lbl);
      dowChart.appendChild(col);
    }
    container.appendChild(dowChart);

    // Long vs Short
    const lsTitle = document.createElement("div");
    lsTitle.style.cssText = "color:#ffab40;font-weight:bold;font-size:14px;margin-bottom:6px;";
    lsTitle.textContent = "Long vs Short";
    container.appendChild(lsTitle);
    const buys = filled.filter(o => o.side === "buy");
    const sells = filled.filter(o => o.side === "sell");
    const lsGrid = document.createElement("div");
    lsGrid.style.cssText = "display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-bottom:16px;";
    const buyBox = document.createElement("div");
    buyBox.style.cssText = "background:#1a2e1a;border:1px solid #4caf50;border-radius:6px;padding:10px;text-align:center;";
    buyBox.innerHTML = `<div style="color:#4caf50;font-size:16px;font-weight:bold;">LONG</div><div style="color:#ccc;font-size:13px;margin-top:4px;">${buys.length} orders</div><div style="color:#888;font-size:11px;">${filled.length > 0 ? ((buys.length / filled.length) * 100).toFixed(1) : 0}% of total</div>`;
    const sellBox = document.createElement("div");
    sellBox.style.cssText = "background:#2e1a1a;border:1px solid #f44336;border-radius:6px;padding:10px;text-align:center;";
    sellBox.innerHTML = `<div style="color:#f44336;font-size:16px;font-weight:bold;">SHORT</div><div style="color:#ccc;font-size:13px;margin-top:4px;">${sells.length} orders</div><div style="color:#888;font-size:11px;">${filled.length > 0 ? ((sells.length / filled.length) * 100).toFixed(1) : 0}% of total</div>`;
    lsGrid.appendChild(buyBox); lsGrid.appendChild(sellBox);
    container.appendChild(lsGrid);

    // Hold Time Distribution
    const htTitle = document.createElement("div");
    htTitle.style.cssText = "color:#ce93d8;font-weight:bold;font-size:14px;margin-bottom:6px;";
    htTitle.textContent = "Hold Time Distribution (estimated)";
    container.appendChild(htTitle);
    let shortHold = 0, medHold = 0, swingHold = 0;
    const sortedFilled = [...filled].sort((a, b) => new Date(a.submitted_at || a.created_at) - new Date(b.submitted_at || b.created_at));
    const openTrades = {};
    for (const o of sortedFilled) {
      const sym = o.symbol;
      if (o.side === "buy") {
        openTrades[sym] = new Date(o.submitted_at || o.created_at);
      } else if (o.side === "sell" && openTrades[sym]) {
        const holdMs = new Date(o.submitted_at || o.created_at) - openTrades[sym];
        const holdHrs = holdMs / 3600000;
        if (holdHrs < 1) shortHold++;
        else if (holdHrs < 24) medHold++;
        else swingHold++;
        delete openTrades[sym];
      }
    }
    const htTotal = shortHold + medHold + swingHold || 1;
    const htGrid = document.createElement("div");
    htGrid.style.cssText = "display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;margin-bottom:10px;";
    for (const [htLabel, count, color] of [["<1h (Scalp)", shortHold, "#64b5f6"], ["1h-1d (Day)", medHold, "#ffab40"], ["1d+ (Swing)", swingHold, "#ce93d8"]]) {
      const box = document.createElement("div");
      box.style.cssText = `background:#111;border:1px solid ${color};border-radius:6px;padding:8px;text-align:center;`;
      box.innerHTML = `<div style="color:${color};font-size:18px;font-weight:bold;">${count}</div><div style="color:#888;font-size:10px;">${htLabel}</div><div style="color:#666;font-size:10px;">${((count / htTotal) * 100).toFixed(0)}%</div>`;
      htGrid.appendChild(box);
    }
    container.appendChild(htGrid);

    win.appendElement(container);
    log(`PROFILE: Analyzed ${filled.length} filled orders across ${Object.keys(bySymbol).length} symbols`, "ok");
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to load profile: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// FIBO+ — Fibonacci Time Zones
// ══════════════════════════════════════════════════════════════
let fiboTimeMarkers = [];

function cmdFiboTime() {
  if (!currentChartData || currentChartData.length < 30) { log("FIBO+: Need at least 30 bars loaded", "warn"); return; }
  const chartData = currentChartData;

  // Find most recent swing low via fractal detection
  let anchorIdx = chartData.length - 10;
  for (let i = chartData.length - 3; i >= 2; i--) {
    if (chartData[i].low < chartData[i - 1].low && chartData[i].low < chartData[i - 2].low &&
        chartData[i].low < chartData[i + 1].low && chartData[i].low < chartData[i + 2].low) {
      anchorIdx = i;
      break;
    }
  }

  const fibIntervals = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89];
  const markers = [];
  const fiboList = [];

  for (const fib of fibIntervals) {
    const targetIdx = anchorIdx + fib;
    if (targetIdx >= chartData.length) {
      fiboList.push({ fib, time: null, label: `+${fib} bars (future)`, projected: true });
      continue;
    }
    const bar = chartData[targetIdx];
    markers.push({ time: bar.time, position: "aboveBar", color: "#9C27B0", shape: "arrowDown", text: `F${fib}` });
    const d = typeof bar.time === "number" ? new Date(bar.time * 1000).toLocaleDateString() : bar.time;
    fiboList.push({ fib, time: bar.time, label: `+${fib} bars = ${d}`, projected: false });
  }

  fiboTimeMarkers = markers;
  try {
    const sortedMarkers = [...markers].sort((a, b) => a.time - b.time);
    candleSeries.setMarkers(sortedMarkers);
  } catch (_) {}

  const win = createWindow({ title: `${currentSymbol} — Fibonacci Time Zones`, width: 380, height: 400 });
  win.contentElement.textContent = "";
  const container = document.createElement("div");
  container.style.cssText = "padding:14px;font-family:monospace;font-size:13px;color:#ddd;overflow-y:auto;height:100%;";

  const anchorDate = typeof chartData[anchorIdx].time === "number" ? new Date(chartData[anchorIdx].time * 1000).toLocaleDateString() : chartData[anchorIdx].time;
  const headerDiv = document.createElement("div");
  headerDiv.style.cssText = "margin-bottom:14px;padding:10px;background:#111;border:1px solid #9C27B0;border-radius:6px;text-align:center;";
  const fiboTitle = document.createElement("div"); fiboTitle.style.cssText = "color:#9C27B0;font-size:14px;font-weight:bold;"; fiboTitle.textContent = "Fibonacci Time Zones"; headerDiv.appendChild(fiboTitle);
  const fiboAnchor = document.createElement("div"); fiboAnchor.style.cssText = "color:#888;font-size:11px;margin-top:4px;"; fiboAnchor.textContent = "Anchor: Bar " + anchorIdx + " (" + anchorDate + ") \u2014 Swing Low at $" + chartData[anchorIdx].low.toFixed(2); headerDiv.appendChild(fiboAnchor);
  container.appendChild(headerDiv);

  const table = document.createElement("table");
  table.style.cssText = "width:100%;border-collapse:collapse;font-size:12px;";
  const fThead = document.createElement("thead");
  const fHeadTr = document.createElement("tr"); fHeadTr.style.cssText = "color:#888;border-bottom:1px solid #444;";
  for (const [text, align] of [["Fib #","left"],["Bars Forward","left"],["Date","left"],["Status","center"]]) { const th = document.createElement("th"); th.style.cssText = `text-align:${align};padding:4px;`; th.textContent = text; fHeadTr.appendChild(th); }
  fThead.appendChild(fHeadTr); table.appendChild(fThead);
  const tbody = document.createElement("tbody");
  for (const f of fiboList) {
    const tr = document.createElement("tr"); tr.style.borderBottom = "1px solid #333";
    const statusColor = f.projected ? "#ff9800" : "#4caf50";
    const statusText = f.projected ? "Projected" : "On Chart";
    const tdFib = document.createElement("td"); tdFib.style.cssText = "padding:4px;color:#9C27B0;font-weight:bold;"; tdFib.textContent = f.fib; tr.appendChild(tdFib);
    const tdBars = document.createElement("td"); tdBars.style.padding = "4px"; tdBars.textContent = "+" + f.fib; tr.appendChild(tdBars);
    const tdDate = document.createElement("td"); tdDate.style.padding = "4px"; tdDate.textContent = f.label.split("= ")[1] || "---"; tr.appendChild(tdDate);
    const tdStatus = document.createElement("td"); tdStatus.style.cssText = `text-align:center;padding:4px;color:${statusColor};font-size:11px;`; tdStatus.textContent = statusText; tr.appendChild(tdStatus);
    tbody.appendChild(tr);
  }
  table.appendChild(tbody); container.appendChild(table);

  const clearBtn = document.createElement("button");
  clearBtn.textContent = "Clear Markers";
  clearBtn.style.cssText = "margin-top:12px;padding:6px 16px;background:#333;color:#ddd;border:1px solid #555;border-radius:4px;cursor:pointer;";
  clearBtn.addEventListener("click", () => { fiboTimeMarkers = []; candleSeries.setMarkers([]); log("FIBO+: Markers cleared", "info"); });
  container.appendChild(clearBtn);

  win.appendElement(container);
  log(`FIBO+: Plotted ${markers.length} Fibonacci time zones from bar ${anchorIdx}`, "ok");
}

// ══════════════════════════════════════════════════════════════
// DARKMODE — Theme Switcher
// ══════════════════════════════════════════════════════════════
const THEMES = {
  dark: { name: "Dark", bg: "#0a0a14", text: "#d1d4dc", grid: "#222", panelBg: "#0a0a14", border: "#333", chartBg: "#000000", chartGrid: "#333333", chartText: "#d1d4dc", fisherBg: "#000000", fisherGrid: "#111" },
  pitchBlack: { name: "Pitch Black", bg: "#000000", text: "#cccccc", grid: "#111", panelBg: "#000000", border: "#222", chartBg: "#000000", chartGrid: "#111111", chartText: "#cccccc", fisherBg: "#000000", fisherGrid: "#0a0a0a" },
  light: { name: "Light", bg: "#f5f5f5", text: "#333333", grid: "#ddd", panelBg: "#ffffff", border: "#ccc", chartBg: "#ffffff", chartGrid: "#e0e0e0", chartText: "#333333", fisherBg: "#fafafa", fisherGrid: "#e8e8e8" },
};

function applyTheme(themeKey) {
  const t = THEMES[themeKey];
  if (!t) return;
  const root = document.documentElement.style;
  root.setProperty("--bg-color", t.bg);
  root.setProperty("--text-color", t.text);
  root.setProperty("--grid-color", t.grid);
  root.setProperty("--panel-bg", t.panelBg);
  root.setProperty("--border-color", t.border);
  document.body.style.backgroundColor = t.bg;
  document.body.style.color = t.text;
  if (chart) {
    chart.applyOptions({
      layout: { background: { color: t.chartBg }, textColor: t.chartText },
      grid: { vertLines: { color: t.chartGrid }, horzLines: { color: t.chartGrid } },
      rightPriceScale: { borderColor: t.border }, timeScale: { borderColor: t.border },
    });
  }
  if (fisherChart) {
    fisherChart.applyOptions({
      layout: { background: { color: t.fisherBg }, textColor: t.text },
      grid: { vertLines: { color: t.fisherGrid }, horzLines: { color: t.fisherGrid } },
      rightPriceScale: { borderColor: t.border },
    });
  }
  if (volumeChart) {
    volumeChart.applyOptions({
      layout: { background: { color: t.fisherBg }, textColor: t.text },
      grid: { vertLines: { color: t.fisherGrid }, horzLines: { color: t.fisherGrid } },
      rightPriceScale: { borderColor: t.border },
    });
  }
  localStorage.setItem("typhoon_theme", themeKey);
  log(`Theme switched to: ${t.name}`, "ok");
}

function loadSavedTheme() {
  const saved = localStorage.getItem("typhoon_theme");
  if (saved && THEMES[saved]) applyTheme(saved);
}

function cmdDarkMode() {
  const win = createWindow({ title: "Theme Switcher", width: 340, height: 220 });
  win.contentElement.textContent = "";
  const container = document.createElement("div");
  container.style.cssText = "padding:18px;font-family:monospace;font-size:13px;color:#ddd;";
  const titleEl = document.createElement("div");
  titleEl.style.cssText = "text-align:center;color:#888;font-size:12px;margin-bottom:16px;text-transform:uppercase;letter-spacing:1px;";
  titleEl.textContent = "Select Theme";
  container.appendChild(titleEl);
  const btnGrid = document.createElement("div");
  btnGrid.style.cssText = "display:grid;grid-template-columns:1fr 1fr 1fr;gap:12px;";
  const currentTheme = localStorage.getItem("typhoon_theme") || "dark";
  for (const [key, t] of Object.entries(THEMES)) {
    const btn = document.createElement("button");
    btn.style.cssText = `display:flex;flex-direction:column;align-items:center;gap:8px;padding:14px 8px;border-radius:8px;cursor:pointer;border:2px solid ${key === currentTheme ? "#64b5f6" : "#444"};background:#1a1a1a;transition:border-color 0.2s;`;
    const swatch = document.createElement("div");
    swatch.style.cssText = `width:48px;height:32px;border-radius:4px;background:${t.chartBg};border:1px solid ${t.border};position:relative;overflow:hidden;`;
    const gl1 = document.createElement("div"); gl1.style.cssText = `position:absolute;top:50%;left:0;right:0;height:1px;background:${t.chartGrid};`;
    const gl2 = document.createElement("div"); gl2.style.cssText = `position:absolute;left:50%;top:0;bottom:0;width:1px;background:${t.chartGrid};`;
    const td = document.createElement("div"); td.style.cssText = `position:absolute;bottom:4px;right:4px;width:6px;height:6px;border-radius:50%;background:${t.chartText};`;
    swatch.appendChild(gl1); swatch.appendChild(gl2); swatch.appendChild(td);
    btn.appendChild(swatch);
    const lbl = document.createElement("span");
    lbl.style.cssText = "color:#ccc;font-size:12px;font-weight:bold;";
    lbl.textContent = t.name;
    btn.appendChild(lbl);
    btn.addEventListener("click", () => {
      applyTheme(key);
      for (const b of btnGrid.querySelectorAll("button")) b.style.borderColor = "#444";
      btn.style.borderColor = "#64b5f6";
    });
    btn.addEventListener("mouseenter", () => { if (key !== (localStorage.getItem("typhoon_theme") || "dark")) btn.style.borderColor = "#666"; });
    btn.addEventListener("mouseleave", () => { const cur = localStorage.getItem("typhoon_theme") || "dark"; btn.style.borderColor = key === cur ? "#64b5f6" : "#444"; });
    btnGrid.appendChild(btn);
  }
  container.appendChild(btnGrid);
  win.appendElement(container);
}

// ── SNAPSHOT — Portfolio Snapshot to Clipboard ─────────────
async function cmdSnapshot() {
  const win = createWindow({ title: "Portfolio Snapshot", width: 500, height: 400 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Building snapshot...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);
  try {
    const [marginJson, posJson] = await Promise.all([
      invoke("get_margin_info"),
      invoke("get_positions"),
    ]);
    const mi = JSON.parse(marginJson);
    const positions = JSON.parse(posJson);
    const hasPositions = mi.gross_lots > 0;
    const mlText = hasPositions ? `${mi.margin_level_pct.toFixed(1)}%` : "N/A";
    const now = new Date();
    const dateStr = now.toISOString().replace("T", " ").slice(0, 16) + " UTC";
    const sep = "\u2500".repeat(44);

    let totalPL = 0;
    const rows = [];
    for (const p of positions) {
      const sym = (p.symbol || "").padEnd(8).slice(0, 8);
      const side = (p.side === "long" ? "L" : "S").padEnd(6);
      const qty = String(Math.abs(p.qty)).padStart(6);
      const entry = `$${Number(p.avg_entry_price).toFixed(2)}`.padStart(10);
      const pl = p.unrealized_pl || 0;
      totalPL += pl;
      const plStr = `$${pl.toFixed(2)}`.padStart(10);
      rows.push(`${sym}${side}${qty}  ${entry}  ${plStr}`);
    }
    if (rows.length === 0) rows.push("  (no open positions)");

    const text = [
      "TyphooN-Terminal Portfolio Snapshot",
      `Date: ${dateStr}`,
      `Equity: $${Math.round(mi.equity).toLocaleString()} | Balance: $${Math.round(mi.balance).toLocaleString()} | ML: ${mlText}`,
      sep,
      "Symbol  Side    Qty     Entry       P&L",
      ...rows,
      sep,
      `Total P&L: $${totalPL.toFixed(2)}`,
    ].join("\n");

    win.contentElement.textContent = "";
    const pre = document.createElement("pre");
    pre.style.cssText = "font-family:'Iosevka Fixed',monospace;font-size:12px;color:#ccc;padding:12px;white-space:pre;overflow:auto;max-height:calc(100% - 50px);margin:0;";
    pre.textContent = text;
    win.appendElement(pre);

    const btnRow = document.createElement("div");
    btnRow.style.cssText = "padding:8px 12px;display:flex;gap:8px;";
    const copyBtn = document.createElement("button");
    copyBtn.textContent = "Copy to Clipboard";
    copyBtn.className = "fw-btn";
    copyBtn.addEventListener("click", () => {
      navigator.clipboard.writeText(text).then(() => log("Snapshot copied to clipboard", "ok")).catch(e => log(`Clipboard copy failed: ${e}`, "error"));
    });
    btnRow.appendChild(copyBtn);
    win.appendElement(btnRow);

    // Auto-copy on open
    navigator.clipboard.writeText(text).then(() => log("Snapshot copied to clipboard", "ok")).catch(() => {});
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to build snapshot: ${e}`);
  }
}

// ── HOTLIST — Real-Time Top Movers Dashboard ───────────────
async function cmdHotlist() {
  const win = createWindow({ title: "Top Movers \u2014 Hotlist", width: 650, height: 550 });
  win.contentElement.textContent = "";

  let activeTab = "gainers";
  let refreshTimer = null;

  const tabBar = document.createElement("div");
  tabBar.style.cssText = "display:flex;gap:0;border-bottom:1px solid #333;";
  const hotlistTabs = [
    { id: "gainers", label: "Top Gainers" },
    { id: "losers", label: "Top Losers" },
    { id: "active", label: "Most Active" },
  ];
  const tabBtns = {};
  for (const t of hotlistTabs) {
    const btn = document.createElement("button");
    btn.textContent = t.label;
    btn.style.cssText = "flex:1;padding:8px;background:none;border:none;color:#888;cursor:pointer;font-size:12px;border-bottom:2px solid transparent;";
    btn.addEventListener("click", () => { activeTab = t.id; updateTabStyles(); renderData(); });
    tabBar.appendChild(btn);
    tabBtns[t.id] = btn;
  }
  win.appendElement(tabBar);

  const contentDiv = document.createElement("div");
  contentDiv.style.cssText = "overflow-y:auto;max-height:calc(100% - 80px);padding:4px;";
  win.appendElement(contentDiv);

  const statusDiv = document.createElement("div");
  statusDiv.style.cssText = "padding:4px 8px;font-size:10px;color:#555;text-align:right;";
  win.appendElement(statusDiv);

  let moversData = { gainers: [], losers: [], active: [] };

  function updateTabStyles() {
    for (const [id, btn] of Object.entries(tabBtns)) {
      btn.style.color = id === activeTab ? "#8ff" : "#888";
      btn.style.borderBottomColor = id === activeTab ? "#8ff" : "transparent";
    }
  }
  updateTabStyles();

  function renderData() {
    contentDiv.textContent = "";
    const list = moversData[activeTab] || [];
    if (list.length === 0) { contentDiv.textContent = "No data"; return; }
    const table = document.createElement("table");
    table.className = "fw-table";
    const thead = document.createElement("tr");
    for (const h of ["Symbol", "Price", "Change %", "Volume"]) {
      const th = document.createElement("td");
      th.style.cssText = "color:#666;font-weight:bold;font-size:10px;text-transform:uppercase;padding:4px 8px;";
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);
    for (const s of list) {
      const tr = document.createElement("tr");
      tr.style.cursor = "pointer";
      const sym = s.symbol || s.ticker || "\u2014";
      const price = s.last ?? s.price ?? "\u2014";
      const chgPct = s.change_percent ?? s.pct_change ?? 0;
      const vol = s.volume ? (s.volume >= 1e6 ? `${(s.volume / 1e6).toFixed(1)}M` : Number(s.volume).toLocaleString()) : "\u2014";
      const isPositive = typeof chgPct === "number" ? chgPct >= 0 : false;
      const chgColor = isPositive ? "#4caf50" : "#f44336";
      const vals = [
        { text: sym, css: "color:#8ff;font-weight:bold;" },
        { text: typeof price === "number" ? `$${price.toFixed(2)}` : String(price), css: "color:#ccc;" },
        { text: typeof chgPct === "number" ? `${chgPct >= 0 ? "+" : ""}${chgPct.toFixed(2)}%` : String(chgPct), css: `color:${chgColor};font-weight:bold;` },
        { text: vol, css: "color:#aaa;" },
      ];
      for (const v of vals) {
        const td = document.createElement("td");
        td.className = "fw-value";
        td.style.cssText = `text-align:left;padding:4px 8px;${v.css}`;
        td.textContent = v.text;
        tr.appendChild(td);
      }
      tr.addEventListener("click", () => {
        document.getElementById("symbol-input").value = sym;
        triggerLoad();
      });
      table.appendChild(tr);
    }
    contentDiv.appendChild(table);
  }

  async function fetchData() {
    try {
      const [moversJson, activeJson] = await Promise.all([
        invoke("get_top_movers", { marketType: "stocks", top: 50 }),
        invoke("get_most_active", { top: 50 }),
      ]);
      const movers = typeof moversJson === "string" ? JSON.parse(moversJson) : moversJson;
      const active = typeof activeJson === "string" ? JSON.parse(activeJson) : activeJson;
      const allMovers = Array.isArray(movers) ? movers : (movers.gainers || movers.movers || movers.stocks || []);
      moversData.gainers = allMovers.filter(s => (s.change_percent ?? s.pct_change ?? 0) >= 0).sort((a, b) => (b.change_percent ?? b.pct_change ?? 0) - (a.change_percent ?? a.pct_change ?? 0));
      moversData.losers = allMovers.filter(s => (s.change_percent ?? s.pct_change ?? 0) < 0).sort((a, b) => (a.change_percent ?? a.pct_change ?? 0) - (b.change_percent ?? b.pct_change ?? 0));
      moversData.active = Array.isArray(active) ? active : (active.most_active || active.stocks || []);
      statusDiv.textContent = `Updated ${new Date().toLocaleTimeString("en-GB", { hour12: false })}`;
      renderData();
    } catch (e) {
      contentDiv.textContent = `Error: ${e}`;
    }
  }

  await fetchData();
  refreshTimer = setInterval(fetchData, 30000);

  // Clean up interval when window is removed from DOM
  const hotlistObs = new MutationObserver(() => {
    if (!document.body.contains(win.element)) { clearInterval(refreshTimer); hotlistObs.disconnect(); }
  });
  hotlistObs.observe(document.body, { childList: true, subtree: true });
}

// ── NOTES — Per-Symbol Trading Notes ───────────────────────
function cmdNotes() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const sym = currentSymbol;
  const storageKey = `typhoon_notes_${sym}`;
  const win = createWindow({ title: `Notes \u2014 ${sym}`, width: 400, height: 350 });
  win.contentElement.textContent = "";

  const container = document.createElement("div");
  container.style.cssText = "display:flex;flex-direction:column;height:100%;padding:8px;box-sizing:border-box;";

  const textarea = document.createElement("textarea");
  textarea.style.cssText = "flex:1;width:100%;background:#1a1a2e;color:#ccc;border:1px solid #333;font-family:'Iosevka Fixed',monospace;font-size:12px;padding:8px;resize:none;border-radius:3px;box-sizing:border-box;";
  textarea.placeholder = `Trading notes for ${sym}...`;
  textarea.value = localStorage.getItem(storageKey) || "";

  let saveTimeout = null;
  textarea.addEventListener("input", () => {
    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      const val = textarea.value;
      if (val) {
        localStorage.setItem(storageKey, val);
      } else {
        localStorage.removeItem(storageKey);
      }
      updateNotesIndicator();
    }, 500);
  });

  const btnRow = document.createElement("div");
  btnRow.style.cssText = "display:flex;gap:8px;margin-top:8px;";
  const clearBtn = document.createElement("button");
  clearBtn.textContent = "Clear";
  clearBtn.className = "fw-btn";
  clearBtn.style.cssText += "background:#611;";
  clearBtn.addEventListener("click", () => {
    textarea.value = "";
    localStorage.removeItem(storageKey);
    updateNotesIndicator();
    log(`Notes cleared for ${sym}`, "ok");
  });
  btnRow.appendChild(clearBtn);
  container.appendChild(textarea);
  container.appendChild(btnRow);
  win.appendElement(container);
  textarea.focus();
}

/** Update the notes indicator in the status bar */
function updateNotesIndicator() {
  const statusBar = document.getElementById("connect-status-bar");
  if (!statusBar) return;
  const existing = statusBar.querySelector(".notes-badge");
  if (existing) existing.remove();
  if (currentSymbol && localStorage.getItem(`typhoon_notes_${currentSymbol}`)) {
    const badge = document.createElement("span");
    badge.className = "notes-badge";
    badge.textContent = " \uD83D\uDCDD";
    badge.title = "Trading notes exist for this symbol";
    badge.style.cursor = "pointer";
    badge.addEventListener("click", () => cmdNotes());
    statusBar.appendChild(badge);
  }
}

// ── TIMER — Custom Countdown Timers ────────────────────────
function cmdTimer() {
  const TIMER_KEY = "typhoon_timers";
  const win = createWindow({ title: "Countdown Timers", width: 450, height: 400 });
  win.contentElement.textContent = "";

  let timers = [];
  try { timers = JSON.parse(localStorage.getItem(TIMER_KEY) || "[]"); } catch { timers = []; }

  const container = document.createElement("div");
  container.style.cssText = "padding:8px;display:flex;flex-direction:column;height:100%;box-sizing:border-box;";

  const addSection = document.createElement("div");
  addSection.style.cssText = "border-bottom:1px solid #333;padding-bottom:8px;margin-bottom:8px;";
  const addTitle = document.createElement("div");
  addTitle.textContent = "Add Timer";
  addTitle.style.cssText = "color:#8ff;font-size:12px;font-weight:bold;margin-bottom:6px;";
  addSection.appendChild(addTitle);

  const row1 = document.createElement("div");
  row1.style.cssText = "display:flex;gap:6px;margin-bottom:6px;align-items:center;";
  const nameInput = document.createElement("input");
  nameInput.type = "text";
  nameInput.placeholder = "Timer name";
  nameInput.style.cssText = "flex:1;background:#1a1a2e;color:#ccc;border:1px solid #333;padding:4px 8px;font-size:12px;border-radius:3px;";
  const hoursInput = document.createElement("input");
  hoursInput.type = "number";
  hoursInput.placeholder = "Hours from now";
  hoursInput.min = "0";
  hoursInput.step = "0.5";
  hoursInput.style.cssText = "width:100px;background:#1a1a2e;color:#ccc;border:1px solid #333;padding:4px 8px;font-size:12px;border-radius:3px;";
  const addBtn = document.createElement("button");
  addBtn.textContent = "+";
  addBtn.className = "fw-btn";
  addBtn.style.cssText += "padding:4px 12px;";
  row1.appendChild(nameInput);
  row1.appendChild(hoursInput);
  row1.appendChild(addBtn);
  addSection.appendChild(row1);

  const presetRow = document.createElement("div");
  presetRow.style.cssText = "display:flex;gap:6px;flex-wrap:wrap;";
  const presets = [
    { label: "London Open (08:00 UTC)", getTarget: () => { const d = new Date(); d.setUTCHours(8, 0, 0, 0); if (d <= new Date()) d.setUTCDate(d.getUTCDate() + 1); return d; } },
    { label: "NY Open (14:30 UTC)", getTarget: () => { const d = new Date(); d.setUTCHours(14, 30, 0, 0); if (d <= new Date()) d.setUTCDate(d.getUTCDate() + 1); return d; } },
    { label: "Next Hour", getTarget: () => { const d = new Date(); d.setMinutes(0, 0, 0); d.setHours(d.getHours() + 1); return d; } },
  ];
  for (const p of presets) {
    const btn = document.createElement("button");
    btn.textContent = p.label;
    btn.className = "fw-btn";
    btn.style.cssText += "font-size:10px;padding:3px 8px;";
    btn.addEventListener("click", () => {
      const target = p.getTarget();
      timers.push({ name: p.label, target: target.getTime() });
      saveTimers();
      renderTimerList();
    });
    presetRow.appendChild(btn);
  }
  addSection.appendChild(presetRow);
  container.appendChild(addSection);

  addBtn.addEventListener("click", () => {
    const name = nameInput.value.trim() || "Timer";
    const hours = parseFloat(hoursInput.value);
    if (isNaN(hours) || hours <= 0) { log("Enter valid hours", "warn"); return; }
    const target = Date.now() + hours * 3600000;
    timers.push({ name, target });
    saveTimers();
    renderTimerList();
    nameInput.value = "";
    hoursInput.value = "";
  });

  const listDiv = document.createElement("div");
  listDiv.style.cssText = "flex:1;overflow-y:auto;";
  container.appendChild(listDiv);
  win.appendElement(container);

  function saveTimers() {
    if (timers.length > 100) timers = timers.slice(-100);
    localStorage.setItem(TIMER_KEY, JSON.stringify(timers));
  }

  function renderTimerList() {
    listDiv.textContent = "";
    if (timers.length === 0) {
      const msg = document.createElement("div");
      msg.textContent = "No active timers";
      msg.style.cssText = "color:#555;padding:20px;text-align:center;";
      listDiv.appendChild(msg);
      return;
    }
    for (let i = 0; i < timers.length; i++) {
      const t = timers[i];
      const row = document.createElement("div");
      row.style.cssText = "display:flex;align-items:center;justify-content:space-between;padding:6px 8px;border-bottom:1px solid #222;";
      row.dataset.timerIdx = i;

      const nameSpan = document.createElement("span");
      nameSpan.textContent = t.name;
      nameSpan.style.cssText = "color:#ccc;font-size:12px;flex:1;";

      const countdownSpan = document.createElement("span");
      countdownSpan.className = "timer-countdown";
      countdownSpan.style.cssText = "color:#8ff;font-family:'Iosevka Fixed',monospace;font-size:14px;font-weight:bold;min-width:80px;text-align:right;margin-right:8px;";

      const removeBtn = document.createElement("button");
      removeBtn.textContent = "\u00D7";
      removeBtn.style.cssText = "background:none;border:1px solid #555;color:#f44;cursor:pointer;font-size:14px;padding:2px 8px;border-radius:3px;";
      removeBtn.addEventListener("click", () => {
        timers.splice(i, 1);
        saveTimers();
        renderTimerList();
      });

      row.appendChild(nameSpan);
      row.appendChild(countdownSpan);
      row.appendChild(removeBtn);
      listDiv.appendChild(row);
    }
  }

  function updateCountdowns() {
    const now = Date.now();
    const rows = listDiv.querySelectorAll("[data-timer-idx]");
    for (const row of rows) {
      const idx = parseInt(row.dataset.timerIdx);
      if (idx >= timers.length) continue;
      const t = timers[idx];
      const remaining = t.target - now;
      const cdSpan = row.querySelector(".timer-countdown");
      if (!cdSpan) continue;
      if (remaining <= 0) {
        cdSpan.textContent = "DONE!";
        cdSpan.style.color = "#f44";
        row.style.background = "rgba(255,0,0,0.08)";
        if (!t.notified) {
          t.notified = true;
          log(`Timer "${t.name}" has expired!`, "warn");
          if (Notification.permission === "granted") {
            new Notification("TyphooN-Terminal Timer", { body: `${t.name} has expired!` });
          } else if (Notification.permission !== "denied") {
            Notification.requestPermission();
          }
        }
      } else {
        const hrs = Math.floor(remaining / 3600000);
        const mins = Math.floor((remaining % 3600000) / 60000);
        const secs = Math.floor((remaining % 60000) / 1000);
        cdSpan.textContent = `${String(hrs).padStart(2, "0")}:${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
      }
    }
  }

  renderTimerList();
  const tickInterval = setInterval(() => {
    if (!document.body.contains(win.element)) { clearInterval(tickInterval); return; }
    updateCountdowns();
  }, 1000);

  const timerObs = new MutationObserver(() => {
    if (!document.body.contains(win.element)) { clearInterval(tickInterval); timerObs.disconnect(); }
  });
  timerObs.observe(document.body, { childList: true, subtree: true });
}

// ── EXPORT — Chart Data Export to CSV ──────────────────────
function cmdExport() {
  if (!currentSymbol || currentChartData.length === 0) { log("No chart data to export", "warn"); return; }
  const win = createWindow({ title: `Export \u2014 ${currentSymbol}`, width: 650, height: 450 });
  win.contentElement.textContent = "";

  const container = document.createElement("div");
  container.style.cssText = "padding:8px;display:flex;flex-direction:column;height:100%;box-sizing:border-box;";

  const optRow = document.createElement("div");
  optRow.style.cssText = "display:flex;gap:12px;align-items:center;margin-bottom:8px;flex-wrap:wrap;";
  const exportIndicators = [
    { id: "kama", label: "KAMA(10)" },
    { id: "fisher", label: "Fisher(32)" },
    { id: "rsi", label: "RSI(14)" },
    { id: "sma200", label: "SMA(200)" },
  ];
  const checks = {};
  for (const ind of exportIndicators) {
    const lbl = document.createElement("label");
    lbl.style.cssText = "color:#ccc;font-size:11px;display:flex;align-items:center;gap:4px;cursor:pointer;";
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = true;
    checks[ind.id] = cb;
    lbl.appendChild(cb);
    lbl.appendChild(document.createTextNode(ind.label));
    optRow.appendChild(lbl);
  }
  const genBtn = document.createElement("button");
  genBtn.textContent = "Generate CSV";
  genBtn.className = "fw-btn";
  optRow.appendChild(genBtn);
  container.appendChild(optRow);

  const previewPre = document.createElement("pre");
  previewPre.style.cssText = "flex:1;font-family:'Iosevka Fixed',monospace;font-size:11px;color:#aaa;background:#111;padding:8px;overflow:auto;border:1px solid #333;border-radius:3px;margin:0;white-space:pre;";
  previewPre.textContent = "Click 'Generate CSV' to preview data";
  container.appendChild(previewPre);

  const btnRow = document.createElement("div");
  btnRow.style.cssText = "display:flex;gap:8px;margin-top:8px;";
  const dlBtn = document.createElement("button");
  dlBtn.textContent = "Download CSV";
  dlBtn.className = "fw-btn";
  dlBtn.disabled = true;
  const copyBtn = document.createElement("button");
  copyBtn.textContent = "Copy to Clipboard";
  copyBtn.className = "fw-btn";
  copyBtn.disabled = true;
  btnRow.appendChild(dlBtn);
  btnRow.appendChild(copyBtn);
  container.appendChild(btnRow);
  win.appendElement(container);

  let csvText = "";

  genBtn.addEventListener("click", () => {
    const data = currentChartData;
    const useKama = checks.kama.checked;
    const useFisher = checks.fisher.checked;
    const useRsi = checks.rsi.checked;
    const useSma200 = checks.sma200.checked;

    let kamaMap = {}, fisherMap = {}, rsiMap = {}, smaMap = {};
    if (useKama) {
      const kamaResult = calcKAMA(data, 10);
      for (const pt of kamaResult) kamaMap[pt.time] = pt.value;
    }
    if (useFisher) {
      const fisherResult = calcEhlersFisher(data, 32);
      for (const pt of fisherResult.fisher) fisherMap[pt.time] = pt.value;
    }
    if (useRsi) {
      const rsiResult = calcRSI(data, 14);
      for (const pt of rsiResult) rsiMap[pt.time] = pt.value;
    }
    if (useSma200) {
      const smaResult = calcSMA(data, 200);
      for (const pt of smaResult) smaMap[pt.time] = pt.value;
    }

    const cols = ["Date", "Open", "High", "Low", "Close", "Volume"];
    if (useKama) cols.push("KAMA");
    if (useFisher) cols.push("Fisher");
    if (useRsi) cols.push("RSI");
    if (useSma200) cols.push("SMA200");

    const csvRows = [cols.join(",")];
    for (const bar of data) {
      const t = bar.time;
      const dStr = typeof t === "number" ? new Date(t * 1000).toISOString().slice(0, 19) : String(t);
      const vals = [dStr, bar.open, bar.high, bar.low, bar.close, bar.volume || 0];
      if (useKama) vals.push(kamaMap[t] !== undefined ? kamaMap[t].toFixed(4) : "");
      if (useFisher) vals.push(fisherMap[t] !== undefined ? fisherMap[t].toFixed(4) : "");
      if (useRsi) vals.push(rsiMap[t] !== undefined ? rsiMap[t].toFixed(4) : "");
      if (useSma200) vals.push(smaMap[t] !== undefined ? smaMap[t].toFixed(4) : "");
      csvRows.push(vals.join(","));
    }
    csvText = csvRows.join("\n");

    const previewLines = csvRows.slice(0, 11).join("\n");
    previewPre.textContent = previewLines + (csvRows.length > 11 ? `\n... (${csvRows.length - 1} total rows)` : "");
    dlBtn.disabled = false;
    copyBtn.disabled = false;
    log(`CSV generated: ${csvRows.length - 1} rows, ${cols.length} columns`, "ok");
  });

  dlBtn.addEventListener("click", () => {
    if (!csvText) return;
    const dStr = new Date().toISOString().slice(0, 10).replace(/-/g, "");
    const filename = `${currentSymbol}_${currentTimeframe}_${dStr}.csv`;
    const blob = new Blob([csvText], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    log(`Downloaded ${filename}`, "ok");
  });

  copyBtn.addEventListener("click", () => {
    if (!csvText) return;
    navigator.clipboard.writeText(csvText).then(() => log("CSV copied to clipboard", "ok")).catch(e => log(`Clipboard copy failed: ${e}`, "error"));
  });
}

// ══════════════════════════════════════════════════════════════
// BOOKMAP — Heatmap Order Book Over Time
// ══════════════════════════════════════════════════════════════
function cmdBookmap() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const sym = currentSymbol;
  const isCryptoSym = /USD$|BTC$|ETH$|\//.test(sym) && !/^[A-Z]{1,5}$/.test(sym);
  const SNAP_COUNT = 100, LEVELS = 30, LEVEL_PX = 4;
  const snapshots = [];
  let bmInterval = null;

  const win = createWindow({ title: `${sym} \u2014 BOOKMAP (Heatmap)`, width: 640, height: 500, onClose() { if (bmInterval) { clearInterval(bmInterval); bmInterval = null; } } });
  win.contentElement.textContent = "";

  const canvas = document.createElement("canvas");
  canvas.width = 600; canvas.height = LEVELS * 2 * LEVEL_PX;
  canvas.style.cssText = "display:block;margin:4px auto;border:1px solid #333;";
  win.appendElement(canvas);

  const bmStatsDiv = document.createElement("div");
  bmStatsDiv.style.cssText = "padding:6px 12px;font-size:10px;font-family:Consolas,monospace;color:#ccc;display:flex;justify-content:space-around;border-top:1px solid #333;";
  win.appendElement(bmStatsDiv);

  const bmCtx = canvas.getContext("2d");
  bmCtx.fillStyle = "#0a0a14"; bmCtx.fillRect(0, 0, canvas.width, canvas.height);

  function renderHeatmap() {
    const W = canvas.width, H = canvas.height;
    bmCtx.fillStyle = "#0a0a14"; bmCtx.fillRect(0, 0, W, H);
    if (snapshots.length === 0) return;

    let maxSize = 1;
    for (const snap of snapshots) {
      for (const b of snap.bids) if (b.size > maxSize) maxSize = b.size;
      for (const a of snap.asks) if (a.size > maxSize) maxSize = a.size;
    }

    const colWidth = Math.max(1, Math.floor(W / SNAP_COUNT));
    for (let si = 0; si < snapshots.length; si++) {
      const snap = snapshots[si];
      const x = si * colWidth;
      if (x >= W) break;
      const mid = snap.mid;
      if (!mid || mid === 0) continue;

      let priceStep = mid * 0.001;
      if (snap.asks.length > 1) {
        const sorted = [...snap.asks].sort((a, b) => a.price - b.price);
        const diff = sorted[1].price - sorted[0].price;
        if (diff > 0) priceStep = diff;
      } else if (snap.bids.length > 1) {
        const sorted = [...snap.bids].sort((a, b) => b.price - a.price);
        const diff = sorted[0].price - sorted[1].price;
        if (diff > 0) priceStep = diff;
      }

      for (let lvl = -LEVELS; lvl < LEVELS; lvl++) {
        const price = mid + lvl * priceStep;
        const row = LEVELS - 1 - lvl;
        const y = row * LEVEL_PX;

        let size = 0, isBid = false, isAsk = false;
        if (lvl <= 0) {
          let closest = null, closestDist = Infinity;
          for (const b of snap.bids) { const d = Math.abs(b.price - price); if (d < closestDist) { closestDist = d; closest = b; } }
          if (closest && closestDist <= priceStep * 1.5) { size = closest.size; isBid = true; }
        }
        if (lvl >= 0) {
          let closest = null, closestDist = Infinity;
          for (const a of snap.asks) { const d = Math.abs(a.price - price); if (d < closestDist) { closestDist = d; closest = a; } }
          if (closest && closestDist <= priceStep * 1.5) { size = closest.size; isAsk = true; }
        }

        if (size > 0) {
          const intensity = Math.min(1, size / maxSize);
          const alpha = 0.15 + intensity * 0.85;
          if (isAsk) bmCtx.fillStyle = `rgba(244,67,54,${alpha.toFixed(2)})`;
          else if (isBid) bmCtx.fillStyle = `rgba(76,175,80,${alpha.toFixed(2)})`;
          bmCtx.fillRect(x, y, colWidth, LEVEL_PX);
        }
      }

      const midRow = LEVELS - 1;
      bmCtx.fillStyle = "rgba(255,255,255,0.7)";
      bmCtx.fillRect(x, midRow * LEVEL_PX, colWidth, 1);
    }

    bmCtx.fillStyle = "#888"; bmCtx.font = "9px Consolas,monospace"; bmCtx.textAlign = "left";
    bmCtx.fillText("Ask \u2191", 4, 12);
    bmCtx.fillText("Bid \u2193", 4, H - 4);
    bmCtx.textAlign = "right";
    if (snapshots.length > 0) {
      const mid = snapshots[snapshots.length - 1].mid;
      bmCtx.fillStyle = "#fff";
      bmCtx.fillText(`Mid: $${mid.toFixed(2)}`, W - 4, LEVELS * LEVEL_PX + 3);
    }
  }

  function updateBmStats() {
    if (snapshots.length === 0) { bmStatsDiv.textContent = "Waiting for data..."; return; }
    const last = snapshots[snapshots.length - 1];
    const largestBid = last.bids.reduce((max, b) => b.size > max.size ? b : max, { size: 0, price: 0 });
    const largestAsk = last.asks.reduce((max, a) => a.size > max.size ? a : max, { size: 0, price: 0 });
    const totalBidVol = last.bids.reduce((s, b) => s + b.size, 0);
    const totalAskVol = last.asks.reduce((s, a) => s + a.size, 0);
    bmStatsDiv.innerHTML = `<span style="color:#4caf50">Largest Bid: ${largestBid.size.toFixed(2)} @ $${largestBid.price.toFixed(2)}</span>` +
      `<span style="color:#f44336">Largest Ask: ${largestAsk.size.toFixed(2)} @ $${largestAsk.price.toFixed(2)}</span>` +
      `<span style="color:#4caf50">Total Bid Vol: ${totalBidVol.toFixed(2)}</span>` +
      `<span style="color:#f44336">Total Ask Vol: ${totalAskVol.toFixed(2)}</span>`;
  }

  async function fetchSnapshot() {
    try {
      let bids = [], asks = [], mid = 0;
      if (isCryptoSym) {
        const json = await invokeQuiet("get_orderbook", { symbol: sym });
        const book = typeof json === "string" ? JSON.parse(json) : json;
        bids = (book.bids || []).map(b => ({ price: parseFloat(b.price || b.p || 0), size: parseFloat(b.size || b.qty || b.s || 0) })).filter(b => b.price > 0);
        asks = (book.asks || []).map(a => ({ price: parseFloat(a.price || a.p || 0), size: parseFloat(a.size || a.qty || a.s || 0) })).filter(a => a.price > 0);
        if (bids.length > 0 && asks.length > 0) mid = (bids[0].price + asks[0].price) / 2;
        else if (bids.length > 0) mid = bids[0].price;
        else if (asks.length > 0) mid = asks[0].price;
      } else {
        const json = await invokeQuiet("get_latest_quote", { symbol: sym });
        const q = typeof json === "string" ? JSON.parse(json) : json;
        const bid = parseFloat(q.bid || q.bp || 0);
        const ask = parseFloat(q.ask || q.ap || 0);
        const bidSize = parseFloat(q.bid_size || q.bs || 1);
        const askSize = parseFloat(q.ask_size || q.as || 1);
        if (bid > 0) bids = [{ price: bid, size: bidSize }];
        if (ask > 0) asks = [{ price: ask, size: askSize }];
        mid = bid > 0 && ask > 0 ? (bid + ask) / 2 : bid || ask || lastPrice || 0;
      }
      if (mid === 0) return;
      snapshots.push({ time: Date.now(), bids, asks, mid });
      if (snapshots.length > SNAP_COUNT) snapshots.shift();
      renderHeatmap();
      updateBmStats();
    } catch (_) {}
  }

  fetchSnapshot();
  bmInterval = setInterval(fetchSnapshot, 2000);
}

// ══════════════════════════════════════════════════════════════
// DASHBOARD — Customizable Widget Dashboard
// ══════════════════════════════════════════════════════════════
function cmdDashboard() {
  const DASH_STORAGE_KEY = "typhoon_dashboard_widgets";
  const ALL_WIDGETS = ["miniChart", "positions", "watchlistPrices", "newsHeadlines", "alertStatus", "accountSummary", "signalScore", "topMovers"];
  const WIDGET_LABELS = { miniChart: "Mini Chart", positions: "Positions Summary", watchlistPrices: "Watchlist Prices", newsHeadlines: "News Headlines", alertStatus: "Alert Status", accountSummary: "Account Summary", signalScore: "Signal Score", topMovers: "Top Movers" };
  let dashEnabled;
  try { dashEnabled = JSON.parse(localStorage.getItem(DASH_STORAGE_KEY)); if (!Array.isArray(dashEnabled)) dashEnabled = null; } catch (_) { dashEnabled = null; }
  if (!dashEnabled) dashEnabled = [...ALL_WIDGETS];

  let dashInterval = null;
  const win = createWindow({ title: "DASHBOARD \u2014 Widget Grid", width: 820, height: 620, onClose() { if (dashInterval) { clearInterval(dashInterval); dashInterval = null; } } });
  win.contentElement.textContent = "";

  const configBar = document.createElement("div");
  configBar.style.cssText = "padding:4px 8px;border-bottom:1px solid #333;display:flex;align-items:center;gap:6px;flex-wrap:wrap;";
  const cfgLabel = document.createElement("span"); cfgLabel.textContent = "Widgets:"; cfgLabel.style.cssText = "color:#888;font-size:10px;font-weight:bold;";
  configBar.appendChild(cfgLabel);
  for (const wid of ALL_WIDGETS) {
    const lbl = document.createElement("label"); lbl.style.cssText = "color:#aaa;font-size:9px;display:flex;align-items:center;gap:2px;cursor:pointer;";
    const cb = document.createElement("input"); cb.type = "checkbox"; cb.checked = dashEnabled.includes(wid);
    cb.addEventListener("change", () => {
      if (cb.checked && !dashEnabled.includes(wid)) dashEnabled.push(wid);
      else if (!cb.checked) dashEnabled = dashEnabled.filter(w => w !== wid);
      localStorage.setItem(DASH_STORAGE_KEY, JSON.stringify(dashEnabled));
      renderDashGrid();
    });
    lbl.appendChild(cb); lbl.appendChild(document.createTextNode(WIDGET_LABELS[wid]));
    configBar.appendChild(lbl);
  }
  win.appendElement(configBar);

  const dashGrid = document.createElement("div");
  dashGrid.style.cssText = "display:grid;grid-template-columns:repeat(3,1fr);gap:6px;padding:6px;overflow-y:auto;max-height:540px;";
  win.appendElement(dashGrid);

  const widgetDivs = {};

  function mkWidget(id, title) {
    const div = document.createElement("div");
    div.style.cssText = "border:1px solid #333;border-radius:4px;padding:6px;min-height:120px;background:#0a0a14;";
    const hdr = document.createElement("div"); hdr.textContent = title; hdr.style.cssText = "font-size:10px;font-weight:bold;color:#ff8;margin-bottom:4px;border-bottom:1px solid #222;padding-bottom:3px;";
    const body = document.createElement("div"); body.style.cssText = "font-size:10px;color:#ccc;font-family:Consolas,monospace;overflow:hidden;";
    div.appendChild(hdr); div.appendChild(body);
    widgetDivs[id] = body;
    return div;
  }

  function renderDashGrid() {
    dashGrid.textContent = "";
    for (const wid of ALL_WIDGETS) {
      if (!dashEnabled.includes(wid)) continue;
      dashGrid.appendChild(mkWidget(wid, WIDGET_LABELS[wid]));
    }
    refreshDashAll();
  }

  async function refreshDashAll() {
    if (widgetDivs.miniChart && dashEnabled.includes("miniChart")) {
      const body = widgetDivs.miniChart; body.textContent = "";
      const sym = currentSymbol || "SPY";
      const cKey = `${sym}:1Day`;
      let bars = barCache[cKey] && barCache[cKey].data ? barCache[cKey].data.slice(-50) : null;
      if (!bars) { try { const bj = await invokeQuiet("get_bars", { symbol: sym, timeframe: "1Day", limit: 50 }); bars = JSON.parse(bj); } catch (_) {} }
      if (bars && bars.length > 2) {
        const cv = document.createElement("canvas"); cv.width = 240; cv.height = 80; cv.style.cssText = "display:block;width:100%;";
        body.appendChild(cv);
        const ct = cv.getContext("2d");
        const closes = bars.map(b => b.close || b.c || 0);
        const minC = Math.min(...closes), maxC = Math.max(...closes), rng = maxC - minC || 1;
        ct.strokeStyle = closes[closes.length - 1] >= closes[0] ? "#4caf50" : "#f44336"; ct.lineWidth = 1.5; ct.beginPath();
        for (let i = 0; i < closes.length; i++) { const px = (i / (closes.length - 1)) * cv.width; const py = cv.height - ((closes[i] - minC) / rng) * (cv.height - 4) - 2; if (i === 0) ct.moveTo(px, py); else ct.lineTo(px, py); }
        ct.stroke();
        const lbl = document.createElement("div"); lbl.style.cssText = "color:#888;font-size:9px;margin-top:2px;"; lbl.textContent = `${sym} $${closes[closes.length - 1].toFixed(2)}`; body.appendChild(lbl);
      } else { body.textContent = "No chart data"; }
    }

    if (widgetDivs.positions && dashEnabled.includes("positions")) {
      const body = widgetDivs.positions; body.textContent = "";
      try {
        const pj = await invokeQuiet("get_positions"); const positions = JSON.parse(pj);
        if (positions.length === 0) { body.textContent = "No open positions"; }
        else {
          let totalPL = 0;
          for (const p of positions.slice(0, 6)) {
            const psym = p.symbol || p.S || "?";
            const pl = parseFloat(p.unrealized_pl || p.unrealized_pnl || 0);
            totalPL += pl;
            const row = document.createElement("div"); row.style.cssText = `display:flex;justify-content:space-between;padding:1px 0;color:${pl >= 0 ? "#4caf50" : "#f44336"};`;
            row.innerHTML = `<span>${psym}</span><span>${pl >= 0 ? "+" : ""}$${pl.toFixed(2)}</span>`;
            body.appendChild(row);
          }
          const tot = document.createElement("div"); tot.style.cssText = `border-top:1px solid #333;margin-top:2px;padding-top:2px;font-weight:bold;color:${totalPL >= 0 ? "#4caf50" : "#f44336"};text-align:right;`;
          tot.textContent = `Total: ${totalPL >= 0 ? "+" : ""}$${totalPL.toFixed(2)}`; body.appendChild(tot);
        }
      } catch (_) { body.textContent = "Failed to load positions"; }
    }

    if (widgetDivs.watchlistPrices && dashEnabled.includes("watchlistPrices")) {
      const body = widgetDivs.watchlistPrices; body.textContent = "";
      const wl = getWatchlist();
      if (wl.length === 0) { body.textContent = "No watchlist symbols"; }
      else {
        for (const wsym of wl.slice(0, 8)) {
          try {
            const qj = await invokeQuiet("get_latest_quote", { symbol: wsym }); const q = JSON.parse(qj);
            const price = parseFloat(q.ask || q.ap || q.price || q.last || 0);
            const ck = `${wsym}:1Day`; const cached = barCache[ck];
            let chg = 0;
            if (cached && cached.data && cached.data.length >= 2) { const prev = cached.data[cached.data.length - 2].close; if (prev > 0) chg = ((price - prev) / prev) * 100; }
            const row = document.createElement("div"); row.style.cssText = "display:flex;justify-content:space-between;padding:1px 0;";
            row.innerHTML = `<span style="color:#8ff">${wsym}</span><span>$${price.toFixed(2)}</span><span style="color:${chg >= 0 ? "#4caf50" : "#f44336"}">${chg >= 0 ? "+" : ""}${chg.toFixed(2)}%</span>`;
            body.appendChild(row);
          } catch (_) {}
        }
      }
    }

    if (widgetDivs.newsHeadlines && dashEnabled.includes("newsHeadlines")) {
      const body = widgetDivs.newsHeadlines; body.textContent = "";
      try {
        const nj = await invokeQuiet("get_news", { symbol: currentSymbol || "SPY", limit: 5 }); const news = JSON.parse(nj);
        if (!Array.isArray(news) || news.length === 0) { body.textContent = "No recent news"; }
        else {
          for (const n of news.slice(0, 5)) {
            const d = document.createElement("div"); d.style.cssText = "padding:2px 0;border-bottom:1px solid #111;";
            d.innerHTML = `<span style="color:#8cf;font-size:9px;">${(n.headline || n.title || "").substring(0, 60)}</span>`;
            body.appendChild(d);
          }
        }
      } catch (_) { body.textContent = "Failed to load news"; }
    }

    if (widgetDivs.alertStatus && dashEnabled.includes("alertStatus")) {
      const body = widgetDivs.alertStatus; body.textContent = "";
      const allAlerts = [...priceAlerts, ...multiConditionAlerts];
      const active = allAlerts.filter(a => !a.triggered);
      body.innerHTML = `<div>Active alerts: <span style="color:#ff8;font-weight:bold;">${active.length}</span></div>` +
        `<div>Total alerts: ${allAlerts.length}</div>`;
      if (active.length > 0) {
        const next = active[0];
        body.innerHTML += `<div style="margin-top:4px;color:#8cf;">Next: ${next.symbol || next.name || "?"} @ $${next.price || "?"}</div>`;
      }
    }

    if (widgetDivs.accountSummary && dashEnabled.includes("accountSummary")) {
      const body = widgetDivs.accountSummary; body.textContent = "";
      try {
        const aj = await invokeQuiet("get_account"); const ac = JSON.parse(aj);
        const equity = parseFloat(ac.equity || ac.portfolio_value || 0);
        const balance = parseFloat(ac.cash || ac.balance || 0);
        const margin = parseFloat(ac.initial_margin || ac.margin_used || 0);
        const bp = parseFloat(ac.buying_power || 0);
        body.innerHTML = `<div>Equity: <span style="color:#4caf50">$${equity.toLocaleString(undefined, { minimumFractionDigits: 2 })}</span></div>` +
          `<div>Cash: $${balance.toLocaleString(undefined, { minimumFractionDigits: 2 })}</div>` +
          `<div>Margin Used: $${margin.toLocaleString(undefined, { minimumFractionDigits: 2 })}</div>` +
          `<div>Buying Power: $${bp.toLocaleString(undefined, { minimumFractionDigits: 2 })}</div>`;
      } catch (_) { body.textContent = "Failed to load account"; }
    }

    if (widgetDivs.signalScore && dashEnabled.includes("signalScore")) {
      const body = widgetDivs.signalScore; body.textContent = "";
      const sym = currentSymbol || "SPY";
      const cKey = `${sym}:1Day`; const cached = barCache[cKey];
      let sigData = cached && cached.data ? cached.data : null;
      if (!sigData) { try { const bj = await invokeQuiet("get_bars", { symbol: sym, timeframe: "1Day", limit: 220 }); sigData = JSON.parse(bj); } catch (_) {} }
      if (sigData && sigData.length > 20) {
        let score = 0, signals = [];
        const rsiData = calcRSI(sigData, 14);
        const latestRSI = rsiData.length > 0 ? rsiData[rsiData.length - 1].value : 50;
        if (latestRSI < 30) { score += 2; signals.push("RSI oversold"); } else if (latestRSI > 70) { score -= 2; signals.push("RSI overbought"); }
        if (sigData.length >= 200) {
          const sma200 = calcSMA(sigData, 200);
          const lastSMA = sma200.length > 0 ? sma200[sma200.length - 1].value : 0;
          if (sigData[sigData.length - 1].close > lastSMA) { score += 1; signals.push("Above SMA200"); } else { score -= 1; signals.push("Below SMA200"); }
        }
        if (sigData.length >= 50) {
          const sma50 = calcSMA(sigData, 50);
          const lastSMA50 = sma50.length > 0 ? sma50[sma50.length - 1].value : 0;
          if (sigData[sigData.length - 1].close > lastSMA50) { score += 1; signals.push("Above SMA50"); } else { score -= 1; signals.push("Below SMA50"); }
        }
        const scoreColor = score >= 2 ? "#4caf50" : score <= -2 ? "#f44336" : "#ff9800";
        const slabel = score >= 2 ? "BULLISH" : score <= -2 ? "BEARISH" : "NEUTRAL";
        body.innerHTML = `<div style="font-size:16px;font-weight:bold;color:${scoreColor};text-align:center;">${slabel} (${score >= 0 ? "+" : ""}${score})</div>` +
          `<div style="margin-top:4px;font-size:9px;color:#888;">${sym} | RSI: ${latestRSI.toFixed(1)}</div>` +
          `<div style="font-size:9px;color:#666;">${signals.join(" | ")}</div>`;
      } else { body.textContent = `No data for ${sym}`; }
    }

    if (widgetDivs.topMovers && dashEnabled.includes("topMovers")) {
      const body = widgetDivs.topMovers; body.textContent = "";
      try {
        const mj = await invokeQuiet("get_top_movers", { marketType: "stocks", top: 6 }); const movers = JSON.parse(mj);
        const gainers = (movers.gainers || []).slice(0, 3);
        const losers = (movers.losers || []).slice(0, 3);
        if (gainers.length > 0) {
          const gh = document.createElement("div"); gh.textContent = "Gainers"; gh.style.cssText = "color:#4caf50;font-weight:bold;font-size:9px;"; body.appendChild(gh);
          for (const g of gainers) {
            const r = document.createElement("div"); r.style.cssText = "display:flex;justify-content:space-between;color:#4caf50;font-size:9px;";
            r.innerHTML = `<span>${g.symbol || g.S || "?"}</span><span>+${(g.percent_change || g.change_percent || 0).toFixed(2)}%</span>`;
            body.appendChild(r);
          }
        }
        if (losers.length > 0) {
          const lh = document.createElement("div"); lh.textContent = "Losers"; lh.style.cssText = "color:#f44336;font-weight:bold;font-size:9px;margin-top:4px;"; body.appendChild(lh);
          for (const lo of losers) {
            const r = document.createElement("div"); r.style.cssText = "display:flex;justify-content:space-between;color:#f44336;font-size:9px;";
            r.innerHTML = `<span>${lo.symbol || lo.S || "?"}</span><span>${(lo.percent_change || lo.change_percent || 0).toFixed(2)}%</span>`;
            body.appendChild(r);
          }
        }
        if (gainers.length === 0 && losers.length === 0) body.textContent = "No mover data";
      } catch (_) { body.textContent = "Failed to load movers"; }
    }
  }

  renderDashGrid();
  dashInterval = setInterval(refreshDashAll, 10000);
}

// ══════════════════════════════════════════════════════════════
// SCANNER-RT — Real-Time Multi-Symbol Scanner
// ══════════════════════════════════════════════════════════════
function cmdScannerRT() {
  let scanInterval = null;
  let scanning = false;

  const win = createWindow({ title: "SCANNER-RT \u2014 Real-Time Scanner", width: 680, height: 500, onClose() { scanning = false; if (scanInterval) { clearInterval(scanInterval); scanInterval = null; } } });
  win.contentElement.textContent = "";

  const scanForm = document.createElement("div");
  scanForm.style.cssText = "padding:8px;border-bottom:1px solid #333;font-size:11px;";

  const condRow = document.createElement("div"); condRow.style.cssText = "display:flex;gap:8px;align-items:center;margin-bottom:6px;";
  const condLabel = document.createElement("span"); condLabel.textContent = "Condition:"; condLabel.style.cssText = "color:#888;font-size:10px;";
  const condSel = document.createElement("select"); condSel.style.cssText = "flex:1;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:4px;";
  const scanConditions = [
    { value: "rsi_below_30", label: "RSI crossed below 30" },
    { value: "rsi_above_70", label: "RSI crossed above 70" },
    { value: "price_above_sma200", label: "Price broke above SMA200" },
    { value: "price_below_sma200", label: "Price broke below SMA200" },
    { value: "volume_2x", label: "Volume > 2x average" },
    { value: "52w_high", label: "New 52-week high" },
    { value: "52w_low", label: "New 52-week low" },
  ];
  for (const c of scanConditions) { const opt = document.createElement("option"); opt.value = c.value; opt.textContent = c.label; condSel.appendChild(opt); }
  condRow.appendChild(condLabel); condRow.appendChild(condSel);
  scanForm.appendChild(condRow);

  const srcRow = document.createElement("div"); srcRow.style.cssText = "display:flex;gap:12px;align-items:center;margin-bottom:6px;";
  const srcLabel = document.createElement("span"); srcLabel.textContent = "Source:"; srcLabel.style.cssText = "color:#888;font-size:10px;";
  const mkScanRadio = (name, value, label, checked) => {
    const lbl = document.createElement("label"); lbl.style.cssText = "color:#ccc;font-size:10px;display:flex;align-items:center;gap:3px;cursor:pointer;";
    const radio = document.createElement("input"); radio.type = "radio"; radio.name = name; radio.value = value; if (checked) radio.checked = true;
    lbl.appendChild(radio); lbl.appendChild(document.createTextNode(label)); return { lbl, radio };
  };
  const srcWL = mkScanRadio("scanrt-src", "watchlist", "Watchlist", true);
  const srcMA = mkScanRadio("scanrt-src", "most_active", "Most Active", false);
  srcRow.appendChild(srcLabel); srcRow.appendChild(srcWL.lbl); srcRow.appendChild(srcMA.lbl);
  scanForm.appendChild(srcRow);

  const btnRow = document.createElement("div"); btnRow.style.cssText = "display:flex;gap:8px;align-items:center;";
  const scanToggleBtn = document.createElement("button"); scanToggleBtn.textContent = "Start Scanning";
  scanToggleBtn.style.cssText = "font-size:11px;padding:4px 14px;background:#1b5e20;color:#8f8;border:1px solid #555;cursor:pointer;font-weight:bold;";
  const scanStatusSpan = document.createElement("span"); scanStatusSpan.style.cssText = "color:#888;font-size:10px;"; scanStatusSpan.textContent = "Stopped";
  btnRow.appendChild(scanToggleBtn); btnRow.appendChild(scanStatusSpan);
  scanForm.appendChild(btnRow);
  win.appendElement(scanForm);

  const resDiv = document.createElement("div"); resDiv.style.cssText = "overflow-y:auto;max-height:360px;font-size:10px;";
  const tblHeader = document.createElement("div"); tblHeader.style.cssText = "display:flex;padding:4px 8px;border-bottom:1px solid #444;color:#666;font-weight:bold;font-size:9px;position:sticky;top:0;background:#0a0a14;";
  for (const h of ["Time", "Symbol", "Condition", "Value", "Action"]) { const s = document.createElement("span"); s.style.cssText = "flex:1;text-align:center;"; s.textContent = h; tblHeader.appendChild(s); }
  resDiv.appendChild(tblHeader);
  const resBody = document.createElement("div");
  resDiv.appendChild(resBody);
  win.appendElement(resDiv);

  function addScanResult(sym, condLbl, value) {
    const time = new Date().toLocaleTimeString("en-GB", { hour12: false });
    const row = document.createElement("div"); row.style.cssText = "display:flex;padding:3px 8px;border-bottom:1px solid #1a1a2e;cursor:pointer;";
    row.addEventListener("mouseenter", function() { this.style.background = "#1a1a2e"; }); row.addEventListener("mouseleave", function() { this.style.background = ""; });
    row.addEventListener("click", () => { document.getElementById("symbol-input").value = sym; triggerLoad(); });
    const vals = [
      { text: time, css: "color:#888;" },
      { text: sym, css: "color:#8ff;font-weight:bold;" },
      { text: condLbl, css: "color:#ccc;" },
      { text: value, css: "color:#ff8;font-family:Consolas,monospace;" },
      { text: "Load", css: "color:#8cf;text-decoration:underline;" },
    ];
    for (const v of vals) { const sp = document.createElement("span"); sp.style.cssText = "flex:1;text-align:center;" + v.css; sp.textContent = v.text; row.appendChild(sp); }
    resBody.insertBefore(row, resBody.firstChild);

    if (Notification.permission === "granted") {
      try { new Notification(`SCANNER-RT: ${sym}`, { body: `${condLbl}: ${value}`, icon: "/favicon.ico" }); } catch (_) {}
    }
  }

  async function runScan() {
    const src = srcWL.radio.checked ? "watchlist" : "most_active";
    let symbols = [];
    if (src === "watchlist") {
      symbols = getWatchlist();
    } else {
      try { const aj = await invokeQuiet("get_most_active", { top: 20 }); const active = JSON.parse(aj); symbols = (active.most_actives || active || []).map(a => a.symbol || a.S || "").filter(s => s); } catch (_) {}
    }
    if (symbols.length === 0) { scanStatusSpan.textContent = "No symbols to scan"; return; }
    scanStatusSpan.textContent = `Scanning ${symbols.length} symbols every 60s...`;
    const cond = condSel.value;

    for (const sym of symbols) {
      if (!scanning) break;
      try {
        const barsJson = await invokeQuiet("get_bars", { symbol: sym, timeframe: "1Day", limit: 220 });
        const bars = JSON.parse(barsJson);
        if (!bars || bars.length < 20) continue;
        barCache[`${sym}:1Day`] = { data: bars, timestamp: Date.now(), lastAccess: Date.now() };

        const closes = bars.map(b => b.close || b.c || 0);
        const curPrice = closes[closes.length - 1];
        const prevPrice = closes[closes.length - 2];

        if (cond === "rsi_below_30") {
          const rsiData = calcRSI(bars, 14);
          if (rsiData.length >= 2) {
            const cur = rsiData[rsiData.length - 1].value;
            const prev = rsiData[rsiData.length - 2].value;
            if (cur < 30 && prev >= 30) addScanResult(sym, "RSI crossed below 30", cur.toFixed(1));
          }
        } else if (cond === "rsi_above_70") {
          const rsiData = calcRSI(bars, 14);
          if (rsiData.length >= 2) {
            const cur = rsiData[rsiData.length - 1].value;
            const prev = rsiData[rsiData.length - 2].value;
            if (cur > 70 && prev <= 70) addScanResult(sym, "RSI crossed above 70", cur.toFixed(1));
          }
        } else if (cond === "price_above_sma200") {
          if (bars.length >= 201) {
            const sma = calcSMA(bars, 200);
            if (sma.length >= 2) {
              const curSMA = sma[sma.length - 1].value;
              const prevSMA = sma[sma.length - 2].value;
              if (curPrice > curSMA && prevPrice <= prevSMA) addScanResult(sym, "Price broke above SMA200", `$${curPrice.toFixed(2)} > $${curSMA.toFixed(2)}`);
            }
          }
        } else if (cond === "price_below_sma200") {
          if (bars.length >= 201) {
            const sma = calcSMA(bars, 200);
            if (sma.length >= 2) {
              const curSMA = sma[sma.length - 1].value;
              const prevSMA = sma[sma.length - 2].value;
              if (curPrice < curSMA && prevPrice >= prevSMA) addScanResult(sym, "Price broke below SMA200", `$${curPrice.toFixed(2)} < $${curSMA.toFixed(2)}`);
            }
          }
        } else if (cond === "volume_2x") {
          const vols = bars.slice(-21, -1).map(b => b.volume || 0);
          const avgVol = vols.reduce((a, b) => a + b, 0) / vols.length;
          const curVol = bars[bars.length - 1].volume || 0;
          if (avgVol > 0 && curVol > avgVol * 2) addScanResult(sym, "Volume > 2x average", `${(curVol / avgVol).toFixed(1)}x`);
        } else if (cond === "52w_high") {
          const high252 = Math.max(...closes.slice(-252));
          if (curPrice >= high252 && curPrice > prevPrice) addScanResult(sym, "New 52-week high", `$${curPrice.toFixed(2)}`);
        } else if (cond === "52w_low") {
          const low252 = Math.min(...closes.slice(-252).filter(c => c > 0));
          if (curPrice <= low252 && curPrice < prevPrice) addScanResult(sym, "New 52-week low", `$${curPrice.toFixed(2)}`);
        }
      } catch (_) {}
    }
    scanStatusSpan.textContent = `Scanned ${symbols.length} symbols. Next scan in 60s...`;
  }

  scanToggleBtn.addEventListener("click", () => {
    scanning = !scanning;
    if (scanning) {
      scanToggleBtn.textContent = "Stop Scanning"; scanToggleBtn.style.background = "#b71c1c"; scanToggleBtn.style.color = "#faa";
      if (Notification.permission === "default") Notification.requestPermission();
      runScan();
      scanInterval = setInterval(runScan, 60000);
    } else {
      scanToggleBtn.textContent = "Start Scanning"; scanToggleBtn.style.background = "#1b5e20"; scanToggleBtn.style.color = "#8f8";
      scanStatusSpan.textContent = "Stopped";
      if (scanInterval) { clearInterval(scanInterval); scanInterval = null; }
    }
  });
}

// ══════════════════════════════════════════════════════════════
// ALGO — Live Algorithm Monitor
// ══════════════════════════════════════════════════════════════
function cmdAlgoMonitor() {
  let algoInterval = null;
  const win = createWindow({ title: "ALGO \u2014 Live Algorithm Monitor", width: 550, height: 420, onClose() { if (algoInterval) { clearInterval(algoInterval); algoInterval = null; } } });
  win.contentElement.textContent = "";

  const algoContainer = document.createElement("div");
  algoContainer.style.cssText = "padding:12px;font-size:11px;font-family:Consolas,monospace;color:#ccc;";
  win.appendElement(algoContainer);

  function renderAlgoMonitor() {
    algoContainer.textContent = "";
    let state = null;
    try {
      const raw = localStorage.getItem("typhoon_autotrade_state");
      if (raw) state = JSON.parse(raw);
    } catch (_) {}

    if (!state) {
      const msg = document.createElement("div");
      msg.style.cssText = "color:#888;padding:30px;text-align:center;line-height:2;";
      msg.innerHTML = `<div style="font-size:14px;color:#ff8;margin-bottom:8px;">No Active Strategies</div>` +
        `<div>No active strategies. Use Ctrl+K \u2192 AUTOTRADE to start one.</div>`;
      algoContainer.appendChild(msg);
      return;
    }

    const hdr = document.createElement("div"); hdr.style.cssText = "margin-bottom:12px;";
    const stratName = state.strategyName || state.strategy || state.plugin || "Unknown Strategy";
    const status = state.status || (state.active ? "Running" : "Stopped");
    const statusColor = status === "Running" ? "#4caf50" : status === "Error" ? "#f44336" : "#ff9800";
    hdr.innerHTML = `<div style="font-size:14px;font-weight:bold;color:#ff8;">${stratName}</div>` +
      `<div style="margin-top:4px;">Status: <span style="color:${statusColor};font-weight:bold;">${status}</span></div>`;
    algoContainer.appendChild(hdr);

    const posSection = document.createElement("div"); posSection.style.cssText = "border:1px solid #333;border-radius:4px;padding:8px;margin-bottom:8px;";
    const posTitle = document.createElement("div"); posTitle.textContent = "Current Position"; posTitle.style.cssText = "font-weight:bold;color:#8cf;margin-bottom:4px;font-size:10px;";
    posSection.appendChild(posTitle);
    if (state.position && state.position.symbol) {
      const p = state.position;
      const pl = parseFloat(p.unrealized_pl || p.pnl || 0);
      posSection.innerHTML += `<div>Symbol: <span style="color:#fff;">${p.symbol}</span></div>` +
        `<div>Side: <span style="color:${p.side === "long" ? "#4caf50" : "#f44336"}">${(p.side || "N/A").toUpperCase()}</span></div>` +
        `<div>Qty: ${p.qty || 0}</div>` +
        `<div>P&L: <span style="color:${pl >= 0 ? "#4caf50" : "#f44336"}">${pl >= 0 ? "+" : ""}$${pl.toFixed(2)}</span></div>`;
    } else {
      posSection.innerHTML += `<div style="color:#666;">No position</div>`;
    }
    algoContainer.appendChild(posSection);

    const sigSection = document.createElement("div"); sigSection.style.cssText = "border:1px solid #333;border-radius:4px;padding:8px;margin-bottom:8px;";
    const sigTitle = document.createElement("div"); sigTitle.textContent = "Signals Generated"; sigTitle.style.cssText = "font-weight:bold;color:#8cf;margin-bottom:4px;font-size:10px;";
    sigSection.appendChild(sigTitle);
    const totalSignals = state.signalCount || state.signals || 0;
    const lastSignal = state.lastSignal || null;
    sigSection.innerHTML += `<div>Total Signals: <span style="color:#ff8;">${totalSignals}</span></div>`;
    if (lastSignal) {
      sigSection.innerHTML += `<div>Last: <span style="color:#ccc;">${lastSignal.type || "?"} ${lastSignal.symbol || ""} @ ${lastSignal.time || "?"}</span></div>`;
    }
    algoContainer.appendChild(sigSection);

    const perfSection = document.createElement("div"); perfSection.style.cssText = "border:1px solid #333;border-radius:4px;padding:8px;margin-bottom:8px;";
    const perfTitle = document.createElement("div"); perfTitle.textContent = "Performance Since Start"; perfTitle.style.cssText = "font-weight:bold;color:#8cf;margin-bottom:4px;font-size:10px;";
    perfSection.appendChild(perfTitle);
    const totalPL = parseFloat(state.totalPL || state.total_pnl || 0);
    const tradeCount = state.tradeCount || state.trades || 0;
    const winRate = state.winRate || (state.wins && tradeCount > 0 ? ((state.wins / tradeCount) * 100) : 0);
    const lastBar = state.lastBarTime || state.lastBar || "N/A";
    perfSection.innerHTML += `<div>Total P&L: <span style="color:${totalPL >= 0 ? "#4caf50" : "#f44336"};font-weight:bold;">${totalPL >= 0 ? "+" : ""}$${totalPL.toFixed(2)}</span></div>` +
      `<div>Trades: ${tradeCount} | Win Rate: ${typeof winRate === "number" ? winRate.toFixed(1) : winRate}%</div>` +
      `<div>Last Bar: <span style="color:#888;">${lastBar}</span></div>`;
    algoContainer.appendChild(perfSection);

    const stopBtn = document.createElement("button"); stopBtn.textContent = "Stop Strategy";
    stopBtn.style.cssText = "padding:6px 16px;font-size:11px;background:#b71c1c;color:#faa;border:1px solid #555;cursor:pointer;font-weight:bold;width:100%;";
    stopBtn.addEventListener("click", () => {
      try {
        const cur = JSON.parse(localStorage.getItem("typhoon_autotrade_state") || "{}");
        cur.status = "Stopped";
        cur.active = false;
        cur.stopRequested = true;
        localStorage.setItem("typhoon_autotrade_state", JSON.stringify(cur));
        log("Strategy stop requested via ALGO monitor", "warn");
        renderAlgoMonitor();
      } catch (_) {}
    });
    algoContainer.appendChild(stopBtn);
  }

  renderAlgoMonitor();
  algoInterval = setInterval(renderAlgoMonitor, 5000);
}

// ══════════════════════════════════════════════════════════════
// JOURNAL+ — Enhanced Trade Journal with Auto-Screenshots
// ══════════════════════════════════════════════════════════════

async function cmdJournalPlus() {
  const win = createWindow({ title: "Journal+ (Enhanced Trade Journal)", width: 800, height: 600 });
  win.contentElement.textContent = "";
  const JOURNAL_PLUS_KEY = "typhoon_journal_plus";
  const TAGS = ["Breakout", "Mean Reversion", "Earnings Play", "FOMO", "Revenge Trade", "Trend Follow", "Scalp", "Swing", "News", "Other"];
  function loadJP() { try { return JSON.parse(localStorage.getItem(JOURNAL_PLUS_KEY) || "[]"); } catch { return []; } }
  function saveJP(entries) { if (entries.length > 500) entries = entries.slice(-500); localStorage.setItem(JOURNAL_PLUS_KEY, JSON.stringify(entries)); }
  let jpEntries = loadJP();
  let filterTag = "ALL", filterSymbol = "", filterPnL = "all", filterDateFrom = "", filterDateTo = "";
  const root = document.createElement("div");
  root.style.cssText = "display:flex;flex-direction:column;height:100%;font-size:11px;";
  const filterBar = document.createElement("div");
  filterBar.style.cssText = "display:flex;gap:4px;padding:4px;border-bottom:1px solid #333;flex-wrap:wrap;align-items:center;";
  const jpInputStyle = "background:#111;color:#fff;border:1px solid #555;padding:3px 4px;font-size:10px;font-family:inherit;";
  const tagFilter = document.createElement("select"); tagFilter.style.cssText = jpInputStyle;
  const allOpt = document.createElement("option"); allOpt.value = "ALL"; allOpt.textContent = "All Tags"; tagFilter.appendChild(allOpt);
  for (const t of TAGS) { const o = document.createElement("option"); o.value = t; o.textContent = t; tagFilter.appendChild(o); }
  tagFilter.addEventListener("change", () => { filterTag = tagFilter.value; renderJP(); });
  const symFilter = document.createElement("input"); symFilter.placeholder = "Symbol"; symFilter.style.cssText = jpInputStyle + "width:60px;";
  symFilter.addEventListener("input", () => { filterSymbol = symFilter.value.trim().toUpperCase(); renderJP(); });
  const pnlFilter = document.createElement("select"); pnlFilter.style.cssText = jpInputStyle;
  for (const [v, l] of [["all","All P&L"],["winners","Winners"],["losers","Losers"]]) { const o = document.createElement("option"); o.value = v; o.textContent = l; pnlFilter.appendChild(o); }
  pnlFilter.addEventListener("change", () => { filterPnL = pnlFilter.value; renderJP(); });
  const dateFrom = document.createElement("input"); dateFrom.type = "date"; dateFrom.style.cssText = jpInputStyle;
  dateFrom.addEventListener("change", () => { filterDateFrom = dateFrom.value; renderJP(); });
  const dateTo = document.createElement("input"); dateTo.type = "date"; dateTo.style.cssText = jpInputStyle;
  dateTo.addEventListener("change", () => { filterDateTo = dateTo.value; renderJP(); });
  const exportBtn = document.createElement("button"); exportBtn.textContent = "Export CSV";
  exportBtn.style.cssText = "padding:3px 8px;background:#0a5f38;color:#8f8;border:1px solid #555;cursor:pointer;font-size:10px;font-family:inherit;";
  exportBtn.addEventListener("click", () => {
    const filtered = getFiltered();
    const header = "Date,Symbol,Side,Qty,Entry,Exit,P&L,Tag,Rating,Notes\n";
    const csvData = header + filtered.map(e => '"' + e.date + '","' + e.symbol + '","' + e.side + '",' + e.qty + ',' + e.entry + ',' + e.exit + ',' + e.pnl + ',"' + e.tag + '",' + e.rating + ',"' + (e.notes||"").replace(/"/g,'""') + '"').join("\n");
    const blob = new Blob([csvData], { type: "text/csv" });
    const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = "journal_plus.csv"; a.click();
    log("Journal+ exported as CSV", "ok");
  });
  const viewToggle = document.createElement("button"); viewToggle.textContent = "Calendar";
  viewToggle.style.cssText = "padding:3px 8px;background:#333;color:#ccc;border:1px solid #555;cursor:pointer;font-size:10px;font-family:inherit;margin-left:auto;";
  let viewMode = "list";
  viewToggle.addEventListener("click", () => { viewMode = viewMode === "list" ? "calendar" : viewMode === "calendar" ? "stats" : "list"; viewToggle.textContent = viewMode === "list" ? "Calendar" : viewMode === "calendar" ? "Stats" : "List"; renderJP(); });
  filterBar.appendChild(tagFilter); filterBar.appendChild(symFilter); filterBar.appendChild(pnlFilter);
  filterBar.appendChild(dateFrom); filterBar.appendChild(dateTo); filterBar.appendChild(exportBtn); filterBar.appendChild(viewToggle);
  root.appendChild(filterBar);
  const jpContent = document.createElement("div"); jpContent.style.cssText = "flex:1;overflow-y:auto;padding:4px;";
  root.appendChild(jpContent);
  function getFiltered() {
    return jpEntries.filter(e => {
      if (filterTag !== "ALL" && e.tag !== filterTag) return false;
      if (filterSymbol && e.symbol !== filterSymbol) return false;
      if (filterPnL === "winners" && e.pnl <= 0) return false;
      if (filterPnL === "losers" && e.pnl >= 0) return false;
      if (filterDateFrom && e.date < filterDateFrom) return false;
      if (filterDateTo && e.date > filterDateTo + "T23:59") return false;
      return true;
    });
  }
  function renderJP() { jpContent.textContent = ""; if (viewMode === "list") renderJPList(); else if (viewMode === "calendar") renderJPCalendar(); else renderJPStats(); }
  function renderJPList() {
    const filtered = getFiltered();
    if (filtered.length === 0) { jpContent.textContent = "No journal entries. Fetch trades below to populate."; jpContent.style.color = "#555"; return; }
    jpContent.style.color = "";
    const table = document.createElement("table"); table.style.cssText = "width:100%;border-collapse:collapse;font-size:10px;";
    const thead = document.createElement("tr");
    for (const h of ["Date","Symbol","Side","Qty","Entry","Exit","P&L","Tag","Rating","Notes",""]) { const th = document.createElement("td"); th.style.cssText = "color:#666;font-weight:bold;padding:3px 4px;border-bottom:1px solid #333;"; th.textContent = h; thead.appendChild(th); }
    table.appendChild(thead);
    for (let idx = 0; idx < filtered.length; idx++) {
      const e = filtered[idx]; const realIdx = jpEntries.indexOf(e);
      const tr = document.createElement("tr"); tr.style.cssText = "border-bottom:1px solid #111;";
      const cells = [(e.date || "").substring(0, 16).replace("T", " "), e.symbol, e.side, e.qty, typeof e.entry === "number" ? e.entry.toFixed(2) : e.entry || "\u2014", typeof e.exit === "number" ? e.exit.toFixed(2) : e.exit || "\u2014", typeof e.pnl === "number" ? "$" + e.pnl.toFixed(2) : "\u2014"];
      for (let ci = 0; ci < cells.length; ci++) { const td = document.createElement("td"); td.style.cssText = "padding:3px 4px;"; td.textContent = cells[ci]; if (ci === 6 && typeof e.pnl === "number") td.style.color = e.pnl >= 0 ? "#4caf50" : "#f44336"; if (ci === 2) td.style.color = e.side === "buy" ? "#4caf50" : "#f44336"; tr.appendChild(td); }
      const tdTag = document.createElement("td"); tdTag.style.cssText = "padding:2px;";
      const tagSel = document.createElement("select"); tagSel.style.cssText = "background:#111;color:#fff;border:1px solid #333;font-size:9px;padding:1px;";
      const emptyOpt = document.createElement("option"); emptyOpt.value = ""; emptyOpt.textContent = "\u2014"; tagSel.appendChild(emptyOpt);
      for (const t of TAGS) { const o = document.createElement("option"); o.value = t; o.textContent = t; if (e.tag === t) o.selected = true; tagSel.appendChild(o); }
      tagSel.addEventListener("change", () => { jpEntries[realIdx].tag = tagSel.value; saveJP(jpEntries); });
      tdTag.appendChild(tagSel); tr.appendChild(tdTag);
      const tdRating = document.createElement("td"); tdRating.style.cssText = "padding:2px;white-space:nowrap;";
      for (let s = 1; s <= 5; s++) { const star = document.createElement("span"); star.textContent = s <= (e.rating || 0) ? "\u2605" : "\u2606"; star.style.cssText = "cursor:pointer;color:#ffc107;font-size:12px;"; star.addEventListener("click", () => { jpEntries[realIdx].rating = s; saveJP(jpEntries); renderJP(); }); tdRating.appendChild(star); }
      tr.appendChild(tdRating);
      const tdNote = document.createElement("td"); tdNote.style.cssText = "padding:2px;";
      const noteInp = document.createElement("input"); noteInp.value = e.notes || ""; noteInp.placeholder = "notes...";
      noteInp.style.cssText = "width:100px;background:#111;color:#ccc;border:1px solid #333;font-size:9px;padding:2px;font-family:inherit;";
      noteInp.addEventListener("change", () => { jpEntries[realIdx].notes = noteInp.value; saveJP(jpEntries); });
      tdNote.appendChild(noteInp); tr.appendChild(tdNote);
      const tdDel = document.createElement("td"); tdDel.style.cssText = "padding:2px;";
      const del = document.createElement("span"); del.textContent = "\u00d7"; del.style.cssText = "color:#f44;cursor:pointer;";
      del.addEventListener("click", () => { jpEntries.splice(realIdx, 1); saveJP(jpEntries); renderJP(); });
      tdDel.appendChild(del); tr.appendChild(tdDel); table.appendChild(tr);
    }
    jpContent.appendChild(table);
  }
  function renderJPCalendar() {
    const now = new Date(); const year = now.getFullYear(); const month = now.getMonth();
    const daysInMonth = new Date(year, month + 1, 0).getDate(); const firstDay = new Date(year, month, 1).getDay();
    const dailyPnL = {};
    for (const e of jpEntries) { if (typeof e.pnl !== "number") continue; const d = (e.date || "").substring(0, 10); dailyPnL[d] = (dailyPnL[d] || 0) + e.pnl; }
    const calTitle = document.createElement("div");
    calTitle.textContent = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"][month] + " " + year + " \u2014 Monthly P&L Calendar";
    calTitle.style.cssText = "font-weight:bold;color:#ccc;padding:4px 0 8px;text-align:center;"; jpContent.appendChild(calTitle);
    const grid = document.createElement("div"); grid.style.cssText = "display:grid;grid-template-columns:repeat(7,1fr);gap:2px;";
    for (const d of ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"]) { const hdr = document.createElement("div"); hdr.textContent = d; hdr.style.cssText = "text-align:center;color:#666;font-size:9px;padding:2px;"; grid.appendChild(hdr); }
    for (let i = 0; i < firstDay; i++) { grid.appendChild(document.createElement("div")); }
    for (let day = 1; day <= daysInMonth; day++) {
      const dateStr = year + "-" + String(month + 1).padStart(2, "0") + "-" + String(day).padStart(2, "0");
      const pnl = dailyPnL[dateStr]; const cell = document.createElement("div");
      cell.style.cssText = "text-align:center;padding:4px 2px;border-radius:3px;font-size:10px;min-height:30px;";
      const dayLabel = document.createElement("div"); dayLabel.textContent = day; dayLabel.style.color = "#888"; cell.appendChild(dayLabel);
      if (pnl !== undefined) { const val = document.createElement("div"); val.textContent = "$" + pnl.toFixed(0); val.style.cssText = "font-size:9px;font-weight:bold;color:" + (pnl >= 0 ? "#4caf50" : "#f44336") + ";"; cell.style.background = pnl >= 0 ? "rgba(76,175,80,0.15)" : "rgba(244,67,54,0.15)"; cell.appendChild(val); }
      grid.appendChild(cell);
    }
    jpContent.appendChild(grid);
    const monthKey = year + "-" + String(month + 1).padStart(2, "0"); let monthTotal = 0;
    for (const [d, v] of Object.entries(dailyPnL)) { if (d.startsWith(monthKey)) monthTotal += v; }
    const totalDiv = document.createElement("div");
    totalDiv.style.cssText = "text-align:center;padding:8px;font-weight:bold;color:" + (monthTotal >= 0 ? "#4caf50" : "#f44336") + ";";
    totalDiv.textContent = "Month Total: $" + monthTotal.toFixed(2); jpContent.appendChild(totalDiv);
  }
  function renderJPStats() {
    const filtered = getFiltered();
    if (filtered.length === 0) { jpContent.textContent = "No entries to analyze."; jpContent.style.color = "#555"; return; }
    jpContent.style.color = "";
    const byTag = {};
    for (const e of filtered) { const t = e.tag || "Untagged"; if (!byTag[t]) byTag[t] = { total: 0, count: 0, wins: 0, totalRating: 0, ratingCount: 0 }; byTag[t].total += (e.pnl || 0); byTag[t].count++; if (e.pnl > 0) byTag[t].wins++; if (e.rating) { byTag[t].totalRating += e.rating; byTag[t].ratingCount++; } }
    const tagTitle = document.createElement("div"); tagTitle.textContent = "P&L by Tag / Setup"; tagTitle.style.cssText = "font-weight:bold;color:#ccc;padding:4px 0;"; jpContent.appendChild(tagTitle);
    const tagTable = document.createElement("table"); tagTable.style.cssText = "width:100%;border-collapse:collapse;font-size:10px;margin-bottom:12px;";
    const tHead = document.createElement("tr");
    for (const h of ["Tag","Trades","Wins","Win%","P&L","Avg Rating"]) { const th = document.createElement("td"); th.style.cssText = "color:#666;font-weight:bold;padding:3px 6px;border-bottom:1px solid #333;"; th.textContent = h; tHead.appendChild(th); }
    tagTable.appendChild(tHead);
    for (const [tag, d] of Object.entries(byTag).sort((a, b) => b[1].total - a[1].total)) {
      const tr = document.createElement("tr"); const winPct = d.count > 0 ? (d.wins / d.count * 100).toFixed(1) : "0"; const avgRating = d.ratingCount > 0 ? (d.totalRating / d.ratingCount).toFixed(1) : "\u2014";
      const vals = [tag, d.count, d.wins, winPct + "%", "$" + d.total.toFixed(2), avgRating];
      for (let i = 0; i < vals.length; i++) { const td = document.createElement("td"); td.style.cssText = "padding:3px 6px;"; td.textContent = vals[i]; if (i === 4) td.style.color = d.total >= 0 ? "#4caf50" : "#f44336"; tr.appendChild(td); }
      tagTable.appendChild(tr);
    }
    jpContent.appendChild(tagTable);
    const ratedEntries = filtered.filter(e => e.rating);
    if (ratedEntries.length > 0) {
      const winners = ratedEntries.filter(e => e.pnl > 0); const losers = ratedEntries.filter(e => e.pnl <= 0);
      const avgWinRating = winners.length > 0 ? (winners.reduce((s, e) => s + e.rating, 0) / winners.length).toFixed(1) : "\u2014";
      const avgLossRating = losers.length > 0 ? (losers.reduce((s, e) => s + e.rating, 0) / losers.length).toFixed(1) : "\u2014";
      const ratingDiv = document.createElement("div"); ratingDiv.style.cssText = "padding:4px;color:#ccc;font-size:11px;";
      const ratingBold = document.createElement("b"); ratingBold.textContent = "Avg Rating:"; ratingDiv.appendChild(ratingBold); ratingDiv.appendChild(document.createTextNode(" Winners: " + avgWinRating + " | Losers: " + avgLossRating)); jpContent.appendChild(ratingDiv);
    }
  }
  const fetchBar = document.createElement("div"); fetchBar.style.cssText = "padding:4px;border-top:1px solid #333;display:flex;gap:4px;align-items:center;";
  const fetchBtn = document.createElement("button"); fetchBtn.textContent = "Fetch Trades from Broker";
  fetchBtn.style.cssText = "padding:4px 10px;background:#1a237e;color:#8af;border:1px solid #555;cursor:pointer;font-size:10px;font-family:inherit;";
  fetchBtn.addEventListener("click", async () => {
    fetchBtn.disabled = true; fetchBtn.textContent = "Fetching...";
    try {
      const histJson = await invoke("get_order_history", { limit: 200 });
      const orders = JSON.parse(histJson).filter(o => o.status === "filled" && o.filled_avg_price);
      if (orders.length === 0) { log("No filled orders found", "warn"); fetchBtn.textContent = "Fetch Trades from Broker"; fetchBtn.disabled = false; return; }
      const openPos = {}; const existingDates = new Set(jpEntries.map(e => e.date)); let added = 0;
      for (const o of orders.reverse()) {
        const sym = o.symbol; const price = parseFloat(o.filled_avg_price); const qty = parseFloat(o.qty) || 1;
        if (!openPos[sym]) { openPos[sym] = { side: o.side, price, qty, date: o.created_at }; }
        else if (openPos[sym].side !== o.side) {
          const entry = openPos[sym]; const pnl = entry.side === "buy" ? (price - entry.price) * Math.min(qty, entry.qty) : (entry.price - price) * Math.min(qty, entry.qty);
          const dateKey = (o.created_at || "").substring(0, 19);
          if (!existingDates.has(dateKey)) { jpEntries.unshift({ date: dateKey, symbol: sym, side: entry.side, qty: Math.min(qty, entry.qty), entry: entry.price, exit: price, pnl, tag: "", rating: 0, notes: "" }); existingDates.add(dateKey); added++; }
          delete openPos[sym];
        }
      }
      saveJP(jpEntries); log("Journal+: imported " + added + " round-trip trades", "ok"); renderJP();
    } catch (e) { log("Journal+ fetch error: " + e, "error"); }
    fetchBtn.textContent = "Fetch Trades from Broker"; fetchBtn.disabled = false;
  });
  fetchBar.appendChild(fetchBtn); root.appendChild(fetchBar);
  win.appendElement(root); renderJP();
}

// ══════════════════════════════════════════════════════════════
// CORRELATION3D — Correlation Network Graph (force-directed)
// ══════════════════════════════════════════════════════════════

async function cmdCorrelation3D() {
  const win = createWindow({ title: "Correlation Network Graph", width: 500, height: 560 });
  win.contentElement.textContent = "";
  const loadingMsg = document.createElement("div"); loadingMsg.textContent = "Building correlation network..."; loadingMsg.style.cssText = "color:#888;padding:20px;"; win.appendElement(loadingMsg);
  try {
    const symbols = getWatchlist();
    if (symbols.length < 2) { win.contentElement.textContent = ""; win.setContent("Need at least 2 watchlist symbols. Add via QM first."); return; }
    const closePrices = {};
    for (const sym of symbols) {
      let data = null;
      for (const tf of ["1Day", "4Hour", "1Hour"]) { const key = getCacheKey(sym, tf); const cached = barCache[key]; if (cached && cached.data && cached.data.length > 20) { data = cached.data; break; } }
      if (!data) { try { const barsJson = await invoke("get_bars", { symbol: sym, timeframe: "1Day", limit: 100 }); const bars = JSON.parse(barsJson); if (bars.length > 20) data = bars; } catch (_) {} }
      if (data) closePrices[sym] = data.slice(-100).map(b => b.close || b.c || 0);
    }
    const validSymbols = Object.keys(closePrices).filter(s => closePrices[s].length > 10);
    if (validSymbols.length < 2) { win.contentElement.textContent = ""; win.setContent("Insufficient cached bar data. Load some charts first."); return; }
    const corrReturns = {};
    for (const sym of validSymbols) { const prices = closePrices[sym]; corrReturns[sym] = []; for (let i = 1; i < prices.length; i++) corrReturns[sym].push(prices[i] > 0 ? (prices[i] - prices[i - 1]) / prices[i - 1] : 0); }
    function pearsonCorr(a, b) { const n = Math.min(a.length, b.length); if (n < 5) return 0; let sA = 0, sB = 0, sAB = 0, sA2 = 0, sB2 = 0; for (let i = 0; i < n; i++) { sA += a[i]; sB += b[i]; sAB += a[i] * b[i]; sA2 += a[i] * a[i]; sB2 += b[i] * b[i]; } const num = n * sAB - sA * sB; const den = Math.sqrt((n * sA2 - sA * sA) * (n * sB2 - sB * sB)); return den > 0 ? num / den : 0; }
    const corrMatrix = {};
    for (let i = 0; i < validSymbols.length; i++) for (let j = i + 1; j < validSymbols.length; j++) corrMatrix[validSymbols[i] + ":" + validSymbols[j]] = pearsonCorr(corrReturns[validSymbols[i]], corrReturns[validSymbols[j]]);
    const corrAvgVol = {}; for (const sym of validSymbols) corrAvgVol[sym] = closePrices[sym].length;
    const corrMaxVol = Math.max(...Object.values(corrAvgVol));
    const CW = 400, CH = 400;
    const corrNodes = validSymbols.map(sym => ({ sym, x: 50 + Math.random() * (CW - 100), y: 50 + Math.random() * (CH - 100), radius: 8 + (corrAvgVol[sym] / corrMaxVol) * 12 }));
    for (let iter = 0; iter < 100; iter++) {
      for (let i = 0; i < corrNodes.length; i++) { for (let j = i + 1; j < corrNodes.length; j++) {
        let dx = corrNodes[i].x - corrNodes[j].x; let dy = corrNodes[i].y - corrNodes[j].y; let dist = Math.sqrt(dx * dx + dy * dy); if (dist < 1) dist = 1;
        const repForce = 500 / (dist * dist + 1); corrNodes[i].x += repForce * dx / dist; corrNodes[i].y += repForce * dy / dist; corrNodes[j].x -= repForce * dx / dist; corrNodes[j].y -= repForce * dy / dist;
        const ck = corrNodes[i].sym + ":" + corrNodes[j].sym; const ckr = corrNodes[j].sym + ":" + corrNodes[i].sym;
        const cc = corrMatrix[ck] !== undefined ? corrMatrix[ck] : (corrMatrix[ckr] !== undefined ? corrMatrix[ckr] : 0);
        if (Math.abs(cc) > 0.3) { const att = cc * 0.1; corrNodes[i].x -= att * dx; corrNodes[i].y -= att * dy; corrNodes[j].x += att * dx; corrNodes[j].y += att * dy; }
      } }
      for (const n of corrNodes) { n.x = Math.max(30, Math.min(CW - 30, n.x)); n.y = Math.max(30, Math.min(CH - 30, n.y)); }
    }
    win.contentElement.textContent = "";
    const corrCanvas = document.createElement("canvas"); corrCanvas.width = CW; corrCanvas.height = CH;
    corrCanvas.style.cssText = "display:block;margin:0 auto;background:#0a0a0a;border:1px solid #333;border-radius:4px;";
    const corrCtx = corrCanvas.getContext("2d");
    for (let i = 0; i < corrNodes.length; i++) { for (let j = i + 1; j < corrNodes.length; j++) {
      const ck = corrNodes[i].sym + ":" + corrNodes[j].sym; const ckr = corrNodes[j].sym + ":" + corrNodes[i].sym;
      const cc = corrMatrix[ck] !== undefined ? corrMatrix[ck] : (corrMatrix[ckr] !== undefined ? corrMatrix[ckr] : 0);
      if (Math.abs(cc) <= 0.3) continue; corrCtx.beginPath(); corrCtx.moveTo(corrNodes[i].x, corrNodes[i].y); corrCtx.lineTo(corrNodes[j].x, corrNodes[j].y);
      corrCtx.lineWidth = Math.abs(cc) * 4; corrCtx.strokeStyle = cc > 0 ? "rgba(76,175,80," + (Math.abs(cc) * 0.8) + ")" : "rgba(244,67,54," + (Math.abs(cc) * 0.8) + ")"; corrCtx.stroke();
    } }
    for (const n of corrNodes) { corrCtx.beginPath(); corrCtx.arc(n.x, n.y, n.radius, 0, Math.PI * 2); corrCtx.fillStyle = "#1a237e"; corrCtx.fill(); corrCtx.strokeStyle = "#4fc3f7"; corrCtx.lineWidth = 1.5; corrCtx.stroke(); corrCtx.fillStyle = "#fff"; corrCtx.font = "bold 9px Consolas, monospace"; corrCtx.textAlign = "center"; corrCtx.textBaseline = "middle"; corrCtx.fillText(n.sym.length > 5 ? n.sym.substring(0, 5) : n.sym, n.x, n.y); }
    win.appendElement(corrCanvas);
    const visited = new Set(); const clusters = [];
    for (let i = 0; i < validSymbols.length; i++) { if (visited.has(i)) continue; const cluster = [validSymbols[i]]; visited.add(i);
      for (let j = i + 1; j < validSymbols.length; j++) { if (visited.has(j)) continue; const ck = validSymbols[i] + ":" + validSymbols[j]; const ckr = validSymbols[j] + ":" + validSymbols[i]; const cc = corrMatrix[ck] !== undefined ? corrMatrix[ck] : (corrMatrix[ckr] !== undefined ? corrMatrix[ckr] : 0); if (cc > 0.5) { cluster.push(validSymbols[j]); visited.add(j); } }
      if (cluster.length > 1) clusters.push(cluster);
    }
    if (clusters.length > 0) { const summary = document.createElement("div"); summary.style.cssText = "padding:8px;font-size:10px;color:#ccc;border-top:1px solid #333;margin-top:4px;"; summary.innerHTML = "<b>Clusters (corr &gt; 0.5):</b><br>" + clusters.map((c, i) => "Cluster " + (i + 1) + ": " + c.join(", ")).join("<br>"); win.appendElement(summary); }
    const legend = document.createElement("div"); legend.style.cssText = "padding:4px 8px;font-size:9px;color:#666;text-align:center;"; legend.textContent = "Green = positive correlation | Red = negative | Thickness = strength | Only |corr| > 0.3 shown"; win.appendElement(legend);
  } catch (e) { win.contentElement.textContent = ""; win.setContent("Failed to build correlation network: " + e); }
}

// ══════════════════════════════════════════════════════════════
// IMPORTTRADES — Import External Trade History (CSV)
// ══════════════════════════════════════════════════════════════

function cmdImportTrades() {
  const win = createWindow({ title: "Import Trade History", width: 600, height: 500 });
  win.contentElement.textContent = "";
  const IMPORT_KEY = "typhoon_imported_trades";
  function loadImported() { try { return JSON.parse(localStorage.getItem(IMPORT_KEY) || "[]"); } catch { return []; } }
  function saveImported(data) { localStorage.setItem(IMPORT_KEY, JSON.stringify(data)); }
  const importRoot = document.createElement("div"); importRoot.style.cssText = "display:flex;flex-direction:column;height:100%;font-size:11px;";
  const topBar = document.createElement("div"); topBar.style.cssText = "padding:6px;border-bottom:1px solid #333;display:flex;gap:6px;align-items:center;flex-wrap:wrap;";
  const impInputStyle = "background:#111;color:#fff;border:1px solid #555;padding:4px;font-size:10px;font-family:inherit;";
  const formatSel = document.createElement("select"); formatSel.style.cssText = impInputStyle;
  for (const [v, l] of [["auto","Auto-Detect"],["generic","Generic CSV"],["mt5","MT5 Export"],["ib","Interactive Brokers"],["tasty","Tastytrade"]]) { const o = document.createElement("option"); o.value = v; o.textContent = l; formatSel.appendChild(o); }
  const fileInput = document.createElement("input"); fileInput.type = "file"; fileInput.accept = ".csv"; fileInput.style.cssText = "font-size:10px;color:#ccc;";
  const importBtn = document.createElement("button"); importBtn.textContent = "Import"; importBtn.style.cssText = "padding:4px 10px;background:#1a237e;color:#8af;border:1px solid #555;cursor:pointer;font-size:10px;font-family:inherit;"; importBtn.disabled = true;
  const clearBtn = document.createElement("button"); clearBtn.textContent = "Clear Imported Data"; clearBtn.style.cssText = "padding:4px 10px;background:#5f0a0a;color:#f88;border:1px solid #555;cursor:pointer;font-size:10px;font-family:inherit;margin-left:auto;";
  clearBtn.addEventListener("click", () => { localStorage.removeItem(IMPORT_KEY); renderImportResults([]); log("Imported trade data cleared", "ok"); });
  topBar.appendChild(formatSel); topBar.appendChild(fileInput); topBar.appendChild(importBtn); topBar.appendChild(clearBtn); importRoot.appendChild(topBar);
  const previewDiv = document.createElement("div"); previewDiv.style.cssText = "padding:4px 6px;border-bottom:1px solid #333;max-height:120px;overflow-y:auto;font-size:9px;color:#888;"; previewDiv.textContent = "Select a CSV file to preview..."; importRoot.appendChild(previewDiv);
  const resultsDiv = document.createElement("div"); resultsDiv.style.cssText = "flex:1;overflow-y:auto;padding:4px;"; importRoot.appendChild(resultsDiv);
  let parsedRows = [];
  function parseCSV(text) {
    const lines = text.split("\n").map(l => l.trim()).filter(l => l.length > 0); if (lines.length < 2) return [];
    const headers = lines[0].split(",").map(h => h.trim().replace(/^"|"$/g, "")); const rows = [];
    for (let i = 1; i < lines.length; i++) { const vals = []; let inQuote = false, current = ""; for (const ch of lines[i]) { if (ch === '"') { inQuote = !inQuote; continue; } if (ch === "," && !inQuote) { vals.push(current.trim()); current = ""; continue; } current += ch; } vals.push(current.trim()); const row = {}; for (let j = 0; j < headers.length; j++) row[headers[j]] = vals[j] || ""; rows.push(row); }
    return rows;
  }
  function detectFormat(headers) { const hSet = new Set(headers.map(h => h.toLowerCase())); if (hSet.has("deal") && hSet.has("profit")) return "mt5"; if (hSet.has("tradeid") || hSet.has("account id")) return "ib"; if (hSet.has("exec time") || hSet.has("net price")) return "tasty"; return "generic"; }
  function normalizeTrades(rows, format) {
    const trades = [];
    for (const r of rows) {
      let trade = {};
      if (format === "mt5") { trade = { date: r["Time"] || r["Open Time"] || "", symbol: r["Symbol"] || "", side: (r["Type"] || "").toLowerCase().includes("buy") ? "buy" : "sell", qty: parseFloat(r["Volume"] || r["Lots"] || "1"), price: parseFloat(r["Price"] || "0"), commission: parseFloat(r["Commission"] || "0"), pnl: parseFloat(r["Profit"] || "0") }; }
      else if (format === "ib") { trade = { date: r["Date/Time"] || r["TradeDate"] || r["DateTime"] || "", symbol: r["Symbol"] || r["UnderlyingSymbol"] || "", side: (r["Buy/Sell"] || r["Side"] || "").toLowerCase().includes("buy") ? "buy" : "sell", qty: parseFloat(r["Quantity"] || r["Qty"] || "1"), price: parseFloat(r["Price"] || r["TradePrice"] || "0"), commission: parseFloat(r["Commission"] || r["IBCommission"] || "0"), pnl: parseFloat(r["RealizedPnL"] || r["FifoPnlRealized"] || "0") }; }
      else if (format === "tasty") { trade = { date: r["Exec Time"] || r["Date"] || "", symbol: r["Symbol"] || r["Underlying Symbol"] || "", side: (r["Side"] || r["Action"] || "").toLowerCase().includes("buy") ? "buy" : "sell", qty: parseFloat(r["Quantity"] || r["Qty"] || "1"), price: parseFloat(r["Price"] || r["Net Price"] || r["Avg Price"] || "0"), commission: parseFloat(r["Commission"] || r["Commissions"] || "0"), pnl: parseFloat(r["P/L"] || r["Net P/L"] || "0") }; }
      else { const keys = Object.keys(r); trade = { date: r[keys[0]] || "", symbol: r[keys[1]] || "", side: (r[keys[2]] || "").toLowerCase().includes("buy") ? "buy" : "sell", qty: parseFloat(r[keys[3]] || "1"), price: parseFloat(r[keys[4]] || "0"), commission: parseFloat(r[keys[5]] || "0"), pnl: parseFloat(r["P&L"] || r["PnL"] || r["Profit"] || "0") }; }
      if (trade.symbol) trades.push(trade);
    }
    return trades;
  }
  fileInput.addEventListener("change", () => {
    const file = fileInput.files[0]; if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target.result; const csvLines = text.split("\n").map(l => l.trim()).filter(l => l.length > 0);
      const rows = parseCSV(text); if (rows.length === 0) { previewDiv.textContent = "No data found in CSV."; return; }
      const headers = csvLines[0].split(",").map(h => h.trim().replace(/^"|"$/g, ""));
      if (formatSel.value === "auto") { const detected = detectFormat(headers); formatSel.value = detected; log("Auto-detected format: " + detected, "info"); }
      previewDiv.textContent = "";
      const preTable = document.createElement("table"); preTable.style.cssText = "border-collapse:collapse;font-size:9px;width:100%;";
      const preHead = document.createElement("tr");
      for (const h of headers.slice(0, 8)) { const th = document.createElement("td"); th.style.cssText = "color:#666;font-weight:bold;padding:2px 4px;border-bottom:1px solid #333;"; th.textContent = h.substring(0, 15); preHead.appendChild(th); }
      preTable.appendChild(preHead);
      for (let i = 0; i < Math.min(5, rows.length); i++) { const tr = document.createElement("tr"); for (const h of headers.slice(0, 8)) { const td = document.createElement("td"); td.style.cssText = "padding:2px 4px;color:#aaa;"; td.textContent = (rows[i][h] || "").substring(0, 20); tr.appendChild(td); } preTable.appendChild(tr); }
      previewDiv.appendChild(preTable);
      const countInfo = document.createElement("div"); countInfo.style.cssText = "color:#888;padding:2px 0;font-size:9px;"; countInfo.textContent = rows.length + " rows found"; previewDiv.appendChild(countInfo);
      parsedRows = rows; importBtn.disabled = false;
    };
    reader.readAsText(file);
  });
  importBtn.addEventListener("click", () => {
    if (parsedRows.length === 0) return;
    const format = formatSel.value === "auto" ? "generic" : formatSel.value;
    const trades = normalizeTrades(parsedRows, format);
    if (trades.length === 0) { log("No valid trades found in CSV", "warn"); return; }
    const serialized = JSON.stringify(trades);
    if (serialized.length > 5 * 1024 * 1024) { log("Import data exceeds 5 MB limit (" + (serialized.length / 1024 / 1024).toFixed(1) + " MB). Reduce CSV size.", "warn"); return; }
    saveImported(trades); log("Imported " + trades.length + " trades", "ok"); renderImportResults(trades);
  });
  function renderImportResults(trades) {
    resultsDiv.textContent = "";
    if (trades.length === 0) { const stored = loadImported(); if (stored.length > 0) { trades = stored; } else { resultsDiv.textContent = "No imported trades."; resultsDiv.style.color = "#555"; return; } }
    resultsDiv.style.color = "";
    const wins = trades.filter(t => t.pnl > 0); const losses = trades.filter(t => t.pnl <= 0 && t.pnl !== 0);
    const totalPnL = trades.reduce((s, t) => s + (t.pnl || 0), 0); const totalComm = trades.reduce((s, t) => s + (t.commission || 0), 0);
    const winRate = trades.length > 0 ? (wins.length / trades.length * 100).toFixed(1) : "0";
    const avgWin = wins.length > 0 ? wins.reduce((s, t) => s + t.pnl, 0) / wins.length : 0;
    const avgLoss = losses.length > 0 ? Math.abs(losses.reduce((s, t) => s + t.pnl, 0) / losses.length) : 0;
    const grossWin = wins.reduce((s, t) => s + t.pnl, 0); const grossLoss = Math.abs(losses.reduce((s, t) => s + t.pnl, 0));
    const profitFactor = grossLoss > 0 ? (grossWin / grossLoss).toFixed(2) : "\u221e";
    const bestTrade = trades.length > 0 ? Math.max(...trades.map(t => t.pnl || 0)) : 0;
    const worstTrade = trades.length > 0 ? Math.min(...trades.map(t => t.pnl || 0)) : 0;
    const bySym = {}; for (const t of trades) { if (!bySym[t.symbol]) bySym[t.symbol] = { total: 0, count: 0 }; bySym[t.symbol].total += (t.pnl || 0); bySym[t.symbol].count++; }
    const statsTitle = document.createElement("div"); statsTitle.textContent = "Trade Statistics"; statsTitle.style.cssText = "font-weight:bold;color:#ccc;padding:4px 0;"; resultsDiv.appendChild(statsTitle);
    const rows = [["Total Trades", trades.length], ["Wins", wins.length], ["Losses", losses.length], ["Win Rate", winRate + "%"], ["Avg Win", "$" + avgWin.toFixed(2)], ["Avg Loss", "$" + avgLoss.toFixed(2)], ["Best Trade", "$" + bestTrade.toFixed(2)], ["Worst Trade", "$" + worstTrade.toFixed(2)], ["Profit Factor", profitFactor], ["Total Commission", "$" + totalComm.toFixed(2)], ["Total P&L", "$" + totalPnL.toFixed(2)]];
    const table = document.createElement("table"); table.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;margin-bottom:12px;";
    for (const [label, val] of rows) { const tr = document.createElement("tr"); const td1 = document.createElement("td"); td1.style.cssText = "padding:3px 8px;color:#888;"; td1.textContent = label; const td2 = document.createElement("td"); td2.style.cssText = "padding:3px 8px;text-align:right;font-family:Consolas,monospace;"; td2.textContent = String(val); if (label === "Total P&L") td2.style.color = totalPnL >= 0 ? "#4caf50" : "#f44336"; if (label === "Win Rate") td2.style.color = parseFloat(String(val)) >= 50 ? "#4caf50" : "#f44336"; tr.appendChild(td1); tr.appendChild(td2); table.appendChild(tr); }
    resultsDiv.appendChild(table);
    const symTitle = document.createElement("div"); symTitle.textContent = "P&L by Symbol"; symTitle.style.cssText = "font-weight:bold;color:#ccc;padding:4px 0;"; resultsDiv.appendChild(symTitle);
    const symTable = document.createElement("table"); symTable.style.cssText = "width:100%;border-collapse:collapse;font-size:10px;";
    const symHead = document.createElement("tr"); for (const h of ["Symbol", "Trades", "P&L"]) { const th = document.createElement("td"); th.style.cssText = "color:#666;font-weight:bold;padding:3px 6px;border-bottom:1px solid #333;"; th.textContent = h; symHead.appendChild(th); } symTable.appendChild(symHead);
    for (const [sym, d] of Object.entries(bySym).sort((a, b) => b[1].total - a[1].total)) { const tr = document.createElement("tr"); const vals = [sym, d.count, "$" + d.total.toFixed(2)]; for (let i = 0; i < vals.length; i++) { const td = document.createElement("td"); td.style.cssText = "padding:3px 6px;"; td.textContent = vals[i]; if (i === 2) td.style.color = d.total >= 0 ? "#4caf50" : "#f44336"; tr.appendChild(td); } symTable.appendChild(tr); }
    resultsDiv.appendChild(symTable);
  }
  win.appendElement(importRoot); renderImportResults([]);
}

const CMD_PALETTE_COMMANDS = [
  { name: "SNAPSHOT", desc: "Portfolio snapshot to clipboard", action: cmdSnapshot },
  { name: "HOTLIST", desc: "Real-time top movers dashboard", action: cmdHotlist },
  { name: "NOTES", desc: "Per-symbol trading notes", action: cmdNotes },
  { name: "TIMER", desc: "Custom countdown timers", action: cmdTimer },
  { name: "EXPORT", desc: "Chart data export to CSV", action: cmdExport },
  { name: "REPLAY", desc: "Bar-by-bar chart replay / practice trading", action: cmdReplay },
  { name: "DES", desc: "Description / Fundamentals", action: cmdDescription },
  { name: "NEWS", desc: "News headlines", action: cmdNews },
  { name: "FA", desc: "Financial Analysis (income, balance, cash flow)", action: cmdFinancialAnalysis },
  { name: "DES", desc: "Description / Fundamentals", action: cmdDescription },
  { name: "NEWS", desc: "News headlines", action: cmdNews },
  { name: "FA", desc: "Financial Analysis (income, balance, cash flow)", action: cmdFinancialAnalysis },
  { name: "OPT", desc: "Options chain (Greeks, bid/ask, strike)", action: cmdOptions },
  { name: "SCAN", desc: "Screener / Scanner", action: cmdScreener },
  { name: "HDS", desc: "Institutional Holders", action: cmdInstitutionalHolders },
  { name: "MOST", desc: "Most Active stocks", action: cmdMostActive },
  { name: "DOM", desc: "DOM / Level 2 Order Book", action: cmdOrderBook },
  { name: "BACKTEST", desc: "Visual Backtester", action: openVisualBacktester },
  { name: "OPTIMIZE", desc: "Genetic Optimizer", action: openOptimizer },
  { name: "HIST", desc: "Trade History / Orders", action: cmdHistory },
  { name: "QM", desc: "Quote Monitor / Watchlist", action: cmdWatchlist },
  { name: "CAL", desc: "Economic Calendar", action: cmdCalendar },
  { name: "T&S", desc: "Time & Sales (live trades)", action: cmdTimeSales },
  { name: "ACTIVITIES", desc: "Account activities (fills, dividends, deposits)", action: cmdActivities },
  { name: "INSIDER", desc: "Insider trading (SEC Form 4)", action: cmdInsider },
  { name: "EARNINGS", desc: "Earnings & Corporate Actions Calendar", action: cmdEarnings },
  { name: "PORTFOLIO", desc: "Portfolio Breakdown by Sector", action: cmdPortfolio },
  { name: "CORR", desc: "Correlation Matrix (open positions)", action: cmdCorrelation },
  { name: "MONTECARLO", desc: "Monte Carlo Risk of Ruin", action: cmdMonteCarlo },
  { name: "ALERTS", desc: "Multi-Condition Alert Manager", action: cmdMultiAlerts },
  { name: "JOURNAL", desc: "Trade Journal (log & review)", action: cmdTradeJournal },
  { name: "CALC", desc: "Position Sizing Calculator", action: cmdPositionCalc },
  { name: "ANNOTATE", desc: "Add chart annotation", action: addChartAnnotation },
  { name: "SETTINGS", desc: "API Keys & Configuration", action: cmdSettings },
  { name: "FRED", desc: "FRED Economic Data (rates, CPI, GDP)", action: cmdFRED },
  { name: "AI", desc: "AI Trading Assistant (Claude/GPT) + Trade Review", action: cmdAIChatWithReview },
  { name: "ALERTBOARD", desc: "Multi-symbol alert dashboard (watchlist)", action: cmdAlertBoard },
  { name: "PATTERNS", desc: "Pattern recognition (Double Top/Bottom, H&S)", action: cmdPatterns },
  { name: "SENTIMENT", desc: "News sentiment analysis", action: cmdSentiment },
  { name: "VOLSURF", desc: "Volatility surface (options IV grid)", action: cmdVolSurf },
  { name: "PCRATIO", desc: "Put/Call ratio dashboard (volume & OI sentiment)", action: cmdPCRatio },
  { name: "UNUSUAL", desc: "Unusual options activity scanner (vol >> OI)", action: cmdUnusual },
  { name: "BRACKET", desc: "Conditional bracket/OCO order placement", action: cmdBracketOrder },
  { name: "MULTILEG", desc: "Multi-leg order builder (bracket, OCO, scale in/out)", action: cmdMultiLeg },
  { name: "HEATMAP", desc: "Portfolio heat map (daily P&L)", action: cmdHeatmap },
  { name: "OPTCALC", desc: "Options P&L calculator (payoff diagram)", action: cmdOptionsCalc },
  { name: "SECTORS", desc: "Sector rotation heatmap (S&P 500 ETFs)", action: cmdSectorRotation },
  { name: "ECON", desc: "Economic calendar with countdown", action: cmdEconCalendar },
  { name: "OPTSTRAT", desc: "Options strategy builder (spreads, condors)", action: cmdOptionsStrategy },
  { name: "AUTOTRADE", desc: "Strategy auto-trading (JS plugin → live orders)", action: cmdAutoTrade },
  { name: "CHAT", desc: "Community chat (Matrix)", action: cmdMatrixChat },
  { name: "HELP", desc: "Keybindings, commands & feature reference", action: showHelpOverlay },
  { name: "TILE", desc: "Tile all floating windows", action: () => tileWindows() },
  { name: "CLOSE", desc: "Close all floating windows", action: () => closeAllWindows() },
  { name: "COMPARE", desc: "Chart comparison overlay (% change)", action: cmdCompare },
  { name: "SPREAD", desc: "Spread/Ratio chart (two symbols)", action: cmdSpread },
  { name: "IVRANK", desc: "IV Percentile from options chain history", action: cmdIVRank },
  { name: "GAPS", desc: "Gap scanner (watchlist daily gaps)", action: cmdGaps },
  { name: "FLOWS", desc: "Sector fund flows (ETF volume × price change)", action: cmdFlows },
  { name: "TRADESTATS", desc: "Trade statistics dashboard (win rate, P&L, expectancy)", action: cmdTradeStats },
  { name: "RELSTRENGTH", desc: "Relative strength ranking (watchlist)", action: cmdRelStrength },
  { name: "SRLEVEL", desc: "Auto support/resistance detection (fractal clustering)", action: cmdSRLevel },
  { name: "SEASONALITY", desc: "Monthly performance patterns (bar chart + table)", action: cmdSeasonality },
  { name: "PAIRS", desc: "Pairs trading analysis (correlation, z-score, half-life)", action: cmdPairs },
  { name: "MTFDIV", desc: "Multi-timeframe divergence alerts (Fisher Transform)", action: cmdMTFDiv },
  { name: "BREADTH", desc: "Market breadth dashboard (SMA200/50, A/D, watchlist)", action: cmdBreadth },
  { name: "DIVERGENCE", desc: "Divergence scanner (Fisher/RSI vs price)", action: cmdDivergence },
  { name: "VOLUME", desc: "Volume Profile (horizontal histogram, POC, Value Area)", action: cmdVolumeProfile },
  { name: "PIVOTS", desc: "Auto Pivot Points (classic floor trader)", action: cmdPivots },
  { name: "PERF", desc: "Symbol Performance Card (returns, 52-week range)", action: cmdPerf },
  { name: "VWAP+", desc: "Anchored VWAP with standard deviation bands", action: cmdAnchoredVWAP },
  { name: "MARKETPROFILE", desc: "Market Profile / TPO chart (intraday price distribution)", action: cmdMarketProfile },
  { name: "BACKTEST+", desc: "Visual Strategy Builder (no code, dropdown conditions)", action: cmdBacktestPlus },
  { name: "FLOWMAP", desc: "Sector Rotation Flow Map (ETF performance + flow direction)", action: cmdFlowMap },
  { name: "REGIME+", desc: "Advanced Regime Detection Dashboard (vol, trend, Hurst, momentum)", action: cmdRegimePlus },
  { name: "RISKSIM", desc: "Scenario Stress Testing (crash, correction, custom shocks)", action: cmdRiskSim },
  { name: "SMARTALERT", desc: "Statistical Anomaly Detection (z-scores, volume, range, RSI)", action: cmdSmartAlert },
  { name: "RISKMAP", desc: "Portfolio Risk Heatmap (VaR treemap by weight)", action: cmdRiskMap },
  { name: "ECALENDAR", desc: "Unified Earnings + Ex-Div Calendar (5-week grid)", action: cmdEarningsCalendar },
  { name: "GREEKS", desc: "Aggregate Portfolio Greeks (delta, gamma, theta, vega)", action: cmdGreeks },
  { name: "SCANNER+", desc: "Multi-condition stock screener (RSI, SMA200, volume)", action: cmdScannerPlus },
  { name: "OPTPROFIT", desc: "Options P&L simulator with time decay (Black-Scholes)", action: cmdOptProfit },
  { name: "ORDERFLOW", desc: "Cumulative delta from WebSocket trades (real-time)", action: cmdOrderFlow },
  { name: "EQUITY", desc: "Account equity curve (P&L, drawdown, Sharpe)", action: cmdEquity },
  { name: "HEATCAL", desc: "Calendar heatmap of daily returns (GitHub-style)", action: cmdHeatCal },
  { name: "CORRWATCH", desc: "Correlation breakdown alerts (60D vs 1Y, watchlist)", action: cmdCorrWatch },
  { name: "LADDER", desc: "Price Ladder / DOM Visualization", action: cmdLadder },
  { name: "CHAIN+", desc: "Enhanced Options Chain Visualizer (IV smile, OI, heatmap)", action: cmdChainPlus },
  { name: "SPREAD+", desc: "Live Bid-Ask Spread Monitor (chart + stats)", action: cmdSpreadMonitor },
  { name: "WEBHOOK", desc: "Custom Webhook Alert Endpoints", action: cmdWebhook },
  { name: "JOURNAL+", desc: "Enhanced Trade Journal (tags, ratings, calendar, stats)", action: cmdJournalPlus },
  { name: "CORRELATION3D", desc: "Correlation Network Graph (force-directed)", action: cmdCorrelation3D },
  { name: "IMPORTTRADES", desc: "Import external trade history (CSV: MT5, IB, Tastytrade)", action: cmdImportTrades },
  { name: "SIGNAL", desc: "Composite trading signal generator (Fisher, RSI, KAMA, SMA200, volume, ATR)", action: cmdSignal },
  { name: "PROFILE", desc: "Trading profile analytics (P&L by symbol, day of week, hold time)", action: cmdProfile },
  { name: "FIBO+", desc: "Fibonacci time zones (markers at Fib intervals from swing low)", action: cmdFiboTime },
  { name: "DARKMODE", desc: "Theme switcher (Dark, Pitch Black, Light)", action: cmdDarkMode },
  { name: "BOOKMAP", desc: "Heatmap order book over time (canvas, bid/ask depth)", action: cmdBookmap },
  { name: "DASHBOARD", desc: "Customizable widget dashboard (positions, watchlist, signals)", action: cmdDashboard },
  { name: "SCANNER-RT", desc: "Real-time multi-symbol scanner (RSI, SMA200, volume, 52W)", action: cmdScannerRT },
  { name: "ALGO", desc: "Live algorithm monitor (auto-trade status, P&L, signals)", action: cmdAlgoMonitor },
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

async function cmdOptions() {
  if (!currentSymbol) { alert("Load a chart first"); return; }
  const win = createWindow({ title: `${currentSymbol} — Options Chain`, width: 700, height: 500 });
  win.contentElement.textContent = "";

  // Expiry selector
  const toolbar = document.createElement("div");
  toolbar.style.cssText = "display:flex;gap:6px;padding:6px;border-bottom:1px solid #333;align-items:center;";
  const label = document.createElement("span");
  label.textContent = "Expiry:";
  label.style.cssText = "color:#888;font-size:10px;";
  const expiryInput = document.createElement("input");
  expiryInput.type = "date";
  expiryInput.style.cssText = "font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
  const nextFri = new Date();
  nextFri.setDate(nextFri.getDate() + (5 - nextFri.getDay() + 7) % 7 + 7);
  expiryInput.value = nextFri.toISOString().split("T")[0];
  const loadBtn = document.createElement("button");
  loadBtn.textContent = "Load";
  loadBtn.style.cssText = "font-size:10px;padding:3px 10px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;";
  toolbar.appendChild(label);
  toolbar.appendChild(expiryInput);
  toolbar.appendChild(loadBtn);
  win.appendElement(toolbar);

  const content = document.createElement("div");
  content.style.cssText = "padding:4px;font-size:10px;overflow-y:auto;max-height:420px;";
  content.textContent = "Select expiry and click Load.";
  win.appendElement(content);

  loadBtn.addEventListener("click", async () => {
    content.textContent = "Loading options chain...";
    try {
      const json = await invoke("get_options", { symbol: currentSymbol, expiry: expiryInput.value });
      const chain = JSON.parse(json);
      if (chain.length === 0) { content.textContent = "No options data"; return; }
      content.textContent = "";

      const calls = chain.filter(c => c.option_type === "call");
      const puts = chain.filter(c => c.option_type === "put");
      const allStrikes = [...new Set(chain.map(c => c.strike))].sort((a, b) => a - b);

      const table = document.createElement("table");
      table.style.cssText = "width:100%;border-collapse:collapse;font-size:10px;";
      const thead = document.createElement("thead");
      const hr = document.createElement("tr");
      hr.style.cssText = "border-bottom:1px solid #444;";
      for (const h of ["C Bid", "C Ask", "C IV", "Delta", "Strike", "Delta", "P IV", "P Bid", "P Ask"]) {
        const th = document.createElement("th");
        th.style.cssText = "padding:2px 3px;color:#888;text-align:center;font-size:9px;";
        th.textContent = h;
        hr.appendChild(th);
      }
      thead.appendChild(hr);
      table.appendChild(thead);

      const tbody = document.createElement("tbody");
      for (const strike of allStrikes) {
        const call = calls.find(c => c.strike === strike);
        const put = puts.find(c => c.strike === strike);
        const tr = document.createElement("tr");
        const itm = lastPrice && strike < lastPrice;
        tr.style.cssText = `border-bottom:1px solid #1a1a2e;${itm ? "background:rgba(76,175,80,0.05);" : ""}`;
        const vals = [
          call ? call.bid.toFixed(2) : "—", call ? call.ask.toFixed(2) : "—",
          call ? (call.implied_volatility * 100).toFixed(1) + "%" : "—",
          call ? call.delta.toFixed(3) : "—", strike.toFixed(2),
          put ? put.delta.toFixed(3) : "—",
          put ? (put.implied_volatility * 100).toFixed(1) + "%" : "—",
          put ? put.bid.toFixed(2) : "—", put ? put.ask.toFixed(2) : "—",
        ];
        for (let i = 0; i < vals.length; i++) {
          const td = document.createElement("td");
          td.style.cssText = `padding:2px 3px;text-align:center;color:${i === 4 ? "#fff" : "#ccc"};${i === 4 ? "font-weight:bold;background:#1a1a2e;" : ""}`;
          td.textContent = vals[i];
          tr.appendChild(td);
        }
        tbody.appendChild(tr);
      }
      table.appendChild(tbody);
      content.appendChild(table);
    } catch (e) { content.textContent = `Error: ${e}`; }
  });
}

async function cmdScreener() {
  const win = createWindow({ title: "Stock Screener", width: 650, height: 500 });
  win.contentElement.textContent = "";

  // Filter form
  const form = document.createElement("div");
  form.style.cssText = "display:flex;flex-wrap:wrap;gap:6px;padding:8px;border-bottom:1px solid #333;align-items:center;";
  const mkFilter = (label, id, ph, val) => {
    const wrap = document.createElement("div");
    wrap.style.cssText = "display:flex;flex-direction:column;gap:2px;";
    const lbl = document.createElement("label");
    lbl.textContent = label;
    lbl.style.cssText = "color:#888;font-size:9px;";
    const inp = document.createElement("input");
    inp.id = id;
    inp.type = "number";
    inp.step = "any";
    inp.placeholder = ph;
    if (val) inp.value = val;
    inp.style.cssText = "width:70px;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
    wrap.appendChild(lbl);
    wrap.appendChild(inp);
    return wrap;
  };
  form.appendChild(mkFilter("Min Price", "scr-min-price", "0", ""));
  form.appendChild(mkFilter("Max Price", "scr-max-price", "∞", ""));
  form.appendChild(mkFilter("Min Volume", "scr-min-vol", "0", "100000"));
  form.appendChild(mkFilter("Min Change%", "scr-min-chg", "-∞", ""));
  form.appendChild(mkFilter("Max Change%", "scr-max-chg", "∞", ""));

  const scanBtn = document.createElement("button");
  scanBtn.textContent = "Scan";
  scanBtn.style.cssText = "font-size:11px;padding:4px 14px;background:#1b5e20;color:#8f8;border:1px solid #555;cursor:pointer;font-weight:bold;align-self:flex-end;";
  form.appendChild(scanBtn);
  win.appendElement(form);

  const results = document.createElement("div");
  results.style.cssText = "padding:4px;font-size:10px;overflow-y:auto;max-height:400px;";
  results.textContent = "Click Scan to search.";
  win.appendElement(results);

  scanBtn.addEventListener("click", async () => {
    results.textContent = "Scanning...";
    try {
      // Build screener data from most active stocks
      const activeJson = await invoke("get_most_active", { top: 100 });
      const active = JSON.parse(activeJson);
      const moversJson = await invoke("get_top_movers", { marketType: "stocks", top: 50 });
      const movers = JSON.parse(moversJson);

      // Combine into screener symbols
      const symbols = [];
      const addSymbols = (data, key) => {
        const arr = data[key] || data.most_actives || data.gainers || data.losers || [];
        for (const item of (Array.isArray(arr) ? arr : [])) {
          const sym = item.symbol || item.S || "";
          if (!sym || symbols.find(s => s.symbol === sym)) continue;
          symbols.push({
            symbol: sym, name: "", asset_class: "us_equity",
            price: item.price || item.p || 0, volume: item.volume || item.v || 0,
            change_pct: item.change_percent || item.percent_change || 0,
            tradable: true, shortable: true, fractionable: true, sector: null,
          });
        }
      };
      addSymbols(active, "most_actives");
      addSymbols(movers, "gainers");
      addSymbols(movers, "losers");

      const filters = {
        min_price: parseFloat(document.getElementById("scr-min-price").value) || null,
        max_price: parseFloat(document.getElementById("scr-max-price").value) || null,
        min_volume: parseFloat(document.getElementById("scr-min-vol").value) || null,
        max_volume: null, sector: null, asset_class: null,
        min_change_pct: parseFloat(document.getElementById("scr-min-chg").value) || null,
        max_change_pct: parseFloat(document.getElementById("scr-max-chg").value) || null,
        tradable_only: true, shortable_only: false, fractionable_only: false,
      };

      const resultJson = await invoke("run_screener", {
        filtersJson: JSON.stringify(filters),
        symbolsJson: JSON.stringify(symbols),
      });
      const res = JSON.parse(resultJson);
      results.textContent = "";

      if (res.symbols.length === 0) { results.textContent = "No matches found."; return; }

      const hdr = document.createElement("div");
      hdr.style.cssText = "color:#888;font-size:9px;margin-bottom:4px;";
      hdr.textContent = `${res.total_matched} matches from ${res.total_scanned} scanned`;
      results.appendChild(hdr);

      for (const s of res.symbols.slice(0, 50)) {
        const row = document.createElement("div");
        row.style.cssText = "display:flex;justify-content:space-between;padding:3px 4px;border-bottom:1px solid #1a1a2e;cursor:pointer;";
        row.addEventListener("click", () => { document.getElementById("symbol-input").value = s.symbol; triggerLoad(); });

        const sym = document.createElement("span");
        sym.style.cssText = "color:#fff;font-weight:bold;width:60px;";
        sym.textContent = s.symbol;
        const price = document.createElement("span");
        price.style.cssText = "color:#ccc;width:60px;text-align:right;font-family:Consolas,monospace;";
        price.textContent = `$${s.price.toFixed(2)}`;
        const vol = document.createElement("span");
        vol.style.cssText = "color:#888;width:70px;text-align:right;";
        vol.textContent = s.volume > 1e6 ? `${(s.volume / 1e6).toFixed(1)}M` : `${(s.volume / 1e3).toFixed(0)}K`;
        const chg = document.createElement("span");
        chg.style.cssText = `width:60px;text-align:right;font-weight:bold;color:${s.change_pct >= 0 ? "#4caf50" : "#f44336"};`;
        chg.textContent = `${s.change_pct >= 0 ? "+" : ""}${s.change_pct.toFixed(2)}%`;

        row.appendChild(sym);
        row.appendChild(price);
        row.appendChild(vol);
        row.appendChild(chg);
        results.appendChild(row);
      }
    } catch (e) { results.textContent = `Error: ${e}`; }
  });
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
      const askBar = document.createElement("div");
      askBar.className = "dom-bar dom-bar-ask";
      askBar.style.width = pct + "%";
      const askSize = document.createElement("span");
      askSize.className = "dom-size";
      askSize.textContent = size.toLocaleString();
      const askPrice = document.createElement("span");
      askPrice.className = "dom-price";
      askPrice.textContent = Number(price).toFixed(2);
      row.appendChild(askBar);
      row.appendChild(askSize);
      row.appendChild(askPrice);
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
      const bidBar = document.createElement("div");
      bidBar.className = "dom-bar dom-bar-bid";
      bidBar.style.width = pct + "%";
      const bidSize = document.createElement("span");
      bidSize.className = "dom-size";
      bidSize.textContent = size.toLocaleString();
      const bidPrice = document.createElement("span");
      bidPrice.className = "dom-price";
      bidPrice.textContent = Number(price).toFixed(2);
      row.appendChild(bidBar);
      row.appendChild(bidSize);
      row.appendChild(bidPrice);
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
// EARNINGS — Corporate Actions Calendar
// ══════════════════════════════════════════════════════════════

async function cmdEarnings() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Earnings & Corporate Actions`, width: 600, height: 450 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Loading corporate actions...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);

  try {
    const types = "dividend,merger,spinoff,split";
    const json = await invoke("get_corporate_actions", { symbol: currentSymbol, types });
    const actions = JSON.parse(json);

    win.contentElement.textContent = "";
    if (!Array.isArray(actions) || actions.length === 0) {
      win.setContent("No corporate actions found for this symbol.");
      return;
    }

    const table = document.createElement("table");
    table.className = "fw-table";

    // Header
    const thead = document.createElement("tr");
    for (const h of ["Date", "Symbol", "Type", "Details"]) {
      const th = document.createElement("td");
      th.style.cssText = "color:#888;font-weight:bold;font-size:10px;border-bottom:1px solid #333;";
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);

    for (const action of actions) {
      const tr = document.createElement("tr");
      const date = action.ex_date || action.effective_date || action.date || "—";
      const type = action.type || action.ca_type || action.sub_type || "—";
      const symbol = action.symbol || currentSymbol;
      let details = "";
      if (action.cash_amount) details = `$${action.cash_amount}/share`;
      else if (action.old_rate && action.new_rate) details = `${action.old_rate}:${action.new_rate}`;
      else if (action.description) details = action.description;
      else details = JSON.stringify(action).substring(0, 80);

      for (const val of [date, symbol, type, details]) {
        const td = document.createElement("td");
        td.className = "fw-value";
        td.style.textAlign = "left";
        td.textContent = val;
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }
    win.appendElement(table);
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to load corporate actions: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// PORTFOLIO — Breakdown by Sector/Asset Class
// ══════════════════════════════════════════════════════════════

async function cmdPortfolio() {
  const win = createWindow({ title: "Portfolio Breakdown", width: 500, height: 450 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Loading positions...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);

  try {
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);

    if (!positions || positions.length === 0) {
      win.contentElement.textContent = "";
      win.setContent("No open positions.");
      return;
    }

    // Group by asset class — fetch asset info for each position
    const groups = {};
    for (const p of positions) {
      let assetClass = "unknown";
      try {
        const assetJson = await invoke("get_asset", { symbol: p.symbol });
        const asset = JSON.parse(assetJson);
        assetClass = asset.class || asset.asset_class || "us_equity";
      } catch (_) {
        assetClass = p.asset_class || "us_equity";
      }

      if (!groups[assetClass]) groups[assetClass] = { count: 0, value: 0, positions: [] };
      const mv = Math.abs(p.market_value || p.qty * (p.current_price || 0));
      groups[assetClass].count++;
      groups[assetClass].value += mv;
      groups[assetClass].positions.push(p);
    }

    const totalValue = Object.values(groups).reduce((s, g) => s + g.value, 0);

    win.contentElement.textContent = "";

    // Summary table
    const table = document.createElement("table");
    table.className = "fw-table";
    const thead = document.createElement("tr");
    for (const h of ["Asset Class", "Positions", "Market Value", "% of Portfolio"]) {
      const th = document.createElement("td");
      th.style.cssText = "color:#888;font-weight:bold;font-size:10px;border-bottom:1px solid #333;";
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);

    for (const [cls, data] of Object.entries(groups).sort((a, b) => b[1].value - a[1].value)) {
      const tr = document.createElement("tr");
      const pct = totalValue > 0 ? (data.value / totalValue * 100).toFixed(1) : "0.0";
      for (const val of [cls.replace(/_/g, " ").toUpperCase(), String(data.count), `$${data.value.toFixed(2)}`, `${pct}%`]) {
        const td = document.createElement("td");
        td.className = "fw-value";
        td.textContent = val;
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }

    // Total row
    const totalRow = document.createElement("tr");
    totalRow.style.borderTop = "1px solid #555";
    for (const val of ["TOTAL", String(positions.length), `$${totalValue.toFixed(2)}`, "100%"]) {
      const td = document.createElement("td");
      td.style.cssText = "font-weight:bold;color:#8cf;";
      td.textContent = val;
      totalRow.appendChild(td);
    }
    table.appendChild(totalRow);
    win.appendElement(table);

    // Detail list per group
    for (const [cls, data] of Object.entries(groups)) {
      const heading = document.createElement("div");
      heading.style.cssText = "color:#888;font-size:10px;margin:12px 0 4px;text-transform:uppercase;border-bottom:1px solid #222;padding-bottom:2px;";
      heading.textContent = `${cls.replace(/_/g, " ")} — ${data.count} positions`;
      win.appendElement(heading);

      for (const p of data.positions) {
        const row = document.createElement("div");
        row.style.cssText = "display:flex;justify-content:space-between;font-size:10px;padding:2px 0;";
        const name = document.createElement("span");
        name.textContent = `${p.symbol} ${p.side === "long" ? "L" : "S"} ${Math.abs(p.qty)}`;
        name.style.color = "#ccc";
        const val = document.createElement("span");
        const mv = Math.abs(p.market_value || p.qty * (p.current_price || 0));
        const pl = p.unrealized_pl || 0;
        val.textContent = `$${mv.toFixed(2)} (${pl >= 0 ? "+" : ""}$${pl.toFixed(2)})`;
        val.style.color = pl >= 0 ? "#4caf50" : "#f44336";
        row.appendChild(name);
        row.appendChild(val);
        win.appendElement(row);
      }
    }
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to load portfolio: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// CORR — Correlation Matrix (from barCache)
// ══════════════════════════════════════════════════════════════

async function cmdCorrelation() {
  const win = createWindow({ title: "Correlation Matrix", width: 600, height: 500 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Calculating correlations from cached data...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);

  try {
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);

    if (!positions || positions.length < 2) {
      win.contentElement.textContent = "";
      win.setContent("Need at least 2 open positions for correlation matrix.");
      return;
    }

    const symbols = positions.map(p => p.symbol);

    // Get close prices from barCache for each symbol
    const closePrices = {};
    for (const sym of symbols) {
      // Try common timeframes in cache
      let data = null;
      for (const tf of ["1Day", "4Hour", "1Hour"]) {
        const key = `${sym}:${tf}`;
        const cached = barCache[key];
        if (cached && cached.data && cached.data.length > 20) {
          data = cached.data;
          break;
        }
      }
      if (!data) {
        // Try to fetch daily bars
        try {
          const barsJson = await invoke("get_bars", { symbol: sym, timeframe: "1Day", limit: 100 });
          const bars = JSON.parse(barsJson);
          if (bars.length > 20) data = bars;
        } catch (_) {}
      }
      if (data) {
        closePrices[sym] = data.slice(-100).map(b => b.close || b.c || 0);
      }
    }

    const validSymbols = Object.keys(closePrices).filter(s => closePrices[s].length > 10);
    if (validSymbols.length < 2) {
      win.contentElement.textContent = "";
      win.setContent("Insufficient cached bar data for correlation. Load some charts first.");
      return;
    }

    // Calculate returns
    const returns = {};
    for (const sym of validSymbols) {
      const prices = closePrices[sym];
      returns[sym] = [];
      for (let i = 1; i < prices.length; i++) {
        returns[sym].push(prices[i] > 0 ? (prices[i] - prices[i - 1]) / prices[i - 1] : 0);
      }
    }

    // Pearson correlation
    function pearson(a, b) {
      const n = Math.min(a.length, b.length);
      if (n < 5) return 0;
      let sumA = 0, sumB = 0, sumAB = 0, sumA2 = 0, sumB2 = 0;
      for (let i = 0; i < n; i++) {
        sumA += a[i]; sumB += b[i];
        sumAB += a[i] * b[i];
        sumA2 += a[i] * a[i];
        sumB2 += b[i] * b[i];
      }
      const num = n * sumAB - sumA * sumB;
      const den = Math.sqrt((n * sumA2 - sumA * sumA) * (n * sumB2 - sumB * sumB));
      return den > 0 ? num / den : 0;
    }

    // Build matrix
    const matrix = [];
    for (let i = 0; i < validSymbols.length; i++) {
      matrix[i] = [];
      for (let j = 0; j < validSymbols.length; j++) {
        matrix[i][j] = i === j ? 1.0 : pearson(returns[validSymbols[i]], returns[validSymbols[j]]);
      }
    }

    win.contentElement.textContent = "";

    // Render heatmap table
    const table = document.createElement("table");
    table.className = "fw-table corr-matrix";
    table.style.cssText = "border-collapse:collapse;font-size:10px;";

    // Header row
    const headerRow = document.createElement("tr");
    headerRow.appendChild(document.createElement("td")); // corner cell
    for (const sym of validSymbols) {
      const th = document.createElement("td");
      th.textContent = sym.substring(0, 6);
      th.style.cssText = "color:#888;font-weight:bold;text-align:center;padding:3px 4px;writing-mode:vertical-lr;font-size:9px;";
      headerRow.appendChild(th);
    }
    table.appendChild(headerRow);

    // Data rows
    for (let i = 0; i < validSymbols.length; i++) {
      const tr = document.createElement("tr");
      const label = document.createElement("td");
      label.textContent = validSymbols[i].substring(0, 6);
      label.style.cssText = "color:#888;font-weight:bold;text-align:right;padding:3px 6px;font-size:9px;";
      tr.appendChild(label);

      for (let j = 0; j < validSymbols.length; j++) {
        const td = document.createElement("td");
        const corr = matrix[i][j];
        td.textContent = corr.toFixed(2);
        td.style.cssText = "text-align:center;padding:3px 4px;font-size:9px;border:1px solid #222;";

        // Color: green for positive, red for negative, intensity by magnitude
        const absCorr = Math.abs(corr);
        const alpha = (absCorr * 0.7 + 0.1).toFixed(2);
        if (i === j) {
          td.style.background = "#333";
          td.style.color = "#fff";
        } else if (corr > 0) {
          td.style.background = `rgba(76, 175, 80, ${alpha})`;
          td.style.color = absCorr > 0.5 ? "#fff" : "#ccc";
        } else {
          td.style.background = `rgba(244, 67, 54, ${alpha})`;
          td.style.color = absCorr > 0.5 ? "#fff" : "#ccc";
        }
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }

    win.appendElement(table);

    // Legend
    const legend = document.createElement("div");
    legend.style.cssText = "margin-top:8px;font-size:9px;color:#666;";
    legend.textContent = "Green = positive correlation, Red = negative. Based on daily returns from cached/fetched bar data.";
    win.appendElement(legend);

  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Failed to build correlation matrix: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// MONTECARLO — Risk of Ruin Simulation
// ══════════════════════════════════════════════════════════════

function cmdMonteCarlo() {
  const win = createWindow({ title: "Monte Carlo Risk of Ruin", width: 550, height: 500 });
  win.contentElement.textContent = "";

  // Controls
  const controls = document.createElement("div");
  controls.style.cssText = "padding:8px;display:flex;gap:8px;flex-wrap:wrap;align-items:center;border-bottom:1px solid #333;";

  const makeField = (label, value) => {
    const wrap = document.createElement("label");
    wrap.style.cssText = "font-size:10px;color:#888;display:flex;gap:4px;align-items:center;";
    wrap.textContent = label;
    const inp = document.createElement("input");
    inp.type = "number";
    inp.value = value;
    inp.style.cssText = "width:60px;font-size:10px;padding:2px 4px;background:#1a1a2e;border:1px solid #333;color:#ccc;";
    wrap.appendChild(inp);
    return { wrap, inp };
  };

  const simCount = makeField("Simulations:", "10000");
  const equityField = makeField("Starting Equity:", "100000");

  const info = document.createElement("div");
  info.style.cssText = "font-size:10px;color:#666;padding:4px 8px;";
  info.textContent = "First run a BACKTEST, then click Run Monte Carlo. It uses the trade P&L array from the last backtest.";

  const runBtn = document.createElement("button");
  runBtn.textContent = "Run Monte Carlo";
  runBtn.className = "bt-run-btn";
  runBtn.style.cssText = "font-size:10px;padding:4px 12px;background:#0f3460;border:1px solid #555;color:#8cf;cursor:pointer;";

  // Manual P&L input option
  const pnlLabel = document.createElement("div");
  pnlLabel.style.cssText = "font-size:10px;color:#888;padding:4px 8px;width:100%;";
  pnlLabel.textContent = "Or paste P&L array (comma-separated):";
  const pnlInput = document.createElement("input");
  pnlInput.type = "text";
  pnlInput.placeholder = "e.g. 120,-80,50,-30,200,-150";
  pnlInput.style.cssText = "width:calc(100% - 16px);margin:0 8px;font-size:10px;padding:2px 4px;background:#1a1a2e;border:1px solid #333;color:#ccc;";

  controls.appendChild(simCount.wrap);
  controls.appendChild(equityField.wrap);
  controls.appendChild(runBtn);
  win.appendElement(controls);
  win.appendElement(info);
  win.appendElement(pnlLabel);
  win.appendElement(pnlInput);

  const resultsDiv = document.createElement("div");
  resultsDiv.style.cssText = "padding:8px;";
  win.appendElement(resultsDiv);

  // Store last backtest trades for Monte Carlo
  runBtn.addEventListener("click", () => {
    let tradePnLs = [];

    // Check for manual input
    const manualText = pnlInput.value.trim();
    if (manualText) {
      tradePnLs = manualText.split(",").map(v => parseFloat(v.trim())).filter(v => !isNaN(v));
    }

    // Try to get from last backtest result (stored globally)
    if (tradePnLs.length === 0 && window._lastBacktestTrades) {
      tradePnLs = window._lastBacktestTrades.map(t => t.pnl || t.profit || 0);
    }

    if (tradePnLs.length < 3) {
      resultsDiv.textContent = "";
      const msg = document.createElement("div");
      msg.style.color = "#f44";
      msg.textContent = "Need at least 3 trade P&Ls. Run a BACKTEST first or paste P&L values above.";
      resultsDiv.appendChild(msg);
      return;
    }

    const numSims = Math.min(parseInt(simCount.inp.value) || 10000, 100000);
    const startingEquity = parseFloat(equityField.inp.value) || 100000;
    const thresholds = [0.25, 0.50, 0.75]; // drawdown levels

    runBtn.disabled = true;
    runBtn.textContent = "Running...";

    // Run Monte Carlo in setTimeout to avoid blocking UI
    setTimeout(() => {
      const ruinCounts = [0, 0, 0]; // count of sims hitting each threshold
      const finalEquities = [];
      const maxDrawdowns = [];
      const numTrades = tradePnLs.length;

      for (let sim = 0; sim < numSims; sim++) {
        // Fisher-Yates shuffle of trade P&Ls
        const shuffled = [...tradePnLs];
        for (let i = shuffled.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
        }

        let equity = startingEquity;
        let peak = equity;
        let maxDD = 0;

        for (const pnl of shuffled) {
          equity += pnl;
          if (equity > peak) peak = equity;
          const dd = (peak - equity) / peak;
          if (dd > maxDD) maxDD = dd;
        }

        finalEquities.push(equity);
        maxDrawdowns.push(maxDD);

        for (let t = 0; t < thresholds.length; t++) {
          if (maxDD >= thresholds[t]) ruinCounts[t]++;
        }
      }

      // Sort for percentile calculations
      finalEquities.sort((a, b) => a - b);
      maxDrawdowns.sort((a, b) => a - b);

      resultsDiv.textContent = "";

      const title = document.createElement("div");
      title.style.cssText = "font-size:11px;color:#8cf;font-weight:bold;margin-bottom:8px;";
      title.textContent = `Monte Carlo Results (${numSims.toLocaleString()} simulations, ${numTrades} trades)`;
      resultsDiv.appendChild(title);

      // Risk of Ruin table
      const table = document.createElement("table");
      table.className = "fw-table";
      const thead = document.createElement("tr");
      for (const h of ["Drawdown Threshold", "Probability of Ruin", "Simulations Hit"]) {
        const th = document.createElement("td");
        th.style.cssText = "color:#888;font-weight:bold;font-size:10px;border-bottom:1px solid #333;";
        th.textContent = h;
        thead.appendChild(th);
      }
      table.appendChild(thead);

      for (let t = 0; t < thresholds.length; t++) {
        const tr = document.createElement("tr");
        const pct = (ruinCounts[t] / numSims * 100).toFixed(2);
        for (const val of [`-${(thresholds[t] * 100).toFixed(0)}%`, `${pct}%`, `${ruinCounts[t].toLocaleString()}`]) {
          const td = document.createElement("td");
          td.className = "fw-value";
          td.textContent = val;
          if (val.endsWith("%") && !val.startsWith("-")) {
            const p = parseFloat(val);
            td.style.color = p > 50 ? "#f44336" : p > 20 ? "#ff9800" : "#4caf50";
          }
          tr.appendChild(td);
        }
        table.appendChild(tr);
      }
      resultsDiv.appendChild(table);

      // Distribution stats
      const stats = document.createElement("div");
      stats.style.cssText = "margin-top:12px;font-size:10px;color:#ccc;";
      const median = finalEquities[Math.floor(numSims / 2)];
      const p5 = finalEquities[Math.floor(numSims * 0.05)];
      const p95 = finalEquities[Math.floor(numSims * 0.95)];
      const avgDD = (maxDrawdowns.reduce((a, b) => a + b, 0) / numSims * 100).toFixed(1);
      const medDD = (maxDrawdowns[Math.floor(numSims / 2)] * 100).toFixed(1);
      stats.textContent = "";
      const addStat = (label, value, color) => {
        const d = document.createElement("div");
        d.textContent = `${label}: `;
        const s = document.createElement("span");
        s.style.color = color;
        s.textContent = value;
        d.appendChild(s);
        stats.appendChild(d);
      };
      const hdr1 = document.createElement("div");
      hdr1.style.cssText = "margin-bottom:4px;font-weight:bold;color:#888;";
      hdr1.textContent = "Equity Distribution";
      stats.appendChild(hdr1);
      addStat("Median Final Equity", `$${median.toFixed(2)}`, "#8cf");
      addStat("5th Percentile", `$${p5.toFixed(2)}`, "#f44");
      addStat("95th Percentile", `$${p95.toFixed(2)}`, "#4caf50");
      const hdr2 = document.createElement("div");
      hdr2.style.cssText = "margin-top:6px;font-weight:bold;color:#888;";
      hdr2.textContent = "Drawdown Distribution";
      stats.appendChild(hdr2);
      addStat("Average Max DD", `${avgDD}%`, "#ff9800");
      addStat("Median Max DD", `${medDD}%`, "#ff9800");
      resultsDiv.appendChild(stats);

      runBtn.disabled = false;
      runBtn.textContent = "Run Monte Carlo";
    }, 50);
  });
}

// ══════════════════════════════════════════════════════════════
// ALERTS — Multi-Condition Alert Manager UI
// ══════════════════════════════════════════════════════════════

function cmdMultiAlerts() {
  const win = createWindow({ title: "Multi-Condition Alerts", width: 500, height: 400 });
  win.contentElement.textContent = "";

  const CONDITIONS = [
    "RSI > 70", "RSI < 30",
    "KAMA > SMA200", "KAMA < SMA200",
    "Fisher > 0", "Fisher < 0",
  ];

  // Add alert controls
  const addRow = document.createElement("div");
  addRow.style.cssText = "padding:8px;display:flex;gap:6px;align-items:center;border-bottom:1px solid #333;";

  const symInput = document.createElement("input");
  symInput.type = "text";
  symInput.value = currentSymbol || "";
  symInput.placeholder = "Symbol";
  symInput.style.cssText = "width:80px;font-size:10px;padding:2px 4px;background:#1a1a2e;border:1px solid #333;color:#ccc;";

  const condSelect = document.createElement("select");
  condSelect.style.cssText = "font-size:10px;padding:2px 4px;background:#1a1a2e;border:1px solid #333;color:#ccc;";
  for (const c of CONDITIONS) {
    const opt = document.createElement("option");
    opt.value = c;
    opt.textContent = c;
    condSelect.appendChild(opt);
  }

  // Also allow price alerts here
  const priceInput = document.createElement("input");
  priceInput.type = "number";
  priceInput.placeholder = "Price (or leave empty)";
  priceInput.style.cssText = "width:80px;font-size:10px;padding:2px 4px;background:#1a1a2e;border:1px solid #333;color:#ccc;";

  const dirSelect = document.createElement("select");
  dirSelect.style.cssText = "font-size:10px;padding:2px 4px;background:#1a1a2e;border:1px solid #333;color:#ccc;";
  for (const d of ["above", "below"]) {
    const opt = document.createElement("option");
    opt.value = d; opt.textContent = d;
    dirSelect.appendChild(opt);
  }

  const addBtn = document.createElement("button");
  addBtn.textContent = "Add";
  addBtn.style.cssText = "font-size:10px;padding:2px 8px;background:#0f3460;border:1px solid #555;color:#8cf;cursor:pointer;";

  addRow.appendChild(symInput);
  addRow.appendChild(condSelect);
  addRow.appendChild(priceInput);
  addRow.appendChild(dirSelect);
  addRow.appendChild(addBtn);
  win.appendElement(addRow);

  const listDiv = document.createElement("div");
  listDiv.style.cssText = "padding:4px;";
  win.appendElement(listDiv);

  function renderAlertList() {
    listDiv.textContent = "";

    // Multi-condition alerts
    if (multiConditionAlerts.length > 0) {
      const heading = document.createElement("div");
      heading.style.cssText = "color:#888;font-size:10px;margin:4px 0;text-transform:uppercase;";
      heading.textContent = "Indicator Alerts";
      listDiv.appendChild(heading);

      for (let i = 0; i < multiConditionAlerts.length; i++) {
        const a = multiConditionAlerts[i];
        const row = document.createElement("div");
        row.style.cssText = "display:flex;justify-content:space-between;align-items:center;padding:3px 0;border-bottom:1px solid #1a1a2e;";
        const text = document.createElement("span");
        text.style.cssText = `font-size:10px;color:${a.triggered ? "#666" : "#ccc"};${a.triggered ? "text-decoration:line-through;" : ""}`;
        text.textContent = `${a.symbol} — ${a.condition}`;
        const delBtn = document.createElement("button");
        delBtn.textContent = "x";
        delBtn.style.cssText = "font-size:9px;padding:1px 4px;background:#3a0a0a;border:1px solid #555;color:#f44;cursor:pointer;";
        delBtn.addEventListener("click", () => {
          multiConditionAlerts.splice(i, 1);
          saveMultiAlerts();
          renderAlertList();
        });
        row.appendChild(text);
        row.appendChild(delBtn);
        listDiv.appendChild(row);
      }
    }

    // Price alerts
    if (priceAlerts.length > 0) {
      const heading2 = document.createElement("div");
      heading2.style.cssText = "color:#888;font-size:10px;margin:8px 0 4px;text-transform:uppercase;";
      heading2.textContent = "Price Alerts";
      listDiv.appendChild(heading2);

      for (let i = 0; i < priceAlerts.length; i++) {
        const a = priceAlerts[i];
        const row = document.createElement("div");
        row.style.cssText = "display:flex;justify-content:space-between;align-items:center;padding:3px 0;border-bottom:1px solid #1a1a2e;";
        const text = document.createElement("span");
        text.style.cssText = `font-size:10px;color:${a.triggered ? "#666" : "#ccc"};${a.triggered ? "text-decoration:line-through;" : ""}`;
        text.textContent = `${a.symbol} — Price ${a.direction} $${a.price.toFixed(4)}`;
        const delBtn = document.createElement("button");
        delBtn.textContent = "x";
        delBtn.style.cssText = "font-size:9px;padding:1px 4px;background:#3a0a0a;border:1px solid #555;color:#f44;cursor:pointer;";
        delBtn.addEventListener("click", () => {
          priceAlerts.splice(i, 1);
          saveAlerts();
          renderAlertList();
        });
        row.appendChild(text);
        row.appendChild(delBtn);
        listDiv.appendChild(row);
      }
    }

    if (multiConditionAlerts.length === 0 && priceAlerts.length === 0) {
      const empty = document.createElement("div");
      empty.style.cssText = "color:#666;font-size:10px;padding:20px;text-align:center;";
      empty.textContent = "No alerts set. Use the controls above to add one.";
      listDiv.appendChild(empty);
    }
  }

  addBtn.addEventListener("click", () => {
    const sym = symInput.value.trim().toUpperCase();
    if (!sym) return;

    const priceVal = parseFloat(priceInput.value);
    if (!isNaN(priceVal) && priceVal > 0) {
      // Price alert
      addPriceAlert(sym, priceVal, dirSelect.value);
    } else {
      // Multi-condition alert
      addMultiConditionAlert(sym, condSelect.value);
    }
    priceInput.value = "";
    renderAlertList();
  });

  renderAlertList();
}

// ══════════════════════════════════════════════════════════════
// Drawing Object Properties Panel
// ══════════════════════════════════════════════════════════════

function setupDrawingPropertiesPanel() {
  const container = document.getElementById("chart-container");
  if (!container) return;

  // Right-click near a drawing to show properties
  container.addEventListener("contextmenu", (e) => {
    if (drawingMode) return; // don't interfere with drawing

    const rect = container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Find nearest drawing
    const HIT_DIST = 15; // pixels
    let nearestIdx = -1;
    let nearestDist = Infinity;

    for (let di = 0; di < drawings.length; di++) {
      const d = drawings[di];
      if (d.type === "horizontal") {
        const drawY = candleSeries.priceToCoordinate(d.p1.price);
        if (drawY === null) continue;
        const dist = Math.abs(y - drawY);
        if (dist < nearestDist && dist < HIT_DIST) {
          nearestDist = dist;
          nearestIdx = di;
        }
      } else if (d.type === "trendline" || d.type === "fibonacci" || d.type === "rectangle" || d.type === "channel") {
        const x1 = chart.timeScale().timeToCoordinate(d.p1.time);
        const y1 = candleSeries.priceToCoordinate(d.p1.price);
        const x2 = chart.timeScale().timeToCoordinate(d.p2.time);
        const y2 = candleSeries.priceToCoordinate(d.p2.price);
        if (x1 === null || y1 === null || x2 === null || y2 === null) continue;

        // Distance from point to line segment
        const dx = x2 - x1, dy2 = y2 - y1;
        const lenSq = dx * dx + dy2 * dy2;
        let t = lenSq > 0 ? ((x - x1) * dx + (y - y1) * dy2) / lenSq : 0;
        t = Math.max(0, Math.min(1, t));
        const projX = x1 + t * dx, projY = y1 + t * dy2;
        const dist = Math.sqrt((x - projX) ** 2 + (y - projY) ** 2);
        if (dist < nearestDist && dist < HIT_DIST) {
          nearestDist = dist;
          nearestIdx = di;
        }
      }
    }

    if (nearestIdx < 0) return;

    e.preventDefault();
    e.stopPropagation();

    // Show properties panel
    showDrawingProperties(nearestIdx, e.clientX, e.clientY);
  });
}

function showDrawingProperties(drawingIndex, screenX, screenY) {
  // Remove any existing properties panel
  const existing = document.getElementById("drawing-props-panel");
  if (existing) existing.remove();

  const d = drawings[drawingIndex];
  if (!d) return;

  const panel = document.createElement("div");
  panel.id = "drawing-props-panel";
  panel.style.cssText = `
    position:fixed;left:${screenX}px;top:${screenY}px;z-index:2000;
    background:#1a1a2e;border:1px solid #555;padding:8px;border-radius:4px;
    font-size:10px;color:#ccc;min-width:180px;box-shadow:0 4px 12px rgba(0,0,0,0.5);
  `;

  // Title
  const title = document.createElement("div");
  title.style.cssText = "font-weight:bold;margin-bottom:6px;color:#8cf;";
  title.textContent = `${d.type.charAt(0).toUpperCase() + d.type.slice(1)} Properties`;
  panel.appendChild(title);

  // Color picker
  const colorRow = document.createElement("div");
  colorRow.style.cssText = "display:flex;align-items:center;gap:6px;margin-bottom:4px;";
  const colorLabel = document.createElement("span");
  colorLabel.textContent = "Color:";
  const colorInput = document.createElement("input");
  colorInput.type = "color";
  colorInput.value = d.color || (d.type === "trendline" ? "#00bcd4" : d.type === "horizontal" ? "#ff9800" : "#00bcd4");
  colorInput.style.cssText = "width:30px;height:20px;border:none;cursor:pointer;background:transparent;";
  colorRow.appendChild(colorLabel);
  colorRow.appendChild(colorInput);
  panel.appendChild(colorRow);

  // Line width slider
  const widthRow = document.createElement("div");
  widthRow.style.cssText = "display:flex;align-items:center;gap:6px;margin-bottom:4px;";
  const widthLabel = document.createElement("span");
  widthLabel.textContent = "Width:";
  const widthInput = document.createElement("input");
  widthInput.type = "range";
  widthInput.min = "0.5";
  widthInput.max = "5";
  widthInput.step = "0.5";
  widthInput.value = d.lineWidth || "1.5";
  widthInput.style.cssText = "width:80px;";
  const widthVal = document.createElement("span");
  widthVal.textContent = widthInput.value;
  widthInput.addEventListener("input", () => { widthVal.textContent = widthInput.value; });
  widthRow.appendChild(widthLabel);
  widthRow.appendChild(widthInput);
  widthRow.appendChild(widthVal);
  panel.appendChild(widthRow);

  // Apply button
  const applyBtn = document.createElement("button");
  applyBtn.textContent = "Apply";
  applyBtn.style.cssText = "font-size:10px;padding:2px 12px;background:#0f3460;border:1px solid #555;color:#8cf;cursor:pointer;margin-right:4px;";
  applyBtn.addEventListener("click", () => {
    drawings[drawingIndex].color = colorInput.value;
    drawings[drawingIndex].lineWidth = parseFloat(widthInput.value);
    saveDrawings();
    renderDrawings();
    panel.remove();
  });

  // Delete button
  const deleteBtn = document.createElement("button");
  deleteBtn.textContent = "Delete";
  deleteBtn.style.cssText = "font-size:10px;padding:2px 12px;background:#3a0a0a;border:1px solid #555;color:#f44;cursor:pointer;";
  deleteBtn.addEventListener("click", () => {
    drawings.splice(drawingIndex, 1);
    saveDrawings();
    renderDrawings();
    panel.remove();
    log("Drawing deleted", "info");
  });

  const btnRow = document.createElement("div");
  btnRow.style.cssText = "margin-top:6px;";
  btnRow.appendChild(applyBtn);
  btnRow.appendChild(deleteBtn);
  panel.appendChild(btnRow);

  document.body.appendChild(panel);

  // Close on click outside
  const closeHandler = (e) => {
    if (!panel.contains(e.target)) {
      panel.remove();
      document.removeEventListener("mousedown", closeHandler);
    }
  };
  setTimeout(() => document.addEventListener("mousedown", closeHandler), 100);
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
  for (const [v, l] of [["1Min","M1"],["5Min","M5"],["15Min","M15"],["30Min","M30"],["1Hour","H1"],["4Hour","H4"],["1Day","D1"],["1Week","W1"]]) {
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
  for (const s of ["SMA Cross", "NNFX (KAMA+Fisher)"]) {
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

  const wfBtn = document.createElement("button");
  wfBtn.textContent = "Walk-Forward";
  wfBtn.className = "bt-run-btn";
  wfBtn.style.cssText = "font-size:10px;padding:4px 10px;background:#3a0f60;border:1px solid #555;color:#c8f;cursor:pointer;";

  controls.appendChild(symInput);
  controls.appendChild(tfSelect);
  controls.appendChild(stratSelect);
  controls.appendChild(fastInput);
  controls.appendChild(slowInput);
  controls.appendChild(runBtn);
  controls.appendChild(wfBtn);
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
      // Store for Monte Carlo
      window._lastBacktestTrades = trades;
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

  // ── Walk-Forward Test ──────────────────────────────────────
  wfBtn.addEventListener("click", async () => {
    const btSym = symInput.querySelector("input").value.trim().toUpperCase() || sym;
    const btTf = tfSel.value;
    const btFast = parseInt(fastInput.querySelector("input").value) || 10;
    const btSlow = parseInt(slowInput.querySelector("input").value) || 50;

    wfBtn.disabled = true;
    wfBtn.textContent = "Running WF...";
    statsDiv.textContent = "";
    tradesDiv.textContent = "";

    try {
      const json = await invoke("run_walk_forward", {
        symbol: btSym,
        timeframe: btTf,
        fast_min: Math.max(2, btFast - 10),
        fast_max: btFast + 10,
        slow_min: Math.max(3, btSlow - 30),
        slow_max: btSlow + 30,
        in_sample_pct: 70.0,
      });
      const result = typeof json === "string" ? JSON.parse(json) : json;

      // Display walk-forward results
      const wfTitle = document.createElement("div");
      wfTitle.style.cssText = "font-size:11px;font-weight:bold;color:#c8f;margin-bottom:8px;";
      wfTitle.textContent = `Walk-Forward: Best params Fast=${result.best_fast} Slow=${result.best_slow}`;
      statsDiv.appendChild(wfTitle);

      const splitInfo = document.createElement("div");
      splitInfo.style.cssText = "font-size:10px;color:#888;margin-bottom:8px;";
      splitInfo.textContent = `In-sample: ${result.in_sample_bars} bars | Out-of-sample: ${result.out_sample_bars} bars`;
      statsDiv.appendChild(splitInfo);

      // Side-by-side results
      const renderSection = (label, data, color) => {
        const heading = document.createElement("div");
        heading.style.cssText = `font-size:10px;font-weight:bold;color:${color};margin:8px 0 4px;border-bottom:1px solid #333;padding-bottom:2px;`;
        heading.textContent = label;
        statsDiv.appendChild(heading);

        const report = data.report || {};
        const table = document.createElement("table");
        table.className = "fw-table";
        const rows = [
          ["Total P/L", report.total_pnl != null ? `$${Number(report.total_pnl).toFixed(2)}` : "—"],
          ["Win Rate", report.win_rate != null ? `${Number(report.win_rate).toFixed(1)}%` : "—"],
          ["Profit Factor", report.profit_factor != null ? Number(report.profit_factor).toFixed(2) : "—"],
          ["Sharpe Ratio", report.sharpe_ratio != null ? Number(report.sharpe_ratio).toFixed(3) : "—"],
          ["Max Drawdown", report.max_drawdown_pct != null ? `${Number(report.max_drawdown_pct).toFixed(1)}%` : "—"],
          ["Trades", report.total_trades ?? "—"],
        ];
        for (const [l, v] of rows) {
          const tr = document.createElement("tr");
          const td1 = document.createElement("td");
          td1.className = "fw-label"; td1.textContent = l;
          const td2 = document.createElement("td");
          td2.className = "fw-value"; td2.textContent = v;
          tr.appendChild(td1); tr.appendChild(td2);
          table.appendChild(tr);
        }
        statsDiv.appendChild(table);

        // Store out-of-sample trades for Monte Carlo
        if (label.includes("Out-of-Sample") && data.trades) {
          window._lastBacktestTrades = data.trades;
        }
      };

      renderSection("In-Sample (Optimization)", result.in_sample, "#4caf50");
      renderSection("Out-of-Sample (Validation)", result.out_sample, "#ff9800");

      // Equity curves on the chart
      if (eqChart) { eqChart.remove(); eqChart = null; }
      eqChart = createChart(eqContainer, {
        width: eqContainer.clientWidth,
        height: 200,
        layout: { background: { color: "#000" }, textColor: "#888", fontFamily: "Consolas, monospace", attributionLogo: false },
        grid: { vertLines: { color: "#1a1a2e" }, horzLines: { color: "#1a1a2e" } },
        rightPriceScale: { borderColor: "#333" },
        timeScale: { borderColor: "#333" },
      });

      const isCurve = result.in_sample.equity_curve || [];
      const osCurve = result.out_sample.equity_curve || [];

      if (isCurve.length > 0) {
        const isS = eqChart.addLineSeries({ color: "#4caf50", lineWidth: 2, title: "In-Sample" });
        isS.setData(isCurve.map((v, i) => ({ time: i + 1, value: typeof v === "number" ? v : v.value || v.equity || 0 })));
      }
      if (osCurve.length > 0) {
        const osS = eqChart.addLineSeries({ color: "#ff9800", lineWidth: 2, title: "Out-of-Sample" });
        osS.setData(osCurve.map((v, i) => ({ time: isCurve.length + i + 1, value: typeof v === "number" ? v : v.value || v.equity || 0 })));
      }
      eqChart.timeScale().fitContent();

    } catch (e) {
      statsDiv.textContent = `Walk-forward failed: ${e}`;
      statsDiv.style.color = "#f44";
    }

    wfBtn.disabled = false;
    wfBtn.textContent = "Walk-Forward";
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

    const optSymbol = symInp.value.trim().toUpperCase() || sym;
    const fastMin = parseInt(fMin.inp.value) || 5;
    const fastMax = parseInt(fMax.inp.value) || 50;
    const slowMin = parseInt(sMin.inp.value) || 20;
    const slowMax = parseInt(sMax.inp.value) || 200;

    try {
      // Try Wasm-accelerated optimization first (50-100x faster)
      const wasm = await loadWasm();
      const cacheKey = getCacheKey(optSymbol, tf);
      const cached = barCache[cacheKey];

      if (wasm && cached && cached.data && cached.data.length > 0) {
        const t0 = performance.now();
        const chartData = cached.data.map(b => ({
          open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume || 0,
        }));
        const flat = packBarsForWasm(chartData);
        const raw = wasm.wasm_optimize_sma(flat, fastMin, fastMax, slowMin, slowMax, 100000, 50);
        // Unpack: 6 values per result [fast, slow, pnl, win_rate, pf, trades]
        lastResults = [];
        for (let i = 0; i < raw.length; i += 6) {
          lastResults.push({
            fast_period: raw[i], slow_period: raw[i + 1],
            total_pnl: raw[i + 2], win_rate: raw[i + 3],
            profit_factor: raw[i + 4], total_trades: raw[i + 5],
          });
        }
        const elapsed = (performance.now() - t0).toFixed(0);
        log(`Wasm optimizer: ${lastResults.length} results in ${elapsed}ms (${((fastMax-fastMin+1)*(slowMax-slowMin+1))} combos)`, "ok");
        sortCol = "pnl";
        sortAsc = false;
        const sorted = [...lastResults].sort((a, b) => (b.total_pnl ?? 0) - (a.total_pnl ?? 0));
        resultsDiv.style.color = "";
        renderOptResults(sorted);
      } else {
        // Fallback: Rust backend optimizer
        const json = await invoke("run_optimization", {
          symbol: optSymbol, timeframe: tf,
          fast_min: fastMin, fast_max: fastMax,
          slow_min: slowMin, slow_max: slowMax,
        });
        const result = typeof json === "string" ? JSON.parse(json) : json;
        lastResults = Array.isArray(result) ? result : (result.results || []);
        sortCol = "pnl";
        sortAsc = false;
        const sorted = [...lastResults].sort((a, b) => (b.total_pnl ?? 0) - (a.total_pnl ?? 0));
        resultsDiv.style.color = "";
        renderOptResults(sorted);
      }
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
// (Split view removed — MTF Grid covers all multi-chart layouts)

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
      ctx.strokeStyle = d.color || "#00bcd4";
      ctx.lineWidth = d.lineWidth || 1.5;
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
      const hColor = d.color || "#ff9800";
      ctx.beginPath();
      ctx.strokeStyle = hColor;
      ctx.lineWidth = d.lineWidth || 1;
      ctx.setLineDash([6, 3]);
      ctx.moveTo(0, y);
      ctx.lineTo(drawCanvas.width, y);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.fillStyle = hColor;
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

    } else if (d.type === "ray") {
      // Ray: line from p1 through p2, extending to right edge
      const x1 = chart.timeScale().timeToCoordinate(d.p1.time);
      const y1 = candleSeries.priceToCoordinate(d.p1.price);
      const x2 = chart.timeScale().timeToCoordinate(d.p2.time);
      const y2 = candleSeries.priceToCoordinate(d.p2.price);
      if (x1 === null || y1 === null || x2 === null || y2 === null) continue;
      ctx.beginPath();
      ctx.strokeStyle = d.color || "#ff9800";
      ctx.lineWidth = d.lineWidth || 1.5;
      ctx.moveTo(x1, y1);
      // Extend to right edge
      const dx = x2 - x1;
      const dy = y2 - y1;
      const extend = dx !== 0 ? (drawCanvas.width - x1) / dx : 1;
      ctx.lineTo(x1 + dx * extend, y1 + dy * extend);
      ctx.stroke();

    } else if (d.type === "ruler") {
      // Ruler: shows price distance, % change, bar count between two points
      const x1 = chart.timeScale().timeToCoordinate(d.p1.time);
      const y1 = candleSeries.priceToCoordinate(d.p1.price);
      const x2 = chart.timeScale().timeToCoordinate(d.p2.time);
      const y2 = candleSeries.priceToCoordinate(d.p2.price);
      if (x1 === null || y1 === null || x2 === null || y2 === null) continue;
      // Dashed line
      ctx.beginPath();
      ctx.strokeStyle = "#888";
      ctx.lineWidth = 1;
      ctx.setLineDash([3, 3]);
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
      ctx.setLineDash([]);
      // Label box
      const priceDiff = d.p2.price - d.p1.price;
      const pctDiff = d.p1.price !== 0 ? (priceDiff / d.p1.price * 100) : 0;
      const midX = (x1 + x2) / 2;
      const midY = (y1 + y2) / 2;
      const sign = priceDiff >= 0 ? "+" : "";
      const label = `${sign}$${priceDiff.toFixed(2)} (${sign}${pctDiff.toFixed(2)}%)`;
      ctx.fillStyle = "#000c";
      ctx.fillRect(midX - 2, midY - 14, ctx.measureText(label).width + 8, 16);
      ctx.fillStyle = priceDiff >= 0 ? "#4caf50" : "#f44336";
      ctx.font = "11px Consolas";
      ctx.fillText(label, midX + 2, midY - 2);
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

// ── Right-Click Context Menu ─────────────────────────────────

function setupChartContextMenu() {
  const container = document.getElementById("chart-container");
  let contextMenu = null;

  function removeMenu() {
    if (contextMenu) {
      contextMenu.remove();
      contextMenu = null;
    }
  }

  document.addEventListener("click", removeMenu);
  document.addEventListener("keydown", (e) => { if (e.key === "Escape") removeMenu(); });

  container.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    removeMenu();

    // Get price at click position
    const rect = container.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const clickPrice = candleSeries ? candleSeries.coordinateToPrice(y) : null;

    contextMenu = document.createElement("div");
    contextMenu.className = "chart-context-menu";
    contextMenu.style.cssText = "position:fixed;z-index:9999;background:#1a1a2e;border:1px solid #333;border-radius:4px;padding:4px 0;min-width:180px;font-size:11px;font-family:Consolas,monospace;box-shadow:0 4px 12px rgba(0,0,0,0.5);";
    contextMenu.style.left = e.clientX + "px";
    contextMenu.style.top = e.clientY + "px";

    const items = [
      { label: "Buy Lines", action: () => {
        if (!currentChartData || currentChartData.length === 0) return;
        const recent = currentChartData.slice(-50);
        createSLLine(Math.min(...recent.map(d => d.low)));
        createTPLine(Math.max(...recent.map(d => d.high)));
      }},
      { label: "Sell Lines", action: () => {
        if (!currentChartData || currentChartData.length === 0) return;
        const recent = currentChartData.slice(-50);
        createSLLine(Math.max(...recent.map(d => d.high)));
        createTPLine(Math.min(...recent.map(d => d.low)));
      }},
      { label: "Destroy Lines", action: () => { removeSLLine(); removeTPLine(); } },
      { type: "separator" },
      { label: "Draw Trend Line", action: () => {
        drawingMode = "trendline";
        drawingAnchor = null;
        container.style.cursor = "crosshair";
        log("Click two points for trend line", "info");
      }},
      { label: "Draw Fibonacci", action: () => {
        drawingMode = "fibonacci";
        drawingAnchor = null;
        container.style.cursor = "crosshair";
        log("Click two points for Fibonacci retracement", "info");
      }},
      { label: "Draw Ray", action: () => {
        drawingMode = "ray";
        drawingAnchor = null;
        container.style.cursor = "crosshair";
        log("Click two points for ray (extends right)", "info");
      }},
      { label: "Ruler (Measure)", action: () => {
        drawingMode = "ruler";
        drawingAnchor = null;
        container.style.cursor = "crosshair";
        log("Click two points to measure distance", "info");
      }},
      { type: "separator" },
      { label: "Set Alert", action: () => {
        if (clickPrice && clickPrice > 0) {
          const price = clickPrice;
          // Use the existing alerts system if available
          if (typeof addPriceAlert === "function") {
            addPriceAlert(currentSymbol, price, price > lastPrice ? "above" : "below");
          } else {
            log(`Alert price: ${price.toFixed(4)} (alerts system not loaded)`, "info");
          }
        }
      }},
      { label: "Copy Price", action: () => {
        if (clickPrice && clickPrice > 0) {
          navigator.clipboard.writeText(clickPrice.toFixed(6)).then(() => {
            log(`Copied price ${clickPrice.toFixed(6)} to clipboard`, "ok");
          }).catch(() => {
            log(`Price: ${clickPrice.toFixed(6)}`, "info");
          });
        }
      }},
    ];

    for (const item of items) {
      if (item.type === "separator") {
        const sep = document.createElement("div");
        sep.style.cssText = "height:1px;background:#333;margin:4px 0;";
        contextMenu.appendChild(sep);
        continue;
      }
      const row = document.createElement("div");
      row.style.cssText = "padding:5px 14px;color:#ccc;cursor:pointer;";
      row.textContent = item.label;
      row.addEventListener("mouseenter", () => { row.style.background = "#2a2a4e"; });
      row.addEventListener("mouseleave", () => { row.style.background = ""; });
      row.addEventListener("click", () => {
        removeMenu();
        item.action();
      });
      contextMenu.appendChild(row);
    }

    document.body.appendChild(contextMenu);

    // Adjust position if menu goes off screen
    const menuRect = contextMenu.getBoundingClientRect();
    if (menuRect.right > window.innerWidth) {
      contextMenu.style.left = (window.innerWidth - menuRect.width - 4) + "px";
    }
    if (menuRect.bottom > window.innerHeight) {
      contextMenu.style.top = (window.innerHeight - menuRect.height - 4) + "px";
    }
  });
}

// ── Pending Order Visualization on Chart ────────────────────

function setupOrderPriceLines() {
  // No-op init — lines are created/updated in updateOrderPriceLines()
}

async function updateOrderPriceLines() {
  // Remove old order lines from main chart
  if (candleSeries) {
    for (const line of orderPriceLines) {
      try { candleSeries.removePriceLine(line); } catch (_) {}
    }
  }
  orderPriceLines = [];

  // Remove old order lines from MTF grid cells
  for (const { cell, line } of mtfGridOrderLines) {
    try { cell.candleSeries.removePriceLine(line); } catch (_) {}
  }
  mtfGridOrderLines = [];

  // Only draw if we have a series and are connected
  if (!candleSeries || !currentSymbol) return;
  const sym = currentSymbol;
  const symNoSlash = sym.replace("/", "");

  try {
    // Fetch orders and positions in parallel
    const [ordersJson, posJson] = await Promise.all([
      invoke("get_open_orders"),
      invoke("get_positions"),
    ]);
    const orders = JSON.parse(ordersJson);
    const positions = JSON.parse(posJson);

    // ── Position SL/TP dotted lines (MT5-style) ──
    // Find open orders that are bracket legs for the current symbol's position.
    // Bracket SL = stop order on opposite side of position.
    // Bracket TP = limit order on opposite side of position.
    const pos = positions.find(p => p.symbol === sym || p.symbol === symNoSlash);
    if (pos) {
      const oppSide = pos.side === "long" ? "sell" : "buy";
      // Collect bracket leg prices for this position
      let posSL = null;
      let posTP = null;
      for (const o of orders) {
        const oSym = o.symbol || "";
        if (oSym !== sym && oSym !== symNoSlash) continue;
        if (o.side !== oppSide) continue;
        // Stop order on opposite side = SL leg
        if (o.order_type === "stop" && o.stop_price) {
          const p = parseFloat(o.stop_price);
          if (p > 0 && isFinite(p)) posSL = p;
        }
        // Limit order on opposite side = TP leg
        if (o.order_type === "limit" && o.limit_price) {
          const p = parseFloat(o.limit_price);
          if (p > 0 && isFinite(p)) posTP = p;
        }
      }

      // Fallback: if no bracket legs found, use locally-tracked SL/TP
      // (set when order was placed via set_sl_level/set_tp_level, or from manual SL/TP lines)
      if (!posSL) posSL = getSLPrice();
      if (!posTP) posTP = getTPPrice();

      // Draw position SL as dotted red line
      if (posSL) {
        try {
          const line = candleSeries.createPriceLine({
            price: posSL,
            color: "#f44336",
            lineWidth: 1,
            lineStyle: 3, // dotted (MT5-style)
            axisLabelVisible: true,
            title: `SL ${pos.side.toUpperCase()}`,
          });
          orderPriceLines.push(line);
        } catch (_) {}
      }

      // Draw position TP as dotted green line
      if (posTP) {
        try {
          const line = candleSeries.createPriceLine({
            price: posTP,
            color: "#4caf50",
            lineWidth: 1,
            lineStyle: 3, // dotted (MT5-style)
            axisLabelVisible: true,
            title: `TP ${pos.side.toUpperCase()}`,
          });
          orderPriceLines.push(line);
        } catch (_) {}
      }

      // Draw position entry as dotted blue line
      if (pos.avg_entry_price > 0) {
        try {
          const line = candleSeries.createPriceLine({
            price: pos.avg_entry_price,
            color: pos.side === "long" ? "#2196f3" : "#e91e63",
            lineWidth: 1,
            lineStyle: 3, // dotted
            axisLabelVisible: true,
            title: `ENTRY ${pos.side.toUpperCase()} ${Math.abs(pos.qty)}`,
          });
          orderPriceLines.push(line);
        } catch (_) {}
      }
    }

    // ── Position lines on MTF grid cells (mirrors main chart) ──
    if (pos && mtfGridActive && mtfGridCells.length > 0) {
      const drawGridLine = (price, color, style, title) => {
        if (!price || price <= 0) return;
        for (const cell of mtfGridCells) {
          try {
            const line = cell.candleSeries.createPriceLine({
              price, color, lineWidth: 1, lineStyle: style,
              axisLabelVisible: true, title,
            });
            mtfGridOrderLines.push({ cell, line });
          } catch (_) {}
        }
      };
      if (posSL) drawGridLine(posSL, "#f44336", 3, `SL`);
      if (posTP) drawGridLine(posTP, "#4caf50", 3, `TP`);
      if (pos.avg_entry_price > 0) drawGridLine(pos.avg_entry_price, pos.side === "long" ? "#2196f3" : "#e91e63", 3, `ENTRY`);
    }

    // ── Pending order lines (existing behavior) ──
    for (const o of orders) {
      const orderSymbol = o.symbol || "";
      if (orderSymbol !== sym && orderSymbol !== symNoSlash) continue;

      // Skip bracket legs already drawn as position SL/TP above
      if (pos) {
        const oppSide = pos.side === "long" ? "sell" : "buy";
        if (o.side === oppSide && (o.order_type === "stop" || o.order_type === "limit")) continue;
      }

      let price = null;
      let color = "#2196f3"; // default blue for limit
      let title = "";

      if (o.order_type === "limit" && o.limit_price) {
        price = parseFloat(o.limit_price);
        color = "#2196f3"; // blue
        title = `LMT ${o.side.toUpperCase()} ${o.qty}`;
      } else if (o.order_type === "stop" && o.stop_price) {
        price = parseFloat(o.stop_price);
        color = "#ff9800"; // orange
        title = `STP ${o.side.toUpperCase()} ${o.qty}`;
      } else if (o.order_type === "stop_limit") {
        price = parseFloat(o.stop_price || o.limit_price);
        color = "#9c27b0"; // purple
        title = `S/L ${o.side.toUpperCase()} ${o.qty}`;
      } else if (o.order_type === "trailing_stop") {
        if (o.stop_price) {
          price = parseFloat(o.stop_price);
          color = "#ff9800";
          title = `TRAIL ${o.side.toUpperCase()} ${o.qty}`;
        }
      }

      if (price && price > 0 && isFinite(price)) {
        try {
          const line = candleSeries.createPriceLine({
            price,
            color,
            lineWidth: 1,
            lineStyle: 2, // dashed
            axisLabelVisible: true,
            title,
          });
          orderPriceLines.push(line);
        } catch (_) {}
      }
    }
  } catch (_) {}
}

document.addEventListener("DOMContentLoaded", () => {
  loadBarCacheFromDisk().then(() => migrateLocalStorageCache());
  loadWasm(); // Preload 32KB Wasm indicator engine
  initChart();
  setupDrawingCanvas();
  loadDrawings();
  setupLineDrag();
  setupExtendedDrawings();
  patchRenderDrawings();
  setupDrawingPropertiesPanel();
  setupPaneResizers();
  setupLogPanel();
  setupNewsPanel();
  setupIndicatorPanel();
  setupPositionsPanel();
  setupOrdersPanel();
  loadAlerts();
  loadMultiAlerts();
  setupAutocomplete();
  setupButtons();
  setupKeyboard();
  setupConnect();
  setupMenuBar();
  setupMTFGrid();
  loadAnnotations();
  setupTabs();
  setupTemplates();
  setupProfiles();
  setupCommandPalette();
  loadSavedTheme(); // Restore persisted theme from localStorage
  // setupSplitButton removed — MTF Grid covers all multi-chart layouts
  setupScreenshotShortcut();
  setupCustomPluginUI();
  setupChartContextMenu();
  setupOrderPriceLines();

  // Auto-save session periodically and on shutdown
  setInterval(saveSession, 30000); // every 30s
  setInterval(checkWatchlistSMA200Alerts, 300000); // every 5min
  window.addEventListener("beforeunload", saveSession);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") saveSession();
  });
});

// ── MTF Grid View (MT5-style multi-timeframe) ───────────

let mtfGridActive = false;
let mtfGridCells = []; // [{ tf, symbol, chart, candleSeries, fisherChart, volumeChart, container }]
let mtfGridSymbol = ""; // symbol the grid was opened for
let mtfActiveCell = null; // currently selected grid cell for trading

function setupMTFGrid() {
  const btn = document.getElementById("btn-mtf-grid");
  const tfCheckboxes = document.getElementById("mtf-grid-tfs");

  btn.addEventListener("click", () => {
    if (mtfGridActive) {
      // Close grid — go back to single chart mode
      closeMTFGrid();
      tfCheckboxes.classList.add("hidden");
    } else {
      // Open grid with currently checked timeframes
      const selectedTFs = [...document.querySelectorAll(".mtf-grid-cb:checked")].map(cb => cb.value);
      if (selectedTFs.length < 2) {
        tfCheckboxes.classList.remove("hidden");
        alert("Select at least 2 timeframes");
        return;
      }
      if (!currentSymbol) { alert("Load a chart first"); return; }
      tfCheckboxes.classList.remove("hidden"); // Keep checkboxes visible while grid is active
      openMTFGrid(currentSymbol, selectedTFs);
    }
  });

  // Live checkbox changes — add/remove cells without closing grid
  for (const cb of document.querySelectorAll(".mtf-grid-cb")) {
    cb.addEventListener("change", () => {
      if (!mtfGridActive) return; // Only live-update when grid is open
      const selectedTFs = [...document.querySelectorAll(".mtf-grid-cb:checked")].map(c => c.value);
      if (selectedTFs.length < 1) return; // Don't close grid from unchecking all
      // Rebuild grid with new TF selection
      closeMTFGrid();
      openMTFGrid(currentSymbol || mtfGridSymbol, selectedTFs);
    });
  }
}

async function openMTFGrid(symbol, timeframes) {
  if (!symbol) { log("MTF grid: no symbol", "warn"); return; }
  mtfGridActive = true;
  mtfGridSymbol = symbol;
  const gridUseGpu = currentChartType.startsWith("gpu");
  const btn = document.getElementById("btn-mtf-grid");
  btn.textContent = "Close Grid";

  // Hide normal chart stack
  const chartStack = document.getElementById("chart-stack");
  chartStack.style.display = "none";

  try { // Wrap entire grid setup in try/catch — restore chart-stack on any error

  // Create grid container
  const gridContainer = document.createElement("div");
  gridContainer.id = "mtf-grid-container";
  const count = timeframes.length;
  gridContainer.className = `grid-${Math.min(count, 8)}`;
  chartStack.parentElement.insertBefore(gridContainer, chartStack);

  const tfLabels = MTF_LABELS; // Use shared auto-generated labels

  for (const tf of timeframes) {
    const cell = document.createElement("div");
    cell.className = "mtf-grid-cell";

    const label = document.createElement("div");
    label.className = "mtf-cell-label";
    label.textContent = `${symbol} ${tfLabels[tf] || tf}`;
    cell.appendChild(label);

    const chartDiv = document.createElement("div");
    chartDiv.className = "mtf-cell-chart";
    cell.appendChild(chartDiv);

    const fisherDiv = document.createElement("div");
    fisherDiv.className = "mtf-cell-fisher";
    cell.appendChild(fisherDiv);

    const volumeDiv = document.createElement("div");
    volumeDiv.className = "mtf-cell-volume";
    cell.appendChild(volumeDiv);

    gridContainer.appendChild(cell);

    // Create chart instances — GPU or CPU based on current chart type
    let cellChart, cellCandleSeries, cellGpuChart = null;

    if (gridUseGpu && gpuChartModule) {
      // GPU mode: create canvas + GpuChart per cell
      const gpuCanvas = document.createElement("canvas");
      gpuCanvas.style.cssText = "width:100%;height:100%;";
      gpuCanvas.id = `mtf-gpu-${tf}`;
      chartDiv.appendChild(gpuCanvas);

      // Need dimensions before creating GPU chart
      requestAnimationFrame(() => {
        gpuCanvas.width = chartDiv.clientWidth || 300;
        gpuCanvas.height = chartDiv.clientHeight || 200;
      });

      cellGpuChart = new gpuChartModule.GpuChart(`mtf-gpu-${tf}`);
      const gpuType = GPU_CHART_TYPES[currentChartType] ?? 0;
      cellGpuChart.set_chart_type(gpuType);

      // Dummy CPU chart objects (not used for rendering, needed for data flow)
      cellChart = { remove: () => {}, timeScale: () => ({ fitContent: () => {}, setVisibleLogicalRange: () => {} }), addLineSeries: () => ({ setData: () => {} }), addBaselineSeries: () => ({ setData: () => {} }), resize: () => {} };
      cellCandleSeries = {
        setData: (data) => {
          if (cellGpuChart && data.length > 0) {
            const flat = packBarsForWasm(data);
            cellGpuChart.set_data(flat);
            cellGpuChart.resize(gpuCanvas.width || 300, gpuCanvas.height || 200);
            cellGpuChart.render();
          }
        },
        createPriceLine: (opts) => ({ options: () => opts }),
        removePriceLine: () => {},
        data: () => [],
        update: () => {},
      };
    } else {
      // CPU mode: use lightweight-charts
      cellChart = createChart(chartDiv, {
        width: 100, height: 100,
        layout: { background: { color: "#000000" }, textColor: "#d1d4dc", fontFamily: "Consolas, Courier New, monospace", attributionLogo: false },
        grid: { vertLines: { color: "#222", style: 3 }, horzLines: { color: "#222", style: 3 } },
        crosshair: { mode: CrosshairMode.Normal },
        rightPriceScale: { borderColor: "#333" },
        timeScale: { borderColor: "#333", timeVisible: true },
      });

      cellCandleSeries = cellChart.addCandlestickSeries({
        upColor: "#00ff00", downColor: "#ff0000",
        borderDownColor: "#ff0000", borderUpColor: "#00ff00",
        wickDownColor: "#ff0000", wickUpColor: "#00ff00",
      });
    }

    let cellFisherChart = createChart(fisherDiv, {
      width: 100, height: 70,
      layout: { background: { color: "#000000" }, textColor: "#888", fontFamily: "Consolas, Courier New, monospace", attributionLogo: false },
      grid: { vertLines: { color: "#111" }, horzLines: { color: "#111" } },
      rightPriceScale: { borderColor: "#333" },
      timeScale: { visible: false },
      crosshair: { mode: CrosshairMode.Normal },
    });

    const cellVolumeChart = createChart(volumeDiv, {
      width: 100, height: 55,
      layout: { background: { color: "#000000" }, textColor: "#888", fontFamily: "Consolas, Courier New, monospace", attributionLogo: false },
      grid: { vertLines: { color: "#111" }, horzLines: { color: "#111" } },
      rightPriceScale: { borderColor: "#333" },
      timeScale: { visible: false },
      crosshair: { mode: CrosshairMode.Normal },
    });

    // Lock Fisher/Volume time scales to main chart — disable independent scrolling
    cellFisherChart.applyOptions({ handleScroll: false, handleScale: false });
    cellVolumeChart.applyOptions({ handleScroll: false, handleScale: false });

    // Sync Fisher/Volume time scale with main (persistent — fires on every scroll/zoom)
    cellChart.timeScale().subscribeVisibleLogicalRangeChange((range) => {
      if (range) {
        cellFisherChart.timeScale().setVisibleLogicalRange(range);
        cellVolumeChart.timeScale().setVisibleLogicalRange(range);
      }
    });

    // Cursor-following tooltip for this grid cell
    const cellTooltip = document.createElement("div");
    cellTooltip.className = "data-window";
    cellTooltip.style.display = "none";
    chartDiv.style.position = "relative";
    chartDiv.appendChild(cellTooltip);

    cellChart.subscribeCrosshairMove((param) => {
      if (!param.time || !param.point || param.point.x < 0) {
        cellTooltip.style.display = "none";
        return;
      }
      cellTooltip.style.display = "";
      const x = param.point.x + 12;
      const y = param.point.y + 12;
      const cw = chartDiv.clientWidth || 400;
      const ch = chartDiv.clientHeight || 300;
      cellTooltip.style.left = (x + 200 > cw ? Math.max(0, param.point.x - 210) : x) + "px";
      cellTooltip.style.top = (y + 120 > ch ? Math.max(0, param.point.y - 130) : y) + "px";

      const ohlc = param.seriesData.get(cellCandleSeries);
      if (ohlc && ohlc.open !== undefined) {
        const dp = ohlc.close > 100 ? 2 : ohlc.close > 1 ? 4 : 6;
        cellTooltip.textContent = `O:${ohlc.open.toFixed(dp)} H:${ohlc.high.toFixed(dp)}\nL:${ohlc.low.toFixed(dp)} C:${ohlc.close.toFixed(dp)}`;
      } else if (ohlc && ohlc.value !== undefined) {
        cellTooltip.textContent = `${ohlc.value.toFixed(4)}`;
      } else {
        cellTooltip.style.display = "none";
      }
    });

    const cellInfo = { tf, chart: cellChart, candleSeries: cellCandleSeries, fisherChart: cellFisherChart, volumeChart: cellVolumeChart, container: cell, chartDiv, fisherDiv, volumeDiv, gpuChart: cellGpuChart };
    mtfGridCells.push(cellInfo);

    // Single click to select as active trading cell
    cell.addEventListener("click", () => {
      mtfGridCells.forEach(c => c.container.style.outline = "none");
      cell.style.outline = "2px solid #4caf50";
      mtfActiveCell = cellInfo;
      currentTimeframe = tf;
      log(`MTF: ${tf} selected for trading`, "info");
    });

    // Auto-select first cell
    if (mtfGridCells.length === 0) {
      cell.style.outline = "2px solid #4caf50";
      mtfActiveCell = cellInfo;
    }

    // Attach SL/TP drag handler to this cell's chart area (capture phase)
    if (window._onDragMouseDown) chartDiv.addEventListener("mousedown", window._onDragMouseDown, true);

    // Double-click to fullscreen/restore
    cell.addEventListener("dblclick", () => {
      if (cell.classList.contains("fullscreen")) {
        cell.classList.remove("fullscreen");
        resizeMTFGrid();
      } else {
        // Remove fullscreen from others first
        mtfGridCells.forEach(c => c.container.classList.remove("fullscreen"));
        cell.classList.add("fullscreen");
        cellChart.resize(window.innerWidth, window.innerHeight - 90);
        cellFisherChart.resize(window.innerWidth, 50);
        cellVolumeChart.resize(window.innerWidth, 40);
      }
    });
  }

  // Split: cached cells load in parallel (instant), uncached load sequentially (rate-limited)
  const cached = [];
  const uncached = [];
  for (const cellInfo of mtfGridCells) {
    const cacheKey = getCacheKey(symbol, cellInfo.tf);
    if (barCache[cacheKey] && barCache[cacheKey].data && barCache[cacheKey].data.length > 0) {
      cached.push(cellInfo);
    } else {
      uncached.push(cellInfo);
    }
  }
  // Parallel: all cached cells at once (no API calls)
  await Promise.all(cached.map(c => loadMTFCellData(c, symbol)));
  // Sequential: uncached cells one at a time (respects rate limiter)
  for (const cellInfo of uncached) {
    await loadMTFCellData(cellInfo, symbol);
  }

  // Initial resize — multiple frames to ensure DOM has settled after grid creation.
  requestAnimationFrame(() => {
    resizeMTFGrid();
    requestAnimationFrame(() => {
      resizeMTFGrid();
      for (const cell of mtfGridCells) {
        cell.chart.timeScale().fitContent();
        // GPU cells: resize canvas and re-render after layout settles
        if (cell.gpuChart) {
          const w = cell.chartDiv.clientWidth || 300;
          const h = cell.chartDiv.clientHeight || 200;
          const canvas = cell.chartDiv.querySelector("canvas");
          if (canvas) { canvas.width = w; canvas.height = h; }
          cell.gpuChart.resize(w, h);
          cell.gpuChart.render();
        }
      }
    });
  });

  // Resize observer
  const ro = new ResizeObserver(() => resizeMTFGrid());
  ro.observe(gridContainer);

  } catch (e) {
    // Grid failed — restore main chart visibility
    log(`MTF grid failed: ${e}`, "error");
    mtfGridActive = false;
    btn.textContent = "MTF Grid";
    chartStack.style.display = "";
    const grid = document.getElementById("mtf-grid-container");
    if (grid) grid.remove();
    mtfGridCells = [];
  }
}

async function loadMTFCellData(cellInfo, symbol) {
  try {
    // Resolve custom timeframes (H12 → 4Hour × 3, etc.)
    const customTF = CUSTOM_TIMEFRAME_MAP[cellInfo.tf];
    const fetchTF = customTF ? customTF.base : cellInfo.tf;
    const aggFactor = customTF ? customTF.factor : 1;
    const limit = aggFactor > 1 ? 1000 * aggFactor : 1000;
    const cacheKey = getCacheKey(symbol, fetchTF);
    let bars;

    // Prefer cache — background pre-fetch should have all TFs cached
    const cached = barCache[cacheKey];
    if (cached && cached.data && cached.data.length > 0) {
      bars = cached.data;
      log(`MTF grid ${cellInfo.tf}: ${bars.length} bars from cache`, "info");
    } else {
      log(`MTF grid ${cellInfo.tf}: fetching ${fetchTF} (not cached)...`, "info");
      const barsJson = await invoke("get_bars", { symbol, timeframe: fetchTF, limit });
      bars = JSON.parse(barsJson);
      barCache[cacheKey] = { data: bars, timestamp: Date.now() };
    }

    let chartData = bars.map(b => ({
      time: Math.floor(new Date(b.timestamp).getTime() / 1000),
      open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume,
    }));

    // Aggregate if custom timeframe
    if (aggFactor > 1) {
      chartData = aggregateBars(chartData, aggFactor);
    }

    if (chartData.length === 0) return;

    // Ensure cell has dimensions before setting data (fixes 0×0 on first load)
    const w = cellInfo.chartDiv.clientWidth;
    const ch = cellInfo.chartDiv.clientHeight;
    if (w > 0 && ch > 0) {
      cellInfo.chart.resize(w, ch);
    }

    cellInfo.candleSeries.setData(chartData);
    // Zoom to recent bars — fewer for higher TFs (MT5-style)
    const tfBars = { "1Min": 120, "5Min": 100, "15Min": 80, "30Min": 70, "1Hour": 60, "4Hour": 50, "1Day": 40, "1Week": 30, "1Month": 24 };
    const visibleBars = Math.min(tfBars[cellInfo.tf] || 50, chartData.length);
    cellInfo.chart.timeScale().setVisibleLogicalRange({
      from: chartData.length - visibleBars,
      to: chartData.length + 2, // small right margin
    });

    // Fisher — color-segmented like main chart (green bullish, red bearish)
    if (chartData.length > 32) {
      const ef = calcEhlersFisher(chartData, 32);
      if (ef.fisher.length > 0) {
        // Build color segments (same as main chart)
        const segments = [];
        let curColor = ef.colors[0];
        let curSeg = [ef.fisher[0]];
        for (let i = 1; i < ef.fisher.length; i++) {
          if (ef.colors[i] !== curColor) {
            curSeg.push(ef.fisher[i]);
            segments.push({ color: curColor, data: curSeg });
            curColor = ef.colors[i];
            curSeg = [ef.fisher[i]];
          } else {
            curSeg.push(ef.fisher[i]);
          }
        }
        if (curSeg.length > 0) segments.push({ color: curColor, data: curSeg });

        // Ensure last segment always has >= 2 points for rendering (fixes last bar color change)
        if (segments.length >= 2) {
          const lastSeg = segments[segments.length - 1];
          if (lastSeg.data.length < 2) {
            const prevSeg = segments[segments.length - 2];
            lastSeg.data.unshift(prevSeg.data[prevSeg.data.length - 1]);
          }
        }

        for (const seg of segments) {
          if (seg.data.length < 2) continue;
          const s = cellInfo.fisherChart.addLineSeries({
            color: seg.color, lineWidth: 1.5,
            lastValueVisible: false, priceLineVisible: false,
          });
          s.setData(seg.data);
        }

        // Signal line (gray)
        const sSignal = cellInfo.fisherChart.addLineSeries({
          color: "#A9A9A9", lineWidth: 1,
          lastValueVisible: false, priceLineVisible: false,
        });
        sSignal.setData(ef.signal);

        // Zero line
        const sZero = cellInfo.fisherChart.addLineSeries({
          color: "#FFFFFF33", lineWidth: 1, lineStyle: 2,
          lastValueVisible: false, priceLineVisible: false,
        });
        sZero.setData(ef.fisher.map(d => ({ time: d.time, value: 0 })));
      }
    }
    // Sync Fisher time scale after data loaded
    cellInfo.fisherChart.timeScale().fitContent();

    // BetterVolume
    if (chartData.length > 22) {
      const bv = calcBetterVolume(chartData);
      if (bv.length > 0) {
        const s = cellInfo.volumeChart.addHistogramSeries({ priceFormat: { type: "volume" } });
        s.setData(bv);
      }
    }
    // Sync Volume time scale after data loaded
    cellInfo.volumeChart.timeScale().fitContent();

    // ── Full indicator set (matches main chart NNFX system) ──
    const addLine = (color, width, data) => {
      if (data.length < 2) return;
      const s = cellInfo.chart.addLineSeries({ color, lineWidth: width, lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false });
      s.setData(data);
    };

    // SMA 200 (yellow) — fall back to SMA 100 if insufficient data
    if (chartData.length > 200) {
      addLine("#FFD700", 1, calcSMA(chartData, 200));
    } else if (chartData.length > 100) {
      addLine("#FFD700", 1, calcSMA(chartData, 100));
    }

    // SMA 100 (magenta) if enough data
    if (chartData.length > 100) addLine("#FF00FF", 1, calcSMA(chartData, 100));

    // KAMA (white)
    if (chartData.length > 11) addLine("#FFFFFF", 2, calcKAMA(chartData, 10));

    // ATR Projection (yellow dotted — horizontal lines at lastOpen ± ATR, matches MQL5)
    if (chartData.length > 15) {
      const atrp = calcATRProjection(chartData, 14);
      if (atrp.atrValues.length > 0) {
        const lastATR = atrp.atrValues[atrp.atrValues.length - 1].value;
        const lastOpen = chartData[chartData.length - 1].open;
        const upper = lastOpen + lastATR;
        const lower = lastOpen - lastATR;
        const startIdx = Math.max(0, chartData.length - 30);
        const levelBars = chartData.slice(startIdx);
        if (levelBars.length >= 2) {
          const su = cellInfo.chart.addLineSeries({ color: "#FFFF00", lineWidth: 1, lineStyle: 1, lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false });
          const sl = cellInfo.chart.addLineSeries({ color: "#FFFF00", lineWidth: 1, lineStyle: 1, lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false });
          su.setData(levelBars.map(d => ({ time: d.time, value: upper })));
          sl.setData(levelBars.map(d => ({ time: d.time, value: lower })));
        }
      }
    }

    // Previous Candle Levels removed from MTF grid (MQL5 only draws HTF levels, not per-bar)

    // Recent price range — used by supply/demand zones + fib to filter out-of-range levels
    const recentBars = chartData.slice(-50);
    const recentHigh = Math.max(...recentBars.map(b => b.high));
    const recentLow = Math.min(...recentBars.map(b => b.low));
    const priceRange = recentHigh - recentLow;

    // Supply/Demand zones (lightweight — lines only, recent + visible range only)
    if (chartData.length > 12) {
      const zones = calcSupplyDemandZones(chartData);
      const relevantZones = zones.filter(z => z.low <= recentHigh + priceRange && z.high >= recentLow - priceRange);
      for (const z of relevantZones.slice(-4)) {
        const color = z.type === "supply" ? "#87CEEB66" : "#8FBC8F66";
        const zoneBars = chartData.filter(d => d.time >= z.startTime);
        if (zoneBars.length < 2) continue;
        addLine(color, 1, zoneBars.map(d => ({ time: d.time, value: z.high })));
        addLine(color, 1, zoneBars.map(d => ({ time: d.time, value: z.low })));
      }
    }

    // Auto Fibonacci (only levels within visible price range — prevents y-axis stretch)
    if (chartData.length > 30) {
      const fib = calcAutoFibonacci(chartData);
      if (fib) {
        const fibBars = chartData.filter(d => d.time >= fib.startTime);
        if (fibBars.length >= 2) {
          const keyLevels = fib.levels.filter(l => {
            // Retracements: always show (within swing range)
            // Extensions: only show within 1.5x recent range to prevent y-axis stretch on higher TFs
            const maxExt = l.type === "extension" ? 1.5 : 3;
            return l.price >= recentLow - priceRange * maxExt && l.price <= recentHigh + priceRange * maxExt;
          });
          const colors = {
            "0%": "#888", "23.6%": "#9c27b0", "38.2%": "#ffeb3b", "50%": "#8bc34a",
            "61.8%": "#00bcd4", "78.6%": "#e91e63", "100%": "#888",
            "127.2%": "#ff9800", "161.8%": "#ff5722", "200%": "#f44336",
            "261.8%": "#d32f2f", "361.8%": "#b71c1c", "423.6%": "#880e4f",
          };
          for (const level of keyLevels) {
            addLine(colors[level.label] || "#888", 0.8, fibBars.map(d => ({ time: d.time, value: level.price })));
          }
        }
      }
    }
  } catch (e) {
    log(`MTF grid load failed for ${cellInfo.tf}: ${e}`, "warn");
  }
}

function resizeMTFGrid() {
  for (const cell of mtfGridCells) {
    if (cell.container.classList.contains("fullscreen")) continue;
    const w = cell.chartDiv.clientWidth;
    const ch = cell.chartDiv.clientHeight;
    if (w > 0 && ch > 0) {
      cell.chart.resize(w, ch);
      cell.fisherChart.resize(cell.fisherDiv.clientWidth, cell.fisherDiv.clientHeight);
      cell.volumeChart.resize(cell.volumeDiv.clientWidth, cell.volumeDiv.clientHeight);
      // GPU cells: resize canvas + re-render
      if (cell.gpuChart) {
        const canvas = cell.chartDiv.querySelector("canvas");
        if (canvas) { canvas.width = w; canvas.height = ch; }
        cell.gpuChart.resize(w, ch);
        cell.gpuChart.render();
      }
    }
  }
}

// Sync live price across all MTF grid cells — update last bar's close to current price
function syncMTFGridLivePrice() {
  if (!mtfGridActive || mtfGridCells.length === 0 || lastPrice <= 0) return;
  // Guard: only sync if the grid symbol matches the current symbol.
  // Prevents cross-symbol contamination during tab switches (e.g. LUMN grid
  // getting SMCI's lastPrice, creating a false $8→$868 wick).
  if (mtfGridSymbol !== currentSymbol) return;
  for (const cell of mtfGridCells) {
    try {
      const data = cell.candleSeries.data();
      if (!data || data.length === 0) continue;
      const lastBar = data[data.length - 1];
      cell.candleSeries.update({
        time: lastBar.time,
        open: lastBar.open,
        high: Math.max(lastBar.high, lastPrice),
        low: Math.min(lastBar.low, lastPrice),
        close: lastPrice,
      });
    } catch (_) {}
  }
}

// ── Risk/Reward Overlay (visual P&L zones on chart) ──────────

let rrOverlaySeries = [];

function updateRiskRewardOverlay() {
  // Clear previous
  for (const s of rrOverlaySeries) { try { chart.removeSeries(s); } catch (_) {} }
  rrOverlaySeries = [];

  const sl = getSLPrice();
  const tp = getTPPrice();
  if (!sl || !tp || !lastPrice || lastPrice <= 0) return;

  const isBuy = tp > sl;

  // Loss zone (entry → SL) — red
  const lossFill = chart.addBaselineSeries({
    topFillColor1: "#f4433620", topFillColor2: "#f4433620",
    bottomFillColor1: "#f4433620", bottomFillColor2: "#f4433620",
    topLineColor: "transparent", bottomLineColor: "transparent",
    lineWidth: 0,
    baseValue: { type: "price", price: sl },
    lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false,
  });
  // Profit zone (entry → TP) — green
  const profitFill = chart.addBaselineSeries({
    topFillColor1: "#4caf5020", topFillColor2: "#4caf5020",
    bottomFillColor1: "#4caf5020", bottomFillColor2: "#4caf5020",
    topLineColor: "transparent", bottomLineColor: "transparent",
    lineWidth: 0,
    baseValue: { type: "price", price: lastPrice },
    lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false,
  });

  // Use last 20 bars as visible range for the overlay
  if (currentChartData.length > 1) {
    const recent = currentChartData.slice(-20);
    lossFill.setData(recent.map(d => ({ time: d.time, value: lastPrice })));
    profitFill.setData(recent.map(d => ({ time: d.time, value: tp })));
    rrOverlaySeries.push(lossFill, profitFill);
  }
}

// ── Live Risk Calculator Panel ────────────────────────────────

function updateRiskCalcPanel() {
  const el = document.getElementById("risk-calc-info");
  if (!el) return;
  const sl = getSLPrice();
  const tp = getTPPrice();
  if (!sl || !tp || !lastPrice || lastPrice <= 0) {
    el.textContent = "Set SL/TP to see risk";
    el.style.color = "#555";
    return;
  }
  const isBuy = tp > sl;
  const slDist = isBuy ? lastPrice - sl : sl - lastPrice;
  const tpDist = isBuy ? tp - lastPrice : lastPrice - tp;
  const rr = slDist > 0 ? (tpDist / slDist) : 0;
  const slPct = lastPrice > 0 ? (slDist / lastPrice * 100) : 0;
  const tpPct = lastPrice > 0 ? (tpDist / lastPrice * 100) : 0;

  // Estimate lot size based on current risk mode
  const mode = document.getElementById("order-mode")?.value || "VaR";
  let riskInfo = "";
  if (mode === "Standard") {
    const riskPct = 0.5; // default
    const riskMoney = 100000 * (riskPct / 100); // assume 100K balance
    const lots = slDist > 0 ? (riskMoney / slDist) : 0;
    riskInfo = `~${lots.toFixed(1)} lots ($${riskMoney.toFixed(0)} risk)`;
  } else if (mode === "Fixed") {
    riskInfo = "Fixed lots";
  } else {
    riskInfo = `${mode} mode`;
  }

  el.textContent = "";
  const lines = [
    { text: `${isBuy ? "BUY" : "SELL"} | R:R ${rr.toFixed(2)}`, color: isBuy ? "#4caf50" : "#f44336" },
    { text: `SL: -${slPct.toFixed(2)}% ($${slDist.toFixed(2)})`, color: "#f44" },
    { text: `TP: +${tpPct.toFixed(2)}% ($${tpDist.toFixed(2)})`, color: "#4caf50" },
    { text: riskInfo, color: "#888" },
  ];
  for (const l of lines) {
    const d = document.createElement("div");
    d.style.color = l.color;
    d.textContent = l.text;
    el.appendChild(d);
  }
}

// ── Trade Journal ────────────────────────────────────────────

const JOURNAL_KEY = "typhoon_journal";

function loadJournal() {
  try { return JSON.parse(localStorage.getItem(JOURNAL_KEY) || "[]"); } catch { return []; }
}
function saveJournal(entries) { localStorage.setItem(JOURNAL_KEY, JSON.stringify(entries)); }

function cmdTradeJournal() {
  const win = createWindow({ title: "Trade Journal", width: 600, height: 500 });
  const entries = loadJournal();
  const c = document.createElement("div");
  c.style.cssText = "display:flex;flex-direction:column;height:100%;";

  // Add entry form
  const form = document.createElement("div");
  form.style.cssText = "display:flex;gap:4px;padding:4px;border-bottom:1px solid #333;";
  const symbolInp = document.createElement("input");
  symbolInp.value = currentSymbol || "";
  symbolInp.placeholder = "Symbol";
  symbolInp.style.cssText = "width:70px;background:#111;color:#fff;border:1px solid #555;padding:4px;font-size:10px;font-family:inherit;";
  const sideInp = document.createElement("select");
  sideInp.style.cssText = "background:#111;color:#fff;border:1px solid #555;padding:4px;font-size:10px;";
  for (const v of ["BUY","SELL"]) { const o = document.createElement("option"); o.value = v; o.textContent = v; sideInp.appendChild(o); }
  const noteInp = document.createElement("input");
  noteInp.placeholder = "Notes (setup, emotion, reason)";
  noteInp.style.cssText = "flex:1;background:#111;color:#fff;border:1px solid #555;padding:4px;font-size:10px;font-family:inherit;";
  const addBtn = document.createElement("button");
  addBtn.textContent = "+";
  addBtn.style.cssText = "padding:4px 8px;background:#0a5f38;color:#8f8;border:1px solid #555;cursor:pointer;font-family:inherit;";
  addBtn.addEventListener("click", () => {
    const entry = {
      date: new Date().toISOString().slice(0,16).replace("T"," "),
      symbol: symbolInp.value.trim().toUpperCase() || currentSymbol,
      side: sideInp.value,
      price: lastPrice,
      sl: getSLPrice(),
      tp: getTPPrice(),
      note: noteInp.value.trim(),
    };
    entries.unshift(entry);
    saveJournal(entries);
    noteInp.value = "";
    renderList();
    log(`Journal: ${entry.side} ${entry.symbol} @ $${entry.price}`, "ok");
  });
  form.appendChild(symbolInp);
  form.appendChild(sideInp);
  form.appendChild(noteInp);
  form.appendChild(addBtn);
  c.appendChild(form);

  // Entry list
  const list = document.createElement("div");
  list.style.cssText = "flex:1;overflow-y:auto;font-size:10px;";
  c.appendChild(list);

  function renderList() {
    list.textContent = "";
    for (let i = 0; i < entries.length && i < 100; i++) {
      const e = entries[i];
      const row = document.createElement("div");
      row.style.cssText = "padding:4px 6px;border-bottom:1px solid #111;display:flex;justify-content:space-between;";
      const left = document.createElement("span");
      left.style.color = e.side === "BUY" ? "#4caf50" : "#f44336";
      left.textContent = `${e.date} ${e.side} ${e.symbol} @ $${e.price?.toFixed?.(2) || "?"}`;
      const right = document.createElement("span");
      right.style.color = "#888";
      right.textContent = e.note || "";
      const del = document.createElement("span");
      del.textContent = "×";
      del.style.cssText = "color:#f44;cursor:pointer;margin-left:8px;";
      del.addEventListener("click", () => { entries.splice(i, 1); saveJournal(entries); renderList(); });
      row.appendChild(left);
      row.appendChild(right);
      row.appendChild(del);
      list.appendChild(row);
    }
    if (entries.length === 0) { list.textContent = "No journal entries yet."; list.style.color = "#555"; list.style.padding = "12px"; }
  }
  renderList();

  win.contentElement.textContent = "";
  win.appendElement(c);
}

// ── Heikin-Ashi Calculation ──────────────────────────────────

function calcHeikinAshi(data) {
  if (data.length === 0) return [];
  const result = [];
  let prevHA = { open: data[0].open, close: (data[0].open + data[0].high + data[0].low + data[0].close) / 4 };
  for (let i = 0; i < data.length; i++) {
    const haClose = (data[i].open + data[i].high + data[i].low + data[i].close) / 4;
    const haOpen = i === 0 ? data[i].open : (prevHA.open + prevHA.close) / 2;
    const haHigh = Math.max(data[i].high, haOpen, haClose);
    const haLow = Math.min(data[i].low, haOpen, haClose);
    result.push({ time: data[i].time, open: haOpen, high: haHigh, low: haLow, close: haClose, volume: data[i].volume });
    prevHA = { open: haOpen, close: haClose };
  }
  return result;
}

// ── Position Sizing Calculator ────────────────────────────────

function cmdPositionCalc() {
  const win = createWindow({ title: "Position Sizing Calculator", width: 400, height: 350 });
  const c = document.createElement("div");
  c.style.cssText = "display:flex;flex-direction:column;gap:8px;padding:4px;font-size:11px;";

  const fields = [
    { id: "pc-balance", label: "Account Balance ($)", value: "100000" },
    { id: "pc-risk-pct", label: "Risk per Trade (%)", value: "0.5" },
    { id: "pc-entry", label: "Entry Price", value: lastPrice ? lastPrice.toFixed(2) : "100" },
    { id: "pc-sl", label: "Stop Loss Price", value: getSLPrice()?.toFixed(2) || "95" },
  ];
  const inputs = {};
  for (const f of fields) {
    const row = document.createElement("div");
    row.style.cssText = "display:flex;justify-content:space-between;align-items:center;";
    const lbl = document.createElement("span");
    lbl.style.color = "#888";
    lbl.textContent = f.label;
    const inp = document.createElement("input");
    inp.type = "number";
    inp.step = "0.01";
    inp.value = f.value;
    inp.style.cssText = "width:120px;background:#111;color:#fff;border:1px solid #555;padding:4px;font-family:inherit;text-align:right;";
    row.appendChild(lbl);
    row.appendChild(inp);
    c.appendChild(row);
    inputs[f.id] = inp;
  }

  const resultDiv = document.createElement("div");
  resultDiv.style.cssText = "margin-top:8px;padding:8px;background:#111;border:1px solid #333;border-radius:4px;";
  c.appendChild(resultDiv);

  const calc = () => {
    const balance = parseFloat(inputs["pc-balance"].value) || 0;
    const riskPct = parseFloat(inputs["pc-risk-pct"].value) || 0;
    const entry = parseFloat(inputs["pc-entry"].value) || 0;
    const sl = parseFloat(inputs["pc-sl"].value) || 0;
    const slDist = Math.abs(entry - sl);
    const riskMoney = balance * (riskPct / 100);
    const lots = slDist > 0 ? riskMoney / slDist : 0;
    const tp = getSLPrice() && getTPPrice() ? getTPPrice() : entry + slDist * 2;
    const tpDist = Math.abs(tp - entry);
    const rr = slDist > 0 ? tpDist / slDist : 0;
    resultDiv.textContent = "";
    const lines = [
      `Risk Amount: $${riskMoney.toFixed(2)}`,
      `SL Distance: $${slDist.toFixed(4)}`,
      `Position Size: ${lots.toFixed(2)} shares`,
      `Position Value: $${(lots * entry).toFixed(2)}`,
      `R:R Ratio: ${rr.toFixed(2)}`,
      `Max Loss: $${riskMoney.toFixed(2)}`,
      `Max Profit (${rr.toFixed(1)}R): $${(riskMoney * rr).toFixed(2)}`,
    ];
    for (const line of lines) {
      const d = document.createElement("div");
      d.style.cssText = "padding:2px 0;color:#ccc;";
      d.textContent = line;
      resultDiv.appendChild(d);
    }
  };

  for (const inp of Object.values(inputs)) inp.addEventListener("input", calc);
  calc();

  const calcBtn = document.createElement("button");
  calcBtn.textContent = "Calculate";
  calcBtn.style.cssText = "padding:6px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;font-family:inherit;";
  calcBtn.addEventListener("click", calc);
  c.appendChild(calcBtn);

  win.contentElement.textContent = "";
  win.appendElement(c);
}

// ── Chart Annotations (text labels) ──────────────────────────

let chartAnnotations = [];
const ANNOTATIONS_KEY = "typhoon_annotations";

function loadAnnotations() {
  try { chartAnnotations = JSON.parse(localStorage.getItem(ANNOTATIONS_KEY) || "[]"); } catch { chartAnnotations = []; }
}
function saveAnnotations() { localStorage.setItem(ANNOTATIONS_KEY, JSON.stringify(chartAnnotations)); }

function addChartAnnotation() {
  const text = prompt("Annotation text:");
  if (!text) return;
  if (!currentChartData || currentChartData.length === 0) return;
  // Place at center of visible range, at last price
  const lastBar = currentChartData[currentChartData.length - 1];
  chartAnnotations.push({
    time: lastBar.time,
    price: lastPrice,
    text,
    symbol: currentSymbol,
  });
  saveAnnotations();
  renderAnnotations();
  log(`Annotation added: "${text}"`, "ok");
}

function renderAnnotations() {
  // Annotations rendered on the drawing canvas overlay
  if (!drawCanvas) return;
  // They'll be drawn in the next renderDrawings call — add to drawings array
  // For simplicity, use markers on the candleSeries
  const markers = chartAnnotations
    .filter(a => a.symbol === currentSymbol)
    .map(a => ({
      time: a.time,
      position: "aboveBar",
      color: "#ffeb3b",
      shape: "arrowDown",
      text: a.text,
    }));
  try { candleSeries.setMarkers(markers); } catch (_) {}
}

// ── Regime Detection (trending/ranging/choppy) ───────────────

function detectRegime(data, period = 20) {
  if (!data || data.length < period * 2) return "unknown";
  const recent = data.slice(-period);
  const closes = recent.map(d => d.close);

  // ADX-based: high ADX = trending, low ADX = ranging
  const adxData = calcADX(data, 14);
  if (adxData.adx.length > 0) {
    const lastADX = adxData.adx[adxData.adx.length - 1].value;
    if (lastADX > 25) return "trending";
    if (lastADX < 15) return "ranging";
    return "choppy";
  }

  // Fallback: simple volatility comparison
  const mean = closes.reduce((a, b) => a + b, 0) / closes.length;
  const variance = closes.reduce((a, c) => a + (c - mean) ** 2, 0) / closes.length;
  const cv = Math.sqrt(variance) / mean; // coefficient of variation
  if (cv > 0.03) return "trending";
  if (cv < 0.01) return "ranging";
  return "choppy";
}

function closeMTFGrid() {
  mtfGridActive = false;
  mtfGridSymbol = "";
  mtfActiveCell = null;
  mtfGridOrderLines = [];
  const btn = document.getElementById("btn-mtf-grid");
  btn.textContent = "MTF Grid";

  // Remove grid container
  const grid = document.getElementById("mtf-grid-container");
  if (grid) {
    // Dispose all charts
    for (const cell of mtfGridCells) {
      cell.chart.remove();
      cell.fisherChart.remove();
      cell.volumeChart.remove();
    }
    grid.remove();
  }
  mtfGridCells = [];

  // Show normal chart stack
  document.getElementById("chart-stack").style.display = "";
}

// ── Settings Panel (API Keys) ────────────────────────────────

const SETTINGS_KEY = "typhoon_settings";
function loadSettings() { try { return JSON.parse(localStorage.getItem(SETTINGS_KEY) || "{}"); } catch { return {}; } }
function saveSettings(s) { localStorage.setItem(SETTINGS_KEY, JSON.stringify(s)); }

function cmdSettings() {
  const win = createWindow({ title: "Settings — API Keys", width: 450, height: 380 });
  const settings = loadSettings();
  const c = document.createElement("div");
  c.style.cssText = "display:flex;flex-direction:column;gap:8px;padding:4px;";
  const fields = [
    { key: "fredApiKey", label: "FRED API Key", ph: "32-char from fred.stlouisfed.org" },
    { key: "aiProvider", label: "AI Provider", ph: "anthropic or openai", val: settings.aiProvider || "anthropic" },
    { key: "aiApiKey", label: "AI API Key", ph: "sk-ant-... or sk-..." },
    { key: "aiModel", label: "AI Model (opt)", ph: "claude-haiku-4-5-20251001 / gpt-4o-mini" },
  ];
  const inputs = {};
  for (const f of fields) {
    const lbl = document.createElement("label");
    lbl.textContent = f.label;
    lbl.style.cssText = "display:block;color:#888;font-size:10px;margin-bottom:1px;";
    const inp = document.createElement("input");
    inp.type = f.key.includes("Key") ? "password" : "text";
    inp.placeholder = f.ph;
    inp.value = f.val || settings[f.key] || "";
    inp.style.cssText = "width:100%;background:#111;color:#fff;border:1px solid #555;padding:5px;font-family:inherit;font-size:11px;box-sizing:border-box;";
    c.appendChild(lbl); c.appendChild(inp);
    inputs[f.key] = inp;
  }
  const btn = document.createElement("button");
  btn.textContent = "Save Settings";
  btn.style.cssText = "padding:8px;background:#0a5f38;color:#8f8;border:1px solid #4caf50;cursor:pointer;font-family:inherit;font-weight:bold;";
  btn.addEventListener("click", () => {
    const s = {}; for (const [k, inp] of Object.entries(inputs)) s[k] = inp.value.trim();
    saveSettings(s); log("Settings saved", "ok"); win.close();
  });
  c.appendChild(btn);
  const note = document.createElement("div");
  note.style.cssText = "font-size:9px;color:#555;";
  note.textContent = "See API_KEYS.md for free signup links.";
  c.appendChild(note);
  win.contentElement.textContent = ""; win.appendElement(c);
}

// ── FRED Economic Data ───────────────────────────────────────

function cmdFRED() {
  const s = loadSettings();
  if (!s.fredApiKey) { alert("FRED API key required. Ctrl+K → SETTINGS.\nFree: https://fred.stlouisfed.org/docs/api/api_key.html"); return; }
  const win = createWindow({ title: "FRED Economic Data", width: 500, height: 400 });
  const presets = [
    ["DFF","Fed Funds"],["CPIAUCSL","CPI"],["UNRATE","Unemployment"],["GDP","GDP"],
    ["DGS10","10Y Treasury"],["DGS2","2Y Treasury"],["T10Y2Y","10Y-2Y Spread"],["VIXCLS","VIX"],["M2SL","M2 Supply"],
  ];
  const c = document.createElement("div"); c.style.cssText = "display:flex;flex-direction:column;gap:6px;padding:4px;";
  const row = document.createElement("div"); row.style.cssText = "display:flex;gap:3px;flex-wrap:wrap;";
  for (const [id,name] of presets) {
    const b = document.createElement("button");
    b.textContent = name; b.style.cssText = "padding:2px 5px;background:#1a3a5a;color:#8cf;border:1px solid #555;cursor:pointer;font-size:9px;font-family:inherit;border-radius:2px;";
    b.addEventListener("click", async () => {
      win.setTitle(`FRED — ${name}...`);
      try {
        const json = await invoke("fetch_fred_series", { seriesId: id, apiKey: s.fredApiKey, limit: 50 });
        const data = JSON.parse(json); const obs = data.observations || [];
        win.setTitle(`FRED — ${name} (${obs.length})`);
        const tbl = document.createElement("table"); tbl.className = "fw-table"; tbl.style.fontSize = "10px";
        for (const o of obs) { const tr = document.createElement("tr"); const td1 = document.createElement("td"); td1.className = "fw-label"; td1.textContent = o.date; const td2 = document.createElement("td"); td2.className = "fw-value"; td2.textContent = o.value; tr.appendChild(td1); tr.appendChild(td2); tbl.appendChild(tr); }
        dataDiv.textContent = ""; dataDiv.appendChild(tbl);
      } catch (e) { win.setTitle(`FRED — ${e}`); }
    });
    row.appendChild(b);
  }
  c.appendChild(row);
  const dataDiv = document.createElement("div"); dataDiv.style.cssText = "overflow-y:auto;max-height:320px;";
  c.appendChild(dataDiv);
  win.contentElement.textContent = ""; win.appendElement(c);
}

// ── AI Chat ──────────────────────────────────────────────────

function cmdAIChat() {
  const s = loadSettings();
  if (!s.aiApiKey) { alert("AI API key required. Ctrl+K → SETTINGS.\nAnthropic: https://console.anthropic.com/"); return; }
  const win = createWindow({ title: "AI Trading Assistant", width: 500, height: 420 });
  const c = document.createElement("div"); c.style.cssText = "display:flex;flex-direction:column;height:100%;";
  const msgs = document.createElement("div"); msgs.style.cssText = "flex:1;overflow-y:auto;padding:4px;font-size:11px;line-height:1.5;";
  c.appendChild(msgs);
  const addMsg = (role, text) => {
    const d = document.createElement("div");
    d.style.cssText = `margin:4px 0;padding:6px 8px;border-radius:4px;white-space:pre-wrap;${role === "user" ? "background:#1a3a5a;color:#8cf;text-align:right;" : "background:#111;color:#ccc;"}`;
    d.textContent = text; msgs.appendChild(d); msgs.scrollTop = msgs.scrollHeight;
  };
  addMsg("assistant", `Connected to ${s.aiProvider || "anthropic"}. Ask about positions, risk, or market analysis.`);
  const ir = document.createElement("div"); ir.style.cssText = "display:flex;gap:4px;padding:4px 0;";
  const inp = document.createElement("input"); inp.placeholder = "Ask about market, positions, risk...";
  inp.style.cssText = "flex:1;background:#111;color:#fff;border:1px solid #555;padding:6px;font-family:inherit;font-size:11px;";
  const sendBtn = document.createElement("button"); sendBtn.textContent = "Send";
  sendBtn.style.cssText = "padding:6px 12px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;font-family:inherit;";
  const send = async () => {
    const msg = inp.value.trim(); if (!msg) return; inp.value = "";
    addMsg("user", msg); addMsg("assistant", "Thinking...");
    try {
      const ctx = `Symbol: ${currentSymbol}, TF: ${currentTimeframe}, Price: $${lastPrice}, SL: ${getSLPrice() || "none"}, TP: ${getTPPrice() || "none"}`;
      const reply = await invoke("ai_chat", { apiKey: s.aiApiKey, provider: s.aiProvider || "anthropic", model: s.aiModel || "", message: msg, context: ctx });
      msgs.lastChild.textContent = reply;
    } catch (e) { msgs.lastChild.textContent = `Error: ${e}`; msgs.lastChild.style.color = "#f44"; }
  };
  sendBtn.addEventListener("click", send);
  inp.addEventListener("keydown", (e) => { if (e.key === "Enter") send(); });
  ir.appendChild(inp); ir.appendChild(sendBtn); c.appendChild(ir);
  win.contentElement.textContent = ""; win.appendElement(c);
}

// ══════════════════════════════════════════════════════════════
// FEATURE: Multi-Symbol Alert Dashboard (ALERTBOARD)
// ══════════════════════════════════════════════════════════════

async function cmdAlertBoard() {
  const win = createWindow({ title: "Alert Dashboard", width: 650, height: 450 });
  win.contentElement.textContent = "";

  const watchlist = getWatchlist();
  if (watchlist.length === 0) {
    win.setContent("No watchlist symbols. Add symbols via QM (Quote Monitor) first.");
    return;
  }

  const allAlerts = [...priceAlerts, ...multiConditionAlerts];
  if (allAlerts.length === 0) {
    win.setContent("No alerts configured. Use ALERTS to set price or indicator alerts first.");
    return;
  }

  const loading = document.createElement("div");
  loading.textContent = "Checking alerts across watchlist...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  const table = document.createElement("table");
  table.className = "fw-table";
  table.style.cssText = "width:100%;font-size:10px;border-collapse:collapse;";
  const thead = document.createElement("tr");
  for (const h of ["Symbol", "Price", "Alert Level", "Direction", "Status"]) {
    const th = document.createElement("td");
    th.style.cssText = "color:#888;font-weight:bold;font-size:10px;padding:4px 6px;border-bottom:1px solid #333;";
    th.textContent = h;
    thead.appendChild(th);
  }
  table.appendChild(thead);

  for (const sym of watchlist) {
    let price = 0;
    try {
      const qJson = await invoke("get_latest_quote", { symbol: sym });
      const q = typeof qJson === "string" ? JSON.parse(qJson) : qJson;
      price = q.last || q.price || q.close || q.ap || 0;
    } catch (_) {}

    const symbolAlerts = allAlerts.filter(a => a.symbol === sym);
    if (symbolAlerts.length === 0) {
      // Show symbol with no alerts
      const tr = document.createElement("tr");
      const tdSym = document.createElement("td"); tdSym.textContent = sym; tdSym.style.cssText = "padding:4px 6px;color:#8cf;cursor:pointer;";
      tdSym.addEventListener("click", () => { document.getElementById("symbol-input").value = sym; triggerLoad(); });
      const tdPrice = document.createElement("td"); tdPrice.textContent = price ? `$${price.toFixed(2)}` : "N/A"; tdPrice.style.cssText = "padding:4px 6px;color:#ccc;";
      const tdLevel = document.createElement("td"); tdLevel.textContent = "—"; tdLevel.style.cssText = "padding:4px 6px;color:#555;";
      const tdDir = document.createElement("td"); tdDir.textContent = "—"; tdDir.style.cssText = "padding:4px 6px;color:#555;";
      const tdStatus = document.createElement("td"); tdStatus.textContent = "No alerts"; tdStatus.style.cssText = "padding:4px 6px;color:#555;";
      tr.appendChild(tdSym); tr.appendChild(tdPrice); tr.appendChild(tdLevel); tr.appendChild(tdDir); tr.appendChild(tdStatus);
      table.appendChild(tr);
      continue;
    }

    for (const a of symbolAlerts) {
      const tr = document.createElement("tr");
      const tdSym = document.createElement("td"); tdSym.textContent = sym; tdSym.style.cssText = "padding:4px 6px;color:#8cf;cursor:pointer;";
      tdSym.addEventListener("click", () => { document.getElementById("symbol-input").value = sym; triggerLoad(); });
      const tdPrice = document.createElement("td"); tdPrice.textContent = price ? `$${price.toFixed(2)}` : "N/A"; tdPrice.style.cssText = "padding:4px 6px;color:#ccc;";

      let alertLevel = "—";
      let direction = "—";
      let triggered = a.triggered || false;

      if (a.price !== undefined) {
        // Price alert
        alertLevel = `$${a.price.toFixed(2)}`;
        direction = a.direction || "above";
        if (price > 0) {
          if (a.direction === "above" && price >= a.price) triggered = true;
          if (a.direction === "below" && price <= a.price) triggered = true;
        }
      } else if (a.condition) {
        // Multi-condition alert
        alertLevel = a.condition;
        direction = "indicator";
      }

      const tdLevel = document.createElement("td"); tdLevel.textContent = alertLevel; tdLevel.style.cssText = "padding:4px 6px;color:#ffeb3b;";
      const tdDir = document.createElement("td"); tdDir.textContent = direction; tdDir.style.cssText = "padding:4px 6px;color:#888;";
      const tdStatus = document.createElement("td");
      tdStatus.textContent = triggered ? "TRIGGERED" : "Watching";
      tdStatus.style.cssText = `padding:4px 6px;font-weight:bold;color:${triggered ? "#f44336" : "#4caf50"};`;
      tr.appendChild(tdSym); tr.appendChild(tdPrice); tr.appendChild(tdLevel); tr.appendChild(tdDir); tr.appendChild(tdStatus);
      table.appendChild(tr);
    }
  }

  win.contentElement.textContent = "";
  const refreshBtn = document.createElement("button");
  refreshBtn.textContent = "Refresh";
  refreshBtn.style.cssText = "margin:4px;padding:4px 12px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;font-family:inherit;font-size:10px;";
  refreshBtn.addEventListener("click", () => { win.close(); cmdAlertBoard(); });
  win.appendElement(refreshBtn);
  win.appendElement(table);
}

// ══════════════════════════════════════════════════════════════
// FEATURE: AI-Powered Trade Review (extends cmdAIChat)
// ══════════════════════════════════════════════════════════════

// Extend the AI chat window — override cmdAIChat to add "Review My Trades" button
const _originalCmdAIChat = cmdAIChat;

// We redefine cmdAIChat below; the original is captured above
function cmdAIChatWithReview() {
  const s = loadSettings();
  if (!s.aiApiKey) { alert("AI API key required. Ctrl+K -> SETTINGS.\nAnthropic: https://console.anthropic.com/"); return; }
  const win = createWindow({ title: "AI Trading Assistant", width: 500, height: 460 });
  const c = document.createElement("div"); c.style.cssText = "display:flex;flex-direction:column;height:100%;";
  const msgs = document.createElement("div"); msgs.style.cssText = "flex:1;overflow-y:auto;padding:4px;font-size:11px;line-height:1.5;";
  c.appendChild(msgs);
  const addMsg = (role, text) => {
    const d = document.createElement("div");
    d.style.cssText = `margin:4px 0;padding:6px 8px;border-radius:4px;white-space:pre-wrap;${role === "user" ? "background:#1a3a5a;color:#8cf;text-align:right;" : "background:#111;color:#ccc;"}`;
    d.textContent = text; msgs.appendChild(d); msgs.scrollTop = msgs.scrollHeight;
  };
  addMsg("assistant", `Connected to ${s.aiProvider || "anthropic"}. Ask about positions, risk, or market analysis.`);

  // "Review My Trades" button
  const reviewBtn = document.createElement("button");
  reviewBtn.textContent = "Review My Trades";
  reviewBtn.style.cssText = "margin:4px;padding:6px 12px;background:#3a0f60;color:#c8f;border:1px solid #555;cursor:pointer;font-family:inherit;font-size:11px;";
  reviewBtn.addEventListener("click", async () => {
    const journal = loadJournal();
    const backtestTrades = window._lastBacktestTrades || [];

    let tradeList = [];
    if (journal.length > 0) {
      const recent = journal.slice(0, 20);
      tradeList = recent.map(e => `${e.date} ${e.side} ${e.symbol} @ $${e.price?.toFixed?.(2) || "?"} SL:${e.sl || "?"} TP:${e.tp || "?"} Note: ${e.note || "none"}`);
    }
    if (backtestTrades.length > 0) {
      const btRecent = backtestTrades.slice(-20);
      for (const t of btRecent) {
        const pnl = t.pnl || t.profit || 0;
        tradeList.push(`[Backtest] ${t.side || "?"} ${t.symbol || currentSymbol} entry:${t.entry?.toFixed?.(2) || "?"} exit:${t.exit?.toFixed?.(2) || "?"} PnL: $${pnl.toFixed(2)}`);
      }
    }

    if (tradeList.length === 0) {
      addMsg("assistant", "No trades found. Use the Trade Journal (JOURNAL) to log trades, or run a backtest first.");
      return;
    }

    const prompt = `Here are my last ${tradeList.length} trades:\n${tradeList.join("\n")}\n\nAnalyze my trading patterns, strengths, and weaknesses. Identify any recurring mistakes, risk management issues, and suggest improvements.`;
    addMsg("user", `Review My Trades (${tradeList.length} trades)`);
    addMsg("assistant", "Analyzing your trade history...");

    try {
      const ctx = `Symbol: ${currentSymbol}, TF: ${currentTimeframe}, Price: $${lastPrice}`;
      const reply = await invoke("ai_chat", { apiKey: s.aiApiKey, provider: s.aiProvider || "anthropic", model: s.aiModel || "", message: prompt, context: ctx });
      msgs.lastChild.textContent = reply;
    } catch (e) { msgs.lastChild.textContent = `Error: ${e}`; msgs.lastChild.style.color = "#f44"; }
  });
  c.appendChild(reviewBtn);

  const ir = document.createElement("div"); ir.style.cssText = "display:flex;gap:4px;padding:4px 0;";
  const inp = document.createElement("input"); inp.placeholder = "Ask about market, positions, risk...";
  inp.style.cssText = "flex:1;background:#111;color:#fff;border:1px solid #555;padding:6px;font-family:inherit;font-size:11px;";
  const sendBtn = document.createElement("button"); sendBtn.textContent = "Send";
  sendBtn.style.cssText = "padding:6px 12px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;font-family:inherit;";
  const send = async () => {
    const msg = inp.value.trim(); if (!msg) return; inp.value = "";
    addMsg("user", msg); addMsg("assistant", "Thinking...");
    try {
      const ctx = `Symbol: ${currentSymbol}, TF: ${currentTimeframe}, Price: $${lastPrice}, SL: ${getSLPrice() || "none"}, TP: ${getTPPrice() || "none"}`;
      const reply = await invoke("ai_chat", { apiKey: s.aiApiKey, provider: s.aiProvider || "anthropic", model: s.aiModel || "", message: msg, context: ctx });
      msgs.lastChild.textContent = reply;
    } catch (e) { msgs.lastChild.textContent = `Error: ${e}`; msgs.lastChild.style.color = "#f44"; }
  };
  sendBtn.addEventListener("click", send);
  inp.addEventListener("keydown", (e) => { if (e.key === "Enter") send(); });
  ir.appendChild(inp); ir.appendChild(sendBtn); c.appendChild(ir);
  win.contentElement.textContent = ""; win.appendElement(c);
}

// ══════════════════════════════════════════════════════════════
// FEATURE: Pattern Recognition (Double Top/Bottom, Head & Shoulders)
// ══════════════════════════════════════════════════════════════

function detectPatterns(data) {
  const patterns = [];
  if (!data || data.length < 20) return patterns;

  const TOLERANCE = 0.02; // 2%

  // Find local highs and lows (window of 5 bars)
  const localHighs = [];
  const localLows = [];
  for (let i = 2; i < data.length - 2; i++) {
    if (data[i].high >= data[i-1].high && data[i].high >= data[i-2].high &&
        data[i].high >= data[i+1].high && data[i].high >= data[i+2].high) {
      localHighs.push({ idx: i, price: data[i].high });
    }
    if (data[i].low <= data[i-1].low && data[i].low <= data[i-2].low &&
        data[i].low <= data[i+1].low && data[i].low <= data[i+2].low) {
      localLows.push({ idx: i, price: data[i].low });
    }
  }

  // Double Top: two highs within 2% with a valley between
  for (let i = 0; i < localHighs.length - 1; i++) {
    const h1 = localHighs[i];
    for (let j = i + 1; j < localHighs.length; j++) {
      const h2 = localHighs[j];
      if (h2.idx - h1.idx < 5 || h2.idx - h1.idx > 80) continue;
      const diff = Math.abs(h1.price - h2.price) / Math.max(h1.price, h2.price);
      if (diff > TOLERANCE) continue;
      // Check for a valley between them
      let minBetween = Infinity;
      for (let k = h1.idx + 1; k < h2.idx; k++) {
        if (data[k].low < minBetween) minBetween = data[k].low;
      }
      const avgHigh = (h1.price + h2.price) / 2;
      if (minBetween < avgHigh * 0.97) {
        patterns.push({ type: "Double Top", startIdx: h1.idx, endIdx: h2.idx, price: avgHigh });
        break;
      }
    }
  }

  // Double Bottom: two lows within 2% with a peak between
  for (let i = 0; i < localLows.length - 1; i++) {
    const l1 = localLows[i];
    for (let j = i + 1; j < localLows.length; j++) {
      const l2 = localLows[j];
      if (l2.idx - l1.idx < 5 || l2.idx - l1.idx > 80) continue;
      const diff = Math.abs(l1.price - l2.price) / Math.max(l1.price, l2.price);
      if (diff > TOLERANCE) continue;
      // Check for a peak between them
      let maxBetween = -Infinity;
      for (let k = l1.idx + 1; k < l2.idx; k++) {
        if (data[k].high > maxBetween) maxBetween = data[k].high;
      }
      const avgLow = (l1.price + l2.price) / 2;
      if (maxBetween > avgLow * 1.03) {
        patterns.push({ type: "Double Bottom", startIdx: l1.idx, endIdx: l2.idx, price: avgLow });
        break;
      }
    }
  }

  // Head & Shoulders: higher high flanked by two lower highs
  for (let i = 0; i < localHighs.length - 2; i++) {
    const left = localHighs[i];
    for (let j = i + 1; j < localHighs.length - 1; j++) {
      const head = localHighs[j];
      if (head.price <= left.price) continue;
      for (let k = j + 1; k < localHighs.length; k++) {
        const right = localHighs[k];
        if (right.idx - left.idx > 120) break;
        if (right.price >= head.price) continue;
        // Left and right shoulders should be roughly similar height (within 5%)
        const shoulderDiff = Math.abs(left.price - right.price) / Math.max(left.price, right.price);
        if (shoulderDiff > 0.05) continue;
        // Head must be notably higher than shoulders
        if (head.price < left.price * 1.02) continue;
        patterns.push({ type: "Head & Shoulders", startIdx: left.idx, endIdx: right.idx, price: head.price });
        break;
      }
      if (patterns.some(p => p.type === "Head & Shoulders" && p.startIdx === left.idx)) break;
    }
  }

  return patterns;
}

function cmdPatterns() {
  if (!currentChartData || currentChartData.length < 20) {
    log("Not enough data for pattern detection", "warn");
    return;
  }

  const patterns = detectPatterns(currentChartData);

  if (patterns.length === 0) {
    log("No patterns detected in current chart data", "info");
    const win = createWindow({ title: `${currentSymbol} — Patterns`, width: 350, height: 200 });
    win.setContent("No Double Top, Double Bottom, or Head & Shoulders patterns detected in the current data.");
    return;
  }

  // Draw markers on chart
  const markers = [];
  const colorMap = {
    "Double Top": "#f44336",
    "Double Bottom": "#4caf50",
    "Head & Shoulders": "#ff9800",
  };
  const shapeMap = {
    "Double Top": "arrowDown",
    "Double Bottom": "arrowUp",
    "Head & Shoulders": "arrowDown",
  };

  for (const p of patterns) {
    markers.push({
      time: currentChartData[p.startIdx].time,
      position: p.type === "Double Bottom" ? "belowBar" : "aboveBar",
      color: colorMap[p.type] || "#fff",
      shape: shapeMap[p.type] || "circle",
      text: p.type,
    });
    if (p.endIdx !== p.startIdx) {
      markers.push({
        time: currentChartData[p.endIdx].time,
        position: p.type === "Double Bottom" ? "belowBar" : "aboveBar",
        color: colorMap[p.type] || "#fff",
        shape: shapeMap[p.type] || "circle",
        text: p.type,
      });
    }
  }

  // Sort markers by time (required by lightweight-charts)
  markers.sort((a, b) => a.time - b.time);
  candleSeries.setMarkers(markers);

  // Show summary window
  const win = createWindow({ title: `${currentSymbol} — Patterns (${patterns.length})`, width: 450, height: 300 });
  win.contentElement.textContent = "";
  const table = document.createElement("table");
  table.className = "fw-table";
  table.style.cssText = "width:100%;font-size:10px;border-collapse:collapse;";
  const thead = document.createElement("tr");
  for (const h of ["Pattern", "Start Bar", "End Bar", "Price Level"]) {
    const th = document.createElement("td");
    th.style.cssText = "color:#888;font-weight:bold;font-size:10px;padding:4px 6px;border-bottom:1px solid #333;";
    th.textContent = h;
    thead.appendChild(th);
  }
  table.appendChild(thead);

  for (const p of patterns) {
    const tr = document.createElement("tr");
    const tdType = document.createElement("td");
    tdType.textContent = p.type;
    tdType.style.cssText = `padding:4px 6px;color:${colorMap[p.type] || "#ccc"};font-weight:bold;`;
    const tdStart = document.createElement("td");
    tdStart.textContent = new Date(currentChartData[p.startIdx].time * 1000).toLocaleDateString();
    tdStart.style.cssText = "padding:4px 6px;color:#ccc;";
    const tdEnd = document.createElement("td");
    tdEnd.textContent = new Date(currentChartData[p.endIdx].time * 1000).toLocaleDateString();
    tdEnd.style.cssText = "padding:4px 6px;color:#ccc;";
    const tdPrice = document.createElement("td");
    tdPrice.textContent = `$${p.price.toFixed(2)}`;
    tdPrice.style.cssText = "padding:4px 6px;color:#ffeb3b;";
    tr.appendChild(tdType); tr.appendChild(tdStart); tr.appendChild(tdEnd); tr.appendChild(tdPrice);
    table.appendChild(tr);
  }

  win.appendElement(table);

  const clearBtn = document.createElement("button");
  clearBtn.textContent = "Clear Markers";
  clearBtn.style.cssText = "margin:8px 4px;padding:4px 12px;background:#3a0a0a;color:#f44;border:1px solid #555;cursor:pointer;font-family:inherit;font-size:10px;";
  clearBtn.addEventListener("click", () => { candleSeries.setMarkers([]); log("Pattern markers cleared", "info"); });
  win.appendElement(clearBtn);

  log(`Detected ${patterns.length} pattern(s) on ${currentSymbol}`, "ok");
}

// ══════════════════════════════════════════════════════════════
// FEATURE: Sentiment Analysis
// ══════════════════════════════════════════════════════════════

async function cmdSentiment() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Sentiment Analysis`, width: 550, height: 450 });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Fetching news for sentiment analysis...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  const POSITIVE_WORDS = ["surge", "rally", "gain", "beat", "upgrade", "bullish", "record", "soar"];
  const NEGATIVE_WORDS = ["crash", "drop", "miss", "downgrade", "bearish", "plunge", "sell-off", "decline"];

  try {
    const json = await invoke("get_news", { symbol: currentSymbol, limit: 30 });
    const articles = JSON.parse(json);
    win.contentElement.textContent = "";

    if (!articles || articles.length === 0) {
      win.setContent("No news available for sentiment analysis.");
      return;
    }

    let bullScore = 0;
    let bearScore = 0;
    const analyzed = [];

    for (const article of articles) {
      const headline = (article.headline || article.title || "").toLowerCase();
      let artBull = 0;
      let artBear = 0;
      for (const w of POSITIVE_WORDS) {
        if (headline.includes(w)) artBull++;
      }
      for (const w of NEGATIVE_WORDS) {
        if (headline.includes(w)) artBear++;
      }
      bullScore += artBull;
      bearScore += artBear;
      analyzed.push({
        headline: article.headline || article.title || "",
        date: (article.created_at || "").substring(0, 16).replace("T", " "),
        source: article.source || "",
        bull: artBull,
        bear: artBear,
      });
    }

    const totalSignals = bullScore + bearScore;
    const bullPct = totalSignals > 0 ? Math.round((bullScore / totalSignals) * 100) : 50;
    const bearPct = totalSignals > 0 ? Math.round((bearScore / totalSignals) * 100) : 50;

    // Summary header
    const summary = document.createElement("div");
    summary.style.cssText = "padding:12px;border-bottom:1px solid #333;";

    const meterBg = document.createElement("div");
    meterBg.style.cssText = "height:24px;background:#3a0a0a;border-radius:4px;overflow:hidden;margin:8px 0;";
    const meterFill = document.createElement("div");
    meterFill.style.cssText = `height:100%;width:${bullPct}%;background:linear-gradient(90deg, #4caf50, #8bc34a);border-radius:4px 0 0 4px;transition:width 0.3s;`;
    meterBg.appendChild(meterFill);

    const overallSentiment = bullScore > bearScore ? "BULLISH" : bullScore < bearScore ? "BEARISH" : "NEUTRAL";
    const overallColor = bullScore > bearScore ? "#4caf50" : bullScore < bearScore ? "#f44336" : "#888";

    const sentLabel = document.createElement("div");
    sentLabel.style.cssText = `font-size:14px;font-weight:bold;color:${overallColor};text-align:center;`;
    sentLabel.textContent = overallSentiment;
    summary.appendChild(sentLabel);
    summary.appendChild(meterBg);
    const sentRow = document.createElement("div");
    sentRow.style.cssText = "display:flex;justify-content:space-between;font-size:11px;margin-top:4px;";
    const bullSpan = document.createElement("span"); bullSpan.style.color = "#4caf50"; bullSpan.textContent = `Bullish: ${bullScore} (${bullPct}%)`;
    const bearSpan = document.createElement("span"); bearSpan.style.color = "#f44336"; bearSpan.textContent = `Bearish: ${bearScore} (${bearPct}%)`;
    sentRow.appendChild(bullSpan); sentRow.appendChild(bearSpan);
    summary.appendChild(sentRow);
    win.appendElement(summary);

    // Headline list
    const listDiv = document.createElement("div");
    listDiv.style.cssText = "overflow-y:auto;max-height:300px;";
    for (const a of analyzed) {
      const row = document.createElement("div");
      row.style.cssText = "padding:4px 8px;border-bottom:1px solid #111;font-size:10px;";
      let hlColor = "#888";
      if (a.bull > 0 && a.bear === 0) hlColor = "#4caf50";
      else if (a.bear > 0 && a.bull === 0) hlColor = "#f44336";
      else if (a.bull > 0 && a.bear > 0) hlColor = "#ff9800";

      const dateSpan = document.createElement("div");
      dateSpan.style.cssText = "color:#555;font-size:9px;";
      dateSpan.textContent = `${a.date} | ${a.source}`;
      const hlSpan = document.createElement("div");
      hlSpan.style.cssText = `color:${hlColor};margin-top:2px;`;
      hlSpan.textContent = a.headline;
      row.appendChild(dateSpan);
      row.appendChild(hlSpan);
      listDiv.appendChild(row);
    }
    win.appendElement(listDiv);

    log(`Sentiment for ${currentSymbol}: ${overallSentiment} (Bull:${bullScore} Bear:${bearScore})`, "ok");
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Sentiment analysis failed: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// FEATURE: Volatility Surface (Options IV Grid)
// ══════════════════════════════════════════════════════════════

async function cmdVolSurf() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Volatility Surface`, width: 700, height: 500 });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Fetching options chain for volatility surface...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  // Calculate next 3 monthly expiration dates (3rd Friday of next 3 months)
  function getMonthlyExpirations(count) {
    const expirations = [];
    const now = new Date();
    let month = now.getMonth();
    let year = now.getFullYear();
    for (let i = 0; i < count + 1; i++) {
      if (i > 0 || now.getDate() > 20) {
        month++;
        if (month > 11) { month = 0; year++; }
      }
      // 3rd Friday: find first Friday, add 14 days
      const firstDay = new Date(year, month, 1).getDay();
      const firstFriday = firstDay <= 5 ? (5 - firstDay + 1) : (12 - firstDay + 1);
      const thirdFriday = firstFriday + 14;
      const expDate = new Date(year, month, thirdFriday);
      if (expDate > now) {
        expirations.push(expDate.toISOString().slice(0, 10));
      }
      if (expirations.length >= count) break;
    }
    return expirations;
  }

  const expirations = getMonthlyExpirations(3);

  try {
    const allData = {};
    const allStrikes = new Set();

    for (const exp of expirations) {
      try {
        const json = await invoke("get_options", { symbol: currentSymbol, expiration: exp });
        const chain = typeof json === "string" ? JSON.parse(json) : json;
        const options = Array.isArray(chain) ? chain : (chain.options || chain.calls || []);
        allData[exp] = {};
        for (const opt of options) {
          const strike = opt.strike || opt.strike_price;
          const iv = opt.implied_volatility || opt.iv || opt.impliedVolatility || 0;
          if (strike && iv > 0) {
            allStrikes.add(strike);
            allData[exp][strike] = iv;
          }
        }
      } catch (_) {
        allData[exp] = {};
      }
    }

    win.contentElement.textContent = "";

    const sortedStrikes = [...allStrikes].sort((a, b) => a - b);

    if (sortedStrikes.length === 0) {
      win.setContent("No options data available for volatility surface. This symbol may not have listed options.");
      return;
    }

    // Filter strikes to +/- 20% of current price
    const priceLow = lastPrice * 0.8;
    const priceHigh = lastPrice * 1.2;
    const filteredStrikes = sortedStrikes.filter(s => s >= priceLow && s <= priceHigh);
    const displayStrikes = filteredStrikes.length > 0 ? filteredStrikes : sortedStrikes.slice(0, 30);

    // Build table
    const table = document.createElement("table");
    table.className = "fw-table";
    table.style.cssText = "width:100%;font-size:10px;border-collapse:collapse;";

    // Header row
    const thead = document.createElement("tr");
    const thStrike = document.createElement("td");
    thStrike.textContent = "Strike";
    thStrike.style.cssText = "color:#888;font-weight:bold;padding:4px 6px;border-bottom:1px solid #333;position:sticky;left:0;background:#0a0a1a;";
    thead.appendChild(thStrike);
    for (const exp of expirations) {
      const th = document.createElement("td");
      th.textContent = exp;
      th.style.cssText = "color:#888;font-weight:bold;padding:4px 6px;border-bottom:1px solid #333;text-align:center;";
      thead.appendChild(th);
    }
    table.appendChild(thead);

    // Find IV range for color coding
    let minIV = Infinity, maxIV = 0;
    for (const exp of expirations) {
      for (const s of displayStrikes) {
        const iv = allData[exp]?.[s];
        if (iv > 0) {
          minIV = Math.min(minIV, iv);
          maxIV = Math.max(maxIV, iv);
        }
      }
    }
    if (minIV === Infinity) minIV = 0;

    // Data rows
    for (const strike of displayStrikes) {
      const tr = document.createElement("tr");
      const tdStrike = document.createElement("td");
      tdStrike.textContent = `$${strike}`;
      tdStrike.style.cssText = `padding:3px 6px;color:${Math.abs(strike - lastPrice) / lastPrice < 0.02 ? "#ffeb3b" : "#ccc"};font-weight:bold;position:sticky;left:0;background:#0a0a1a;`;
      tr.appendChild(tdStrike);

      for (const exp of expirations) {
        const td = document.createElement("td");
        const iv = allData[exp]?.[strike];
        if (iv > 0) {
          const ivPct = (iv * 100).toFixed(1);
          td.textContent = `${ivPct}%`;
          // Color: green (low) -> yellow (mid) -> red (high)
          const ratio = maxIV > minIV ? (iv - minIV) / (maxIV - minIV) : 0.5;
          let r, g;
          if (ratio < 0.5) {
            r = Math.round(ratio * 2 * 255);
            g = 255;
          } else {
            r = 255;
            g = Math.round((1 - (ratio - 0.5) * 2) * 255);
          }
          td.style.cssText = `padding:3px 6px;text-align:center;color:rgb(${r},${g},0);background:rgba(${r},${g},0,0.08);`;
        } else {
          td.textContent = "—";
          td.style.cssText = "padding:3px 6px;text-align:center;color:#333;";
        }
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }

    const wrapper = document.createElement("div");
    wrapper.style.cssText = "overflow:auto;max-height:420px;";
    wrapper.appendChild(table);
    win.appendElement(wrapper);

    log(`Vol surface: ${displayStrikes.length} strikes x ${expirations.length} expirations`, "ok");
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Volatility surface failed: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// FEATURE: Put/Call Ratio Dashboard
// ══════════════════════════════════════════════════════════════

async function cmdPCRatio() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Put/Call Ratio`, width: 500, height: 400 });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Fetching options chain for put/call ratio...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  function getNextExpiry() {
    const now = new Date();
    let month = now.getMonth();
    let year = now.getFullYear();
    for (let i = 0; i < 2; i++) {
      const firstDay = new Date(year, month, 1).getDay();
      const firstFriday = firstDay <= 5 ? (5 - firstDay + 1) : (12 - firstDay + 1);
      const thirdFriday = firstFriday + 14;
      const expDate = new Date(year, month, thirdFriday);
      if (expDate > now) return expDate.toISOString().slice(0, 10);
      month++;
      if (month > 11) { month = 0; year++; }
    }
    return now.toISOString().slice(0, 10);
  }

  try {
    const expiry = getNextExpiry();
    const json = await invoke("get_options", { symbol: currentSymbol, expiry });
    const chain = typeof json === "string" ? JSON.parse(json) : json;
    const options = Array.isArray(chain) ? chain : (chain.options || []);
    win.contentElement.textContent = "";

    if (!options || options.length === 0) {
      win.setContent("No options data available. This symbol may not have listed options.");
      return;
    }

    let callVolume = 0, putVolume = 0, callOI = 0, putOI = 0;
    for (const opt of options) {
      const vol = opt.volume || 0;
      const oi = opt.open_interest || opt.openInterest || 0;
      if (opt.option_type === "call") { callVolume += vol; callOI += oi; }
      else if (opt.option_type === "put") { putVolume += vol; putOI += oi; }
    }

    const volRatio = callVolume > 0 ? putVolume / callVolume : 0;
    const oiRatio = callOI > 0 ? putOI / callOI : 0;

    function classify(ratio) {
      if (ratio > 1.0) return { label: "BEARISH", color: "#f44336" };
      if (ratio < 0.7) return { label: "BULLISH", color: "#4caf50" };
      return { label: "NEUTRAL", color: "#ff9800" };
    }

    const volSent = classify(volRatio);
    const oiSent = classify(oiRatio);

    const header = document.createElement("div");
    header.style.cssText = "padding:12px;border-bottom:1px solid #333;text-align:center;";
    header.innerHTML = `<div style="color:#888;font-size:11px;margin-bottom:4px;">Expiry: ${expiry}</div>`;
    win.appendElement(header);

    const table = document.createElement("table");
    table.style.cssText = "width:100%;border-collapse:collapse;font-size:12px;margin-top:8px;";

    const rows = [
      ["", "Puts", "Calls", "P/C Ratio", "Sentiment"],
      ["Volume", putVolume.toLocaleString(), callVolume.toLocaleString(), volRatio.toFixed(3), volSent],
      ["Open Interest", putOI.toLocaleString(), callOI.toLocaleString(), oiRatio.toFixed(3), oiSent],
    ];

    for (let r = 0; r < rows.length; r++) {
      const tr = document.createElement("tr");
      tr.style.cssText = r === 0 ? "border-bottom:1px solid #444;" : "border-bottom:1px solid #1a1a2e;";
      for (let c = 0; c < rows[r].length; c++) {
        const td = document.createElement(r === 0 ? "th" : "td");
        td.style.cssText = `padding:8px 10px;text-align:${c === 0 ? "left" : "center"};`;
        if (r === 0) {
          td.style.color = "#888";
          td.style.fontSize = "10px";
          td.textContent = rows[r][c];
        } else if (c === 4) {
          const sent = rows[r][c];
          td.style.color = sent.color;
          td.style.fontWeight = "bold";
          td.textContent = sent.label;
        } else {
          td.style.color = c === 3 ? "#fff" : "#ccc";
          if (c === 3) td.style.fontWeight = "bold";
          td.textContent = rows[r][c];
        }
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }
    win.appendElement(table);

    const legend = document.createElement("div");
    legend.style.cssText = "padding:12px;font-size:10px;color:#666;border-top:1px solid #333;margin-top:12px;";
    legend.innerHTML = `<span style="color:#f44336;">&#9632;</span> Bearish: ratio &gt; 1.0 &nbsp;&nbsp;`
      + `<span style="color:#ff9800;">&#9632;</span> Neutral: 0.7 - 1.0 &nbsp;&nbsp;`
      + `<span style="color:#4caf50;">&#9632;</span> Bullish: ratio &lt; 0.7`;
    win.appendElement(legend);

    log(`P/C Ratio for ${currentSymbol}: Vol ${volRatio.toFixed(3)} (${volSent.label}), OI ${oiRatio.toFixed(3)} (${oiSent.label})`, "ok");
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Put/Call ratio failed: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// FEATURE: Unusual Options Activity Scanner
// ══════════════════════════════════════════════════════════════

async function cmdUnusual() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Unusual Options Activity`, width: 700, height: 500 });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Scanning options chain for unusual activity...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  function getNextExpiry() {
    const now = new Date();
    let month = now.getMonth();
    let year = now.getFullYear();
    for (let i = 0; i < 2; i++) {
      const firstDay = new Date(year, month, 1).getDay();
      const firstFriday = firstDay <= 5 ? (5 - firstDay + 1) : (12 - firstDay + 1);
      const thirdFriday = firstFriday + 14;
      const expDate = new Date(year, month, thirdFriday);
      if (expDate > now) return expDate.toISOString().slice(0, 10);
      month++;
      if (month > 11) { month = 0; year++; }
    }
    return now.toISOString().slice(0, 10);
  }

  try {
    const expiry = getNextExpiry();
    const json = await invoke("get_options", { symbol: currentSymbol, expiry });
    const chain = typeof json === "string" ? JSON.parse(json) : json;
    const options = Array.isArray(chain) ? chain : (chain.options || []);
    win.contentElement.textContent = "";

    if (!options || options.length === 0) {
      win.setContent("No options data available. This symbol may not have listed options.");
      return;
    }

    const unusual = [];
    for (const opt of options) {
      const vol = opt.volume || 0;
      const oi = opt.open_interest || opt.openInterest || 0;
      if (vol > 100 && oi > 0 && vol > 3 * oi) {
        unusual.push({
          strike: opt.strike,
          type: opt.option_type === "call" ? "Call" : "Put",
          volume: vol,
          oi: oi,
          ratio: vol / oi,
          sentiment: opt.option_type === "call" ? "Bullish" : "Bearish",
          sentColor: opt.option_type === "call" ? "#4caf50" : "#f44336",
        });
      }
    }
    unusual.sort((a, b) => b.volume - a.volume);

    const header = document.createElement("div");
    header.style.cssText = "padding:8px 12px;border-bottom:1px solid #333;color:#888;font-size:11px;";
    header.textContent = `Expiry: ${expiry} — Flagged: volume > 3x open interest AND volume > 100`;
    win.appendElement(header);

    if (unusual.length === 0) {
      const msg = document.createElement("div");
      msg.style.cssText = "padding:20px;text-align:center;color:#888;font-size:12px;";
      msg.textContent = "No unusual options activity detected for this expiration.";
      win.appendElement(msg);
      log(`Unusual options for ${currentSymbol}: none detected`, "ok");
      return;
    }

    const table = document.createElement("table");
    table.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;";
    const thead = document.createElement("tr");
    thead.style.cssText = "border-bottom:1px solid #444;";
    for (const h of ["Strike", "Type", "Volume", "OI", "Vol/OI", "Side"]) {
      const th = document.createElement("th");
      th.style.cssText = "padding:6px 8px;color:#888;text-align:center;font-size:10px;";
      th.textContent = h;
      thead.appendChild(th);
    }
    table.appendChild(thead);

    for (const u of unusual) {
      const tr = document.createElement("tr");
      tr.style.cssText = "border-bottom:1px solid #1a1a2e;";
      const vals = [
        { text: u.strike.toFixed(2), color: "#fff", bold: true },
        { text: u.type, color: u.type === "Call" ? "#4caf50" : "#f44336", bold: false },
        { text: u.volume.toLocaleString(), color: "#ff9800", bold: true },
        { text: u.oi.toLocaleString(), color: "#ccc", bold: false },
        { text: u.ratio.toFixed(1) + "x", color: "#ff9800", bold: true },
        { text: u.sentiment, color: u.sentColor, bold: true },
      ];
      for (const v of vals) {
        const td = document.createElement("td");
        td.style.cssText = `padding:5px 8px;text-align:center;color:${v.color};${v.bold ? "font-weight:bold;" : ""}`;
        td.textContent = v.text;
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }

    const wrapper = document.createElement("div");
    wrapper.style.cssText = "overflow-y:auto;max-height:380px;";
    wrapper.appendChild(table);
    win.appendElement(wrapper);

    log(`Unusual options for ${currentSymbol}: ${unusual.length} strikes flagged`, "ok");
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Unusual options scan failed: ${e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// FEATURE: Conditional Bracket/OCO Order Placement UI
// ══════════════════════════════════════════════════════════════

function cmdBracketOrder() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Bracket/OCO Order`, width: 420, height: 380 });
  win.contentElement.textContent = "";
  const c = document.createElement("div");
  c.style.cssText = "display:flex;flex-direction:column;gap:8px;padding:8px;font-size:11px;";

  const sl = getSLPrice();
  const tp = getTPPrice();
  const side = (sl && tp && sl < lastPrice) ? "buy" : "sell";

  const fields = [
    { id: "brk-symbol", label: "Symbol", value: currentSymbol, disabled: true },
    { id: "brk-side", label: "Side", type: "select", options: ["buy", "sell"], value: side },
    { id: "brk-entry", label: "Entry Price", value: lastPrice ? lastPrice.toFixed(4) : "" },
    { id: "brk-tp", label: "Take Profit", value: tp ? tp.toFixed(4) : "" },
    { id: "brk-sl", label: "Stop Loss", value: sl ? sl.toFixed(4) : "" },
    { id: "brk-qty", label: "Quantity", value: "1" },
  ];

  const inputs = {};
  for (const f of fields) {
    const row = document.createElement("div");
    row.style.cssText = "display:flex;align-items:center;gap:8px;";
    const lbl = document.createElement("label");
    lbl.textContent = f.label;
    lbl.style.cssText = "width:90px;color:#888;font-size:10px;";
    row.appendChild(lbl);

    let inp;
    if (f.type === "select") {
      inp = document.createElement("select");
      inp.style.cssText = "flex:1;background:#111;color:#fff;border:1px solid #555;padding:4px;font-family:inherit;font-size:11px;";
      for (const opt of f.options) {
        const o = document.createElement("option");
        o.value = opt; o.textContent = opt.toUpperCase();
        if (opt === f.value) o.selected = true;
        inp.appendChild(o);
      }
    } else {
      inp = document.createElement("input");
      inp.type = "text";
      inp.value = f.value || "";
      inp.style.cssText = "flex:1;background:#111;color:#fff;border:1px solid #555;padding:4px;font-family:inherit;font-size:11px;";
      if (f.disabled) inp.disabled = true;
    }
    inputs[f.id] = inp;
    row.appendChild(inp);
    c.appendChild(row);
  }

  const statusDiv = document.createElement("div");
  statusDiv.style.cssText = "color:#888;font-size:10px;min-height:20px;";
  c.appendChild(statusDiv);

  // Place Bracket button
  const bracketBtn = document.createElement("button");
  bracketBtn.textContent = "Place Bracket Order (Entry + TP + SL)";
  bracketBtn.style.cssText = "padding:8px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;font-family:inherit;font-size:11px;font-weight:bold;";
  bracketBtn.addEventListener("click", async () => {
    const qty = parseInt(inputs["brk-qty"].value) || 0;
    const tpPrice = parseFloat(inputs["brk-tp"].value) || 0;
    const slPrice = parseFloat(inputs["brk-sl"].value) || 0;
    const orderSide = inputs["brk-side"].value;
    if (qty <= 0) { statusDiv.textContent = "Invalid quantity"; statusDiv.style.color = "#f44"; return; }
    if (tpPrice <= 0 || slPrice <= 0) { statusDiv.textContent = "TP and SL prices required"; statusDiv.style.color = "#f44"; return; }

    const confirmMsg = `Place BRACKET: ${orderSide.toUpperCase()} ${qty}x ${currentSymbol}\nTP: $${tpPrice} | SL: $${slPrice}`;
    if (!confirm(confirmMsg)) return;

    try {
      statusDiv.textContent = "Placing bracket order..."; statusDiv.style.color = "#888";
      await invoke("place_bracket_order", { symbol: currentSymbol, qty, side: orderSide, tpPrice, slPrice });
      statusDiv.textContent = "Bracket order placed successfully!"; statusDiv.style.color = "#4caf50";
      log(`Bracket: ${orderSide} ${qty}x ${currentSymbol} TP:${tpPrice} SL:${slPrice}`, "ok");
    } catch (e) { statusDiv.textContent = `Error: ${e}`; statusDiv.style.color = "#f44"; }
  });
  c.appendChild(bracketBtn);

  // Place OCO button (TP + SL only, no entry)
  const ocoBtn = document.createElement("button");
  ocoBtn.textContent = "Place OCO (TP Limit + SL Stop only)";
  ocoBtn.style.cssText = "padding:8px;background:#3a0f60;color:#c8f;border:1px solid #555;cursor:pointer;font-family:inherit;font-size:11px;";
  ocoBtn.addEventListener("click", async () => {
    const qty = parseInt(inputs["brk-qty"].value) || 0;
    const tpPrice = parseFloat(inputs["brk-tp"].value) || 0;
    const slPrice = parseFloat(inputs["brk-sl"].value) || 0;
    const orderSide = inputs["brk-side"].value;
    // For OCO exit: if position is long, TP is sell limit, SL is sell stop
    const exitSide = orderSide === "buy" ? "sell" : "buy";
    if (qty <= 0) { statusDiv.textContent = "Invalid quantity"; statusDiv.style.color = "#f44"; return; }
    if (tpPrice <= 0 || slPrice <= 0) { statusDiv.textContent = "TP and SL prices required"; statusDiv.style.color = "#f44"; return; }

    const confirmMsg = `Place OCO exits for ${orderSide.toUpperCase()} position:\n${exitSide.toUpperCase()} LIMIT @ $${tpPrice} (TP)\n${exitSide.toUpperCase()} STOP @ $${slPrice} (SL)\nQty: ${qty}x ${currentSymbol}`;
    if (!confirm(confirmMsg)) return;

    try {
      statusDiv.textContent = "Placing OCO orders..."; statusDiv.style.color = "#888";
      await invoke("place_limit_order", { symbol: currentSymbol, qty, side: exitSide, limitPrice: tpPrice, tif: "gtc" });
      await invoke("place_stop_order", { symbol: currentSymbol, qty, side: exitSide, stopPrice: slPrice, tif: "gtc" });
      statusDiv.textContent = "OCO orders placed (TP limit + SL stop)!"; statusDiv.style.color = "#4caf50";
      log(`OCO: ${exitSide} limit@${tpPrice} + stop@${slPrice} ${qty}x ${currentSymbol}`, "ok");
    } catch (e) { statusDiv.textContent = `Error: ${e}`; statusDiv.style.color = "#f44"; }
  });
  c.appendChild(ocoBtn);

  win.contentElement.textContent = "";
  win.appendElement(c);
}

// ══════════════════════════════════════════════════════════════
// FEATURE: Multi-Leg Order Builder
// ══════════════════════════════════════════════════════════════

function cmdMultiLeg() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }

  const win = createWindow({ title: `${currentSymbol} — Multi-Leg Order Builder`, width: 680, height: 520 });
  win.contentElement.textContent = "";

  const c = document.createElement("div");
  c.style.cssText = "display:flex;flex-direction:column;gap:8px;padding:8px;font-size:11px;";

  const legs = []; // { side, type, qty, price, symbol, rowEl }

  const inputStyle = "background:#111;color:#fff;border:1px solid #555;padding:4px;font-family:inherit;font-size:11px;";
  const btnStyle = "padding:6px 12px;border:1px solid #555;cursor:pointer;font-family:inherit;font-size:11px;";

  // ── Leg Table ──
  const tableWrap = document.createElement("div");
  tableWrap.style.cssText = "max-height:220px;overflow-y:auto;";
  const table = document.createElement("table");
  table.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;";

  const thead = document.createElement("thead");
  thead.innerHTML = `<tr style="color:#888;text-align:left;border-bottom:1px solid #333;">
    <th style="padding:4px;">#</th>
    <th style="padding:4px;">Side</th>
    <th style="padding:4px;">Type</th>
    <th style="padding:4px;">Qty</th>
    <th style="padding:4px;">Price</th>
    <th style="padding:4px;">Symbol</th>
    <th style="padding:4px;"></th>
  </tr>`;
  table.appendChild(thead);

  const tbody = document.createElement("tbody");
  table.appendChild(tbody);
  tableWrap.appendChild(table);
  c.appendChild(tableWrap);

  function addLeg(opts = {}) {
    const idx = legs.length;
    const leg = {
      side: opts.side || "buy",
      type: opts.type || "market",
      qty: opts.qty || 1,
      price: opts.price || (lastPrice || 0),
      symbol: opts.symbol || currentSymbol,
    };

    const tr = document.createElement("tr");
    tr.style.cssText = "border-bottom:1px solid #222;";

    // #
    const tdNum = document.createElement("td");
    tdNum.style.cssText = "padding:4px;color:#888;";
    tdNum.textContent = idx + 1;
    tr.appendChild(tdNum);

    // Side
    const tdSide = document.createElement("td");
    tdSide.style.padding = "4px";
    const selSide = document.createElement("select");
    selSide.style.cssText = inputStyle + "width:60px;";
    for (const s of ["buy", "sell"]) {
      const o = document.createElement("option");
      o.value = s; o.textContent = s.toUpperCase();
      if (s === leg.side) o.selected = true;
      selSide.appendChild(o);
    }
    selSide.addEventListener("change", () => { leg.side = selSide.value; updateSummary(); });
    tdSide.appendChild(selSide);
    tr.appendChild(tdSide);

    // Type
    const tdType = document.createElement("td");
    tdType.style.padding = "4px";
    const selType = document.createElement("select");
    selType.style.cssText = inputStyle + "width:80px;";
    for (const t of ["market", "limit", "stop", "stop_limit"]) {
      const o = document.createElement("option");
      o.value = t; o.textContent = t.replace("_", "-").toUpperCase();
      if (t === leg.type) o.selected = true;
      selType.appendChild(o);
    }
    selType.addEventListener("change", () => {
      leg.type = selType.value;
      inpPrice.disabled = (leg.type === "market");
      updateSummary();
    });
    tdType.appendChild(selType);
    tr.appendChild(tdType);

    // Qty
    const tdQty = document.createElement("td");
    tdQty.style.padding = "4px";
    const inpQty = document.createElement("input");
    inpQty.type = "number"; inpQty.min = "1"; inpQty.value = leg.qty;
    inpQty.style.cssText = inputStyle + "width:60px;";
    inpQty.addEventListener("input", () => { leg.qty = parseInt(inpQty.value) || 0; updateSummary(); });
    tdQty.appendChild(inpQty);
    tr.appendChild(tdQty);

    // Price
    const tdPrice = document.createElement("td");
    tdPrice.style.padding = "4px";
    const inpPrice = document.createElement("input");
    inpPrice.type = "text"; inpPrice.value = leg.price ? leg.price.toFixed(4) : "0";
    inpPrice.style.cssText = inputStyle + "width:80px;";
    inpPrice.disabled = (leg.type === "market");
    inpPrice.addEventListener("input", () => { leg.price = parseFloat(inpPrice.value) || 0; updateSummary(); });
    tdPrice.appendChild(inpPrice);
    tr.appendChild(tdPrice);

    // Symbol
    const tdSym = document.createElement("td");
    tdSym.style.padding = "4px";
    const inpSym = document.createElement("input");
    inpSym.type = "text"; inpSym.value = leg.symbol;
    inpSym.style.cssText = inputStyle + "width:70px;";
    inpSym.addEventListener("input", () => { leg.symbol = inpSym.value; });
    tdSym.appendChild(inpSym);
    tr.appendChild(tdSym);

    // Remove
    const tdRm = document.createElement("td");
    tdRm.style.padding = "4px";
    const btnRm = document.createElement("button");
    btnRm.textContent = "X";
    btnRm.style.cssText = btnStyle + "background:#600;color:#f88;padding:2px 8px;";
    btnRm.addEventListener("click", () => {
      const i = legs.indexOf(leg);
      if (i >= 0) { legs.splice(i, 1); tr.remove(); renumberLegs(); updateSummary(); }
    });
    tdRm.appendChild(btnRm);
    tr.appendChild(tdRm);

    leg.rowEl = tr;
    leg.setQty = (v) => { leg.qty = v; inpQty.value = v; };
    leg.setPrice = (v) => { leg.price = v; inpPrice.value = v.toFixed(4); };
    leg.setSide = (v) => { leg.side = v; selSide.value = v; };
    leg.setType = (v) => { leg.type = v; selType.value = v; inpPrice.disabled = (v === "market"); };
    legs.push(leg);
    tbody.appendChild(tr);
    updateSummary();
    return leg;
  }

  function renumberLegs() {
    for (let i = 0; i < legs.length; i++) {
      const firstTd = legs[i].rowEl.querySelector("td");
      if (firstTd) firstTd.textContent = i + 1;
    }
  }

  // ── Buttons Row ──
  const btnRow = document.createElement("div");
  btnRow.style.cssText = "display:flex;gap:6px;flex-wrap:wrap;";

  const addBtn = document.createElement("button");
  addBtn.textContent = "+ Add Leg";
  addBtn.style.cssText = btnStyle + "background:#0a2a0a;color:#8f8;";
  addBtn.addEventListener("click", () => addLeg());
  btnRow.appendChild(addBtn);

  // Preset: Bracket
  const bracketBtn = document.createElement("button");
  bracketBtn.textContent = "Bracket";
  bracketBtn.title = "Buy market + Sell stop (SL) + Sell limit (TP)";
  bracketBtn.style.cssText = btnStyle + "background:#0f3460;color:#8cf;";
  bracketBtn.addEventListener("click", () => {
    clearLegs();
    const sl = getSLPrice() || (lastPrice * 0.98);
    const tp = getTPPrice() || (lastPrice * 1.02);
    addLeg({ side: "buy", type: "market", qty: 1, price: lastPrice });
    addLeg({ side: "sell", type: "stop", qty: 1, price: sl });
    addLeg({ side: "sell", type: "limit", qty: 1, price: tp });
  });
  btnRow.appendChild(bracketBtn);

  // Preset: OCO
  const ocoBtn = document.createElement("button");
  ocoBtn.textContent = "OCO";
  ocoBtn.title = "Sell stop (SL) + Sell limit (TP)";
  ocoBtn.style.cssText = btnStyle + "background:#3a0f60;color:#c8f;";
  ocoBtn.addEventListener("click", () => {
    clearLegs();
    const sl = getSLPrice() || (lastPrice * 0.98);
    const tp = getTPPrice() || (lastPrice * 1.02);
    addLeg({ side: "sell", type: "stop", qty: 1, price: sl });
    addLeg({ side: "sell", type: "limit", qty: 1, price: tp });
  });
  btnRow.appendChild(ocoBtn);

  // Preset: Scale In
  const scaleInBtn = document.createElement("button");
  scaleInBtn.textContent = "Scale In";
  scaleInBtn.title = "3 buy limits at -1%, -2%, -3%";
  scaleInBtn.style.cssText = btnStyle + "background:#2a2a00;color:#ff8;";
  scaleInBtn.addEventListener("click", () => {
    clearLegs();
    for (let i = 1; i <= 3; i++) {
      addLeg({ side: "buy", type: "limit", qty: 1, price: lastPrice * (1 - i * 0.01) });
    }
  });
  btnRow.appendChild(scaleInBtn);

  // Preset: Scale Out
  const scaleOutBtn = document.createElement("button");
  scaleOutBtn.textContent = "Scale Out";
  scaleOutBtn.title = "3 sell limits at +1%, +2%, +3%";
  scaleOutBtn.style.cssText = btnStyle + "background:#2a1500;color:#fa8;";
  scaleOutBtn.addEventListener("click", () => {
    clearLegs();
    for (let i = 1; i <= 3; i++) {
      addLeg({ side: "sell", type: "limit", qty: 1, price: lastPrice * (1 + i * 0.01) });
    }
  });
  btnRow.appendChild(scaleOutBtn);

  c.appendChild(btnRow);

  function clearLegs() {
    while (legs.length > 0) { legs.pop().rowEl.remove(); }
    updateSummary();
  }

  // ── Summary ──
  const summaryDiv = document.createElement("div");
  summaryDiv.style.cssText = "background:#0a0a0a;border:1px solid #333;padding:8px;font-size:11px;color:#aaa;";
  c.appendChild(summaryDiv);

  function updateSummary() {
    const totalLegs = legs.length;
    let buyQty = 0, sellQty = 0, maxCost = 0, totalRisk = 0;
    let entryPrice = 0;

    for (const leg of legs) {
      if (leg.side === "buy") {
        buyQty += leg.qty;
        maxCost += leg.price * leg.qty;
        if (leg.type === "market" || leg.type === "limit") entryPrice = entryPrice || leg.price;
      } else {
        sellQty += leg.qty;
      }
    }

    // Risk = distance from entry to stop legs
    if (entryPrice > 0) {
      for (const leg of legs) {
        if (leg.side === "sell" && leg.type === "stop") {
          totalRisk += Math.abs(entryPrice - leg.price) * leg.qty;
        }
      }
    }

    const net = buyQty - sellQty;
    summaryDiv.innerHTML = `<b style="color:#fff;">Summary</b><br>` +
      `Legs: <span style="color:#fff;">${totalLegs}</span> &nbsp;|&nbsp; ` +
      `Net Qty: <span style="color:${net > 0 ? "#4caf50" : net < 0 ? "#f44" : "#fff"};">${net > 0 ? "+" : ""}${net}</span> &nbsp;|&nbsp; ` +
      `Buy: <span style="color:#4caf50;">${buyQty}</span> &nbsp; Sell: <span style="color:#f44;">${sellQty}</span><br>` +
      `Risk (entry→stop): <span style="color:#fa0;">$${totalRisk.toFixed(2)}</span> &nbsp;|&nbsp; ` +
      `Max Cost: <span style="color:#8cf;">$${maxCost.toFixed(2)}</span>`;
  }
  updateSummary();

  // ── Status ──
  const statusDiv = document.createElement("div");
  statusDiv.style.cssText = "color:#888;font-size:10px;min-height:20px;";
  c.appendChild(statusDiv);

  // ── Action Buttons ──
  const actionRow = document.createElement("div");
  actionRow.style.cssText = "display:flex;gap:6px;margin-top:4px;";

  // Preview
  const previewBtn = document.createElement("button");
  previewBtn.textContent = "Preview";
  previewBtn.style.cssText = btnStyle + "background:#222;color:#fff;";
  previewBtn.addEventListener("click", () => {
    if (legs.length === 0) { statusDiv.textContent = "No legs to preview"; statusDiv.style.color = "#f44"; return; }
    let preview = "Order Preview:\n";
    for (let i = 0; i < legs.length; i++) {
      const l = legs[i];
      const priceStr = l.type === "market" ? "MKT" : `$${l.price.toFixed(4)}`;
      preview += `  ${i + 1}. ${l.side.toUpperCase()} ${l.qty}x ${l.symbol} ${l.type.toUpperCase()} @ ${priceStr}\n`;
    }
    statusDiv.textContent = preview;
    statusDiv.style.cssText = "color:#ccc;font-size:10px;min-height:20px;white-space:pre;font-family:monospace;";
  });
  actionRow.appendChild(previewBtn);

  // Execute All
  const execBtn = document.createElement("button");
  execBtn.textContent = "Execute All";
  execBtn.style.cssText = btnStyle + "background:#0f3460;color:#8cf;font-weight:bold;";
  execBtn.addEventListener("click", async () => {
    if (legs.length === 0) { statusDiv.textContent = "No legs to execute"; statusDiv.style.color = "#f44"; return; }

    // Safety confirm
    const symbols = [...new Set(legs.map(l => l.symbol))].join(", ");
    if (!confirm(`Place ${legs.length} orders for ${symbols}?`)) return;

    execBtn.disabled = true;
    let succeeded = 0, failed = 0;
    const errors = [];

    for (let i = 0; i < legs.length; i++) {
      const l = legs[i];
      statusDiv.textContent = `Placing leg ${i + 1}/${legs.length}...`;
      statusDiv.style.color = "#888";

      try {
        if (l.type === "market") {
          await invoke("place_order", { symbol: l.symbol, qty: l.qty, side: l.side });
        } else if (l.type === "limit") {
          await invoke("place_limit_order", { symbol: l.symbol, qty: l.qty, side: l.side, limitPrice: l.price, tif: "gtc" });
        } else if (l.type === "stop") {
          await invoke("place_stop_order", { symbol: l.symbol, qty: l.qty, side: l.side, stopPrice: l.price, tif: "gtc" });
        } else if (l.type === "stop_limit") {
          await invoke("place_stop_limit_order", { symbol: l.symbol, qty: l.qty, side: l.side, stopPrice: l.price, limitPrice: l.price, tif: "gtc" });
        }
        succeeded++;
        log(`MultiLeg ${i + 1}/${legs.length}: ${l.side} ${l.qty}x ${l.symbol} ${l.type} OK`, "ok");
      } catch (e) {
        failed++;
        errors.push(`Leg ${i + 1}: ${e}`);
        log(`MultiLeg ${i + 1}/${legs.length}: FAILED — ${e}`, "error");
      }
    }

    execBtn.disabled = false;
    if (failed === 0) {
      statusDiv.textContent = `Done! All ${succeeded} legs placed successfully.`;
      statusDiv.style.color = "#4caf50";
    } else {
      statusDiv.textContent = `${succeeded} succeeded, ${failed} failed:\n${errors.join("\n")}`;
      statusDiv.style.cssText = "color:#f44;font-size:10px;min-height:20px;white-space:pre;";
    }
    log(`MultiLeg complete: ${succeeded} OK, ${failed} failed`, succeeded > 0 && failed === 0 ? "ok" : "warn");
  });
  actionRow.appendChild(execBtn);

  // Cancel
  const cancelBtn = document.createElement("button");
  cancelBtn.textContent = "Cancel";
  cancelBtn.style.cssText = btnStyle + "background:#333;color:#aaa;";
  cancelBtn.addEventListener("click", () => win.close());
  actionRow.appendChild(cancelBtn);

  c.appendChild(actionRow);

  win.contentElement.textContent = "";
  win.appendElement(c);
}

// ══════════════════════════════════════════════════════════════
// FEATURE: Portfolio Heat Map
// ══════════════════════════════════════════════════════════════

async function cmdHeatmap() {
  const win = createWindow({ title: "Portfolio Heat Map", width: 600, height: 450 });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Loading positions for heat map...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  try {
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);

    if (!positions || positions.length === 0) {
      win.contentElement.textContent = "";
      win.setContent("No open positions for heat map.");
      return;
    }

    // Calculate daily P&L % for each position
    const items = [];
    let totalValue = 0;
    for (const p of positions) {
      const mv = Math.abs(p.market_value || p.qty * (p.current_price || 0));
      const unrealizedPl = p.unrealized_pl || p.unrealized_plpc * mv || 0;
      const costBasis = mv - unrealizedPl;
      const pctChange = costBasis > 0 ? (unrealizedPl / costBasis) * 100 : 0;
      const dailyPl = p.unrealized_intraday_pl || unrealizedPl;
      const dailyPct = p.unrealized_intraday_plpc ? p.unrealized_intraday_plpc * 100 : pctChange;

      items.push({
        symbol: p.symbol,
        value: mv,
        pctChange: dailyPct,
        pl: dailyPl,
        qty: p.qty,
        price: p.current_price || 0,
      });
      totalValue += mv;
    }

    win.contentElement.textContent = "";

    // Build finviz-style heat map grid
    const grid = document.createElement("div");
    grid.style.cssText = "display:flex;flex-wrap:wrap;gap:3px;padding:8px;";

    // Find max absolute % for scaling
    const maxPct = Math.max(...items.map(i => Math.abs(i.pctChange)), 1);

    for (const item of items) {
      // Size proportional to market value (min 60px, max 180px)
      const sizeFactor = totalValue > 0 ? item.value / totalValue : 1 / items.length;
      const boxSize = Math.max(60, Math.min(180, Math.round(sizeFactor * 500 + 60)));

      const box = document.createElement("div");
      const intensity = Math.min(Math.abs(item.pctChange) / maxPct, 1);
      let bgColor;
      if (item.pctChange > 0) {
        const g = Math.round(100 + intensity * 155);
        bgColor = `rgba(0, ${g}, 0, ${0.3 + intensity * 0.5})`;
      } else if (item.pctChange < 0) {
        const r = Math.round(100 + intensity * 155);
        bgColor = `rgba(${r}, 0, 0, ${0.3 + intensity * 0.5})`;
      } else {
        bgColor = "rgba(128, 128, 128, 0.2)";
      }

      box.style.cssText = `width:${boxSize}px;height:${boxSize * 0.6}px;background:${bgColor};border:1px solid #333;border-radius:4px;display:flex;flex-direction:column;justify-content:center;align-items:center;cursor:pointer;padding:4px;`;
      box.addEventListener("click", () => {
        document.getElementById("symbol-input").value = item.symbol;
        triggerLoad();
      });

      const symEl = document.createElement("div");
      symEl.textContent = item.symbol;
      symEl.style.cssText = "font-size:12px;font-weight:bold;color:#fff;";

      const pctEl = document.createElement("div");
      const sign = item.pctChange >= 0 ? "+" : "";
      pctEl.textContent = `${sign}${item.pctChange.toFixed(2)}%`;
      pctEl.style.cssText = `font-size:14px;font-weight:bold;color:${item.pctChange >= 0 ? "#4caf50" : "#f44336"};`;

      const valEl = document.createElement("div");
      valEl.textContent = `$${item.value.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
      valEl.style.cssText = "font-size:9px;color:#888;";

      box.appendChild(symEl);
      box.appendChild(pctEl);
      box.appendChild(valEl);
      grid.appendChild(box);
    }

    win.appendElement(grid);

    // Summary row
    const totalPl = items.reduce((s, i) => s + i.pl, 0);
    const summary = document.createElement("div");
    summary.style.cssText = "padding:8px;border-top:1px solid #333;font-size:11px;display:flex;justify-content:space-between;";
    const sp1 = document.createElement("span"); sp1.style.color = "#888"; sp1.textContent = `Positions: ${items.length}`;
    const sp2 = document.createElement("span"); sp2.style.color = "#888"; sp2.textContent = `Total Value: $${totalValue.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
    const sp3 = document.createElement("span"); sp3.style.color = totalPl >= 0 ? "#4caf50" : "#f44336"; sp3.textContent = `P&L: ${totalPl >= 0 ? "+" : ""}$${totalPl.toFixed(2)}`;
    summary.appendChild(sp1); summary.appendChild(sp2); summary.appendChild(sp3);
    win.appendElement(summary);

    log(`Heat map: ${items.length} positions, total $${totalValue.toFixed(0)}`, "ok");
  } catch (e) {
    win.contentElement.textContent = "";
    win.setContent(`Heat map failed: ${e}`);
  }
}

function setupMenuBar() {
  const menuActions = {
    // File
    "new-tab": () => document.getElementById("btn-new-tab").click(),
    "close-tab": () => { if (activeTabId !== null) closeTab(activeTabId); },
    "save-template": () => { const name = prompt("Template name:"); if (name) { saveChartTemplate(name); log(`Template "${name}" saved`, "ok"); } },
    "save-profile": () => { const name = prompt("Profile name:"); if (name) { saveWorkspaceProfile(name); log(`Profile "${name}" saved`, "ok"); } },
    "export-csv": async () => { try { const csv = await invoke("export_trade_history", { limit: 500 }); const blob = new Blob([csv], { type: "text/csv" }); const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = "trades.csv"; a.click(); } catch (e) { alert(`Export failed: ${e}`); } },
    "settings": () => cmdSettings(),
    // View
    "mtf-grid": () => document.getElementById("btn-mtf-grid").click(),
    // "split" removed — MTF Grid covers all multi-chart layouts
    "screenshot": () => { document.dispatchEvent(new KeyboardEvent("keydown", { key: "S", shiftKey: true, ctrlKey: true })); },
    "cmd-palette": () => { document.getElementById("command-palette").classList.remove("hidden"); document.getElementById("cmd-palette-input").focus(); },
    // Trading
    "buy-lines": () => document.getElementById("btn-buy-lines").click(),
    "sell-lines": () => document.getElementById("btn-sell-lines").click(),
    "destroy-lines": () => document.getElementById("btn-destroy-lines").click(),
    "open-trade": () => document.getElementById("btn-trade").click(),
    "bracket": () => cmdBracketOrder(),
    "close-all": () => document.getElementById("btn-close-all").click(),
    "close-partial": () => document.getElementById("btn-close-partial").click(),
    "calc": () => cmdPositionCalc(),
    // Tools
    "draw-trend": () => { drawingMode = "trendline"; drawingAnchor = null; document.getElementById("chart-container").style.cursor = "crosshair"; },
    "draw-fib": () => { drawingMode = "fibonacci"; drawingAnchor = null; document.getElementById("chart-container").style.cursor = "crosshair"; },
    "draw-ray": () => { drawingMode = "ray"; drawingAnchor = null; document.getElementById("chart-container").style.cursor = "crosshair"; },
    "draw-ruler": () => { drawingMode = "ruler"; drawingAnchor = null; document.getElementById("chart-container").style.cursor = "crosshair"; },
    "draw-hline": () => { drawingMode = "horizontal"; drawingAnchor = null; document.getElementById("chart-container").style.cursor = "crosshair"; },
    "draw-rect": () => { drawingMode = "rectangle"; drawingAnchor = null; document.getElementById("chart-container").style.cursor = "crosshair"; },
    "alert": () => { if (currentSymbol && lastPrice > 0) { const dir = prompt("Direction (above/below):", "above"); if (dir) addPriceAlert(currentSymbol, lastPrice, dir); } },
    "annotate": () => addChartAnnotation(),
    "delete-drawing": () => { if (drawings.length > 0) { drawings.pop(); saveDrawings(); renderDrawings(); renderDrawingsExtended(); } },
    // Research
    "fundamentals": () => { const cmds = document.querySelectorAll(".cmd-result-item"); /* trigger via palette */ document.getElementById("command-palette").classList.remove("hidden"); document.getElementById("cmd-palette-input").value = "DES"; document.getElementById("cmd-palette-input").dispatchEvent(new Event("input")); },
    "news": () => { document.getElementById("command-palette").classList.remove("hidden"); document.getElementById("cmd-palette-input").value = "NEWS"; document.getElementById("cmd-palette-input").dispatchEvent(new Event("input")); },
    "filings": () => { document.getElementById("command-palette").classList.remove("hidden"); document.getElementById("cmd-palette-input").value = "HDS"; document.getElementById("cmd-palette-input").dispatchEvent(new Event("input")); },
    "insider": () => cmdInsider(),
    "options": () => { document.getElementById("command-palette").classList.remove("hidden"); document.getElementById("cmd-palette-input").value = "OPT"; document.getElementById("cmd-palette-input").dispatchEvent(new Event("input")); },
    "screener": () => { document.getElementById("command-palette").classList.remove("hidden"); document.getElementById("cmd-palette-input").value = "SCAN"; document.getElementById("cmd-palette-input").dispatchEvent(new Event("input")); },
    "most-active": () => cmdMostActive(),
    "sentiment": () => cmdSentiment(),
    "patterns": () => cmdPatterns(),
    "fred": () => cmdFRED(),
    "ai": () => cmdAIChat(),
    // Analysis
    "backtest": () => openVisualBacktester(),
    "optimize": () => openOptimizer(),
    "montecarlo": () => cmdMonteCarlo(),
    "correlation": () => cmdCorrelation(),
    "portfolio": () => cmdPortfolio(),
    "heatmap": () => cmdHeatmap(),
    "volsurf": () => cmdVolSurf(),
    "pcratio": () => cmdPCRatio(),
    "unusual": () => cmdUnusual(),
    "journal": () => cmdTradeJournal(),
    "activities": () => cmdActivities(),
    "alertboard": () => cmdAlertBoard(),
  };

  document.querySelectorAll(".menu-entry").forEach(entry => {
    const action = entry.dataset.action;
    if (action && menuActions[action]) {
      entry.addEventListener("click", (e) => {
        e.stopPropagation();
        menuActions[action]();
        // Close menu after action
        document.querySelectorAll(".menu-dropdown").forEach(d => d.style.display = "");
      });
    }
  });
}

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

// ══════════════════════════════════════════════════════════════
// SRLEVEL — Auto Support/Resistance Detection (fractal clustering)
// ══════════════════════════════════════════════════════════════

function cmdTradeStats() {
  const win = createWindow({ title: "Trade Statistics", width: 420, height: 380 });
  (async () => {
    try {
      const histJson = await invoke("get_order_history", { limit: 500 });
      const orders = JSON.parse(histJson).filter(o => o.status === "filled" && o.filled_avg_price);
      if (orders.length === 0) { win.setContent("No filled orders found."); return; }

      // Match buys to sells by symbol for P&L
      const trades = [];
      const openPos = {}; // symbol → { side, price, qty }
      for (const o of orders.reverse()) { // oldest first
        const sym = o.symbol;
        const price = parseFloat(o.filled_avg_price);
        const qty = parseFloat(o.qty) || 1;
        if (!openPos[sym]) {
          openPos[sym] = { side: o.side, price, qty };
        } else if (openPos[sym].side !== o.side) {
          const entry = openPos[sym];
          const pnl = entry.side === "buy" ? (price - entry.price) * Math.min(qty, entry.qty) : (entry.price - price) * Math.min(qty, entry.qty);
          trades.push({ symbol: sym, pnl, side: entry.side });
          delete openPos[sym];
        }
      }
      if (trades.length === 0) { win.setContent("No round-trip trades found (need buy+sell pairs)."); return; }

      const wins = trades.filter(t => t.pnl > 0);
      const losses = trades.filter(t => t.pnl <= 0);
      const winRate = (wins.length / trades.length * 100).toFixed(1);
      const avgWin = wins.length > 0 ? wins.reduce((s, t) => s + t.pnl, 0) / wins.length : 0;
      const avgLoss = losses.length > 0 ? Math.abs(losses.reduce((s, t) => s + t.pnl, 0) / losses.length) : 0;
      const grossWin = wins.reduce((s, t) => s + t.pnl, 0);
      const grossLoss = Math.abs(losses.reduce((s, t) => s + t.pnl, 0));
      const profitFactor = grossLoss > 0 ? (grossWin / grossLoss).toFixed(2) : "∞";
      const expectancy = (avgWin * wins.length / trades.length - avgLoss * losses.length / trades.length).toFixed(2);
      const totalPnL = trades.reduce((s, t) => s + t.pnl, 0);
      const largest = { win: wins.length > 0 ? Math.max(...wins.map(t => t.pnl)) : 0, loss: losses.length > 0 ? Math.min(...losses.map(t => t.pnl)) : 0 };

      // Consecutive
      let maxConsW = 0, maxConsL = 0, cw = 0, cl = 0;
      for (const t of trades) {
        if (t.pnl > 0) { cw++; cl = 0; maxConsW = Math.max(maxConsW, cw); }
        else { cl++; cw = 0; maxConsL = Math.max(maxConsL, cl); }
      }

      const rows = [
        ["Total Trades", trades.length], ["Wins", wins.length], ["Losses", losses.length],
        ["Win Rate", winRate + "%"], ["Avg Win", "$" + avgWin.toFixed(2)], ["Avg Loss", "$" + avgLoss.toFixed(2)],
        ["Largest Win", "$" + largest.win.toFixed(2)], ["Largest Loss", "$" + largest.loss.toFixed(2)],
        ["Profit Factor", profitFactor], ["Expectancy", "$" + expectancy],
        ["Max Consec. Wins", maxConsW], ["Max Consec. Losses", maxConsL],
        ["Total P&L", "$" + totalPnL.toFixed(2)],
      ];
      win.contentElement.textContent = "";
      const table = document.createElement("table");
      table.style.cssText = "width:100%;border-collapse:collapse;font-size:12px;";
      for (const [label, val] of rows) {
        const tr = document.createElement("tr");
        const td1 = document.createElement("td"); td1.style.cssText = "padding:4px 8px;color:#888;"; td1.textContent = label;
        const td2 = document.createElement("td"); td2.style.cssText = "padding:4px 8px;text-align:right;font-family:Consolas,monospace;";
        const valStr = String(val);
        td2.textContent = valStr;
        if (label === "Total P&L") td2.style.color = totalPnL >= 0 ? "#4caf50" : "#f44336";
        if (label === "Win Rate") td2.style.color = parseFloat(valStr) >= 50 ? "#4caf50" : "#f44336";
        tr.appendChild(td1); tr.appendChild(td2); table.appendChild(tr);
      }
      win.appendElement(table);
    } catch (e) { win.setContent("Failed: " + e); }
  })();
}

function cmdRelStrength() {
  const win = createWindow({ title: "Relative Strength Ranking", width: 500, height: 400 });
  (async () => {
    try {
      const symbols = getWatchlist();
      if (symbols.length === 0) { win.setContent("Add symbols to watchlist first (Ctrl+K → QM)."); return; }

      const results = [];
      for (const sym of symbols) {
        const cacheKey = getCacheKey(sym, "1Day");
        const cached = barCache[cacheKey];
        if (!cached || !cached.data || cached.data.length < 10) continue;
        const bars = cached.data.map(b => ({ close: b.close || b.c || parseFloat(b.close) }));
        const len = bars.length;
        const cur = bars[len - 1].close;
        const chg5 = len > 5 ? ((cur - bars[len - 6].close) / bars[len - 6].close * 100) : null;
        const chg20 = len > 20 ? ((cur - bars[len - 21].close) / bars[len - 21].close * 100) : null;
        const chg60 = len > 60 ? ((cur - bars[len - 61].close) / bars[len - 61].close * 100) : null;
        results.push({ symbol: sym, chg5, chg20, chg60 });
      }

      if (results.length === 0) { win.setContent("No cached daily data for watchlist symbols. Load a symbol first."); return; }
      results.sort((a, b) => (b.chg20 || 0) - (a.chg20 || 0));

      win.contentElement.textContent = "";
      const table = document.createElement("table");
      table.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;";
      const hdr = document.createElement("tr");
      for (const h of ["Symbol", "1W %", "1M %", "3M %", "Trend"]) {
        const th = document.createElement("td");
        th.style.cssText = "padding:4px 6px;color:#666;font-weight:bold;border-bottom:1px solid #333;";
        th.textContent = h; hdr.appendChild(th);
      }
      table.appendChild(hdr);

      for (const r of results) {
        const tr = document.createElement("tr");
        tr.style.cssText = "cursor:pointer;";
        tr.addEventListener("click", () => { document.getElementById("symbol-input").value = r.symbol; triggerLoad(); });
        const fmt = (v) => v !== null ? v.toFixed(1) + "%" : "—";
        const clr = (v) => v !== null ? (v >= 0 ? "#4caf50" : "#f44336") : "#666";
        const trend = r.chg5 !== null && r.chg20 !== null ? (r.chg5 > r.chg20 ? "▲ Accel" : "▼ Decel") : "—";
        const trendClr = r.chg5 !== null && r.chg20 !== null ? (r.chg5 > r.chg20 ? "#4caf50" : "#f44336") : "#666";
        const vals = [
          { text: r.symbol, color: "#ccc" },
          { text: fmt(r.chg5), color: clr(r.chg5) },
          { text: fmt(r.chg20), color: clr(r.chg20) },
          { text: fmt(r.chg60), color: clr(r.chg60) },
          { text: trend, color: trendClr },
        ];
        for (const v of vals) {
          const td = document.createElement("td");
          td.style.cssText = `padding:3px 6px;color:${v.color};font-family:Consolas,monospace;`;
          td.textContent = v.text; tr.appendChild(td);
        }
        table.appendChild(tr);
      }
      win.appendElement(table);
    } catch (e) { win.setContent("Failed: " + e); }
  })();
}

function cmdPairs() {
  const sym2 = prompt("Enter second symbol for pairs analysis:", "SPY");
  if (!sym2 || !sym2.trim()) return;
  const symA = currentSymbol, symB = sym2.trim().toUpperCase();
  const win = createWindow({ title: `${symA} / ${symB} Pairs Analysis`, width: 600, height: 500 });
  (async () => {
    try {
      // Fetch bars for both symbols
      const [jsonA, jsonB] = await Promise.all([
        invoke("get_bars", { symbol: symA, timeframe: "1Day", limit: 500 }),
        invoke("get_bars", { symbol: symB, timeframe: "1Day", limit: 500 }),
      ]);
      const barsA = JSON.parse(jsonA), barsB = JSON.parse(jsonB);
      // Align by date
      const mapB = {};
      for (const b of barsB) mapB[b.timestamp.substring(0, 10)] = b;
      const aligned = [];
      for (const a of barsA) {
        const d = a.timestamp.substring(0, 10);
        if (mapB[d]) aligned.push({ date: d, a: a.close, b: mapB[d].close });
      }
      if (aligned.length < 30) { win.setContent("Not enough overlapping data (need 30+ bars)."); return; }

      // Compute stats
      const n = aligned.length;
      const retA = [], retB = [];
      for (let i = 1; i < n; i++) {
        retA.push((aligned[i].a - aligned[i-1].a) / aligned[i-1].a);
        retB.push((aligned[i].b - aligned[i-1].b) / aligned[i-1].b);
      }
      const meanA = retA.reduce((s,v) => s+v, 0) / retA.length;
      const meanB = retB.reduce((s,v) => s+v, 0) / retB.length;
      let cov = 0, varA = 0, varB = 0;
      for (let i = 0; i < retA.length; i++) {
        cov += (retA[i] - meanA) * (retB[i] - meanB);
        varA += (retA[i] - meanA) ** 2;
        varB += (retB[i] - meanB) ** 2;
      }
      cov /= retA.length; varA /= retA.length; varB /= retA.length;
      const corr = (Math.sqrt(varA) * Math.sqrt(varB)) > 0 ? cov / (Math.sqrt(varA) * Math.sqrt(varB)) : 0;
      const hedgeRatio = varB > 0 ? cov / varB : 1;

      // Spread and Z-score
      const spread = aligned.map(d => d.a - hedgeRatio * d.b);
      const window20 = 20;
      const zScores = [];
      for (let i = window20; i < spread.length; i++) {
        const slice = spread.slice(i - window20, i);
        const m = slice.reduce((s,v) => s+v, 0) / slice.length;
        const sd = Math.sqrt(slice.reduce((s,v) => s + (v-m)**2, 0) / slice.length);
        zScores.push({ date: aligned[i].date, z: sd > 0 ? (spread[i] - m) / sd : 0, spread: spread[i] });
      }
      const curZ = zScores.length > 0 ? zScores[zScores.length - 1].z : 0;
      const signal = curZ < -2 ? "ENTER LONG SPREAD" : curZ > 2 ? "ENTER SHORT SPREAD" : Math.abs(curZ) < 0.5 ? "EXIT / FLAT" : "HOLD";
      const sigClr = curZ < -2 ? "#4caf50" : curZ > 2 ? "#f44336" : "#888";

      // Half-life (OLS: Δspread = α + β*spread → half-life = -ln(2)/β)
      let sumX = 0, sumY = 0, sumXY = 0, sumX2 = 0, hln = spread.length - 1;
      for (let i = 1; i < spread.length; i++) {
        const x = spread[i-1], y = spread[i] - spread[i-1];
        sumX += x; sumY += y; sumXY += x*y; sumX2 += x*x;
      }
      const beta = (hln * sumXY - sumX * sumY) / (hln * sumX2 - sumX * sumX);
      const halfLife = beta < 0 ? (-Math.log(2) / beta).toFixed(1) : "N/A";

      // Display summary
      win.contentElement.textContent = "";
      const stats = [
        ["Correlation", corr.toFixed(3)], ["Hedge Ratio", hedgeRatio.toFixed(4)],
        ["Current Z-Score", curZ.toFixed(2)], ["Half-Life (days)", halfLife],
        ["Signal", signal], ["Aligned Bars", n],
      ];
      const table = document.createElement("table");
      table.style.cssText = "width:100%;border-collapse:collapse;font-size:12px;margin-bottom:8px;";
      for (const [k, v] of stats) {
        const tr = document.createElement("tr");
        const td1 = document.createElement("td"); td1.style.cssText = "padding:3px 8px;color:#888;"; td1.textContent = k;
        const td2 = document.createElement("td"); td2.style.cssText = "padding:3px 8px;text-align:right;font-family:Consolas,monospace;";
        td2.textContent = v;
        if (k === "Signal") td2.style.color = sigClr;
        if (k === "Correlation") td2.style.color = Math.abs(corr) > 0.7 ? "#4caf50" : "#f44336";
        tr.appendChild(td1); tr.appendChild(td2); table.appendChild(tr);
      }
      win.appendElement(table);

      // Embed z-score chart
      const chartDiv = document.createElement("div");
      chartDiv.style.cssText = "width:100%;height:200px;";
      win.appendElement(chartDiv);
      const pairChart = createChart(chartDiv, {
        width: 560, height: 200,
        layout: { background: { color: "#000" }, textColor: "#888", fontFamily: "Consolas, monospace", attributionLogo: false },
        grid: { vertLines: { color: "#111" }, horzLines: { color: "#111" } },
        rightPriceScale: { borderColor: "#333" }, timeScale: { borderColor: "#333" },
      });
      const zLine = pairChart.addLineSeries({ color: "#2196f3", lineWidth: 2, title: "Z-Score", lastValueVisible: true });
      zLine.setData(zScores.map(d => ({ time: d.date, value: d.z })));
      // Z-score threshold lines
      const addThresh = (val, clr) => {
        const s = pairChart.addLineSeries({ color: clr, lineWidth: 1, lineStyle: 2, lastValueVisible: false, priceLineVisible: false });
        s.setData(zScores.map(d => ({ time: d.date, value: val })));
      };
      addThresh(2, "#f4433666"); addThresh(-2, "#4caf5066"); addThresh(0, "#ffffff33");
      pairChart.timeScale().fitContent();
    } catch (e) { win.setContent("Failed: " + e); }
  })();
}

function cmdMTFDiv() {
  const win = createWindow({ title: `MTF Divergence — ${currentSymbol}`, width: 400, height: 300 });
  try {
    if (!mtfData || Object.keys(mtfData).length === 0) {
      win.setContent("MTF data not loaded. Load a chart first and wait for MTF indicators.");
      return;
    }
    const tfs = ["1Hour", "4Hour", "1Day", "1Week"];
    const tfLabels = { "1Hour": "H1", "4Hour": "H4", "1Day": "D1", "1Week": "W1" };
    const results = [];
    for (const tf of tfs) {
      const bars = mtfData[tf];
      if (!bars || bars.length < 33) { results.push({ tf, bias: "N/A", fisher: 0, signal: 0 }); continue; }
      const ef = calcEhlersFisher(bars, 32);
      if (ef.fisher.length === 0) { results.push({ tf, bias: "N/A", fisher: 0, signal: 0 }); continue; }
      const last = ef.fisher[ef.fisher.length - 1].value;
      const sig = ef.signal[ef.signal.length - 1].value;
      const bias = last > sig ? "BULLISH" : last < sig ? "BEARISH" : "NEUTRAL";
      results.push({ tf, bias, fisher: last, signal: sig });
    }

    // Determine alignment
    const biases = results.filter(r => r.bias !== "N/A").map(r => r.bias);
    const allBull = biases.every(b => b === "BULLISH");
    const allBear = biases.every(b => b === "BEARISH");
    const htfBias = results.filter(r => r.tf === "1Day" || r.tf === "1Week").map(r => r.bias);
    const ltfBias = results.filter(r => r.tf === "1Hour" || r.tf === "4Hour").map(r => r.bias);
    const htfConflict = htfBias.length >= 2 && ltfBias.length >= 1 &&
      htfBias.some(b => b === "BULLISH") !== ltfBias.some(b => b === "BULLISH") &&
      !htfBias.includes("N/A") && !ltfBias.includes("N/A") &&
      htfBias[0] !== ltfBias[0];
    let overall, overallClr;
    if (allBull) { overall = "ALIGNED BULLISH"; overallClr = "#4caf50"; }
    else if (allBear) { overall = "ALIGNED BEARISH"; overallClr = "#f44336"; }
    else if (htfConflict) { overall = "HTF CONFLICT ⚠️"; overallClr = "#ff9800"; }
    else { overall = "MIXED"; overallClr = "#888"; }

    win.contentElement.textContent = "";
    // Overall
    const hdr = document.createElement("div");
    hdr.style.cssText = `text-align:center;font-size:16px;font-weight:bold;padding:8px;color:${overallClr};`;
    hdr.textContent = overall;
    win.appendElement(hdr);

    // Table
    const table = document.createElement("table");
    table.style.cssText = "width:100%;border-collapse:collapse;font-size:12px;";
    const thead = document.createElement("tr");
    for (const h of ["TF", "Fisher", "Signal", "Bias"]) {
      const td = document.createElement("td");
      td.style.cssText = "padding:4px 8px;color:#666;font-weight:bold;border-bottom:1px solid #333;";
      td.textContent = h; thead.appendChild(td);
    }
    table.appendChild(thead);
    for (const r of results) {
      const tr = document.createElement("tr");
      const biasClr = r.bias === "BULLISH" ? "#4caf50" : r.bias === "BEARISH" ? "#f44336" : "#888";
      const vals = [
        { text: tfLabels[r.tf] || r.tf, color: "#ccc" },
        { text: r.fisher.toFixed(3), color: "#2196f3" },
        { text: r.signal.toFixed(3), color: "#A9A9A9" },
        { text: r.bias, color: biasClr },
      ];
      for (const v of vals) {
        const td = document.createElement("td");
        td.style.cssText = `padding:4px 8px;font-family:Consolas,monospace;color:${v.color};`;
        td.textContent = v.text; tr.appendChild(td);
      }
      table.appendChild(tr);
    }
    win.appendElement(table);

    // NNFX recommendation
    const rec = document.createElement("div");
    rec.style.cssText = "padding:8px;margin-top:8px;font-size:11px;color:#888;border-top:1px solid #333;";
    const d1Bias = results.find(r => r.tf === "1Day");
    const w1Bias = results.find(r => r.tf === "1Week");
    if (d1Bias && w1Bias && d1Bias.bias !== "N/A" && w1Bias.bias !== "N/A") {
      if (d1Bias.bias === w1Bias.bias) {
        rec.textContent = `NNFX: Trade ${d1Bias.bias === "BULLISH" ? "LONG" : "SHORT"} only (D1+W1 aligned ${d1Bias.bias}).`;
        rec.style.color = d1Bias.bias === "BULLISH" ? "#4caf50" : "#f44336";
      } else {
        rec.textContent = "NNFX: D1 and W1 disagree — NO TRADE. Wait for alignment.";
        rec.style.color = "#ff9800";
      }
    } else {
      rec.textContent = "NNFX: Insufficient HTF data for recommendation.";
    }
    win.appendElement(rec);
  } catch (e) { win.setContent("Failed: " + e); }
}

function cmdSRLevel() {
  if (!currentChartData || currentChartData.length < 30) {
    log("SRLEVEL: Need at least 30 bars loaded", "warn");
    return;
  }

  const data = currentChartData;
  const fractalLookback = 5; // 5-bar fractal method (same as auto-fib)

  // Step 1: Find all swing highs and swing lows
  const swingHighs = [];
  const swingLows = [];

  for (let i = fractalLookback; i < data.length - fractalLookback; i++) {
    let isHigh = true, isLow = true;
    for (let j = 1; j <= fractalLookback; j++) {
      if (data[i - j].high >= data[i].high || data[i + j].high >= data[i].high) isHigh = false;
      if (data[i - j].low <= data[i].low || data[i + j].low <= data[i].low) isLow = false;
    }
    if (isHigh) swingHighs.push({ idx: i, price: data[i].high, time: data[i].time, type: "resistance" });
    if (isLow) swingLows.push({ idx: i, price: data[i].low, time: data[i].time, type: "support" });
  }

  const allSwings = [...swingHighs, ...swingLows];
  if (allSwings.length === 0) {
    log("SRLEVEL: No swing points found", "warn");
    return;
  }

  // Step 2: Cluster nearby swing points within 0.5% of each other
  const clusters = [];
  const used = new Set();

  // Sort by price for efficient clustering
  allSwings.sort((a, b) => a.price - b.price);

  for (let i = 0; i < allSwings.length; i++) {
    if (used.has(i)) continue;
    const cluster = { points: [allSwings[i]], priceSum: allSwings[i].price, type: allSwings[i].type };
    used.add(i);

    for (let j = i + 1; j < allSwings.length; j++) {
      if (used.has(j)) continue;
      const avgPrice = cluster.priceSum / cluster.points.length;
      const pctDiff = Math.abs(allSwings[j].price - avgPrice) / avgPrice;
      if (pctDiff <= 0.005) { // 0.5% threshold
        cluster.points.push(allSwings[j]);
        cluster.priceSum += allSwings[j].price;
        used.add(j);
        // Determine cluster type by majority
        const supportCount = cluster.points.filter(p => p.type === "support").length;
        cluster.type = supportCount > cluster.points.length / 2 ? "support" : "resistance";
      }
    }
    clusters.push(cluster);
  }

  // Step 3: Filter — keep clusters with 3+ touches
  const validClusters = clusters
    .filter(c => c.points.length >= 3)
    .map(c => {
      const avgPrice = c.priceSum / c.points.length;
      const times = c.points.map(p => p.time).sort((a, b) => a - b);
      return {
        price: avgPrice,
        touches: c.points.length,
        type: c.type,
        firstTouch: times[0],
        lastTouch: times[times.length - 1],
      };
    })
    .sort((a, b) => b.touches - a.touches); // Rank by touch count

  if (validClusters.length === 0) {
    log("SRLEVEL: No significant S/R levels found (need 3+ touches per cluster)", "warn");
    return;
  }

  // Step 4: Draw on chart
  const dp = lastPrice > 100 ? 2 : lastPrice > 1 ? 4 : 6;

  for (let i = 0; i < validClusters.length; i++) {
    const level = validClusters[i];
    const isStrong = level.touches >= 5;
    const isSupport = level.type === "support";

    const color = isSupport
      ? (isStrong ? "#00e676" : "#4caf5099")   // green for support
      : (isStrong ? "#ff1744" : "#f4433699");   // red for resistance

    const lineWidth = isStrong ? 2 : 1;
    const lineStyle = isStrong ? 0 : 2; // solid for strong, dashed for medium

    const s = chart.addLineSeries({
      color,
      lineWidth,
      lineStyle,
      lastValueVisible: true,
      priceLineVisible: false,
      crosshairMarkerVisible: false,
      title: `${level.type === "support" ? "S" : "R"} (${level.touches})`,
    });

    // Line extends from first touch to current bar
    const lineData = data
      .filter(d => d.time >= level.firstTouch)
      .map(d => ({ time: d.time, value: level.price }));

    if (lineData.length >= 2) {
      s.setData(lineData);
      indicatorSeries[`sr_${i}`] = s;
    } else {
      chart.removeSeries(s);
    }
  }

  log(`SRLEVEL: Found ${validClusters.length} levels (${validClusters.filter(l => l.touches >= 5).length} strong)`, "ok");

  // Step 5: Summary floating window
  const win = createWindow({ title: `${currentSymbol} — S/R Levels`, width: 650, height: 400 });
  win.contentElement.textContent = "";

  const table = document.createElement("table");
  table.style.cssText = "border-collapse:collapse;font-size:11px;width:100%;font-family:Consolas,monospace;";

  // Header
  const thead = document.createElement("tr");
  for (const hdr of ["Price Level", "Type", "Touches", "Strength", "First Touch", "Last Touch"]) {
    const th = document.createElement("td");
    th.style.cssText = "padding:4px 8px;border-bottom:1px solid #333;color:#888;font-weight:bold;";
    th.textContent = hdr;
    thead.appendChild(th);
  }
  table.appendChild(thead);

  for (const level of validClusters) {
    const tr = document.createElement("tr");
    const isSupport = level.type === "support";
    const color = isSupport ? "#4caf50" : "#f44336";
    const strength = level.touches >= 5 ? "STRONG" : "MEDIUM";

    const formatDate = (ts) => {
      const d = new Date(ts * 1000);
      return d.toISOString().substring(0, 10);
    };

    const cells = [
      level.price.toFixed(dp),
      isSupport ? "SUPPORT" : "RESISTANCE",
      level.touches.toString(),
      strength,
      formatDate(level.firstTouch),
      formatDate(level.lastTouch),
    ];

    for (let ci = 0; ci < cells.length; ci++) {
      const td = document.createElement("td");
      td.style.cssText = `padding:4px 8px;border-bottom:1px solid #1a1a2e;color:${ci <= 1 ? color : "#ccc"};`;
      td.textContent = cells[ci];
      tr.appendChild(td);
    }
    table.appendChild(tr);
  }

  win.appendElement(table);
}

// ══════════════════════════════════════════════════════════════
// SEASONALITY — Monthly Performance Patterns
// ══════════════════════════════════════════════════════════════

async function cmdSeasonality() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }

  const win = createWindow({ title: `${currentSymbol} — Seasonality`, width: 700, height: 600 });
  win.contentElement.textContent = "";

  const loading = document.createElement("div");
  loading.textContent = "Loading daily bars for seasonality analysis...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);

  // Get daily bars — try cache first, then fetch
  let bars = null;
  const cacheKey = currentSymbol + ":1Day";
  if (barCache[cacheKey] && barCache[cacheKey].data && barCache[cacheKey].data.length > 50) {
    bars = barCache[cacheKey].data;
    barCache[cacheKey].lastAccess = Date.now();
  }

  if (!bars) {
    try {
      const barsJson = await invoke("get_bars", { symbol: currentSymbol, timeframe: "1Day", limit: 2000 });
      bars = JSON.parse(barsJson);
      if (bars && bars.length > 0) {
        barCache[cacheKey] = { data: bars, timestamp: Date.now(), lastAccess: Date.now() };
      }
    } catch (e) {
      win.contentElement.textContent = "";
      win.setContent(`Failed to fetch daily bars: ${e}`);
      return;
    }
  }

  if (!bars || bars.length < 30) {
    win.contentElement.textContent = "";
    win.setContent("Insufficient daily bar data for seasonality analysis.");
    return;
  }

  // Group bars by calendar month and compute monthly returns
  const monthlyReturns = {}; // { 0..11: [return1, return2, ...] }
  const monthNames = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

  // Group bars by year-month
  const yearMonthBars = {}; // "2023-03" -> [bars]
  for (const bar of bars) {
    const d = new Date(bar.time * 1000);
    const key = `${d.getFullYear()}-${String(d.getMonth()).padStart(2, "0")}`;
    if (!yearMonthBars[key]) yearMonthBars[key] = [];
    yearMonthBars[key].push(bar);
  }

  // Compute return for each year-month
  for (const [key, monthBars] of Object.entries(yearMonthBars)) {
    if (monthBars.length < 2) continue;
    const monthIdx = parseInt(key.split("-")[1]);
    const openPrice = monthBars[0].open;
    const closePrice = monthBars[monthBars.length - 1].close;
    if (openPrice <= 0) continue;
    const ret = ((closePrice - openPrice) / openPrice) * 100;

    if (!monthlyReturns[monthIdx]) monthlyReturns[monthIdx] = [];
    monthlyReturns[monthIdx].push(ret);
  }

  // Compute statistics for each month
  const stats = [];
  for (let m = 0; m < 12; m++) {
    const returns = monthlyReturns[m] || [];
    if (returns.length === 0) {
      stats.push({ month: monthNames[m], avgReturn: 0, winRate: 0, best: 0, worst: 0, stdDev: 0, count: 0 });
      continue;
    }
    const avg = returns.reduce((a, b) => a + b, 0) / returns.length;
    const wins = returns.filter(r => r > 0).length;
    const winRate = (wins / returns.length) * 100;
    const best = Math.max(...returns);
    const worst = Math.min(...returns);
    const variance = returns.reduce((sum, r) => sum + (r - avg) ** 2, 0) / returns.length;
    const stdDev = Math.sqrt(variance);

    stats.push({ month: monthNames[m], avgReturn: avg, winRate, best, worst, stdDev, count: returns.length });
  }

  win.contentElement.textContent = "";

  // Bar chart visualization
  const chartDiv = document.createElement("div");
  chartDiv.style.cssText = "display:flex;align-items:flex-end;justify-content:space-around;height:150px;padding:20px 10px 5px;border-bottom:1px solid #333;";

  const maxAbs = Math.max(...stats.map(s => Math.abs(s.avgReturn)), 0.1);

  for (const s of stats) {
    const barContainer = document.createElement("div");
    barContainer.style.cssText = "display:flex;flex-direction:column;align-items:center;flex:1;height:100%;justify-content:flex-end;position:relative;";

    // Value label above bar
    const valLabel = document.createElement("div");
    valLabel.style.cssText = "font-size:9px;color:#aaa;position:absolute;width:100%;text-align:center;";
    valLabel.textContent = s.avgReturn.toFixed(1) + "%";

    const bar = document.createElement("div");
    const heightPct = Math.abs(s.avgReturn) / maxAbs * 60; // max 60% of container height
    const isPositive = s.avgReturn >= 0;
    bar.style.cssText = `width:70%;min-width:20px;background:${isPositive ? "#4caf50" : "#f44336"};border-radius:2px 2px 0 0;transition:height 0.3s;`;
    bar.style.height = Math.max(heightPct, 2) + "%";

    valLabel.style.bottom = `${Math.max(heightPct, 2) + 2}%`;

    const monthLabel = document.createElement("div");
    monthLabel.style.cssText = "font-size:10px;color:#888;margin-top:4px;";
    monthLabel.textContent = s.month;

    barContainer.appendChild(valLabel);
    barContainer.appendChild(bar);
    barContainer.appendChild(monthLabel);
    chartDiv.appendChild(barContainer);
  }

  win.appendElement(chartDiv);

  // Summary info
  const info = document.createElement("div");
  info.style.cssText = "padding:8px 10px;color:#666;font-size:10px;border-bottom:1px solid #222;";
  info.textContent = `${currentSymbol} — ${bars.length} daily bars analyzed | ${Object.keys(yearMonthBars).length} months of data`;
  win.appendElement(info);

  // Table
  const table = document.createElement("table");
  table.style.cssText = "border-collapse:collapse;font-size:11px;width:100%;font-family:Consolas,monospace;";

  // Header
  const thead = document.createElement("tr");
  for (const hdr of ["Month", "Avg Return%", "Win Rate%", "Best%", "Worst%", "StdDev", "Years"]) {
    const th = document.createElement("td");
    th.style.cssText = "padding:4px 8px;border-bottom:1px solid #333;color:#888;font-weight:bold;";
    th.textContent = hdr;
    thead.appendChild(th);
  }
  table.appendChild(thead);

  for (const s of stats) {
    const tr = document.createElement("tr");
    const color = s.avgReturn >= 0 ? "#4caf50" : "#f44336";

    const cells = [
      s.month,
      s.avgReturn.toFixed(2),
      s.winRate.toFixed(1),
      s.best.toFixed(2),
      s.worst.toFixed(2),
      s.stdDev.toFixed(2),
      s.count.toString(),
    ];

    for (let ci = 0; ci < cells.length; ci++) {
      const td = document.createElement("td");
      td.style.cssText = `padding:4px 8px;border-bottom:1px solid #1a1a2e;color:${ci === 0 ? "#ccc" : color};`;
      td.textContent = cells[ci];
      tr.appendChild(td);
    }
    table.appendChild(tr);
  }

  win.appendElement(table);
  log(`SEASONALITY: ${currentSymbol} — ${bars.length} bars, ${Object.keys(yearMonthBars).length} months analyzed`, "ok");
}

// ══════════════════════════════════════════════════════════════
// COMPARE — Chart Comparison Overlay (% change from first visible bar)
// ══════════════════════════════════════════════════════════════

async function cmdCompare() {
  const sym = prompt("Enter comparison symbol (e.g. SPY, QQQ):");
  if (!sym || !sym.trim()) return;
  const compSymbol = sym.trim().toUpperCase();

  try {
    const barsJson = await invoke("get_bars", { symbol: compSymbol, timeframe: "1Day", limit: 500 });
    const bars = JSON.parse(barsJson);
    if (!bars || bars.length === 0) {
      alert(`No data found for ${compSymbol}`);
      return;
    }

    // Build comparison chart data keyed by time
    const compData = bars.map(b => ({
      time: Math.floor(new Date(b.timestamp).getTime() / 1000),
      close: b.close,
    }));

    // Get current chart data closes keyed by time
    const mainByTime = {};
    for (const d of currentChartData) {
      mainByTime[d.time] = d.close;
    }
    const compByTime = {};
    for (const d of compData) {
      compByTime[d.time] = d.close;
    }

    // Find common timestamps
    const commonTimes = Object.keys(mainByTime)
      .map(Number)
      .filter(t => compByTime[t] !== undefined)
      .sort((a, b) => a - b);

    if (commonTimes.length < 2) {
      alert(`Not enough overlapping data between ${currentSymbol} and ${compSymbol}`);
      return;
    }

    // Normalize both to % change from first common bar
    const mainBase = mainByTime[commonTimes[0]];
    const compBase = compByTime[commonTimes[0]];

    const mainNorm = commonTimes.map(t => ({
      time: t,
      value: ((mainByTime[t] - mainBase) / mainBase) * 100,
    }));
    const compNorm = commonTimes.map(t => ({
      time: t,
      value: ((compByTime[t] - compBase) / compBase) * 100,
    }));

    // Add main symbol % change line (white, dashed)
    const mainPctSeries = chart.addLineSeries({
      color: "#FFFFFF",
      lineWidth: 1,
      lineStyle: 2,
      title: `${currentSymbol} %`,
      lastValueVisible: true,
      priceLineVisible: false,
      priceScaleId: "compare",
    });
    mainPctSeries.setData(mainNorm);
    indicatorSeries[`compare_main_${compSymbol}`] = mainPctSeries;

    // Add comparison symbol line (cyan)
    const compSeries = chart.addLineSeries({
      color: "#00BCD4",
      lineWidth: 2,
      title: `${compSymbol} %`,
      lastValueVisible: true,
      priceLineVisible: false,
      priceScaleId: "compare",
    });
    compSeries.setData(compNorm);
    indicatorSeries[`compare_${compSymbol}`] = compSeries;

    // Configure the compare price scale on the left
    chart.priceScale("compare").applyOptions({
      scaleMargins: { top: 0.1, bottom: 0.1 },
      autoScale: true,
    });

  } catch (e) {
    alert(`Failed to load ${compSymbol}: ${e.message || e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// SPREAD — Spread/Ratio Chart (floating window)
// ══════════════════════════════════════════════════════════════

async function cmdSpread() {
  const sym = prompt("Enter second symbol for spread (e.g. SPY, QQQ):");
  if (!sym || !sym.trim()) return;
  const symB = sym.trim().toUpperCase();
  const symA = currentSymbol;

  try {
    // Fetch bars for both symbols
    const [jsonA, jsonB] = await Promise.all([
      invoke("get_bars", { symbol: symA, timeframe: "1Day", limit: 500 }),
      invoke("get_bars", { symbol: symB, timeframe: "1Day", limit: 500 }),
    ]);
    const barsA = JSON.parse(jsonA);
    const barsB = JSON.parse(jsonB);

    if (!barsA.length || !barsB.length) {
      alert(`No data for ${!barsA.length ? symA : symB}`);
      return;
    }

    // Build maps keyed by date string for alignment
    const mapA = {};
    for (const b of barsA) {
      const t = Math.floor(new Date(b.timestamp).getTime() / 1000);
      mapA[t] = b.close;
    }
    const mapB = {};
    for (const b of barsB) {
      const t = Math.floor(new Date(b.timestamp).getTime() / 1000);
      mapB[t] = b.close;
    }

    // Find common timestamps
    const commonTimes = Object.keys(mapA)
      .map(Number)
      .filter(t => mapB[t] !== undefined)
      .sort((a, b) => a - b);

    if (commonTimes.length < 2) {
      alert(`Not enough overlapping data between ${symA} and ${symB}`);
      return;
    }

    // Compute spread and ratio
    const spreadData = commonTimes.map(t => ({
      time: t,
      value: mapA[t] - mapB[t],
    }));
    const ratioData = commonTimes.map(t => ({
      time: t,
      value: mapA[t] / mapB[t],
    }));

    // Create floating window
    const win = createWindow({
      title: `${symA} / ${symB} Spread`,
      width: 700,
      height: 400,
    });
    win.contentElement.textContent = "";
    win.contentElement.style.padding = "0";

    const chartDiv = document.createElement("div");
    chartDiv.style.cssText = "width:100%;height:100%;";
    win.contentElement.appendChild(chartDiv);

    const spreadChart = createChart(chartDiv, {
      layout: {
        background: { color: "#1a1a2e" },
        textColor: "#d1d4dc",
        fontSize: 11,
      },
      grid: {
        vertLines: { color: "rgba(42,46,57,0.5)" },
        horzLines: { color: "rgba(42,46,57,0.5)" },
      },
      crosshair: { mode: CrosshairMode.Normal },
      rightPriceScale: { visible: true, borderColor: "rgba(197,203,206,0.3)" },
      leftPriceScale: { visible: true, borderColor: "rgba(197,203,206,0.3)" },
      timeScale: { borderColor: "rgba(197,203,206,0.3)" },
    });

    // Spread line on left price scale
    const spreadSeries = spreadChart.addLineSeries({
      color: "#2196f3",
      lineWidth: 2,
      title: `Spread (${symA} - ${symB})`,
      priceScaleId: "left",
      lastValueVisible: true,
      priceLineVisible: false,
    });
    spreadSeries.setData(spreadData);

    // Ratio line on right price scale
    const ratioSeries = spreadChart.addLineSeries({
      color: "#ff9800",
      lineWidth: 2,
      title: `Ratio (${symA} / ${symB})`,
      priceScaleId: "right",
      lastValueVisible: true,
      priceLineVisible: false,
    });
    ratioSeries.setData(ratioData);

    spreadChart.timeScale().fitContent();

    // Resize chart when window resizes
    const ro = new ResizeObserver(() => {
      const rect = chartDiv.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        spreadChart.resize(rect.width, rect.height);
      }
    });
    ro.observe(chartDiv);

  } catch (e) {
    alert(`Failed to load spread data: ${e.message || e}`);
  }
}

// ══════════════════════════════════════════════════════════════
// NEW COMMAND PALETTE FEATURES
// ══════════════════════════════════════════════════════════════

// ── Options P&L Calculator (OPTCALC) ────────────────────────
function cmdOptionsCalc() {
  const win = createWindow({ title: "Options P&L Calculator", width: 700, height: 500 });
  win.contentElement.textContent = "";

  const form = document.createElement("div");
  form.style.cssText = "padding:8px;font-size:11px;";

  // Leg inputs
  const legsDiv = document.createElement("div");
  legsDiv.id = "optcalc-legs";
  const hdr = document.createElement("div");
  hdr.style.cssText = "display:flex;gap:4px;color:#888;font-weight:bold;padding:2px 0;border-bottom:1px solid #333;margin-bottom:4px;";
  for (const h of ["B/S", "Type", "Strike", "Premium", "Qty"]) {
    const s = document.createElement("span");
    s.style.cssText = "flex:1;text-align:center;";
    s.textContent = h;
    hdr.appendChild(s);
  }
  legsDiv.appendChild(hdr);

  function addLegRow(bs = "buy", type = "call", strike = "", prem = "", qty = "1") {
    const row = document.createElement("div");
    row.style.cssText = "display:flex;gap:4px;margin:2px 0;";
    const mkSel = (opts, val) => { const s = document.createElement("select"); s.style.cssText = "flex:1;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:2px;"; for (const o of opts) { const op = document.createElement("option"); op.value = o; op.textContent = o; if (o === val) op.selected = true; s.appendChild(op); } return s; };
    const mkInp = (v, ph) => { const i = document.createElement("input"); i.type = "number"; i.step = "0.01"; i.value = v; i.placeholder = ph; i.style.cssText = "flex:1;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:2px;text-align:center;"; return i; };
    row.appendChild(mkSel(["buy", "sell"], bs));
    row.appendChild(mkSel(["call", "put"], type));
    row.appendChild(mkInp(strike, "Strike"));
    row.appendChild(mkInp(prem, "Prem"));
    row.appendChild(mkInp(qty, "Qty"));
    const del = document.createElement("button");
    del.textContent = "×";
    del.style.cssText = "background:none;border:1px solid #f44;color:#f44;cursor:pointer;padding:0 4px;";
    del.addEventListener("click", () => row.remove());
    row.appendChild(del);
    legsDiv.appendChild(row);
  }
  addLegRow("buy", "call", lastPrice ? lastPrice.toFixed(2) : "", "1.00");

  const btnRow = document.createElement("div");
  btnRow.style.cssText = "display:flex;gap:6px;margin:6px 0;";
  const addBtn = document.createElement("button");
  addBtn.textContent = "+ Add Leg";
  addBtn.style.cssText = "font-size:10px;padding:3px 8px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;";
  addBtn.addEventListener("click", () => addLegRow());
  const calcBtn = document.createElement("button");
  calcBtn.textContent = "Calculate Payoff";
  calcBtn.style.cssText = "font-size:10px;padding:3px 12px;background:#1b5e20;color:#8f8;border:1px solid #555;cursor:pointer;font-weight:bold;";
  btnRow.appendChild(addBtn);
  btnRow.appendChild(calcBtn);

  const chartDiv = document.createElement("canvas");
  chartDiv.width = 660;
  chartDiv.height = 300;
  chartDiv.style.cssText = "border:1px solid #333;margin-top:6px;";

  form.appendChild(legsDiv);
  form.appendChild(btnRow);
  form.appendChild(chartDiv);
  win.appendElement(form);

  calcBtn.addEventListener("click", () => {
    const rows = legsDiv.querySelectorAll("div:not(:first-child)");
    const legs = [];
    for (const r of rows) {
      const els = r.querySelectorAll("select, input");
      if (els.length < 5) continue;
      legs.push({ bs: els[0].value, type: els[1].value, strike: parseFloat(els[2].value), prem: parseFloat(els[3].value), qty: parseInt(els[4].value) || 1 });
    }
    if (legs.length === 0 || legs.some(l => isNaN(l.strike) || isNaN(l.prem))) { alert("Fill all fields"); return; }

    // Calculate payoff at expiry across price range
    const strikes = legs.map(l => l.strike);
    const minP = Math.min(...strikes) * 0.7;
    const maxP = Math.max(...strikes) * 1.3;
    const step = (maxP - minP) / 200;
    const points = [];
    let maxProfit = -Infinity, maxLoss = Infinity;
    for (let p = minP; p <= maxP; p += step) {
      let pnl = 0;
      for (const l of legs) {
        const dir = l.bs === "buy" ? 1 : -1;
        let intrinsic = l.type === "call" ? Math.max(0, p - l.strike) : Math.max(0, l.strike - p);
        pnl += dir * (intrinsic - l.prem) * l.qty * 100;
      }
      points.push({ price: p, pnl });
      maxProfit = Math.max(maxProfit, pnl);
      maxLoss = Math.min(maxLoss, pnl);
    }

    // Draw payoff diagram on canvas
    const ctx = chartDiv.getContext("2d");
    const W = chartDiv.width, H = chartDiv.height;
    ctx.clearRect(0, 0, W, H);
    ctx.fillStyle = "#0a0a14";
    ctx.fillRect(0, 0, W, H);
    const pad = 40;
    const pRange = maxP - minP;
    const vRange = Math.max(maxProfit - maxLoss, 1);
    const toX = p => pad + (p - minP) / pRange * (W - 2 * pad);
    const toY = v => H - pad - (v - maxLoss) / vRange * (H - 2 * pad);

    // Zero line
    ctx.strokeStyle = "#ffffff33";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(pad, toY(0));
    ctx.lineTo(W - pad, toY(0));
    ctx.stroke();

    // Payoff line
    ctx.strokeStyle = "#4caf50";
    ctx.lineWidth = 2;
    ctx.beginPath();
    for (let i = 0; i < points.length; i++) {
      const x = toX(points[i].price), y = toY(points[i].pnl);
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.stroke();

    // Fill profit/loss zones
    for (let i = 0; i < points.length - 1; i++) {
      const x1 = toX(points[i].price), x2 = toX(points[i + 1].price);
      const y0 = toY(0);
      const y1 = toY(points[i].pnl), y2 = toY(points[i + 1].pnl);
      ctx.fillStyle = points[i].pnl >= 0 ? "rgba(76,175,80,0.15)" : "rgba(244,67,54,0.15)";
      ctx.beginPath();
      ctx.moveTo(x1, y0); ctx.lineTo(x1, y1); ctx.lineTo(x2, y2); ctx.lineTo(x2, y0);
      ctx.fill();
    }

    // Axis labels
    ctx.fillStyle = "#888";
    ctx.font = "10px Consolas, monospace";
    ctx.textAlign = "center";
    for (let i = 0; i <= 5; i++) {
      const p = minP + (pRange * i / 5);
      ctx.fillText(`$${p.toFixed(0)}`, toX(p), H - 5);
    }
    ctx.textAlign = "right";
    for (let i = 0; i <= 4; i++) {
      const v = maxLoss + (vRange * i / 4);
      ctx.fillText(`$${v.toFixed(0)}`, pad - 4, toY(v) + 3);
    }

    // Max profit/loss labels
    ctx.fillStyle = "#4caf50";
    ctx.textAlign = "left";
    ctx.fillText(`Max Profit: $${maxProfit.toFixed(0)}`, pad + 10, 15);
    ctx.fillStyle = "#f44336";
    ctx.fillText(`Max Loss: $${maxLoss.toFixed(0)}`, pad + 10, 28);

    // Strike price markers
    ctx.strokeStyle = "#ffeb3b44";
    ctx.setLineDash([3, 3]);
    for (const l of legs) {
      const x = toX(l.strike);
      ctx.beginPath();
      ctx.moveTo(x, pad); ctx.lineTo(x, H - pad);
      ctx.stroke();
    }
    ctx.setLineDash([]);
  });
}

// ── Sector Rotation Heatmap (SECTORS) ────────────────────────
async function cmdSectorRotation() {
  const win = createWindow({ title: "Sector Rotation Heatmap", width: 650, height: 400 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Loading sector data...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  const SECTOR_ETFS = [
    { sym: "XLK", name: "Technology" }, { sym: "XLF", name: "Financials" },
    { sym: "XLE", name: "Energy" }, { sym: "XLV", name: "Healthcare" },
    { sym: "XLI", name: "Industrials" }, { sym: "XLP", name: "Consumer Staples" },
    { sym: "XLY", name: "Consumer Disc" }, { sym: "XLU", name: "Utilities" },
    { sym: "XLB", name: "Materials" }, { sym: "XLRE", name: "Real Estate" },
    { sym: "XLC", name: "Comm Services" }, { sym: "SPY", name: "S&P 500" },
    { sym: "QQQ", name: "Nasdaq 100" }, { sym: "IWM", name: "Russell 2000" },
    { sym: "DIA", name: "Dow Jones" }, { sym: "GLD", name: "Gold" },
  ];

  try {
    const items = [];
    for (const etf of SECTOR_ETFS) {
      try {
        const json = await invoke("get_bars", { symbol: etf.sym, timeframe: "1Day", limit: 5 });
        const bars = JSON.parse(json);
        if (bars.length >= 2) {
          const prev = bars[bars.length - 2].close;
          const cur = bars[bars.length - 1].close;
          const pct = ((cur - prev) / prev) * 100;
          // Weekly change
          const weekStart = bars.length >= 5 ? bars[0].close : prev;
          const weekPct = ((cur - weekStart) / weekStart) * 100;
          items.push({ ...etf, price: cur, dailyPct: pct, weeklyPct: weekPct });
        }
      } catch (_) {}
    }

    win.contentElement.textContent = "";
    const grid = document.createElement("div");
    grid.style.cssText = "display:flex;flex-wrap:wrap;gap:4px;padding:8px;";
    const maxPct = Math.max(...items.map(i => Math.abs(i.dailyPct)), 0.5);

    for (const item of items) {
      const box = document.createElement("div");
      const intensity = Math.min(Math.abs(item.dailyPct) / maxPct, 1);
      const bg = item.dailyPct > 0
        ? `rgba(0, ${Math.round(80 + intensity * 175)}, 0, ${0.3 + intensity * 0.5})`
        : item.dailyPct < 0
        ? `rgba(${Math.round(80 + intensity * 175)}, 0, 0, ${0.3 + intensity * 0.5})`
        : "rgba(128,128,128,0.2)";
      box.style.cssText = `width:120px;height:75px;background:${bg};border:1px solid #333;border-radius:4px;display:flex;flex-direction:column;justify-content:center;align-items:center;cursor:pointer;padding:4px;`;

      const sym = document.createElement("div");
      sym.textContent = item.sym;
      sym.style.cssText = "font-size:13px;font-weight:bold;color:#fff;";
      const name = document.createElement("div");
      name.textContent = item.name;
      name.style.cssText = "font-size:9px;color:#aaa;";
      const daily = document.createElement("div");
      daily.textContent = `${item.dailyPct >= 0 ? "+" : ""}${item.dailyPct.toFixed(2)}%`;
      daily.style.cssText = `font-size:12px;font-weight:bold;color:${item.dailyPct >= 0 ? "#4caf50" : "#f44336"};`;
      const weekly = document.createElement("div");
      weekly.textContent = `W: ${item.weeklyPct >= 0 ? "+" : ""}${item.weeklyPct.toFixed(2)}%`;
      weekly.style.cssText = `font-size:9px;color:${item.weeklyPct >= 0 ? "#8f8" : "#f88"};`;

      box.appendChild(sym);
      box.appendChild(name);
      box.appendChild(daily);
      box.appendChild(weekly);
      box.addEventListener("click", () => {
        document.getElementById("symbol-input").value = item.sym;
        triggerLoad();
      });
      grid.appendChild(box);
    }
    win.appendElement(grid);
  } catch (e) { win.setContent(`Error: ${e}`); }
}

// ── Economic Calendar with Countdown (ECON) ──────────────────
async function cmdEconCalendar() {
  const win = createWindow({ title: "Economic Calendar", width: 600, height: 450 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Loading calendar...";
  loading.style.cssText = "color:#888;padding:12px;";
  win.appendElement(loading);

  try {
    const json = await invoke("get_corporate_actions", { symbol: "SPY", types: "dividend" });
    const calJson = await invoke("get_bars", { symbol: "SPY", timeframe: "1Day", limit: 5 });

    win.contentElement.textContent = "";

    // Key upcoming dates
    const now = new Date();
    const events = [
      { name: "Market Open", time: getNextMarketTime(9, 30), impact: "low" },
      { name: "Market Close", time: getNextMarketTime(16, 0), impact: "low" },
      { name: "FOMC Meeting", time: getNextFOMC(), impact: "high" },
      { name: "CPI Release", time: getNextCPI(), impact: "high" },
      { name: "NFP (Jobs)", time: getNextNFP(), impact: "high" },
      { name: "GDP Release", time: getNextGDP(), impact: "medium" },
    ];

    events.sort((a, b) => a.time - b.time);

    const table = document.createElement("div");
    table.style.cssText = "padding:8px;";

    const hdr = document.createElement("div");
    hdr.style.cssText = "display:flex;gap:8px;padding:4px 0;border-bottom:1px solid #444;color:#888;font-size:10px;font-weight:bold;";
    for (const h of ["Event", "Date/Time", "Countdown", "Impact"]) {
      const s = document.createElement("span");
      s.style.cssText = h === "Event" ? "flex:2;" : "flex:1;text-align:center;";
      s.textContent = h;
      hdr.appendChild(s);
    }
    table.appendChild(hdr);

    for (const ev of events) {
      const row = document.createElement("div");
      row.style.cssText = "display:flex;gap:8px;padding:4px 0;border-bottom:1px solid #1a1a2e;font-size:11px;";

      const nameEl = document.createElement("span");
      nameEl.style.cssText = "flex:2;color:#ccc;";
      nameEl.textContent = ev.name;

      const dateEl = document.createElement("span");
      dateEl.style.cssText = "flex:1;text-align:center;color:#aaa;";
      dateEl.textContent = ev.time.toLocaleDateString() + " " + ev.time.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

      const countEl = document.createElement("span");
      countEl.style.cssText = "flex:1;text-align:center;color:#ff8;font-family:Consolas,monospace;";
      const diff = ev.time - now;
      if (diff > 0) {
        const d = Math.floor(diff / 86400000);
        const h = Math.floor((diff % 86400000) / 3600000);
        const m = Math.floor((diff % 3600000) / 60000);
        countEl.textContent = d > 0 ? `${d}d ${h}h ${m}m` : `${h}h ${m}m`;
      } else {
        countEl.textContent = "PASSED";
        countEl.style.color = "#888";
      }

      const impactEl = document.createElement("span");
      impactEl.style.cssText = "flex:1;text-align:center;font-weight:bold;";
      impactEl.style.color = ev.impact === "high" ? "#f44336" : ev.impact === "medium" ? "#ff9800" : "#4caf50";
      impactEl.textContent = ev.impact.toUpperCase();

      row.appendChild(nameEl);
      row.appendChild(dateEl);
      row.appendChild(countEl);
      row.appendChild(impactEl);
      table.appendChild(row);
    }
    win.appendElement(table);
  } catch (e) { win.setContent(`Error: ${e}`); }
}

function getNextMarketTime(hour, min) {
  const d = new Date();
  d.setHours(hour, min, 0, 0);
  if (d <= new Date()) d.setDate(d.getDate() + 1);
  while (d.getDay() === 0 || d.getDay() === 6) d.setDate(d.getDate() + 1);
  return d;
}
function getNextFOMC() { const d = new Date(); d.setMonth(d.getMonth() + 1); d.setDate(15); d.setHours(14, 0, 0, 0); while (d.getDay() !== 3) d.setDate(d.getDate() + 1); return d; }
function getNextCPI() { const d = new Date(); d.setDate(13); if (d <= new Date()) d.setMonth(d.getMonth() + 1); d.setHours(8, 30, 0, 0); return d; }
function getNextNFP() { const d = new Date(); d.setDate(1); if (d <= new Date()) d.setMonth(d.getMonth() + 1); while (d.getDay() !== 5) d.setDate(d.getDate() + 1); d.setHours(8, 30, 0, 0); return d; }
function getNextGDP() { const d = new Date(); d.setDate(28); if (d <= new Date()) d.setMonth(d.getMonth() + 1); d.setHours(8, 30, 0, 0); return d; }

// ── Options Strategy Builder (OPTSTRAT) ──────────────────────
async function cmdOptionsStrategy() {
  if (!currentSymbol) { alert("Load a chart first"); return; }
  const win = createWindow({ title: `Options Strategy: ${currentSymbol}`, width: 750, height: 550 });
  win.contentElement.textContent = "";

  const toolbar = document.createElement("div");
  toolbar.style.cssText = "display:flex;gap:6px;padding:8px;border-bottom:1px solid #333;flex-wrap:wrap;";

  // Preset strategies
  const presets = ["Custom", "Long Call", "Long Put", "Bull Call Spread", "Bear Put Spread", "Straddle", "Iron Condor"];
  const presetSel = document.createElement("select");
  presetSel.style.cssText = "font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
  for (const p of presets) { const o = document.createElement("option"); o.value = p; o.textContent = p; presetSel.appendChild(o); }

  // Expiry
  const expiryInput = document.createElement("input");
  expiryInput.type = "date";
  expiryInput.style.cssText = "font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
  const nextFri = new Date(); nextFri.setDate(nextFri.getDate() + (5 - nextFri.getDay() + 7) % 7 + 7);
  expiryInput.value = nextFri.toISOString().split("T")[0];

  const loadBtn = document.createElement("button");
  loadBtn.textContent = "Load Chain";
  loadBtn.style.cssText = "font-size:10px;padding:3px 10px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;";

  toolbar.appendChild(presetSel);
  toolbar.appendChild(expiryInput);
  toolbar.appendChild(loadBtn);
  win.appendElement(toolbar);

  const content = document.createElement("div");
  content.style.cssText = "padding:8px;font-size:10px;overflow-y:auto;max-height:450px;";
  content.textContent = "Select expiry and click Load Chain to view options.";
  win.appendElement(content);

  loadBtn.addEventListener("click", async () => {
    content.textContent = "Loading options chain...";
    try {
      const json = await invoke("get_options", { symbol: currentSymbol, expiry: expiryInput.value });
      const chain = JSON.parse(json);
      if (chain.length === 0) { content.textContent = "No options data available"; return; }

      content.textContent = "";

      // Split calls and puts
      const calls = chain.filter(c => c.option_type === "call");
      const puts = chain.filter(c => c.option_type === "put");

      const table = document.createElement("table");
      table.style.cssText = "width:100%;border-collapse:collapse;font-size:10px;";
      const thead = document.createElement("thead");
      const hr = document.createElement("tr");
      hr.style.cssText = "border-bottom:1px solid #444;";
      for (const h of ["Call Bid", "Call Ask", "Delta", "Strike", "Delta", "Put Bid", "Put Ask"]) {
        const th = document.createElement("th");
        th.style.cssText = "padding:3px 4px;color:#888;text-align:center;";
        th.textContent = h;
        hr.appendChild(th);
      }
      thead.appendChild(hr);
      table.appendChild(thead);

      const tbody = document.createElement("tbody");
      const allStrikes = [...new Set(chain.map(c => c.strike))].sort((a, b) => a - b);
      for (const strike of allStrikes) {
        const call = calls.find(c => c.strike === strike);
        const put = puts.find(c => c.strike === strike);
        const tr = document.createElement("tr");
        const itm = lastPrice && strike < lastPrice;
        tr.style.cssText = `border-bottom:1px solid #1a1a2e;${itm ? "background:rgba(76,175,80,0.05);" : ""}`;

        const vals = [
          call ? call.bid.toFixed(2) : "—", call ? call.ask.toFixed(2) : "—",
          call ? call.delta.toFixed(3) : "—", strike.toFixed(2),
          put ? put.delta.toFixed(3) : "—", put ? put.bid.toFixed(2) : "—",
          put ? put.ask.toFixed(2) : "—",
        ];
        for (let i = 0; i < vals.length; i++) {
          const td = document.createElement("td");
          td.style.cssText = `padding:2px 4px;text-align:center;color:${i === 3 ? "#fff" : "#ccc"};${i === 3 ? "font-weight:bold;background:#1a1a2e;" : ""}`;
          td.textContent = vals[i];
          tr.appendChild(td);
        }
        tbody.appendChild(tr);
      }
      table.appendChild(tbody);
      content.appendChild(table);

      // Aggregate Greeks summary
      const summary = document.createElement("div");
      summary.style.cssText = "margin-top:8px;padding:6px;border-top:1px solid #333;color:#888;";
      const totalDelta = chain.reduce((s, c) => s + c.delta, 0);
      const totalGamma = chain.reduce((s, c) => s + c.gamma, 0);
      const totalTheta = chain.reduce((s, c) => s + c.theta, 0);
      const totalVega = chain.reduce((s, c) => s + c.vega, 0);
      summary.textContent = `Aggregate Greeks — Delta: ${totalDelta.toFixed(3)} | Gamma: ${totalGamma.toFixed(4)} | Theta: ${totalTheta.toFixed(3)} | Vega: ${totalVega.toFixed(3)}`;
      content.appendChild(summary);
    } catch (e) { content.textContent = `Error: ${e}`; }
  });
}

// ── Strategy Auto-Trading (AUTOTRADE) ────────────────────────
function cmdAutoTrade() {
  const win = createWindow({ title: "Strategy Auto-Trading", width: 550, height: 400 });
  win.contentElement.textContent = "";

  const info = document.createElement("div");
  info.style.cssText = "padding:12px;font-size:11px;color:#ccc;";

  const title = document.createElement("div");
  title.textContent = "Auto-Trade via JS Plugin";
  title.style.cssText = "font-size:14px;font-weight:bold;color:#ff8;margin-bottom:8px;";
  info.appendChild(title);

  const desc = document.createElement("div");
  desc.style.cssText = "color:#aaa;margin-bottom:12px;line-height:1.6;";
  desc.textContent = "Create a JS plugin in ~/.config/typhoon-terminal/indicators/ that exports an onSignal() function. " +
    "The function receives bar data and indicator values, and returns { action: 'buy'|'sell'|'close', qty: N }. " +
    "Enable auto-trading below to execute signals automatically.";
  info.appendChild(desc);

  // Plugin selector
  const pluginRow = document.createElement("div");
  pluginRow.style.cssText = "display:flex;gap:6px;align-items:center;margin:8px 0;";
  const pluginLabel = document.createElement("label");
  pluginLabel.textContent = "Plugin:";
  pluginLabel.style.cssText = "color:#888;";
  const pluginSel = document.createElement("select");
  pluginSel.style.cssText = "flex:1;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:4px;";
  const noneOpt = document.createElement("option");
  noneOpt.value = "";
  noneOpt.textContent = "— Select Plugin —";
  pluginSel.appendChild(noneOpt);
  pluginRow.appendChild(pluginLabel);
  pluginRow.appendChild(pluginSel);
  info.appendChild(pluginRow);

  // Load available plugins
  invoke("list_custom_indicators").then(json => {
    const plugins = JSON.parse(json);
    for (const p of plugins) {
      const opt = document.createElement("option");
      opt.value = p.filename;
      opt.textContent = p.name;
      pluginSel.appendChild(opt);
    }
  }).catch(() => {});

  // Settings
  const settingsDiv = document.createElement("div");
  settingsDiv.style.cssText = "border:1px solid #333;border-radius:4px;padding:8px;margin:8px 0;";
  const mkRow = (label, id, val) => {
    const r = document.createElement("div");
    r.style.cssText = "display:flex;justify-content:space-between;align-items:center;margin:4px 0;";
    const l = document.createElement("span");
    l.textContent = label;
    l.style.cssText = "color:#aaa;font-size:10px;";
    const i = document.createElement("input");
    i.id = id;
    i.value = val;
    i.style.cssText = "width:80px;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;text-align:center;";
    r.appendChild(l);
    r.appendChild(i);
    return r;
  };

  settingsDiv.appendChild(mkRow("Max position size:", "at-max-qty", "100"));
  settingsDiv.appendChild(mkRow("Cooldown (seconds):", "at-cooldown", "60"));

  const paperOnly = document.createElement("div");
  paperOnly.style.cssText = "display:flex;align-items:center;gap:6px;margin:4px 0;";
  const paperCb = document.createElement("input");
  paperCb.type = "checkbox";
  paperCb.checked = true;
  paperCb.id = "at-paper-only";
  const paperLabel = document.createElement("label");
  paperLabel.textContent = "Paper trading only (safety)";
  paperLabel.style.cssText = "color:#ff8;font-size:10px;";
  paperOnly.appendChild(paperCb);
  paperOnly.appendChild(paperLabel);
  settingsDiv.appendChild(paperOnly);

  info.appendChild(settingsDiv);

  // Enable/disable toggle
  const toggleBtn = document.createElement("button");
  toggleBtn.textContent = "Enable Auto-Trading";
  toggleBtn.style.cssText = "padding:6px 16px;font-size:11px;background:#1b5e20;color:#8f8;border:1px solid #555;cursor:pointer;font-weight:bold;width:100%;";
  let autoTradeActive = false;
  toggleBtn.addEventListener("click", () => {
    autoTradeActive = !autoTradeActive;
    toggleBtn.textContent = autoTradeActive ? "STOP Auto-Trading" : "Enable Auto-Trading";
    toggleBtn.style.background = autoTradeActive ? "#b71c1c" : "#1b5e20";
    toggleBtn.style.color = autoTradeActive ? "#faa" : "#8f8";
    log(`Auto-trading ${autoTradeActive ? "ENABLED" : "DISABLED"}`, autoTradeActive ? "warn" : "ok");
  });
  info.appendChild(toggleBtn);

  win.appendElement(info);
}

// ── Watchlist SMA200 Cross Alerts ────────────────────────────
// (Runs automatically in dashboard cycle if watchlist exists)
let watchlistSMA200Cache = {};

// ── Matrix Community Chat (CHAT) ─────────────────────────────
let matrixState = {
  homeserver: "https://matrix.org",
  accessToken: "",
  userId: "",
  roomId: "",
  roomAlias: "#typhoon-terminal:matrix.org",
  nextBatch: "",
  pollActive: false,
};

// Load saved Matrix config from localStorage
try {
  const saved = localStorage.getItem("matrix_config");
  if (saved) {
    const parsed = JSON.parse(saved);
    if (parsed.homeserver) matrixState.homeserver = parsed.homeserver;
    if (parsed.accessToken) matrixState.accessToken = parsed.accessToken;
    if (parsed.userId) matrixState.userId = parsed.userId;
    if (parsed.roomId) matrixState.roomId = parsed.roomId;
    if (parsed.roomAlias) matrixState.roomAlias = parsed.roomAlias;
  }
} catch (_) {}

function saveMatrixConfig() {
  try {
    localStorage.setItem("matrix_config", JSON.stringify({
      homeserver: matrixState.homeserver,
      accessToken: matrixState.accessToken,
      userId: matrixState.userId,
      roomId: matrixState.roomId,
      roomAlias: matrixState.roomAlias,
    }));
  } catch (_) {}
}

function cmdMatrixChat() {
  const win = createWindow({ title: "Community Chat (Matrix)", width: 500, height: 550 });
  win.contentElement.textContent = "";

  const container = document.createElement("div");
  container.style.cssText = "display:flex;flex-direction:column;height:100%;font-size:11px;";

  // ── Login / Config bar ──
  const configBar = document.createElement("div");
  configBar.style.cssText = "padding:6px;border-bottom:1px solid #333;display:flex;flex-direction:column;gap:4px;";

  if (!matrixState.accessToken) {
    // Login form
    const loginRow1 = document.createElement("div");
    loginRow1.style.cssText = "display:flex;gap:4px;";
    const hsInput = document.createElement("input");
    hsInput.placeholder = "Homeserver (https://matrix.org)";
    hsInput.value = matrixState.homeserver;
    hsInput.style.cssText = "flex:2;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
    const roomInput = document.createElement("input");
    roomInput.placeholder = "Room (#typhoon-terminal:matrix.org)";
    roomInput.value = matrixState.roomAlias;
    roomInput.style.cssText = "flex:2;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
    loginRow1.appendChild(hsInput);
    loginRow1.appendChild(roomInput);

    const loginRow2 = document.createElement("div");
    loginRow2.style.cssText = "display:flex;gap:4px;";
    const userInput = document.createElement("input");
    userInput.placeholder = "Username";
    userInput.style.cssText = "flex:1;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
    const passInput = document.createElement("input");
    passInput.placeholder = "Password";
    passInput.type = "password";
    passInput.style.cssText = "flex:1;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
    const loginBtn = document.createElement("button");
    loginBtn.textContent = "Login";
    loginBtn.style.cssText = "font-size:10px;padding:3px 10px;background:#1b5e20;color:#8f8;border:1px solid #555;cursor:pointer;";
    loginRow2.appendChild(userInput);
    loginRow2.appendChild(passInput);
    loginRow2.appendChild(loginBtn);

    configBar.appendChild(loginRow1);
    configBar.appendChild(loginRow2);

    loginBtn.addEventListener("click", async () => {
      loginBtn.textContent = "Logging in...";
      loginBtn.disabled = true;
      try {
        matrixState.homeserver = hsInput.value || "https://matrix.org";
        matrixState.roomAlias = roomInput.value || "#typhoon-terminal:matrix.org";
        const json = await invoke("matrix_login", {
          homeserver: matrixState.homeserver,
          username: userInput.value,
          password: passInput.value,
        });
        const result = JSON.parse(json);
        matrixState.accessToken = result.access_token;
        matrixState.userId = result.user_id;
        saveMatrixConfig();

        // Join the room
        try {
          const roomId = await invoke("matrix_join", {
            homeserver: matrixState.homeserver,
            accessToken: matrixState.accessToken,
            room: matrixState.roomAlias,
          });
          matrixState.roomId = roomId;
          saveMatrixConfig();
        } catch (e) {
          log(`Matrix join failed: ${e}`, "warn");
        }

        // Rebuild the chat window
        cmdMatrixChat();
      } catch (e) {
        loginBtn.textContent = "Login";
        loginBtn.disabled = false;
        alert(`Matrix login failed: ${e}`);
      }
    });
  } else {
    // Logged in status
    const statusRow = document.createElement("div");
    statusRow.style.cssText = "display:flex;justify-content:space-between;align-items:center;";
    const userLabel = document.createElement("span");
    userLabel.textContent = `${matrixState.userId} — ${matrixState.roomAlias}`;
    userLabel.style.cssText = "color:#8f8;font-size:10px;";
    const logoutBtn = document.createElement("button");
    logoutBtn.textContent = "Logout";
    logoutBtn.style.cssText = "font-size:9px;padding:2px 6px;background:#b71c1c;color:#faa;border:1px solid #555;cursor:pointer;";
    logoutBtn.addEventListener("click", () => {
      matrixState.accessToken = "";
      matrixState.userId = "";
      matrixState.roomId = "";
      matrixState.nextBatch = "";
      matrixState.pollActive = false;
      saveMatrixConfig();
      cmdMatrixChat();
    });
    statusRow.appendChild(userLabel);
    statusRow.appendChild(logoutBtn);
    configBar.appendChild(statusRow);
  }
  container.appendChild(configBar);

  // ── Message area ──
  const msgArea = document.createElement("div");
  msgArea.style.cssText = "flex:1;overflow-y:auto;padding:6px;min-height:300px;";
  container.appendChild(msgArea);

  // ── Input bar ──
  if (matrixState.accessToken) {
    const inputBar = document.createElement("div");
    inputBar.style.cssText = "display:flex;gap:4px;padding:6px;border-top:1px solid #333;";
    const msgInput = document.createElement("input");
    msgInput.placeholder = "Type a message...";
    msgInput.style.cssText = "flex:1;font-size:11px;background:#111;color:#ccc;border:1px solid #333;padding:4px 6px;";
    const sendBtn = document.createElement("button");
    sendBtn.textContent = "Send";
    sendBtn.style.cssText = "font-size:10px;padding:4px 10px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;";

    const doSend = async () => {
      const msg = msgInput.value.trim();
      if (!msg || !matrixState.roomId) return;
      msgInput.value = "";
      try {
        await invoke("matrix_send", {
          homeserver: matrixState.homeserver,
          accessToken: matrixState.accessToken,
          roomId: matrixState.roomId,
          message: msg,
        });
      } catch (e) {
        log(`Matrix send failed: ${e}`, "error");
        alert(`Send failed: ${e}`);
      }
    };

    sendBtn.addEventListener("click", doSend);
    msgInput.addEventListener("keydown", (e) => {
      if (e.key === "Enter") { e.preventDefault(); doSend(); }
    });

    inputBar.appendChild(msgInput);
    inputBar.appendChild(sendBtn);
    container.appendChild(inputBar);

    // ── Start polling for messages ──
    matrixState.pollActive = true;
    const poll = async () => {
      while (matrixState.pollActive) {
        try {
          const json = await invoke("matrix_poll", {
            homeserver: matrixState.homeserver,
            accessToken: matrixState.accessToken,
            since: matrixState.nextBatch || null,
          });
          const result = JSON.parse(json);
          matrixState.nextBatch = result.next_batch;

          for (const msg of result.messages) {
            const row = document.createElement("div");
            row.style.cssText = "margin:2px 0;line-height:1.4;";

            const time = new Date(msg.timestamp);
            const timeStr = time.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

            const timeEl = document.createElement("span");
            timeEl.textContent = `[${timeStr}] `;
            timeEl.style.cssText = "color:#666;font-size:9px;";

            const senderEl = document.createElement("span");
            const senderName = msg.sender.split(":")[0].replace("@", "");
            const isMe = msg.sender === matrixState.userId;
            senderEl.textContent = `${senderName}: `;
            senderEl.style.cssText = `font-weight:bold;color:${isMe ? "#8cf" : "#ff8"};`;

            const bodyEl = document.createElement("span");
            bodyEl.textContent = msg.body;
            bodyEl.style.cssText = "color:#ccc;";

            row.appendChild(timeEl);
            row.appendChild(senderEl);
            row.appendChild(bodyEl);
            msgArea.appendChild(row);
          }

          if (result.messages.length > 0) {
            msgArea.scrollTop = msgArea.scrollHeight;
          }
        } catch (_) {
          // Sync timeout or error — retry
          await new Promise(r => setTimeout(r, 2000));
        }
      }
    };
    poll();

    // Stop polling when window closes
    const origClose = win.close;
    win.close = () => {
      matrixState.pollActive = false;
      if (origClose) origClose();
    };
  } else {
    const info = document.createElement("div");
    info.textContent = "Log in with your Matrix account to join the community chat.";
    info.style.cssText = "color:#888;padding:12px;text-align:center;";
    msgArea.appendChild(info);
  }

  win.appendElement(container);
}

// ══════════════════════════════════════════════════════════════
// SCANNER+ — Multi-Condition Stock Screener
// ══════════════════════════════════════════════════════════════
async function cmdScannerPlus() {
  const win = createWindow({ title: "SCANNER+ \u2014 Multi-Condition Screener", width: 720, height: 550 });
  win.contentElement.textContent = "";
  const form = document.createElement("div");
  form.style.cssText = "display:flex;flex-wrap:wrap;gap:8px;padding:8px;border-bottom:1px solid #333;align-items:flex-end;";
  const mkSelect = (label, options, defaultVal) => {
    const wrap = document.createElement("div"); wrap.style.cssText = "display:flex;flex-direction:column;gap:2px;";
    const lbl = document.createElement("label"); lbl.textContent = label; lbl.style.cssText = "color:#888;font-size:9px;";
    const sel = document.createElement("select"); sel.style.cssText = "font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
    for (const o of options) { const op = document.createElement("option"); op.value = o.value !== undefined ? o.value : o; op.textContent = o.label !== undefined ? o.label : o; if (op.value === defaultVal) op.selected = true; sel.appendChild(op); }
    wrap.appendChild(lbl); wrap.appendChild(sel); return { wrap, sel };
  };
  const mkInput = (label, val, width) => {
    width = width || "60px";
    const wrap = document.createElement("div"); wrap.style.cssText = "display:flex;flex-direction:column;gap:2px;";
    const lbl = document.createElement("label"); lbl.textContent = label; lbl.style.cssText = "color:#888;font-size:9px;";
    const inp = document.createElement("input"); inp.type = "number"; inp.step = "any"; inp.value = val;
    inp.style.cssText = "width:" + width + ";font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
    wrap.appendChild(lbl); wrap.appendChild(inp); return { wrap, inp };
  };
  const rsiCond = mkSelect("RSI Condition", [{ value: "<", label: "<" }, { value: ">", label: ">" }], "<");
  const rsiVal = mkInput("RSI Value", "30");
  const smaCond = mkSelect("Price vs SMA200", [{ value: "above", label: "Above" }, { value: "below", label: "Below" }], "below");
  const volCond = mkSelect("Volume >", [{ value: ">", label: "> Nx Avg" }], ">");
  const volVal = mkInput("Vol Multiple", "1.5");
  const minP = mkInput("Min Price", "5"); const maxP = mkInput("Max Price", "500");
  form.appendChild(rsiCond.wrap); form.appendChild(rsiVal.wrap); form.appendChild(smaCond.wrap);
  form.appendChild(volCond.wrap); form.appendChild(volVal.wrap); form.appendChild(minP.wrap); form.appendChild(maxP.wrap);
  const scanBtn = document.createElement("button"); scanBtn.textContent = "Scan";
  scanBtn.style.cssText = "font-size:11px;padding:4px 14px;background:#1b5e20;color:#8f8;border:1px solid #555;cursor:pointer;font-weight:bold;";
  form.appendChild(scanBtn); win.appendElement(form);
  const results = document.createElement("div"); results.style.cssText = "padding:4px;font-size:10px;overflow-y:auto;max-height:420px;";
  results.textContent = "Configure filters and click Scan."; win.appendElement(results);
  scanBtn.addEventListener("click", async () => {
    results.textContent = "";
    const progressEl = document.createElement("div"); progressEl.style.cssText = "color:#8cf;padding:8px;"; progressEl.textContent = "Scanning... fetching candidates"; results.appendChild(progressEl);
    try {
      const minPrice = parseFloat(minP.inp.value) || 5, maxPrice = parseFloat(maxP.inp.value) || 500;
      const rsiThresh = parseFloat(rsiVal.inp.value) || 30, rsiOp = rsiCond.sel.value, smaDir = smaCond.sel.value, volMult = parseFloat(volVal.inp.value) || 1.5;
      const activeJson = await invoke("get_most_active", { top: 100 }); const active = JSON.parse(activeJson);
      const moversJson = await invoke("get_top_movers", { marketType: "stocks", top: 50 }); const movers = JSON.parse(moversJson);
      const symbols = [];
      const addSymbols = (data, key) => { const arr = data[key] || data.most_actives || data.gainers || data.losers || []; for (const item of (Array.isArray(arr) ? arr : [])) { const sym = item.symbol || item.S || ""; const pr = item.price || item.p || 0; if (!sym || symbols.find(s => s.symbol === sym)) continue; if (pr > 0 && (pr < minPrice || pr > maxPrice)) continue; symbols.push({ symbol: sym, price: pr, change_pct: item.change_percent || item.percent_change || 0 }); } };
      addSymbols(active, "most_actives"); addSymbols(movers, "gainers"); addSymbols(movers, "losers");
      const candidates = symbols.slice(0, 20); const matches = [];
      for (let i = 0; i < candidates.length; i++) {
        const sym = candidates[i]; progressEl.textContent = "Scanning... " + (i + 1) + "/" + candidates.length + " (" + sym.symbol + ")";
        try {
          const barsJson = await invoke("get_bars", { symbol: sym.symbol, timeframe: "1Day", limit: 220 }); const bars = JSON.parse(barsJson);
          if (bars.length < 30) continue;
          const rsiData = calcRSI(bars, 14); const latestRSI = rsiData.length > 0 ? rsiData[rsiData.length - 1].value : null;
          let sma200 = null;
          if (bars.length >= 200) { let sum = 0; for (let j = bars.length - 200; j < bars.length; j++) sum += bars[j].close; sma200 = sum / 200; }
          let avgVol = 0; const volSlice = bars.slice(-20); for (const b of volSlice) avgVol += (b.volume || 0); avgVol = avgVol / volSlice.length;
          const curPrice = bars[bars.length - 1].close, curVol = bars[bars.length - 1].volume || 0, volRatio = avgVol > 0 ? curVol / avgVol : 0;
          if (latestRSI !== null) { if (rsiOp === "<" && latestRSI >= rsiThresh) continue; if (rsiOp === ">" && latestRSI <= rsiThresh) continue; }
          if (sma200 !== null) { if (smaDir === "above" && curPrice <= sma200) continue; if (smaDir === "below" && curPrice >= sma200) continue; }
          if (volRatio < volMult) continue;
          matches.push({ symbol: sym.symbol, price: curPrice, rsi: latestRSI, vsSMA200: sma200 !== null ? (curPrice > sma200 ? "ABOVE" : "BELOW") : "N/A", sma200: sma200, volRatio: volRatio, changePct: sym.change_pct });
        } catch (_) {}
      }
      results.textContent = "";
      const hdr = document.createElement("div"); hdr.style.cssText = "color:#888;font-size:9px;margin-bottom:4px;"; hdr.textContent = matches.length + " matches from " + candidates.length + " scanned"; results.appendChild(hdr);
      if (matches.length === 0) { const noRes = document.createElement("div"); noRes.textContent = "No symbols matched all conditions."; noRes.style.cssText = "color:#888;padding:8px;"; results.appendChild(noRes); return; }
      const tblHdr = document.createElement("div"); tblHdr.style.cssText = "display:flex;justify-content:space-between;padding:3px 4px;border-bottom:1px solid #444;color:#666;font-weight:bold;font-size:9px;";
      for (const h of ["Symbol", "Price", "RSI", "vs SMA200", "Vol Ratio", "Change%"]) { const s = document.createElement("span"); s.style.cssText = "flex:1;text-align:center;"; s.textContent = h; tblHdr.appendChild(s); }
      results.appendChild(tblHdr);
      for (const m of matches) {
        const row = document.createElement("div"); row.style.cssText = "display:flex;justify-content:space-between;padding:3px 4px;border-bottom:1px solid #1a1a2e;cursor:pointer;";
        row.addEventListener("mouseenter", function() { this.style.background = "#1a1a2e"; }); row.addEventListener("mouseleave", function() { this.style.background = ""; });
        row.addEventListener("click", function() { document.getElementById("symbol-input").value = m.symbol; triggerLoad(); });
        var vals = [
          { text: m.symbol, css: "color:#fff;font-weight:bold;" }, { text: "$" + m.price.toFixed(2), css: "color:#ccc;font-family:Consolas,monospace;" },
          { text: m.rsi !== null ? m.rsi.toFixed(1) : "\u2014", css: "color:" + (m.rsi !== null && m.rsi < 30 ? "#4caf50" : m.rsi !== null && m.rsi > 70 ? "#f44336" : "#ccc") + ";" },
          { text: m.vsSMA200, css: "color:" + (m.vsSMA200 === "ABOVE" ? "#4caf50" : m.vsSMA200 === "BELOW" ? "#f44336" : "#888") + ";" },
          { text: m.volRatio.toFixed(2) + "x", css: "color:" + (m.volRatio > 2 ? "#ff9800" : "#ccc") + ";" },
          { text: (m.changePct >= 0 ? "+" : "") + m.changePct.toFixed(2) + "%", css: "color:" + (m.changePct >= 0 ? "#4caf50" : "#f44336") + ";font-weight:bold;" },
        ];
        for (const v of vals) { const s = document.createElement("span"); s.style.cssText = "flex:1;text-align:center;" + v.css; s.textContent = v.text; row.appendChild(s); }
        results.appendChild(row);
      }
    } catch (e) { results.textContent = "Error: " + e; }
  });
}

// ══════════════════════════════════════════════════════════════
// OPTPROFIT — Options P&L Simulator with Time Decay
// ══════════════════════════════════════════════════════════════
function cmdOptProfit() {
  const win = createWindow({ title: "OPTPROFIT \u2014 Options P&L Simulator", width: 750, height: 600 });
  win.contentElement.textContent = "";
  const container = document.createElement("div"); container.style.cssText = "padding:8px;font-size:11px;";
  const presetBar = document.createElement("div"); presetBar.style.cssText = "display:flex;gap:4px;margin-bottom:8px;flex-wrap:wrap;";
  const presetLabel = document.createElement("span"); presetLabel.textContent = "Presets:"; presetLabel.style.cssText = "color:#888;font-size:9px;align-self:center;margin-right:4px;"; presetBar.appendChild(presetLabel);
  const price = lastPrice || 100;
  const presets = { "Long Call": [{ type: "call", strike: price, prem: 3.0, qty: 1 }], "Long Put": [{ type: "put", strike: price, prem: 3.0, qty: 1 }], "Bull Call Spread": [{ type: "call", strike: Math.round(price * 0.97), prem: 5.0, qty: 1 }, { type: "call", strike: Math.round(price * 1.03), prem: 2.0, qty: -1 }], "Bear Put Spread": [{ type: "put", strike: Math.round(price * 1.03), prem: 5.0, qty: 1 }, { type: "put", strike: Math.round(price * 0.97), prem: 2.0, qty: -1 }], "Iron Condor": [{ type: "put", strike: Math.round(price * 0.90), prem: 1.0, qty: 1 }, { type: "put", strike: Math.round(price * 0.95), prem: 2.5, qty: -1 }, { type: "call", strike: Math.round(price * 1.05), prem: 2.5, qty: -1 }, { type: "call", strike: Math.round(price * 1.10), prem: 1.0, qty: 1 }], "Straddle": [{ type: "call", strike: price, prem: 4.0, qty: 1 }, { type: "put", strike: price, prem: 4.0, qty: 1 }] };
  const legsDiv = document.createElement("div"); legsDiv.style.cssText = "margin-bottom:6px;";
  const legHdr = document.createElement("div"); legHdr.style.cssText = "display:flex;gap:4px;color:#888;font-weight:bold;padding:2px 0;border-bottom:1px solid #333;margin-bottom:4px;font-size:9px;";
  for (const h of ["Type", "Strike", "Premium", "Qty (+buy/-sell)"]) { const s = document.createElement("span"); s.style.cssText = "flex:1;text-align:center;"; s.textContent = h; legHdr.appendChild(s); }
  legsDiv.appendChild(legHdr);
  function addLegRow(type, strike, prem, qty) { type = type || "call"; strike = strike || ""; prem = prem || ""; qty = qty || "1"; const row = document.createElement("div"); row.style.cssText = "display:flex;gap:4px;margin:2px 0;"; const mkSel = function(opts, val) { const s = document.createElement("select"); s.style.cssText = "flex:1;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:2px;"; for (const o of opts) { const op = document.createElement("option"); op.value = o; op.textContent = o; if (o === val) op.selected = true; s.appendChild(op); } return s; }; const mkInp = function(v, ph) { const i = document.createElement("input"); i.type = "number"; i.step = "0.01"; i.value = v; i.placeholder = ph; i.style.cssText = "flex:1;font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:2px;text-align:center;"; return i; }; row.appendChild(mkSel(["call", "put"], type)); row.appendChild(mkInp(strike, "Strike")); row.appendChild(mkInp(prem, "Premium")); row.appendChild(mkInp(qty, "Qty")); const del = document.createElement("button"); del.textContent = "\u00d7"; del.style.cssText = "background:none;border:1px solid #f44;color:#f44;cursor:pointer;padding:0 4px;"; del.addEventListener("click", function() { row.remove(); }); row.appendChild(del); legsDiv.appendChild(row); }
  function clearLegs() { const rows = legsDiv.querySelectorAll("div:not(:first-child)"); for (const r of rows) r.remove(); }
  for (const [name, legs] of Object.entries(presets)) { const btn = document.createElement("button"); btn.textContent = name; btn.style.cssText = "font-size:9px;padding:2px 8px;background:#0f3460;color:#8cf;border:1px solid #444;cursor:pointer;"; btn.addEventListener("click", function() { clearLegs(); for (const l of legs) addLegRow(l.type, l.strike.toFixed(2), l.prem.toFixed(2), String(l.qty)); }); presetBar.appendChild(btn); }
  const dteRow = document.createElement("div"); dteRow.style.cssText = "display:flex;gap:6px;align-items:center;margin:6px 0;";
  const dteLbl = document.createElement("span"); dteLbl.textContent = "Days to Expiry:"; dteLbl.style.cssText = "color:#888;font-size:10px;";
  const dteSlider = document.createElement("input"); dteSlider.type = "range"; dteSlider.min = "0"; dteSlider.max = "60"; dteSlider.value = "30"; dteSlider.style.cssText = "flex:1;";
  const dteVal = document.createElement("span"); dteVal.textContent = "30"; dteVal.style.cssText = "color:#ccc;font-size:10px;min-width:30px;text-align:right;";
  dteSlider.addEventListener("input", function() { dteVal.textContent = dteSlider.value; }); dteRow.appendChild(dteLbl); dteRow.appendChild(dteSlider); dteRow.appendChild(dteVal);
  const btnRow = document.createElement("div"); btnRow.style.cssText = "display:flex;gap:6px;margin:6px 0;";
  const addBtn = document.createElement("button"); addBtn.textContent = "+ Add Leg"; addBtn.style.cssText = "font-size:10px;padding:3px 8px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;"; addBtn.addEventListener("click", function() { addLegRow(); });
  const calcBtn = document.createElement("button"); calcBtn.textContent = "Calculate"; calcBtn.style.cssText = "font-size:10px;padding:3px 12px;background:#1b5e20;color:#8f8;border:1px solid #555;cursor:pointer;font-weight:bold;"; btnRow.appendChild(addBtn); btnRow.appendChild(calcBtn);
  const chartCanvas = document.createElement("canvas"); chartCanvas.width = 710; chartCanvas.height = 280; chartCanvas.style.cssText = "border:1px solid #333;margin-top:6px;";
  const statsDiv = document.createElement("div"); statsDiv.style.cssText = "display:flex;gap:16px;padding:6px 0;font-size:10px;";
  container.appendChild(presetBar); container.appendChild(legsDiv); container.appendChild(dteRow); container.appendChild(btnRow); container.appendChild(chartCanvas); container.appendChild(statsDiv); win.appendElement(container);
  addLegRow("call", price.toFixed(2), "3.00", "1");
  calcBtn.addEventListener("click", function() {
    const rows = legsDiv.querySelectorAll("div:not(:first-child)"); const legs = [];
    for (const r of rows) { const els = r.querySelectorAll("select, input"); if (els.length < 4) continue; legs.push({ type: els[0].value, strike: parseFloat(els[1].value), prem: parseFloat(els[2].value), qty: parseInt(els[3].value) || 1 }); }
    if (legs.length === 0 || legs.some(function(l) { return isNaN(l.strike) || isNaN(l.prem); })) { alert("Fill all leg fields"); return; }
    const dte = parseInt(dteSlider.value) || 0; const strikes = legs.map(function(l) { return l.strike; }); const center = strikes.reduce(function(a, b) { return a + b; }, 0) / strikes.length;
    const minPr = center * 0.7, maxPr = center * 1.3, step = (maxPr - minPr) / 200;
    const expiryPoints = [], dtePoints = []; let maxProfit = -Infinity, maxLoss = Infinity; const breakevens = [];
    for (let p = minPr; p <= maxPr; p += step) { let pnlExpiry = 0, pnlDTE = 0; for (const l of legs) { const dir = l.qty > 0 ? 1 : -1, absQty = Math.abs(l.qty); const intrinsic = l.type === "call" ? Math.max(0, p - l.strike) : Math.max(0, l.strike - p); pnlExpiry += dir * (intrinsic - l.prem) * absQty * 100; const currentIntrinsic = l.type === "call" ? Math.max(0, price - l.strike) : Math.max(0, l.strike - price); const timeValue = Math.max(0, l.prem - currentIntrinsic); const remainingTV = dte > 0 ? timeValue * (dte / 60) : 0; pnlDTE += dir * (intrinsic + remainingTV - l.prem) * absQty * 100; } expiryPoints.push({ price: p, pnl: pnlExpiry }); dtePoints.push({ price: p, pnl: pnlDTE }); maxProfit = Math.max(maxProfit, pnlExpiry); maxLoss = Math.min(maxLoss, pnlExpiry); }
    for (let i = 1; i < expiryPoints.length; i++) { if ((expiryPoints[i - 1].pnl <= 0 && expiryPoints[i].pnl > 0) || (expiryPoints[i - 1].pnl >= 0 && expiryPoints[i].pnl < 0)) { const ratio = Math.abs(expiryPoints[i - 1].pnl) / (Math.abs(expiryPoints[i - 1].pnl) + Math.abs(expiryPoints[i].pnl)); breakevens.push(expiryPoints[i - 1].price + ratio * step); } }
    const ctx = chartCanvas.getContext("2d"); const W = chartCanvas.width, H = chartCanvas.height; ctx.clearRect(0, 0, W, H); ctx.fillStyle = "#0a0a14"; ctx.fillRect(0, 0, W, H);
    const pad = 50, pRange = maxPr - minPr; const allPnl = expiryPoints.map(function(pt) { return pt.pnl; }).concat(dtePoints.map(function(pt) { return pt.pnl; })); const vMin = Math.min.apply(null, allPnl), vMax = Math.max.apply(null, allPnl), vRange = Math.max(vMax - vMin, 1);
    const toX = function(pr) { return pad + (pr - minPr) / pRange * (W - 2 * pad); }; const toY = function(v) { return H - pad - (v - vMin) / vRange * (H - 2 * pad); };
    ctx.strokeStyle = "#ffffff33"; ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(pad, toY(0)); ctx.lineTo(W - pad, toY(0)); ctx.stroke();
    for (let i = 0; i < expiryPoints.length - 1; i++) { const x1 = toX(expiryPoints[i].price), x2 = toX(expiryPoints[i + 1].price), y0 = toY(0); ctx.fillStyle = expiryPoints[i].pnl >= 0 ? "rgba(76,175,80,0.1)" : "rgba(244,67,54,0.1)"; ctx.beginPath(); ctx.moveTo(x1, y0); ctx.lineTo(x1, toY(expiryPoints[i].pnl)); ctx.lineTo(x2, toY(expiryPoints[i + 1].pnl)); ctx.lineTo(x2, y0); ctx.fill(); }
    if (dte > 0) { ctx.strokeStyle = "#2196f3"; ctx.lineWidth = 1.5; ctx.setLineDash([5, 4]); ctx.beginPath(); for (let i = 0; i < dtePoints.length; i++) { const x = toX(dtePoints[i].price), y = toY(dtePoints[i].pnl); if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y); } ctx.stroke(); ctx.setLineDash([]); }
    ctx.strokeStyle = "#4caf50"; ctx.lineWidth = 2; ctx.beginPath(); for (let i = 0; i < expiryPoints.length; i++) { const x = toX(expiryPoints[i].price), y = toY(expiryPoints[i].pnl); if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y); } ctx.stroke();
    ctx.strokeStyle = "#ffeb3b44"; ctx.lineWidth = 1; ctx.setLineDash([3, 3]); for (const l of legs) { const x = toX(l.strike); ctx.beginPath(); ctx.moveTo(x, pad - 10); ctx.lineTo(x, H - pad); ctx.stroke(); } ctx.setLineDash([]);
    for (const be of breakevens) { const x = toX(be), y = toY(0); ctx.fillStyle = "#ff9800"; ctx.beginPath(); ctx.arc(x, y, 4, 0, Math.PI * 2); ctx.fill(); ctx.font = "9px Consolas, monospace"; ctx.textAlign = "center"; ctx.fillText("$" + be.toFixed(1), x, y - 8); }
    ctx.fillStyle = "#888"; ctx.font = "10px Consolas, monospace"; ctx.textAlign = "center"; for (let i = 0; i <= 5; i++) { const pr = minPr + (pRange * i / 5); ctx.fillText("$" + pr.toFixed(0), toX(pr), H - 5); } ctx.textAlign = "right"; for (let i = 0; i <= 4; i++) { const v = vMin + (vRange * i / 4); ctx.fillText("$" + v.toFixed(0), pad - 4, toY(v) + 3); }
    ctx.fillStyle = "#4caf50"; ctx.textAlign = "left"; ctx.fillText("At Expiry", pad + 10, 15); if (dte > 0) { ctx.fillStyle = "#2196f3"; ctx.fillText("At " + dte + " DTE (dashed)", pad + 10, 28); }
    statsDiv.textContent = ""; var addStat = function(label, value, color) { const s = document.createElement("span"); s.style.cssText = "color:" + color + ";"; s.textContent = label + ": " + value; statsDiv.appendChild(s); };
    addStat("Max Profit", maxProfit >= 1e9 ? "Unlimited" : "$" + maxProfit.toFixed(0), "#4caf50"); addStat("Max Loss", "$" + maxLoss.toFixed(0), "#f44336"); for (let i = 0; i < breakevens.length; i++) addStat("BE" + (i + 1), "$" + breakevens[i].toFixed(2), "#ff9800");
  });
}

// ══════════════════════════════════════════════════════════════
// ORDERFLOW — Cumulative Delta from WebSocket Trades
// ══════════════════════════════════════════════════════════════
function cmdOrderFlow() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  let ofPollInterval = null, cumDelta = 0, totalBuys = 0, totalSells = 0;
  let deltaHistory = [], volumeHistory = [], lastBidOF = 0, lastAskOF = 0;
  const win = createWindow({ title: currentSymbol + " \u2014 Order Flow (Cumulative Delta)", width: 650, height: 520, onClose: function() { if (ofPollInterval) { clearInterval(ofPollInterval); ofPollInterval = null; } } });
  win.contentElement.textContent = "";
  const statsBar = document.createElement("div"); statsBar.style.cssText = "display:flex;justify-content:space-around;padding:6px 8px;border-bottom:1px solid #333;font-family:Consolas,monospace;font-size:11px;";
  const mkStat = function(label, color) { const s = document.createElement("span"); s.style.cssText = "color:" + color + ";"; s.textContent = label + ": 0"; return s; };
  const statBuys = mkStat("Buys", "#4caf50"), statSells = mkStat("Sells", "#f44336"), statDelta = mkStat("Net Delta", "#2196f3"), statRatio = mkStat("B/S Ratio", "#ff9800");
  statsBar.appendChild(statBuys); statsBar.appendChild(statSells); statsBar.appendChild(statDelta); statsBar.appendChild(statRatio); win.appendElement(statsBar);
  const chartDiv = document.createElement("div"); chartDiv.style.cssText = "width:100%;height:300px;"; win.appendElement(chartDiv);
  const volDiv = document.createElement("div"); volDiv.style.cssText = "width:100%;height:120px;"; win.appendElement(volDiv);
  const deltaChart = createChart(chartDiv, { width: 620, height: 300, layout: { background: { color: "#000" }, textColor: "#888", fontFamily: "Consolas, monospace", attributionLogo: false }, grid: { vertLines: { color: "#111" }, horzLines: { color: "#111" } }, rightPriceScale: { borderColor: "#333" }, timeScale: { borderColor: "#333", timeVisible: true, secondsVisible: true } });
  const deltaLine = deltaChart.addLineSeries({ color: "#2196f3", lineWidth: 2, title: "Cum. Delta", lastValueVisible: true, priceLineVisible: true });
  const volChart = createChart(volDiv, { width: 620, height: 120, layout: { background: { color: "#000" }, textColor: "#888", fontFamily: "Consolas, monospace", attributionLogo: false }, grid: { vertLines: { color: "#111" }, horzLines: { color: "#111" } }, rightPriceScale: { borderColor: "#333" }, timeScale: { borderColor: "#333", timeVisible: true, secondsVisible: true } });
  const ofVolSeries = volChart.addHistogramSeries({ title: "Trade Vol", priceFormat: { type: "volume" }, lastValueVisible: false, priceLineVisible: false });
  invoke("get_latest_quote", { symbol: currentSymbol }).then(function(json) { const q = JSON.parse(json); if (q.bid > 0) lastBidOF = q.bid; if (q.ask > 0) lastAskOF = q.ask; }).catch(function() {});
  ofPollInterval = setInterval(async function() {
    try {
      const json = await invoke("poll_stream"); const messages = JSON.parse(json); let updated = false;
      for (const msg of messages) {
        if (msg.Trade) {
          const t = msg.Trade, tradePrice = t.price, tradeVol = t.size || 0;
          const ts = t.timestamp ? Math.floor(new Date(t.timestamp).getTime() / 1000) : Math.floor(Date.now() / 1000);
          let buyVol = 0, sellVol = 0;
          if (lastAskOF > 0 && tradePrice >= lastAskOF) { buyVol = tradeVol; } else if (lastBidOF > 0 && tradePrice <= lastBidOF) { sellVol = tradeVol; } else { buyVol = tradeVol / 2; sellVol = tradeVol / 2; }
          cumDelta += buyVol - sellVol; totalBuys += buyVol; totalSells += sellVol;
          deltaHistory.push({ time: ts, value: cumDelta }); volumeHistory.push({ time: ts, value: tradeVol, color: buyVol > sellVol ? "rgba(76,175,80,0.7)" : "rgba(244,67,54,0.7)" }); updated = true;
        }
      }
      if (updated) {
        const seenDelta = new Map(); for (const d of deltaHistory) seenDelta.set(d.time, d);
        const dedupDelta = Array.from(seenDelta.values()).sort(function(a, b) { return a.time - b.time; });
        const seenVol = new Map(); for (const v of volumeHistory) { const existing = seenVol.get(v.time); if (existing) { existing.value += v.value; } else { seenVol.set(v.time, Object.assign({}, v)); } }
        const dedupVol = Array.from(seenVol.values()).sort(function(a, b) { return a.time - b.time; });
        deltaLine.setData(dedupDelta); ofVolSeries.setData(dedupVol);
        statBuys.textContent = "Buys: " + totalBuys.toFixed(0); statSells.textContent = "Sells: " + totalSells.toFixed(0);
        statDelta.textContent = "Net Delta: " + (cumDelta >= 0 ? "+" : "") + cumDelta.toFixed(0); statDelta.style.color = cumDelta >= 0 ? "#4caf50" : "#f44336";
        const ratio = totalSells > 0 ? (totalBuys / totalSells) : totalBuys > 0 ? 999 : 0; statRatio.textContent = "B/S Ratio: " + ratio.toFixed(2);
      }
    } catch (_) {}
  }, 1000);
}

// ── Help / About Overlay (? or F1) ──────────────────────────
function showHelpOverlay() {
  // Remove existing overlay if open
  const existing = document.getElementById("help-overlay");
  if (existing) { existing.remove(); return; }

  const overlay = document.createElement("div");
  overlay.id = "help-overlay";
  overlay.style.cssText = "position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.85);z-index:10000;display:flex;justify-content:center;align-items:center;";
  overlay.addEventListener("click", (e) => { if (e.target === overlay) overlay.remove(); });

  const panel = document.createElement("div");
  panel.style.cssText = "background:#0a0a14;border:1px solid #333;border-radius:8px;padding:24px;max-width:800px;max-height:80vh;overflow-y:auto;color:#ccc;font-family:Consolas,monospace;font-size:12px;";

  const title = document.createElement("h2");
  title.textContent = "TyphooN Terminal — Help & Keybindings";
  title.style.cssText = "color:#4caf50;margin:0 0 16px;font-size:18px;text-align:center;";
  panel.appendChild(title);

  const subtitle = document.createElement("div");
  subtitle.textContent = "GPU-accelerated trading terminal for Alpaca Markets. Press ? or F1 to toggle this help.";
  subtitle.style.cssText = "color:#888;text-align:center;margin-bottom:16px;font-size:11px;";
  panel.appendChild(subtitle);

  const sections = [
    { title: "F-Keys (Trading)", keys: [
      ["F1", "Help & Keybindings (this screen)"],
      ["F2", "Buy Lines (SL low, TP high)"],
      ["F3", "Sell Lines (SL high, TP low)"],
      ["F4", "Open Trade (calculates lots, places order)"],
      ["F5", "Destroy SL/TP Lines"],
      ["F6", "Cycle Martingale Mode"],
      ["F7", "Close All Positions"],
      ["F8", "Close Partial Position"],
    ]},
    { title: "Drawing Tools", keys: [
      ["L", "Trend Line (click two points)"],
      ["F", "Fibonacci Retracement (click high/low)"],
      ["H", "Horizontal Line (click to place)"],
      ["R", "Rectangle (click two corners)"],
      ["E", "Ray (extends right from two points)"],
      ["C", "Channel (parallel lines)"],
      ["Delete", "Remove Last Drawing"],
    ]},
    { title: "Navigation", keys: [
      ["Ctrl+K", "Command Palette (search all commands)"],
      ["Ctrl+T", "New Tab"],
      ["Ctrl+W", "Close Tab"],
      ["Esc", "Clear SL/TP Lines"],
      ["?", "This Help Screen"],
    ]},
    { title: "Chart Types (GPU + CPU)", keys: [
      ["GPU Candles", "WebGL2 candlestick rendering (default)"],
      ["GPU Heikin-Ashi", "Smoothed HA candles on GPU"],
      ["GPU Line", "Close-price line on GPU"],
      ["GPU OHLC Bars", "Open-High-Low-Close bars on GPU"],
      ["GPU Renko", "ATR-based brick chart on GPU"],
      ["CPU variants", "lightweight-charts fallback (bottom of dropdown)"],
    ]},
    { title: "Command Palette (Ctrl+K)", keys: [
      ["DES", "Company fundamentals (SEC EDGAR)"],
      ["NEWS", "News headlines"],
      ["FA", "Financial analysis (income/balance/cash flow)"],
      ["OPT", "Options chain (Greeks, bid/ask)"],
      ["SCAN", "Stock screener"],
      ["BACKTEST", "Visual backtester (SMA Cross, NNFX)"],
      ["OPTIMIZE", "Grid search optimizer"],
      ["OPTCALC", "Options P&L calculator (payoff diagram)"],
      ["OPTSTRAT", "Options strategy builder"],
      ["SECTORS", "Sector rotation heatmap (S&P 500 ETFs)"],
      ["ECON", "Economic calendar with countdown"],
      ["CHAT", "Community chat (Matrix protocol)"],
      ["AUTOTRADE", "Strategy auto-trading framework"],
      ["ALERTS", "Multi-condition alert manager"],
      ["ALERTBOARD", "Multi-symbol alert dashboard"],
      ["PORTFOLIO", "Portfolio breakdown by sector"],
      ["CORR", "Correlation matrix"],
      ["MONTECARLO", "Monte Carlo risk of ruin"],
      ["PATTERNS", "Pattern recognition (H&S, Double Top)"],
      ["SENTIMENT", "News sentiment analysis"],
      ["PCRATIO", "Put/Call ratio dashboard"],
      ["UNUSUAL", "Unusual options activity scanner"],
      ["AI", "AI trading assistant (Claude/GPT)"],
      ["SETTINGS", "API keys & configuration"],
      ["HELP", "This help screen"],
    ]},
    { title: "Architecture", keys: [
      ["GPU Charts", "WebGL2 (49KB Wasm) — all GPUs via WebGL2"],
      ["Wasm Indicators", "32KB Wasm — 50-100x faster optimizer"],
      ["Binary Storage", "Packed f64 + zstd — 3-5x smaller than JSON"],
      ["Encryption", "AES-256-GCM + PBKDF2 100K iterations"],
      ["Headless CLI", "--backtest flag for VPS/SSH strategy runs"],
    ]},
  ];

  for (const section of sections) {
    const hdr = document.createElement("div");
    hdr.textContent = section.title;
    hdr.style.cssText = "color:#ff8;font-weight:bold;font-size:13px;margin:12px 0 6px;border-bottom:1px solid #333;padding-bottom:4px;";
    panel.appendChild(hdr);

    const grid = document.createElement("div");
    grid.style.cssText = "display:grid;grid-template-columns:120px 1fr;gap:2px 12px;";
    for (const [key, desc] of section.keys) {
      const k = document.createElement("span");
      k.textContent = key;
      k.style.cssText = "color:#8cf;font-weight:bold;";
      const d = document.createElement("span");
      d.textContent = desc;
      d.style.cssText = "color:#aaa;";
      grid.appendChild(k);
      grid.appendChild(d);
    }
    panel.appendChild(grid);
  }

  const footer = document.createElement("div");
  footer.style.cssText = "margin-top:16px;text-align:center;color:#555;font-size:10px;";
  footer.textContent = "TyphooN Terminal v0.1.0 — Apache-2.0 — github.com/TyphooN-/TyphooN-Terminal";
  panel.appendChild(footer);

  overlay.appendChild(panel);
  document.body.appendChild(overlay);

  // Close on Escape
  const closeHandler = (e) => {
    if (e.key === "Escape" || e.key === "?" || e.key === "F1") {
      overlay.remove();
      document.removeEventListener("keydown", closeHandler);
    }
  };
  document.addEventListener("keydown", closeHandler);
}

// ── IVRANK ────────────────────────────────────────────────────
async function cmdIVRank() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — IV Rank`, width: 420, height: 320 });
  win.contentElement.textContent = "";
  const ivC = document.createElement("div");
  ivC.style.cssText = "padding:10px;font-size:11px;color:#ccc;";
  ivC.textContent = "Loading options chain...";
  win.appendElement(ivC);
  try {
    const nf = new Date();
    nf.setDate(nf.getDate() + (5 - nf.getDay() + 7) % 7 + 7);
    const expiry = nf.toISOString().split("T")[0];
    const json = await invoke("get_options", { symbol: currentSymbol, expiry });
    const chain = JSON.parse(json);
    if (!chain || chain.length === 0) { ivC.textContent = "No options data available."; return; }
    const price = lastPrice || 0;
    if (price <= 0) { ivC.textContent = "No current price available."; return; }
    const nearMoney = chain.filter(c => c.strike && Math.abs(c.strike - price) / price <= 0.10);
    const ivVals = nearMoney.map(c => c.implied_volatility).filter(v => v && v > 0);
    if (ivVals.length === 0) { ivC.textContent = "No IV data in near-money strikes."; return; }
    const curIV = ivVals.reduce((a, b) => a + b, 0) / ivVals.length;
    const hk = `typhoon_iv_history_${currentSymbol}`;
    let hist = [];
    try { hist = JSON.parse(localStorage.getItem(hk) || "[]"); } catch (_) {}
    hist.push({ iv: curIV, ts: Date.now() });
    if (hist.length > 365) hist = hist.slice(-365);
    localStorage.setItem(hk, JSON.stringify(hist));
    ivC.textContent = "";
    if (hist.length < 5) {
      const n2 = document.createElement("div");
      n2.style.cssText = "padding:20px;text-align:center;color:#ff0;font-size:13px;";
      n2.textContent = `Collecting data... ${hist.length} sample${hist.length !== 1 ? "s" : ""}`;
      const s2 = document.createElement("div");
      s2.style.cssText = "color:#888;font-size:10px;margin-top:8px;";
      s2.textContent = "Check back later — need at least 5 data points for IV Rank.";
      n2.appendChild(s2);
      ivC.appendChild(n2);
      return;
    }
    const sorted = hist.map(h => h.iv).sort((a, b) => a - b);
    const ivHi = sorted[sorted.length - 1];
    const ivLo = sorted[0];
    const ivRank = ivHi !== ivLo ? ((curIV - ivLo) / (ivHi - ivLo)) * 100 : 50;
    const t30 = Date.now() - 30 * 24 * 60 * 60 * 1000;
    const rec = hist.filter(h => h.ts >= t30);
    const avg30 = rec.length > 0 ? rec.reduce((a, b) => a + b.iv, 0) / rec.length : curIV;
    const rc = ivRank < 25 ? "#4caf50" : ivRank > 75 ? "#f44336" : "#ff0";
    const rl = ivRank < 25 ? "CHEAP OPTIONS" : ivRank > 75 ? "EXPENSIVE OPTIONS" : "NEUTRAL";
    const tbl = document.createElement("table");
    tbl.style.cssText = "width:100%;border-collapse:collapse;font-size:12px;margin-top:6px;";
    for (const [lbl, val] of [["Current IV", `${(curIV * 100).toFixed(1)}%`], ["IV Rank", `${ivRank.toFixed(1)}%`], ["IV High", `${(ivHi * 100).toFixed(1)}%`], ["IV Low", `${(ivLo * 100).toFixed(1)}%`], ["30-Day Avg IV", `${(avg30 * 100).toFixed(1)}%`], ["Samples", `${hist.length}`]]) {
      const tr = document.createElement("tr");
      tr.style.cssText = "border-bottom:1px solid #1a1a2e;";
      const tdL = document.createElement("td");
      tdL.style.cssText = "padding:6px 10px;color:#888;";
      tdL.textContent = lbl;
      const tdR = document.createElement("td");
      tdR.style.cssText = `padding:6px 10px;text-align:right;font-weight:bold;color:${lbl === "IV Rank" ? rc : "#ccc"};`;
      tdR.textContent = val;
      tr.appendChild(tdL);
      tr.appendChild(tdR);
      tbl.appendChild(tr);
    }
    ivC.appendChild(tbl);
    const badge = document.createElement("div");
    badge.style.cssText = `text-align:center;padding:10px;margin-top:10px;font-size:14px;font-weight:bold;color:${rc};border:1px solid ${rc};border-radius:4px;`;
    badge.textContent = rl;
    ivC.appendChild(badge);
  } catch (e) { ivC.textContent = `Error: ${e}`; }
}

// ── GAPS ──────────────────────────────────────────────────────
async function cmdGaps() {
  const win = createWindow({ title: "Gap Scanner", width: 600, height: 450 });
  win.contentElement.textContent = "";
  const gc = document.createElement("div");
  gc.style.cssText = "padding:6px;font-size:10px;color:#ccc;";
  gc.textContent = "Scanning watchlist for gaps...";
  win.appendElement(gc);
  try {
    const wl = getWatchlist();
    if (!wl || wl.length === 0) { gc.textContent = "No symbols in watchlist. Add symbols to QM first."; return; }
    const MIN_GAP = 1;
    const gaps = [];
    for (const sym of wl) {
      try {
        const ck = `${sym}:1Day`;
        let bars;
        if (barCache[ck] && barCache[ck].data && barCache[ck].data.length >= 2) {
          bars = barCache[ck].data;
          barCache[ck].lastAccess = Date.now();
        } else {
          const bj = await invoke("get_bars", { symbol: sym, timeframe: "1Day", limit: 5 });
          bars = JSON.parse(bj);
          barCache[ck] = { data: bars, timestamp: Date.now(), lastAccess: Date.now() };
        }
        if (bars.length < 2) continue;
        const prev = bars[bars.length - 2];
        const cur = bars[bars.length - 1];
        const gp = ((cur.open - prev.close) / prev.close) * 100;
        if (Math.abs(gp) >= MIN_GAP) gaps.push({ sym, gapPct: gp, direction: gp > 0 ? "UP" : "DOWN", prevClose: prev.close, open: cur.open });
      } catch (_) {}
    }
    gaps.sort((a, b) => Math.abs(b.gapPct) - Math.abs(a.gapPct));
    gc.textContent = "";
    if (gaps.length === 0) { gc.textContent = `No gaps > ${MIN_GAP}% found in ${wl.length} symbols.`; return; }
    const tbl = document.createElement("table");
    tbl.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;";
    const thead = document.createElement("thead");
    const hr = document.createElement("tr");
    hr.style.cssText = "border-bottom:1px solid #444;";
    for (const h of ["Symbol", "Gap %", "Direction", "Prev Close", "Open"]) {
      const th = document.createElement("th");
      th.style.cssText = "padding:4px 6px;color:#888;text-align:left;font-size:10px;";
      th.textContent = h;
      hr.appendChild(th);
    }
    thead.appendChild(hr);
    tbl.appendChild(thead);
    const tbody = document.createElement("tbody");
    for (const g of gaps) {
      const tr = document.createElement("tr");
      tr.style.cssText = "border-bottom:1px solid #1a1a2e;cursor:pointer;";
      tr.addEventListener("mouseenter", () => { tr.style.background = "rgba(255,255,255,0.05)"; });
      tr.addEventListener("mouseleave", () => { tr.style.background = ""; });
      tr.addEventListener("click", () => { document.getElementById("symbol-input").value = g.sym; triggerLoad(); });
      const clr = g.direction === "UP" ? "#4caf50" : "#f44336";
      for (const v of [
        { text: g.sym, style: "color:#fff;font-weight:bold;" },
        { text: `${g.gapPct >= 0 ? "+" : ""}${g.gapPct.toFixed(2)}%`, style: `color:${clr};font-weight:bold;` },
        { text: g.direction, style: `color:${clr};` },
        { text: `$${g.prevClose.toFixed(2)}`, style: "color:#ccc;" },
        { text: `$${g.open.toFixed(2)}`, style: "color:#ccc;" },
      ]) {
        const td = document.createElement("td");
        td.style.cssText = `padding:4px 6px;${v.style}`;
        td.textContent = v.text;
        tr.appendChild(td);
      }
      tbody.appendChild(tr);
    }
    tbl.appendChild(tbody);
    gc.appendChild(tbl);
    const sm = document.createElement("div");
    sm.style.cssText = "padding:6px;color:#666;font-size:9px;margin-top:6px;";
    sm.textContent = `${gaps.length} gap${gaps.length !== 1 ? "s" : ""} found across ${wl.length} symbols (threshold: ${MIN_GAP}%)`;
    gc.appendChild(sm);
  } catch (e) { gc.textContent = `Error: ${e}`; }
}

// ── FLOWS ─────────────────────────────────────────────────────
async function cmdFlows() {
  const win = createWindow({ title: "Sector Fund Flows", width: 700, height: 450 });
  win.contentElement.textContent = "";
  const fc = document.createElement("div");
  fc.style.cssText = "padding:6px;font-size:10px;color:#ccc;";
  fc.textContent = "Loading sector fund flow data...";
  win.appendElement(fc);
  const etfs = [
    { sym: "XLK", name: "Technology" }, { sym: "XLF", name: "Finance" },
    { sym: "XLE", name: "Energy" }, { sym: "XLV", name: "Health Care" },
    { sym: "XLI", name: "Industrials" }, { sym: "XLC", name: "Communication" },
    { sym: "XLRE", name: "Real Estate" }, { sym: "XLU", name: "Utilities" },
    { sym: "XLB", name: "Materials" }, { sym: "XLP", name: "Consumer Staples" },
    { sym: "XLY", name: "Consumer Disc" },
  ];
  try {
    const fi = [];
    for (const etf of etfs) {
      try {
        const ck = `${etf.sym}:1Day`;
        let bars;
        if (barCache[ck] && barCache[ck].data && barCache[ck].data.length >= 6) {
          bars = barCache[ck].data;
          barCache[ck].lastAccess = Date.now();
        } else {
          const bj = await invoke("get_bars", { symbol: etf.sym, timeframe: "1Day", limit: 20 });
          bars = JSON.parse(bj);
          barCache[ck] = { data: bars, timestamp: Date.now(), lastAccess: Date.now() };
        }
        if (bars.length < 2) continue;
        const last = bars[bars.length - 1];
        const f1d = last.volume * (last.close - last.open);
        const p1d = ((last.close - bars[bars.length - 2].close) / bars[bars.length - 2].close) * 100;
        const l5 = bars.slice(-5);
        const f5d = l5.reduce((sum, b) => sum + b.volume * (b.close - b.open), 0);
        const fdac = bars.length >= 6 ? bars[bars.length - 6].close : l5[0].open;
        const p5d = ((last.close - fdac) / fdac) * 100;
        fi.push({ ...etf, flow1d: f1d, flow5d: f5d, pctChg1d: p1d, pctChg5d: p5d });
      } catch (_) {}
    }
    fi.sort((a, b) => b.flow1d - a.flow1d);
    fc.textContent = "";
    if (fi.length === 0) { fc.textContent = "No sector data available."; return; }
    function fmt(v) {
      const a = Math.abs(v);
      if (a >= 1e9) return `$${(v / 1e9).toFixed(1)}B`;
      if (a >= 1e6) return `$${(v / 1e6).toFixed(1)}M`;
      if (a >= 1e3) return `$${(v / 1e3).toFixed(0)}K`;
      return `$${v.toFixed(0)}`;
    }
    const tbl = document.createElement("table");
    tbl.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;";
    const thead = document.createElement("thead");
    const hr = document.createElement("tr");
    hr.style.cssText = "border-bottom:1px solid #444;";
    for (const h of ["Sector", "1D Flow", "5D Flow", "1D %Chg", "5D %Chg"]) {
      const th = document.createElement("th");
      th.style.cssText = "padding:4px 8px;color:#888;text-align:right;font-size:10px;";
      if (h === "Sector") th.style.textAlign = "left";
      th.textContent = h;
      hr.appendChild(th);
    }
    thead.appendChild(hr);
    tbl.appendChild(thead);
    const tbody = document.createElement("tbody");
    for (const it of fi) {
      const tr = document.createElement("tr");
      tr.style.cssText = "border-bottom:1px solid #1a1a2e;cursor:pointer;";
      tr.addEventListener("mouseenter", () => { tr.style.background = "rgba(255,255,255,0.05)"; });
      tr.addEventListener("mouseleave", () => { tr.style.background = ""; });
      tr.addEventListener("click", () => { document.getElementById("symbol-input").value = it.sym; triggerLoad(); });
      const c1 = it.flow1d >= 0 ? "#4caf50" : "#f44336";
      const c5 = it.flow5d >= 0 ? "#4caf50" : "#f44336";
      const cp1 = it.pctChg1d >= 0 ? "#4caf50" : "#f44336";
      const cp5 = it.pctChg5d >= 0 ? "#4caf50" : "#f44336";
      for (const c of [
        { text: `${it.sym} — ${it.name}`, style: "text-align:left;color:#fff;font-weight:bold;" },
        { text: `${it.flow1d >= 0 ? "+" : ""}${fmt(it.flow1d)}`, style: `text-align:right;color:${c1};font-weight:bold;` },
        { text: `${it.flow5d >= 0 ? "+" : ""}${fmt(it.flow5d)}`, style: `text-align:right;color:${c5};` },
        { text: `${it.pctChg1d >= 0 ? "+" : ""}${it.pctChg1d.toFixed(2)}%`, style: `text-align:right;color:${cp1};` },
        { text: `${it.pctChg5d >= 0 ? "+" : ""}${it.pctChg5d.toFixed(2)}%`, style: `text-align:right;color:${cp5};` },
      ]) {
        const td = document.createElement("td");
        td.style.cssText = `padding:5px 8px;${c.style}`;
        td.textContent = c.text;
        tr.appendChild(td);
      }
      tbody.appendChild(tr);
    }
    tbl.appendChild(tbody);
    fc.appendChild(tbl);
    const lg = document.createElement("div");
    lg.style.cssText = "padding:8px;color:#666;font-size:9px;margin-top:6px;";
    lg.textContent = "Flow = Volume x (Close - Open). Positive = buying pressure (inflow), Negative = selling pressure (outflow). Click row to load chart.";
    fc.appendChild(lg);
  } catch (e) { fc.textContent = `Error: ${e}`; }
}


// ══════════════════════════════════════════════════════════════
// VOLUME — Volume Profile (Horizontal Histogram)
// ══════════════════════════════════════════════════════════════
function cmdVolumeProfile() {
  if (!currentChartData || currentChartData.length < 10) {
    log("VOLUME: Need at least 10 bars loaded", "warn");
    return;
  }
  const data = currentChartData;
  const NUM_BINS = 50;
  let minLow = Infinity, maxHigh = -Infinity, totalVolume = 0;
  for (const bar of data) {
    if (bar.low < minLow) minLow = bar.low;
    if (bar.high > maxHigh) maxHigh = bar.high;
    totalVolume += (bar.volume || 0);
  }
  if (maxHigh <= minLow || totalVolume === 0) {
    log("VOLUME: Invalid price range or no volume data", "warn");
    return;
  }
  const binSize = (maxHigh - minLow) / NUM_BINS;
  const bins = new Array(NUM_BINS).fill(0);
  for (const bar of data) {
    const vol = bar.volume || 0;
    if (vol === 0) continue;
    const barRange = bar.high - bar.low;
    if (barRange <= 0) {
      const idx = Math.min(Math.floor((bar.close - minLow) / binSize), NUM_BINS - 1);
      bins[idx] += vol;
      continue;
    }
    for (let b = 0; b < NUM_BINS; b++) {
      const bLow = minLow + b * binSize;
      const bHigh = bLow + binSize;
      const overlap = Math.max(0, Math.min(bar.high, bHigh) - Math.max(bar.low, bLow));
      if (overlap > 0) bins[b] += vol * (overlap / barRange);
    }
  }
  let pocIdx = 0;
  for (let i = 1; i < NUM_BINS; i++) { if (bins[i] > bins[pocIdx]) pocIdx = i; }
  const pocPrice = minLow + (pocIdx + 0.5) * binSize;
  const vaTarget = totalVolume * 0.70;
  let vaVolume = bins[pocIdx], vaLowIdx = pocIdx, vaHighIdx = pocIdx;
  while (vaVolume < vaTarget && (vaLowIdx > 0 || vaHighIdx < NUM_BINS - 1)) {
    const below = vaLowIdx > 0 ? bins[vaLowIdx - 1] : 0;
    const above = vaHighIdx < NUM_BINS - 1 ? bins[vaHighIdx + 1] : 0;
    if (below >= above && vaLowIdx > 0) { vaLowIdx--; vaVolume += bins[vaLowIdx]; }
    else if (vaHighIdx < NUM_BINS - 1) { vaHighIdx++; vaVolume += bins[vaHighIdx]; }
    else { vaLowIdx--; vaVolume += bins[vaLowIdx]; }
  }
  const vaLow = minLow + vaLowIdx * binSize;
  const vaHigh = minLow + (vaHighIdx + 1) * binSize;
  const lineData = data.map(d => ({ time: d.time }));
  const pocSeries = chart.addLineSeries({ color: "#FFFF00", lineWidth: 2, lineStyle: 0, lastValueVisible: true, priceLineVisible: false, crosshairMarkerVisible: false, title: "POC" });
  pocSeries.setData(lineData.map(d => ({ time: d.time, value: pocPrice })));
  indicatorSeries["vpoc"] = pocSeries;
  const vahSeries = chart.addLineSeries({ color: "#00FFFF", lineWidth: 1, lineStyle: 2, lastValueVisible: true, priceLineVisible: false, crosshairMarkerVisible: false, title: "VA High" });
  vahSeries.setData(lineData.map(d => ({ time: d.time, value: vaHigh })));
  indicatorSeries["vah"] = vahSeries;
  const valSeries = chart.addLineSeries({ color: "#00FFFF", lineWidth: 1, lineStyle: 2, lastValueVisible: true, priceLineVisible: false, crosshairMarkerVisible: false, title: "VA Low" });
  valSeries.setData(lineData.map(d => ({ time: d.time, value: vaLow })));
  indicatorSeries["val"] = valSeries;
  const dp = lastPrice > 100 ? 2 : lastPrice > 1 ? 4 : 6;
  log(`VOLUME: POC ${pocPrice.toFixed(dp)}, VA ${vaLow.toFixed(dp)}\u2013${vaHigh.toFixed(dp)}`, "ok");
  const win = createWindow({ title: `${currentSymbol} \u2014 Volume Profile`, width: 380, height: 280 });
  win.contentElement.textContent = "";
  const table = document.createElement("table");
  table.style.cssText = "border-collapse:collapse;font-size:12px;width:100%;font-family:Consolas,monospace;";
  const rows = [
    ["Point of Control (POC)", pocPrice.toFixed(dp), "#FFFF00"],
    ["Value Area High", vaHigh.toFixed(dp), "#00FFFF"],
    ["Value Area Low", vaLow.toFixed(dp), "#00FFFF"],
    ["Value Area Width", (vaHigh - vaLow).toFixed(dp), "#ccc"],
    ["Total Volume", totalVolume.toLocaleString(), "#ccc"],
    ["Bars Analyzed", data.length.toString(), "#888"],
    ["Bins", NUM_BINS.toString(), "#888"],
  ];
  for (const [label, value, color] of rows) {
    const tr = document.createElement("tr");
    const tdL = document.createElement("td");
    tdL.style.cssText = "padding:6px 10px;border-bottom:1px solid #1a1a2e;color:#888;";
    tdL.textContent = label;
    const tdV = document.createElement("td");
    tdV.style.cssText = `padding:6px 10px;border-bottom:1px solid #1a1a2e;color:${color};text-align:right;font-weight:bold;`;
    tdV.textContent = value;
    tr.appendChild(tdL);
    tr.appendChild(tdV);
    table.appendChild(tr);
  }
  win.appendElement(table);
}

// ══════════════════════════════════════════════════════════════
// PIVOTS — Auto Pivot Points (Classic Floor Trader)
// ══════════════════════════════════════════════════════════════
async function cmdPivots() {
  const cKey = getCacheKey(currentSymbol, "1Day");
  let dailyBars = barCache[cKey] && barCache[cKey].data;
  if (!dailyBars || dailyBars.length < 3) {
    try {
      const barsJson = await invoke("get_bars", { symbol: currentSymbol, timeframe: "1Day", limit: 10 });
      dailyBars = JSON.parse(barsJson);
      if (dailyBars && dailyBars.length > 0) barCache[cKey] = { data: dailyBars, timestamp: Date.now(), lastAccess: Date.now() };
    } catch (e) { log(`PIVOTS: Failed to fetch daily bars: ${e}`, "warn"); return; }
  }
  if (!dailyBars || dailyBars.length < 2) { log("PIVOTS: Need at least 2 daily bars", "warn"); return; }
  const prevBar = dailyBars[dailyBars.length - 2];
  const H = prevBar.high, L = prevBar.low, C = prevBar.close;
  const PP = (H + L + C) / 3;
  const R1 = 2 * PP - L, S1 = 2 * PP - H;
  const R2 = PP + (H - L), S2 = PP - (H - L);
  const R3 = H + 2 * (PP - L), S3 = L - 2 * (H - PP);
  const levels = [
    { name: "R3", price: R3, color: "#f44336", lineStyle: 3 },
    { name: "R2", price: R2, color: "#f44336", lineStyle: 2 },
    { name: "R1", price: R1, color: "#f44336", lineStyle: 0 },
    { name: "PP", price: PP, color: "#FFFFFF", lineStyle: 0 },
    { name: "S1", price: S1, color: "#4caf50", lineStyle: 0 },
    { name: "S2", price: S2, color: "#4caf50", lineStyle: 2 },
    { name: "S3", price: S3, color: "#4caf50", lineStyle: 3 },
  ];
  if (!currentChartData || currentChartData.length < 2) { log("PIVOTS: No chart data loaded", "warn"); return; }
  const startIdx = Math.max(0, currentChartData.length - 30);
  const lineSegment = currentChartData.slice(startIdx);
  const dp = lastPrice > 100 ? 2 : lastPrice > 1 ? 4 : 6;
  for (const level of levels) {
    const s = chart.addLineSeries({ color: level.color, lineWidth: level.name === "PP" ? 2 : 1, lineStyle: level.lineStyle, lastValueVisible: true, priceLineVisible: false, crosshairMarkerVisible: false, title: level.name });
    s.setData(lineSegment.map(d => ({ time: d.time, value: level.price })));
    indicatorSeries[`pivot_${level.name}`] = s;
  }
  log(`PIVOTS: PP ${PP.toFixed(dp)}, R1-R3 ${R1.toFixed(dp)}/${R2.toFixed(dp)}/${R3.toFixed(dp)}, S1-S3 ${S1.toFixed(dp)}/${S2.toFixed(dp)}/${S3.toFixed(dp)}`, "ok");
  const win = createWindow({ title: `${currentSymbol} \u2014 Pivot Points`, width: 420, height: 350 });
  win.contentElement.textContent = "";
  const table = document.createElement("table");
  table.style.cssText = "border-collapse:collapse;font-size:11px;width:100%;font-family:Consolas,monospace;";
  const thead = document.createElement("tr");
  for (const hdr of ["Level", "Price", "Distance"]) {
    const th = document.createElement("td");
    th.style.cssText = "padding:5px 10px;border-bottom:1px solid #333;color:#888;font-weight:bold;";
    th.textContent = hdr;
    thead.appendChild(th);
  }
  table.appendChild(thead);
  for (const level of levels) {
    const tr = document.createElement("tr");
    const dist = lastPrice > 0 ? ((level.price - lastPrice) / lastPrice * 100) : 0;
    const cells = [
      { text: level.name, color: level.color },
      { text: level.price.toFixed(dp), color: level.color },
      { text: `${dist >= 0 ? "+" : ""}${dist.toFixed(2)}%`, color: dist >= 0 ? "#4caf50" : "#f44336" },
    ];
    for (const cell of cells) {
      const td = document.createElement("td");
      td.style.cssText = `padding:5px 10px;border-bottom:1px solid #1a1a2e;color:${cell.color};`;
      td.textContent = cell.text;
      tr.appendChild(td);
    }
    table.appendChild(tr);
  }
  const note = document.createElement("tr");
  const noteCell = document.createElement("td");
  noteCell.colSpan = 3;
  noteCell.style.cssText = "padding:8px 10px;color:#555;font-size:9px;";
  noteCell.textContent = `Based on previous daily bar: H=${H.toFixed(dp)} L=${L.toFixed(dp)} C=${C.toFixed(dp)}`;
  note.appendChild(noteCell);
  table.appendChild(note);
  win.appendElement(table);
}

// ══════════════════════════════════════════════════════════════
// PERF — Symbol Performance Card
// ══════════════════════════════════════════════════════════════
async function cmdPerf() {
  const cKey = getCacheKey(currentSymbol, "1Day");
  let dailyBars = barCache[cKey] && barCache[cKey].data;
  if (!dailyBars || dailyBars.length < 30) {
    try {
      const barsJson = await invoke("get_bars", { symbol: currentSymbol, timeframe: "1Day", limit: 500 });
      dailyBars = JSON.parse(barsJson);
      if (dailyBars && dailyBars.length > 0) barCache[cKey] = { data: dailyBars, timestamp: Date.now(), lastAccess: Date.now() };
    } catch (e) { log(`PERF: Failed to fetch daily bars: ${e}`, "warn"); return; }
  }
  if (!dailyBars || dailyBars.length < 2) { log("PERF: Need at least 2 daily bars", "warn"); return; }
  const closes = dailyBars.map(b => b.close);
  const current = closes[closes.length - 1];
  const dp = current > 100 ? 2 : current > 1 ? 4 : 6;
  const calcReturn = (periods) => {
    if (closes.length <= periods) return null;
    const prev = closes[closes.length - 1 - periods];
    return ((current - prev) / prev) * 100;
  };
  const now = new Date();
  const yearStart = new Date(now.getFullYear(), 0, 1).getTime() / 1000;
  let ytdReturn = null;
  for (let i = 0; i < dailyBars.length; i++) {
    if (dailyBars[i].time >= yearStart) { ytdReturn = ((current - dailyBars[i].close) / dailyBars[i].close) * 100; break; }
  }
  const returns = [
    { label: "1D", value: calcReturn(1) },
    { label: "1W", value: calcReturn(5) },
    { label: "1M", value: calcReturn(21) },
    { label: "3M", value: calcReturn(63) },
    { label: "6M", value: calcReturn(126) },
    { label: "1Y", value: calcReturn(252) },
    { label: "YTD", value: ytdReturn },
  ];
  const yearBars = dailyBars.slice(-252);
  let high52 = -Infinity, low52 = Infinity;
  for (const bar of yearBars) { if (bar.high > high52) high52 = bar.high; if (bar.low < low52) low52 = bar.low; }
  const distFromHigh = ((current - high52) / high52) * 100;
  const recentBars = dailyBars.slice(-20);
  let avgVol = 0;
  for (const bar of recentBars) avgVol += (bar.volume || 0);
  avgVol = recentBars.length > 0 ? avgVol / recentBars.length : 0;
  const win = createWindow({ title: `${currentSymbol} \u2014 Performance Card`, width: 420, height: 440 });
  win.contentElement.textContent = "";
  const container = document.createElement("div");
  container.style.cssText = "font-family:Consolas,monospace;padding:4px;";
  const header = document.createElement("div");
  header.style.cssText = "text-align:center;margin-bottom:12px;";
  header.innerHTML = `<div style="font-size:18px;font-weight:bold;color:#fff;">${currentSymbol}</div><div style="font-size:22px;color:#00e5ff;margin-top:4px;">$${current.toFixed(dp)}</div>`;
  container.appendChild(header);
  const gridTitle = document.createElement("div");
  gridTitle.style.cssText = "color:#888;font-size:10px;margin-bottom:6px;text-transform:uppercase;letter-spacing:1px;";
  gridTitle.textContent = "Returns";
  container.appendChild(gridTitle);
  const grid = document.createElement("div");
  grid.style.cssText = "display:grid;grid-template-columns:repeat(4,1fr);gap:6px;margin-bottom:14px;";
  for (const r of returns) {
    const cell = document.createElement("div");
    cell.style.cssText = "background:#1a1a2e;border-radius:4px;padding:8px 4px;text-align:center;";
    const labelDiv = document.createElement("div");
    labelDiv.style.cssText = "color:#666;font-size:9px;margin-bottom:2px;";
    labelDiv.textContent = r.label;
    const valDiv = document.createElement("div");
    const val = r.value;
    const color = val === null ? "#555" : val >= 0 ? "#4caf50" : "#f44336";
    const arrow = val === null ? "" : val >= 0 ? "\u25B2 " : "\u25BC ";
    valDiv.style.cssText = `color:${color};font-size:12px;font-weight:bold;`;
    valDiv.textContent = val === null ? "N/A" : `${arrow}${val.toFixed(2)}%`;
    cell.appendChild(labelDiv);
    cell.appendChild(valDiv);
    grid.appendChild(cell);
  }
  container.appendChild(grid);
  const rangeTitle = document.createElement("div");
  rangeTitle.style.cssText = "color:#888;font-size:10px;margin-bottom:6px;text-transform:uppercase;letter-spacing:1px;";
  rangeTitle.textContent = "52-Week Range";
  container.appendChild(rangeTitle);
  const rangeContainer = document.createElement("div");
  rangeContainer.style.cssText = "background:#1a1a2e;border-radius:4px;padding:10px;margin-bottom:14px;";
  const rangeLabels = document.createElement("div");
  rangeLabels.style.cssText = "display:flex;justify-content:space-between;font-size:10px;margin-bottom:4px;";
  rangeLabels.innerHTML = `<span style="color:#f44336;">$${low52.toFixed(dp)}</span><span style="color:#4caf50;">$${high52.toFixed(dp)}</span>`;
  rangeContainer.appendChild(rangeLabels);
  const barOuter = document.createElement("div");
  barOuter.style.cssText = "height:8px;background:#333;border-radius:4px;position:relative;";
  const pct = high52 > low52 ? ((current - low52) / (high52 - low52)) * 100 : 50;
  const barInner = document.createElement("div");
  barInner.style.cssText = "height:100%;background:linear-gradient(90deg,#f44336,#ffeb3b,#4caf50);border-radius:4px;width:100%;";
  barOuter.appendChild(barInner);
  const mkr = document.createElement("div");
  mkr.style.cssText = `position:absolute;top:-3px;left:${Math.min(Math.max(pct, 2), 98)}%;width:2px;height:14px;background:#fff;border-radius:1px;transform:translateX(-50%);`;
  barOuter.appendChild(mkr);
  rangeContainer.appendChild(barOuter);
  const distLabel = document.createElement("div");
  distLabel.style.cssText = "text-align:center;font-size:10px;color:#888;margin-top:4px;";
  distLabel.textContent = `${distFromHigh.toFixed(2)}% from 52w high`;
  rangeContainer.appendChild(distLabel);
  container.appendChild(rangeContainer);
  const volTitle = document.createElement("div");
  volTitle.style.cssText = "color:#888;font-size:10px;margin-bottom:6px;text-transform:uppercase;letter-spacing:1px;";
  volTitle.textContent = "Volume";
  container.appendChild(volTitle);
  const volRow = document.createElement("div");
  volRow.style.cssText = "background:#1a1a2e;border-radius:4px;padding:10px;display:flex;justify-content:space-between;";
  const fmtVol = (v) => v >= 1e6 ? (v / 1e6).toFixed(2) + "M" : v >= 1e3 ? (v / 1e3).toFixed(1) + "K" : Math.round(v).toString();
  const latestVol = dailyBars[dailyBars.length - 1].volume || 0;
  volRow.innerHTML = `<div><div style="color:#666;font-size:9px;">Avg Daily (20d)</div><div style="color:#ccc;font-size:13px;font-weight:bold;">${fmtVol(avgVol)}</div></div><div style="text-align:right;"><div style="color:#666;font-size:9px;">Latest Volume</div><div style="color:#ccc;font-size:13px;font-weight:bold;">${fmtVol(latestVol)}</div></div>`;
  container.appendChild(volRow);
  win.appendElement(container);
  log(`PERF: ${currentSymbol} card loaded (${dailyBars.length} daily bars)`, "ok");
}

// ══════════════════════════════════════════════════════════════
// VWAP+ — Anchored VWAP with Standard Deviation Bands
// ══════════════════════════════════════════════════════════════
async function cmdAnchoredVWAP() {
  if (!currentChartData || currentChartData.length < 5) { log("VWAP+: Need at least 5 bars loaded", "warn"); return; }
  const now = new Date();
  const dayOfWeek = now.getDay();
  const mondayOffset = dayOfWeek === 0 ? 6 : dayOfWeek - 1;
  const monday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - mondayOffset);
  const defaultAnchor = monday.toISOString().substring(0, 10);
  const anchorStr = prompt("Anchor date for VWAP (YYYY-MM-DD):", defaultAnchor);
  if (!anchorStr) return;
  const anchorDate = new Date(anchorStr + "T00:00:00Z");
  if (isNaN(anchorDate.getTime())) { log("VWAP+: Invalid date format", "warn"); return; }
  const anchorTs = Math.floor(anchorDate.getTime() / 1000);
  let startIdx = -1;
  for (let i = 0; i < currentChartData.length; i++) {
    if (currentChartData[i].time >= anchorTs) { startIdx = i; break; }
  }
  if (startIdx < 0) { log("VWAP+: No bars found at or after anchor date", "warn"); return; }
  const bars = currentChartData.slice(startIdx);
  if (bars.length < 2) { log("VWAP+: Need at least 2 bars from anchor date", "warn"); return; }
  const vwapData = [], sd1Upper = [], sd1Lower = [], sd2Upper = [], sd2Lower = [];
  let cumPV = 0, cumVol = 0, cumPV2 = 0;
  for (const bar of bars) {
    const typPrice = (bar.high + bar.low + bar.close) / 3;
    const vol = bar.volume || 0;
    if (vol === 0) continue;
    cumPV += typPrice * vol;
    cumVol += vol;
    cumPV2 += typPrice * typPrice * vol;
    const vwap = cumPV / cumVol;
    const variance = Math.max(0, cumPV2 / cumVol - vwap * vwap);
    const sd = Math.sqrt(variance);
    vwapData.push({ time: bar.time, value: vwap });
    sd1Upper.push({ time: bar.time, value: vwap + sd });
    sd1Lower.push({ time: bar.time, value: vwap - sd });
    sd2Upper.push({ time: bar.time, value: vwap + 2 * sd });
    sd2Lower.push({ time: bar.time, value: vwap - 2 * sd });
  }
  if (vwapData.length < 2) { log("VWAP+: Not enough volume data from anchor", "warn"); return; }
  const sVwap = chart.addLineSeries({ color: "#FF00FF", lineWidth: 2, lineStyle: 0, lastValueVisible: true, priceLineVisible: false, crosshairMarkerVisible: false, title: "VWAP" });
  sVwap.setData(vwapData);
  indicatorSeries["avwap"] = sVwap;
  const sSD1U = chart.addLineSeries({ color: "#FF00FF", lineWidth: 1, lineStyle: 2, lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false });
  sSD1U.setData(sd1Upper);
  indicatorSeries["avwap_sd1u"] = sSD1U;
  const sSD1L = chart.addLineSeries({ color: "#FF00FF", lineWidth: 1, lineStyle: 2, lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false });
  sSD1L.setData(sd1Lower);
  indicatorSeries["avwap_sd1l"] = sSD1L;
  const sSD2U = chart.addLineSeries({ color: "#FF00FF", lineWidth: 1, lineStyle: 3, lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false });
  sSD2U.setData(sd2Upper);
  indicatorSeries["avwap_sd2u"] = sSD2U;
  const sSD2L = chart.addLineSeries({ color: "#FF00FF", lineWidth: 1, lineStyle: 3, lastValueVisible: false, priceLineVisible: false, crosshairMarkerVisible: false });
  sSD2L.setData(sd2Lower);
  indicatorSeries["avwap_sd2l"] = sSD2L;
  const dp = lastPrice > 100 ? 2 : lastPrice > 1 ? 4 : 6;
  const lastVwap = vwapData[vwapData.length - 1].value;
  log(`VWAP+: Anchored from ${anchorStr}, VWAP=${lastVwap.toFixed(dp)}, ${vwapData.length} bars`, "ok");
}

// ══════════════════════════════════════════════════════════════
// MARKETPROFILE — Market Profile / TPO Chart
// ══════════════════════════════════════════════════════════════
async function cmdMarketProfile() {
  const sym = currentSymbol || "SPY";
  const win = createWindow({ title: `${sym} — Market Profile / TPO`, width: 700, height: 550 });
  win.contentElement.textContent = "";
  const loading = document.createElement("div");
  loading.textContent = "Building Market Profile...";
  loading.style.cssText = "color:#888;padding:20px;";
  win.appendElement(loading);
  try {
    let bars = null;
    const key30 = `${sym}:30Min`;
    const key60 = `${sym}:1Hour`;
    if (barCache[key30] && barCache[key30].data && barCache[key30].data.length > 20) {
      bars = barCache[key30].data;
      barCache[key30].lastAccess = Date.now();
    } else if (barCache[key60] && barCache[key60].data && barCache[key60].data.length > 20) {
      bars = barCache[key60].data;
      barCache[key60].lastAccess = Date.now();
    } else {
      try {
        const barsJson = await invoke("get_bars", { symbol: sym, timeframe: "1Hour", limit: 500 });
        bars = JSON.parse(barsJson);
        if (bars.length > 0) barCache[key60] = { data: bars, timestamp: Date.now(), lastAccess: Date.now() };
      } catch (_) {}
    }
    if (!bars || bars.length < 10) { win.contentElement.textContent = ""; win.setContent("Not enough intraday bar data for Market Profile."); return; }
    const dayMap = {};
    for (const b of bars) {
      const d = typeof b.time === "number" ? new Date(b.time * 1000).toISOString().slice(0, 10) : String(b.time).slice(0, 10);
      if (!dayMap[d]) dayMap[d] = [];
      dayMap[d].push(b);
    }
    const days = Object.keys(dayMap).sort();
    let globalHigh = -Infinity, globalLow = Infinity;
    for (const b of bars) { if (b.high > globalHigh) globalHigh = b.high; if (b.low < globalLow) globalLow = b.low; }
    const NUM_BINS = 40;
    const binSize = (globalHigh - globalLow) / NUM_BINS;
    if (binSize <= 0) { win.contentElement.textContent = ""; win.setContent("Price range too narrow for profile."); return; }
    const binPrices = [];
    for (let i = 0; i < NUM_BINS; i++) binPrices.push(globalLow + (i + 0.5) * binSize);
    const tpoGrid = Array.from({ length: NUM_BINS }, () => []);
    const tpoCounts = new Array(NUM_BINS).fill(0);
    const LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    const todayKey = days[days.length - 1];
    for (let di = 0; di < days.length; di++) {
      const dayBars = dayMap[days[di]];
      for (let pi = 0; pi < dayBars.length && pi < LETTERS.length; pi++) {
        const b = dayBars[pi];
        const letter = LETTERS[pi];
        const lowBin = Math.max(0, Math.floor((b.low - globalLow) / binSize));
        const highBin = Math.min(NUM_BINS - 1, Math.floor((b.high - globalLow) / binSize));
        for (let bin = lowBin; bin <= highBin; bin++) { tpoGrid[bin].push({ letter, dayIdx: di, isToday: days[di] === todayKey }); tpoCounts[bin]++; }
      }
    }
    let pocBin = 0, pocCount = 0;
    for (let i = 0; i < NUM_BINS; i++) { if (tpoCounts[i] > pocCount) { pocCount = tpoCounts[i]; pocBin = i; } }
    const pocPrice = binPrices[pocBin];
    const totalTPOs = tpoCounts.reduce((a, b) => a + b, 0);
    const vaTarget = Math.floor(totalTPOs * 0.7);
    let vaCount = tpoCounts[pocBin];
    let vaLow = pocBin, vaHigh = pocBin;
    while (vaCount < vaTarget) {
      const upBin = vaHigh + 1 < NUM_BINS ? vaHigh + 1 : -1;
      const dnBin = vaLow - 1 >= 0 ? vaLow - 1 : -1;
      const upVal = upBin >= 0 ? tpoCounts[upBin] : -1;
      const dnVal = dnBin >= 0 ? tpoCounts[dnBin] : -1;
      if (upVal < 0 && dnVal < 0) break;
      if (upVal >= dnVal) { vaHigh = upBin; vaCount += upVal; } else { vaLow = dnBin; vaCount += dnVal; }
    }
    const vaHighPrice = binPrices[vaHigh] + binSize / 2;
    const vaLowPrice = binPrices[vaLow] - binSize / 2;
    const todayBars = dayMap[todayKey] || [];
    let ibHigh = -Infinity, ibLow = Infinity;
    const ibPeriods = Math.min(2, todayBars.length);
    for (let i = 0; i < ibPeriods; i++) { if (todayBars[i].high > ibHigh) ibHigh = todayBars[i].high; if (todayBars[i].low < ibLow) ibLow = todayBars[i].low; }
    let todayHigh = -Infinity, todayLow = Infinity;
    for (const b of todayBars) { if (b.high > todayHigh) todayHigh = b.high; if (b.low < todayLow) todayLow = b.low; }
    const priceSample = bars[bars.length - 1].close;
    const priceDecimals = priceSample > 100 ? 2 : priceSample > 1 ? 4 : 6;
    win.contentElement.textContent = "";
    const mpContainer = document.createElement("div");
    mpContainer.style.cssText = "padding:12px;font-family:monospace;font-size:12px;color:#ddd;overflow:auto;height:100%;";
    const mpHeader = document.createElement("div");
    mpHeader.style.cssText = "margin-bottom:12px;line-height:1.6;";
    mpHeader.innerHTML =
      `<div style="font-size:16px;font-weight:bold;color:#ffd700;margin-bottom:8px;">Market Profile — ${sym}</div>` +
      `<span style="color:#ffd700;">POC: ${pocPrice.toFixed(priceDecimals)}</span>` +
      `<span style="color:#888;margin:0 12px;">|</span>` +
      `<span style="color:#00e5ff;">VA High: ${vaHighPrice.toFixed(priceDecimals)}</span>` +
      `<span style="color:#888;margin:0 12px;">|</span>` +
      `<span style="color:#00e5ff;">VA Low: ${vaLowPrice.toFixed(priceDecimals)}</span>` +
      (todayHigh > -Infinity ? `<span style="color:#888;margin:0 12px;">|</span><span style="color:#aaa;">Today: ${todayLow.toFixed(priceDecimals)} — ${todayHigh.toFixed(priceDecimals)}</span>` : "") +
      (ibHigh > -Infinity ? `<br><span style="color:#c0c;">IB Range: ${ibLow.toFixed(priceDecimals)} — ${ibHigh.toFixed(priceDecimals)}</span>` : "") +
      `<br><span style="color:#666;">Sessions: ${days.length} days, ${bars.length} bars, ${totalTPOs} TPOs</span>`;
    mpContainer.appendChild(mpHeader);
    const pre = document.createElement("pre");
    pre.style.cssText = "margin:0;line-height:1.3;font-size:11px;white-space:pre;";
    for (let bin = NUM_BINS - 1; bin >= 0; bin--) {
      const price = binPrices[bin].toFixed(priceDecimals);
      const pricePad = price.padStart(priceDecimals + 8);
      const isPOC = bin === pocBin;
      const isVAEdge = bin === vaHigh || bin === vaLow;
      const isVA = bin >= vaLow && bin <= vaHigh;
      const priceSpan = document.createElement("span");
      priceSpan.style.color = isPOC ? "#ffd700" : isVAEdge ? "#00e5ff" : isVA ? "#88aacc" : "#888";
      priceSpan.textContent = pricePad + " | ";
      pre.appendChild(priceSpan);
      for (const tpo of tpoGrid[bin]) {
        const ls = document.createElement("span");
        ls.style.color = tpo.isToday ? "#4caf50" : "#b0b0b0";
        if (isPOC) ls.style.background = "rgba(255,215,0,0.15)";
        ls.textContent = tpo.letter;
        pre.appendChild(ls);
      }
      let tag = "";
      if (isPOC) tag = " << POC";
      else if (bin === vaHigh) tag = " VH";
      else if (bin === vaLow) tag = " VL";
      if (tag) { const tagSpan = document.createElement("span"); tagSpan.style.color = isPOC ? "#ffd700" : "#00e5ff"; tagSpan.textContent = tag; pre.appendChild(tagSpan); }
      pre.appendChild(document.createTextNode("\n"));
    }
    mpContainer.appendChild(pre);
    const legend = document.createElement("div");
    legend.style.cssText = "margin-top:12px;color:#666;font-size:10px;";
    legend.innerHTML = '<span style="color:#4caf50;">Green</span> = today\'s session. <span style="color:#ffd700;">Yellow</span> = POC (Point of Control). <span style="color:#00e5ff;">Cyan</span> = Value Area edges. Letters A,B,C... = 30-min periods within each day.';
    mpContainer.appendChild(legend);
    win.appendElement(mpContainer);
  } catch (e) { win.contentElement.textContent = ""; win.setContent(`Failed to build Market Profile: ${e}`); }
}

// ══════════════════════════════════════════════════════════════
// BACKTEST+ — Visual Strategy Builder (No Code)
// ══════════════════════════════════════════════════════════════
function cmdBacktestPlus() {
  const sym = currentSymbol || "SPY";
  const win = createWindow({ title: `${sym} — Visual Strategy Builder`, width: 750, height: 620 });
  win.contentElement.textContent = "";
  const bpContainer = document.createElement("div");
  bpContainer.style.cssText = "padding:12px;font-family:Consolas,monospace;font-size:12px;color:#ddd;overflow-y:auto;height:100%;";
  function bpSelect(label, options, defaultVal) {
    const wrap = document.createElement("div"); wrap.style.cssText = "display:inline-block;margin-right:8px;margin-bottom:6px;";
    const lbl = document.createElement("label"); lbl.style.cssText = "color:#888;font-size:10px;display:block;margin-bottom:2px;"; lbl.textContent = label; wrap.appendChild(lbl);
    const sel = document.createElement("select"); sel.style.cssText = "background:#1a1a2e;color:#ddd;border:1px solid #444;padding:4px 6px;font-size:11px;font-family:inherit;";
    for (const o of options) { const opt = document.createElement("option"); opt.value = typeof o === "object" ? o.value : o; opt.textContent = typeof o === "object" ? o.label : o; if ((typeof o === "object" ? o.value : o) === defaultVal) opt.selected = true; sel.appendChild(opt); }
    wrap.appendChild(sel); return { el: wrap, sel };
  }
  function bpInput(label, value, width) {
    const wrap = document.createElement("div"); wrap.style.cssText = "display:inline-block;margin-right:8px;margin-bottom:6px;";
    const lbl = document.createElement("label"); lbl.style.cssText = "color:#888;font-size:10px;display:block;margin-bottom:2px;"; lbl.textContent = label; wrap.appendChild(lbl);
    const inp = document.createElement("input"); inp.type = "text"; inp.value = value; inp.style.cssText = `background:#1a1a2e;color:#ddd;border:1px solid #444;padding:4px 6px;font-size:11px;font-family:inherit;width:${width || "60px"};`; wrap.appendChild(inp);
    return { el: wrap, inp };
  }
  const bpIndicators = [{ value: "fisher", label: "Fisher Transform" }, { value: "rsi", label: "RSI (14)" }, { value: "kama_slope", label: "KAMA Slope" }, { value: "price_vs_sma", label: "Price vs SMA" }];
  const bpConditions = [{ value: "crosses_above", label: "crosses above" }, { value: "crosses_below", label: "crosses below" }, { value: "is_above", label: "is above" }, { value: "is_below", label: "is below" }];
  // Entry
  const entryTitle = document.createElement("div"); entryTitle.style.cssText = "font-size:14px;font-weight:bold;color:#4caf50;margin-bottom:8px;"; entryTitle.textContent = "Entry Conditions"; bpContainer.appendChild(entryTitle);
  const entryRow1 = document.createElement("div");
  const e1Ind = bpSelect("Indicator 1", bpIndicators, "fisher"); const e1Cond = bpSelect("Condition", bpConditions, "crosses_above"); const e1Val = bpInput("Value", "0");
  entryRow1.appendChild(e1Ind.el); entryRow1.appendChild(e1Cond.el); entryRow1.appendChild(e1Val.el); bpContainer.appendChild(entryRow1);
  const logicRow = document.createElement("div"); logicRow.style.cssText = "margin:4px 0 8px;";
  const logicSel = bpSelect("Logic", ["AND", "OR"], "AND"); logicRow.appendChild(logicSel.el); bpContainer.appendChild(logicRow);
  const entryRow2 = document.createElement("div");
  const e2Ind = bpSelect("Indicator 2 (optional)", [{ value: "none", label: "(none)" }, ...bpIndicators], "none"); const e2Cond = bpSelect("Condition", bpConditions, "is_above"); const e2Val = bpInput("Value", "50");
  entryRow2.appendChild(e2Ind.el); entryRow2.appendChild(e2Cond.el); entryRow2.appendChild(e2Val.el); bpContainer.appendChild(entryRow2);
  // Exit
  const exitTitle = document.createElement("div"); exitTitle.style.cssText = "font-size:14px;font-weight:bold;color:#f44336;margin:16px 0 8px;"; exitTitle.textContent = "Exit Conditions"; bpContainer.appendChild(exitTitle);
  const exitRow = document.createElement("div");
  const exitType = bpSelect("Exit Method", [{ value: "indicator", label: "Indicator condition" }, { value: "fixed_bars", label: "Hold for N bars" }, { value: "trailing_stop", label: "Trailing stop (%)" }], "indicator");
  exitRow.appendChild(exitType.el); bpContainer.appendChild(exitRow);
  const exitIndRow = document.createElement("div");
  const exInd = bpSelect("Indicator", bpIndicators, "fisher"); const exCond = bpSelect("Condition", bpConditions, "crosses_below"); const exVal = bpInput("Value", "0");
  exitIndRow.appendChild(exInd.el); exitIndRow.appendChild(exCond.el); exitIndRow.appendChild(exVal.el); bpContainer.appendChild(exitIndRow);
  const exitParamRow = document.createElement("div"); exitParamRow.style.display = "none";
  const exitParamInput = bpInput("Value", "10", "80px"); exitParamRow.appendChild(exitParamInput.el); bpContainer.appendChild(exitParamRow);
  exitType.sel.addEventListener("change", () => { const v = exitType.sel.value; exitIndRow.style.display = v === "indicator" ? "" : "none"; exitParamRow.style.display = v !== "indicator" ? "" : "none"; if (v === "fixed_bars") exitParamInput.inp.value = "10"; else if (v === "trailing_stop") exitParamInput.inp.value = "2"; });
  // Settings
  const settingsTitle = document.createElement("div"); settingsTitle.style.cssText = "font-size:14px;font-weight:bold;color:#ff9800;margin:16px 0 8px;"; settingsTitle.textContent = "Settings"; bpContainer.appendChild(settingsTitle);
  const settingsRow = document.createElement("div");
  const sEquity = bpInput("Initial Equity ($)", "10000", "80px"); const sPosSize = bpInput("Position Size (%)", "10", "60px"); const sCommission = bpInput("Commission ($)", "1", "60px");
  settingsRow.appendChild(sEquity.el); settingsRow.appendChild(sPosSize.el); settingsRow.appendChild(sCommission.el); bpContainer.appendChild(settingsRow);
  const btnRow = document.createElement("div"); btnRow.style.cssText = "margin:16px 0 12px;";
  const runBtn = document.createElement("button"); runBtn.textContent = "Run Backtest"; runBtn.style.cssText = "background:#3a0f60;color:#c8f;border:1px solid #555;padding:8px 20px;cursor:pointer;font-family:inherit;font-size:12px;font-weight:bold;";
  btnRow.appendChild(runBtn); bpContainer.appendChild(btnRow);
  const resultsArea = document.createElement("div"); bpContainer.appendChild(resultsArea);
  win.appendElement(bpContainer);
  // Indicator helpers
  function bpGetSeries(name, data) {
    if (name === "fisher") { const f = calcEhlersFisher(data, 32); const m = {}; for (const pt of f.fisher) m[pt.time] = pt.value; return m; }
    if (name === "rsi") { const r = calcRSI(data, 14); const m = {}; for (const pt of r) m[pt.time] = pt.value; return m; }
    if (name === "kama_slope") { const k = calcKAMA(data, 10); const m = {}; for (let i = 1; i < k.length; i++) m[k[i].time] = k[i].value - k[i - 1].value; return m; }
    if (name === "price_vs_sma") { const sma = calcSMA(data, 50); const sm = {}; for (const pt of sma) sm[pt.time] = pt.value; const m = {}; for (const bar of data) { if (sm[bar.time] !== undefined) m[bar.time] = bar.close - sm[bar.time]; } return m; }
    return {};
  }
  function bpCheckCond(condType, curVal, prevVal, threshold) {
    if (curVal === undefined || prevVal === undefined) return false;
    const thr = parseFloat(threshold) || 0;
    if (condType === "crosses_above") return prevVal <= thr && curVal > thr;
    if (condType === "crosses_below") return prevVal >= thr && curVal < thr;
    if (condType === "is_above") return curVal > thr;
    if (condType === "is_below") return curVal < thr;
    return false;
  }
  runBtn.addEventListener("click", () => {
    const data = currentChartData;
    if (!data || data.length < 60) { resultsArea.textContent = "Need at least 60 bars loaded on the chart."; return; }
    runBtn.disabled = true; runBtn.textContent = "Running..."; resultsArea.textContent = "";
    const equity0 = parseFloat(sEquity.inp.value) || 10000;
    const posPct = (parseFloat(sPosSize.inp.value) || 10) / 100;
    const commission = parseFloat(sCommission.inp.value) || 1;
    const ind1Map = bpGetSeries(e1Ind.sel.value, data);
    const ind2Name = e2Ind.sel.value;
    const ind2Map = ind2Name !== "none" ? bpGetSeries(ind2Name, data) : null;
    const exIndMap = exitType.sel.value === "indicator" ? bpGetSeries(exInd.sel.value, data) : null;
    const logic = logicSel.sel.value;
    const exitMethod = exitType.sel.value;
    const exitParamVal = parseFloat(exitParamInput.inp.value) || 10;
    const trades = []; let equity = equity0; let inTrade = false, entryPrice = 0, entryIdx = 0, peakPrice = 0;
    const equityCurve = [{ idx: 0, equity }];
    for (let i = 1; i < data.length; i++) {
      const bar = data[i]; const prevBar = data[i - 1]; const t = bar.time; const pt = prevBar.time;
      if (!inTrade) {
        const c1 = bpCheckCond(e1Cond.sel.value, ind1Map[t], ind1Map[pt], e1Val.inp.value);
        let enter = c1;
        if (ind2Map) { const c2 = bpCheckCond(e2Cond.sel.value, ind2Map[t], ind2Map[pt], e2Val.inp.value); enter = logic === "AND" ? (c1 && c2) : (c1 || c2); }
        if (enter) { inTrade = true; entryPrice = bar.close; entryIdx = i; peakPrice = bar.close; equity -= commission; }
      } else {
        if (bar.high > peakPrice) peakPrice = bar.high;
        let doExit = false;
        if (exitMethod === "indicator") { doExit = bpCheckCond(exCond.sel.value, exIndMap[t], exIndMap[pt], exVal.inp.value); }
        else if (exitMethod === "fixed_bars") { doExit = (i - entryIdx) >= exitParamVal; }
        else if (exitMethod === "trailing_stop") { doExit = bar.close <= peakPrice * (1 - exitParamVal / 100); }
        if (doExit || i === data.length - 1) {
          const exitPrice = bar.close; const shares = Math.floor((equity * posPct) / entryPrice) || 1;
          const pnl = (exitPrice - entryPrice) * shares - commission; equity += pnl;
          trades.push({ entryIdx, exitIdx: i, entryTime: data[entryIdx].time, exitTime: bar.time, entryPrice, exitPrice, pnl, shares });
          inTrade = false;
        }
      }
      equityCurve.push({ idx: i, equity });
    }
    const totalTrades = trades.length; const wins = trades.filter(t => t.pnl > 0).length;
    const winRate = totalTrades > 0 ? (wins / totalTrades * 100).toFixed(1) : "0.0";
    const grossProfit = trades.filter(t => t.pnl > 0).reduce((s, t) => s + t.pnl, 0);
    const grossLoss = Math.abs(trades.filter(t => t.pnl <= 0).reduce((s, t) => s + t.pnl, 0));
    const profitFactor = grossLoss > 0 ? (grossProfit / grossLoss).toFixed(2) : "N/A";
    const totalReturn = ((equity - equity0) / equity0 * 100).toFixed(2);
    let peakEq = equity0, maxDD = 0;
    for (const eqPt of equityCurve) { if (eqPt.equity > peakEq) peakEq = eqPt.equity; const dd = peakEq - eqPt.equity; if (dd > maxDD) maxDD = dd; }
    const statsHtml = document.createElement("div"); statsHtml.style.cssText = "margin-bottom:12px;";
    statsHtml.innerHTML =
      `<div style="font-size:14px;font-weight:bold;color:#ffd700;margin-bottom:8px;">Results</div>` +
      `<table style="border-collapse:collapse;font-size:12px;">` +
      `<tr><td style="padding:3px 12px 3px 0;color:#888;">Total Trades</td><td style="color:#ddd;">${totalTrades}</td></tr>` +
      `<tr><td style="padding:3px 12px 3px 0;color:#888;">Win Rate</td><td style="color:${parseFloat(winRate) >= 50 ? "#4caf50" : "#f44336"};">${winRate}%</td></tr>` +
      `<tr><td style="padding:3px 12px 3px 0;color:#888;">Profit Factor</td><td style="color:#ddd;">${profitFactor}</td></tr>` +
      `<tr><td style="padding:3px 12px 3px 0;color:#888;">Max Drawdown</td><td style="color:#f44336;">$${maxDD.toFixed(2)}</td></tr>` +
      `<tr><td style="padding:3px 12px 3px 0;color:#888;">Total Return</td><td style="color:${parseFloat(totalReturn) >= 0 ? "#4caf50" : "#f44336"};">${totalReturn}%</td></tr>` +
      `<tr><td style="padding:3px 12px 3px 0;color:#888;">Final Equity</td><td style="color:#ddd;">$${equity.toFixed(2)}</td></tr></table>`;
    resultsArea.appendChild(statsHtml);
    // Equity curve CSS bars
    if (equityCurve.length > 1) {
      const eqDiv = document.createElement("div"); eqDiv.style.cssText = "margin-bottom:12px;";
      const eqLabel = document.createElement("div"); eqLabel.style.cssText = "color:#888;font-size:10px;margin-bottom:4px;"; eqLabel.textContent = "Equity Curve"; eqDiv.appendChild(eqLabel);
      const chartDiv = document.createElement("div"); chartDiv.style.cssText = "display:flex;align-items:flex-end;height:80px;gap:0;background:#0a0a1a;border:1px solid #333;padding:4px;overflow:hidden;";
      const minEq = Math.min(...equityCurve.map(e => e.equity)); const maxEq = Math.max(...equityCurve.map(e => e.equity)); const eqRange = maxEq - minEq || 1;
      const step = Math.max(1, Math.floor(equityCurve.length / 200));
      for (let i = 0; i < equityCurve.length; i += step) { const eqPt = equityCurve[i]; const h = Math.max(1, ((eqPt.equity - minEq) / eqRange) * 70); const cssBar = document.createElement("div"); cssBar.style.cssText = `flex:1;min-width:1px;max-width:4px;height:${h}px;background:${eqPt.equity >= equity0 ? "#4caf50" : "#f44336"};`; chartDiv.appendChild(cssBar); }
      eqDiv.appendChild(chartDiv); resultsArea.appendChild(eqDiv);
    }
    // Trade list
    if (trades.length > 0) {
      const tlDiv = document.createElement("div"); tlDiv.style.cssText = "max-height:180px;overflow-y:auto;";
      const tlLabel = document.createElement("div"); tlLabel.style.cssText = "color:#888;font-size:10px;margin-bottom:4px;"; tlLabel.textContent = `Trades (${trades.length})`; tlDiv.appendChild(tlLabel);
      const tbl = document.createElement("table"); tbl.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;";
      const thead = document.createElement("tr");
      for (const h of ["Entry Date", "Exit Date", "Side", "Entry$", "Exit$", "P&L"]) { const th = document.createElement("td"); th.style.cssText = "color:#666;font-weight:bold;padding:3px 6px;border-bottom:1px solid #444;font-size:10px;"; th.textContent = h; thead.appendChild(th); }
      tbl.appendChild(thead);
      for (const trade of trades) {
        const row = document.createElement("tr"); row.style.cssText = "border-bottom:1px solid #222;";
        const entryDate = typeof trade.entryTime === "number" ? new Date(trade.entryTime * 1000).toISOString().slice(0, 10) : String(trade.entryTime).slice(0, 10);
        const exitDate = typeof trade.exitTime === "number" ? new Date(trade.exitTime * 1000).toISOString().slice(0, 10) : String(trade.exitTime).slice(0, 10);
        const pnlColor = trade.pnl >= 0 ? "#4caf50" : "#f44336";
        for (const [val, style] of [[entryDate, ""], [exitDate, ""], ["Long", "color:#4caf50;"], [`$${trade.entryPrice.toFixed(2)}`, ""], [`$${trade.exitPrice.toFixed(2)}`, ""], [`$${trade.pnl.toFixed(2)}`, `color:${pnlColor};font-weight:bold;`]]) { const td = document.createElement("td"); td.style.cssText = `padding:3px 6px;${style}`; td.textContent = val; row.appendChild(td); }
        tbl.appendChild(row);
      }
      tlDiv.appendChild(tbl); resultsArea.appendChild(tlDiv);
    }
    runBtn.disabled = false; runBtn.textContent = "Run Backtest";
  });
}

// ══════════════════════════════════════════════════════════════
// FLOWMAP — Sector Rotation Flow Map
// ══════════════════════════════════════════════════════════════
async function cmdFlowMap() {
  const SECTORS = [
    { sym: "XLK", name: "Technology" }, { sym: "XLF", name: "Financials" }, { sym: "XLE", name: "Energy" },
    { sym: "XLV", name: "Healthcare" }, { sym: "XLI", name: "Industrials" }, { sym: "XLC", name: "Communication" },
    { sym: "XLRE", name: "Real Estate" }, { sym: "XLU", name: "Utilities" }, { sym: "XLB", name: "Materials" },
    { sym: "XLP", name: "Cons. Staples" }, { sym: "XLY", name: "Cons. Discr." },
  ];
  const win = createWindow({ title: "Sector Rotation Flow Map", width: 650, height: 550 });
  win.contentElement.textContent = "";
  const fmLoading = document.createElement("div"); fmLoading.textContent = "Fetching sector data..."; fmLoading.style.cssText = "color:#888;padding:20px;"; win.appendElement(fmLoading);
  try {
    const results = [];
    for (const sector of SECTORS) {
      let bars = null;
      const cacheKey = `${sector.sym}:1Day`;
      if (barCache[cacheKey] && barCache[cacheKey].data && barCache[cacheKey].data.length >= 12) { bars = barCache[cacheKey].data; barCache[cacheKey].lastAccess = Date.now(); }
      else { try { const barsJson = await invoke("get_bars", { symbol: sector.sym, timeframe: "1Day", limit: 30 }); bars = JSON.parse(barsJson); if (bars.length > 0) barCache[cacheKey] = { data: bars, timestamp: Date.now(), lastAccess: Date.now() }; } catch (_) {} }
      if (!bars || bars.length < 12) { results.push({ ...sector, thisWeek: null, prevWeek: null, improving: null }); continue; }
      const len = bars.length;
      const thisWeekPct = bars[Math.max(0, len - 5)].open > 0 ? ((bars[len - 1].close - bars[Math.max(0, len - 5)].open) / bars[Math.max(0, len - 5)].open) * 100 : 0;
      const prevWeekPct = bars[Math.max(0, len - 10)].open > 0 ? ((bars[Math.max(0, len - 6)].close - bars[Math.max(0, len - 10)].open) / bars[Math.max(0, len - 10)].open) * 100 : 0;
      results.push({ ...sector, thisWeek: thisWeekPct, prevWeek: prevWeekPct, improving: thisWeekPct > prevWeekPct });
    }
    const valid = results.filter(r => r.thisWeek !== null);
    if (valid.length === 0) { win.contentElement.textContent = ""; win.setContent("No sector data available. Connect to a broker first."); return; }
    const maxMag = Math.max(...valid.map(r => Math.abs(r.thisWeek)), 1);
    win.contentElement.textContent = "";
    const fmContainer = document.createElement("div"); fmContainer.style.cssText = "padding:12px;font-family:Consolas,monospace;font-size:12px;color:#ddd;overflow-y:auto;height:100%;";
    const fmHeader = document.createElement("div"); fmHeader.style.cssText = "font-size:16px;font-weight:bold;color:#ffd700;margin-bottom:12px;text-align:center;"; fmHeader.textContent = "Sector Rotation Flow Map"; fmContainer.appendChild(fmHeader);
    const grid = document.createElement("div"); grid.style.cssText = "display:grid;grid-template-columns:repeat(4,1fr);gap:10px;margin-bottom:16px;";
    for (const r of results) {
      const box = document.createElement("div");
      const isValid = r.thisWeek !== null; const pct = isValid ? r.thisWeek : 0; const isImproving = r.improving;
      const scale = isValid ? Math.min(Math.abs(pct) / maxMag, 1) : 0.3;
      const borderColor = !isValid ? "#444" : isImproving ? "#4caf50" : "#f44336";
      const bgColor = !isValid ? "#111" : isImproving ? "rgba(76,175,80,0.08)" : "rgba(244,67,54,0.08)";
      box.style.cssText = `border:2px solid ${borderColor};background:${bgColor};border-radius:6px;padding:10px;text-align:center;min-height:${80 + scale * 50}px;display:flex;flex-direction:column;justify-content:center;align-items:center;`;
      const symLabel = document.createElement("div"); symLabel.style.cssText = "font-weight:bold;font-size:14px;color:#fff;"; symLabel.textContent = r.sym; box.appendChild(symLabel);
      const nameLabel = document.createElement("div"); nameLabel.style.cssText = "font-size:10px;color:#888;margin:2px 0 6px;"; nameLabel.textContent = r.name; box.appendChild(nameLabel);
      if (isValid) {
        const arrow = document.createElement("div"); arrow.style.cssText = `font-size:20px;color:${isImproving ? "#4caf50" : "#f44336"};`; arrow.textContent = isImproving ? "\u25B2" : "\u25BC"; box.appendChild(arrow);
        const pctLabel = document.createElement("div"); pctLabel.style.cssText = `font-size:16px;font-weight:bold;color:${pct >= 0 ? "#4caf50" : "#f44336"};margin-top:4px;`; pctLabel.textContent = `${pct >= 0 ? "+" : ""}${pct.toFixed(2)}%`; box.appendChild(pctLabel);
        const flowLabel = document.createElement("div"); flowLabel.style.cssText = `font-size:9px;color:${isImproving ? "#4caf50" : "#f44336"};margin-top:2px;`; flowLabel.textContent = isImproving ? "IMPROVING" : "DETERIORATING"; box.appendChild(flowLabel);
      } else { const na = document.createElement("div"); na.style.cssText = "color:#666;font-size:11px;"; na.textContent = "No data"; box.appendChild(na); }
      grid.appendChild(box);
    }
    fmContainer.appendChild(grid);
    const improvingSectors = valid.filter(r => r.improving); const deterioratingSectors = valid.filter(r => !r.improving);
    const summary = document.createElement("div"); summary.style.cssText = "border-top:1px solid #333;padding-top:12px;line-height:1.8;";
    if (improvingSectors.length > 0) { const inFlow = document.createElement("div"); inFlow.innerHTML = `<span style="color:#4caf50;font-weight:bold;">Money flowing INTO:</span> <span style="color:#ddd;">${improvingSectors.map(r => r.name).join(", ")}</span>`; summary.appendChild(inFlow); }
    if (deterioratingSectors.length > 0) { const outFlow = document.createElement("div"); outFlow.innerHTML = `<span style="color:#f44336;font-weight:bold;">Money flowing OUT OF:</span> <span style="color:#ddd;">${deterioratingSectors.map(r => r.name).join(", ")}</span>`; summary.appendChild(outFlow); }
    const sorted = [...valid].sort((a, b) => b.thisWeek - a.thisWeek);
    const topPerf = document.createElement("div"); topPerf.style.cssText = "margin-top:8px;color:#888;font-size:11px;";
    topPerf.innerHTML = `<span style="color:#4caf50;">Best:</span> ${sorted[0].sym} (${sorted[0].thisWeek >= 0 ? "+" : ""}${sorted[0].thisWeek.toFixed(2)}%)<span style="margin:0 12px;color:#444;">|</span><span style="color:#f44336;">Worst:</span> ${sorted[sorted.length - 1].sym} (${sorted[sorted.length - 1].thisWeek >= 0 ? "+" : ""}${sorted[sorted.length - 1].thisWeek.toFixed(2)}%)`;
    summary.appendChild(topPerf); fmContainer.appendChild(summary); win.appendElement(fmContainer);
  } catch (e) { win.contentElement.textContent = ""; win.setContent(`Failed to build flow map: ${e}`); }
}

// ══════════════════════════════════════════════════════════════
// REGIME+ — Advanced Regime Detection Dashboard
// ══════════════════════════════════════════════════════════════
async function cmdRegimePlus() {
  if (!currentSymbol) { log("Load a chart first", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} \u2014 Regime Detection`, width: 640, height: 580 });
  win.contentElement.textContent = "";
  const ld = document.createElement("div"); ld.textContent = "Computing regime indicators..."; ld.style.cssText = "color:#888;padding:20px;"; win.appendElement(ld);
  try {
    let data = null; const ck = `${currentSymbol}:1Day`; const cc = barCache[ck];
    if (cc && cc.data && cc.data.length > 100) { data = cc.data; cc.lastAccess = Date.now(); }
    if (!data) { const j = await invoke("get_bars", { symbol: currentSymbol, timeframe: "1Day", limit: 500 }); const b = JSON.parse(j); if (b.length > 100) { data = b; barCache[ck] = { data: b, timestamp: Date.now(), lastAccess: Date.now() }; } }
    if (!data || data.length < 100) { win.contentElement.textContent = ""; win.setContent("Need at least 100 daily bars for regime analysis."); return; }
    const closes = data.map(b => b.close); const n = closes.length;
    const rets = []; for (let i = 1; i < n; i++) rets.push(closes[i] > 0 ? (closes[i] - closes[i-1]) / closes[i-1] : 0);
    function rStd(a, w) { const o = []; for (let i = w-1; i < a.length; i++) { let s=0,q=0; for (let j=i-w+1;j<=i;j++){s+=a[j];q+=a[j]*a[j];} const m=s/w; o.push(Math.sqrt(Math.max(0,q/w-m*m))); } return o; }
    const s20 = rStd(rets, 20), s60 = rStd(rets, 60);
    const ls20 = s20.length > 0 ? s20[s20.length-1] : 0, as60 = s60.length > 0 ? s60[s60.length-1] : ls20;
    const vr = as60 > 0 ? ls20 / as60 : 1;
    let volR, volC; if (vr > 1.5) { volR = "HIGH VOL"; volC = "#f44336"; } else if (vr < 0.7) { volR = "LOW VOL"; volC = "#4caf50"; } else { volR = "NORMAL"; volC = "#ff9800"; }
    const adx = calcADX(data, 14); const lADX = adx.adx.length > 0 ? adx.adx[adx.adx.length-1].value : 0;
    let tR, tC; if (lADX > 25) { tR = "TRENDING"; tC = "#4caf50"; } else if (lADX >= 15) { tR = "TRANSITIONING"; tC = "#ff9800"; } else { tR = "RANGING"; tC = "#f44336"; }
    function cHurst(ser) { const len = Math.min(ser.length, 100); if (len < 20) return 0.5; const seg = ser.slice(ser.length - len); const parts = [10,20,25,50]; const lRS = [], lN = [];
      for (const sz of parts) { if (sz > len) continue; const nb = Math.floor(len / sz); if (nb < 1) continue; let tRS = 0, vb = 0;
        for (let b = 0; b < nb; b++) { const bl = seg.slice(b*sz, (b+1)*sz); const mn = bl.reduce((a,v)=>a+v,0)/bl.length; const dv = bl.map(v=>v-mn); let cs=0,mx=-Infinity,mi=Infinity; for (const d of dv){cs+=d;if(cs>mx)mx=cs;if(cs<mi)mi=cs;} const R=mx-mi; let sq=0; for(const d of dv)sq+=d*d; const S=Math.sqrt(sq/bl.length); if(S>0){tRS+=R/S;vb++;} }
        if (vb > 0) { lRS.push(Math.log(tRS/vb)); lN.push(Math.log(sz)); } }
      if (lRS.length < 2) return 0.5; let sx=0,sy=0,sxy=0,sx2=0; for(let i=0;i<lRS.length;i++){sx+=lN[i];sy+=lRS[i];sxy+=lN[i]*lRS[i];sx2+=lN[i]*lN[i];} const m=lRS.length; const dn=m*sx2-sx*sx; return dn!==0?(m*sxy-sx*sy)/dn:0.5; }
    const hurst = cHurst(rets);
    let hR, hC; if (hurst > 0.55) { hR = "TRENDING"; hC = "#4caf50"; } else if (hurst < 0.45) { hR = "MEAN REVERTING"; hC = "#2196f3"; } else { hR = "RANDOM WALK"; hC = "#ff9800"; }
    const roc20 = n > 20 ? (closes[n-1] - closes[n-21]) / closes[n-21] : 0;
    const kd = calcKAMA(data, 10); let ks = 0; if (kd.length >= 5) ks = kd[kd.length-1].value - kd[kd.length-5].value;
    let mR, mC; if (roc20 > 0 && ks > 0) { mR = "BULLISH MOMENTUM"; mC = "#4caf50"; } else if (roc20 < 0 && ks < 0) { mR = "BEARISH MOMENTUM"; mC = "#f44336"; } else { mR = "CHOPPY"; mC = "#ff9800"; }
    const trans = []; const lb = Math.min(data.length - 61, 200); let pVR = "";
    for (let i = Math.max(0, rets.length - lb); i < rets.length; i++) { if (i < 59) continue; const a = rStd(rets.slice(0,i+1),20), b = rStd(rets.slice(0,i+1),60); if(!a.length||!b.length)continue; const r=b[b.length-1]>0?a[a.length-1]/b[b.length-1]:1; const rg=r>1.5?"HIGH VOL":r<0.7?"LOW VOL":"NORMAL"; if(rg!==pVR&&pVR!==""){const bi=i+1;const bt=bi<data.length?data[bi].time:0;trans.push({time:bt,from:pVR,to:rg});} pVR=rg; }
    const rTrans = trans.slice(-10);
    let rec, recC; const tc = [tR==="TRENDING",hR==="TRENDING",mR==="BULLISH MOMENTUM"||mR==="BEARISH MOMENTUM"].filter(Boolean).length;
    if (volR==="HIGH VOL"&&tR==="RANGING") { rec = "Reduce size / stay flat"; recC = "#f44336"; } else if (tc >= 2) { rec = "Trend-following strategies"; recC = "#4caf50"; } else if (hR==="MEAN REVERTING"&&tR!=="TRENDING") { rec = "Mean-reversion strategies"; recC = "#2196f3"; } else { rec = "Reduce size / stay flat"; recC = "#ff9800"; }
    win.contentElement.textContent = "";
    const ct = document.createElement("div"); ct.style.cssText = "padding:12px;font-family:monospace;font-size:13px;color:#ddd;overflow-y:auto;height:100%;";
    const gr = document.createElement("div"); gr.style.cssText = "display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-bottom:16px;";
    function mc(ti, st, co, dt) { const cd = document.createElement("div"); cd.style.cssText = "background:#1a1a1a;border:1px solid #333;border-radius:6px;padding:12px;text-align:center;"; const d = document.createElement("span"); d.style.cssText = `display:inline-block;width:10px;height:10px;border-radius:50%;background:${co};margin-right:6px;`; const te = document.createElement("div"); te.style.cssText = "color:#888;font-size:11px;margin-bottom:6px;"; te.textContent = ti; const se = document.createElement("div"); se.style.cssText = `font-size:16px;font-weight:bold;color:${co};margin-bottom:4px;`; se.appendChild(d); se.appendChild(document.createTextNode(st)); const de = document.createElement("div"); de.style.cssText = "color:#666;font-size:10px;"; de.textContent = dt; cd.appendChild(te); cd.appendChild(se); cd.appendChild(de); return cd; }
    gr.appendChild(mc("Volatility Regime", volR, volC, `Ratio: ${vr.toFixed(2)}x (20d vs 60d)`));
    gr.appendChild(mc("Trend Regime", tR, tC, `ADX(14): ${lADX.toFixed(1)}`));
    gr.appendChild(mc("Mean Reversion", hR, hC, `Hurst: ${hurst.toFixed(3)}`));
    gr.appendChild(mc("Momentum", mR, mC, `ROC(20): ${(roc20*100).toFixed(2)}%, KAMA slope: ${ks>=0?"+":""}${ks.toFixed(4)}`));
    ct.appendChild(gr);
    const rd = document.createElement("div"); rd.style.cssText = `text-align:center;padding:10px;background:#111;border:1px solid ${recC};border-radius:6px;margin-bottom:16px;`; rd.innerHTML = `<span style="color:#888;font-size:11px;">RECOMMENDATION</span><br><span style="color:${recC};font-size:15px;font-weight:bold;">${rec}</span>`; ct.appendChild(rd);
    if (rTrans.length > 0) { const tt = document.createElement("div"); tt.style.cssText = "color:#888;font-size:11px;margin-bottom:6px;"; tt.textContent = "Recent Volatility Regime Transitions"; ct.appendChild(tt); const tbl = document.createElement("table"); tbl.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;"; tbl.innerHTML = `<thead><tr style="color:#888;border-bottom:1px solid #444;"><th style="text-align:left;padding:4px;">Date</th><th style="text-align:left;padding:4px;">From</th><th style="text-align:center;padding:4px;"></th><th style="text-align:left;padding:4px;">To</th></tr></thead>`; const tb = document.createElement("tbody");
      for (const t of rTrans) { const tr = document.createElement("tr"); tr.style.cssText = "border-bottom:1px solid #222;"; const ds = typeof t.time === "number" ? new Date(t.time*1000).toLocaleDateString() : String(t.time); const fc = t.from==="HIGH VOL"?"#f44336":t.from==="LOW VOL"?"#4caf50":"#ff9800"; const tc2 = t.to==="HIGH VOL"?"#f44336":t.to==="LOW VOL"?"#4caf50":"#ff9800"; tr.innerHTML = `<td style="padding:4px;color:#aaa;">${ds}</td><td style="padding:4px;color:${fc};">${t.from}</td><td style="padding:4px;text-align:center;color:#666;">&rarr;</td><td style="padding:4px;color:${tc2};">${t.to}</td>`; tb.appendChild(tr); }
      tbl.appendChild(tb); ct.appendChild(tbl); }
    win.appendElement(ct);
    log(`REGIME+ computed for ${currentSymbol}: ${volR} | ${tR} | ${hR} | ${mR}`, "ok");
  } catch (e) { win.contentElement.textContent = ""; win.setContent(`Failed to compute regime: ${e}`); }
}

// ══════════════════════════════════════════════════════════════
// RISKSIM — Scenario Stress Testing
// ══════════════════════════════════════════════════════════════
async function cmdRiskSim() {
  const win = createWindow({ title: "Risk Scenario Simulator", width: 700, height: 560 });
  win.contentElement.textContent = "";
  const ld = document.createElement("div"); ld.textContent = "Loading positions..."; ld.style.cssText = "color:#888;padding:20px;"; win.appendElement(ld);
  try {
    const pj = await invoke("get_positions"); const positions = JSON.parse(pj);
    if (!positions || positions.length === 0) { win.contentElement.textContent = ""; win.setContent("No open positions to stress test."); return; }
    let acEq = 0, acMg = 0;
    try { const aj = await invoke("get_account"); const ac = JSON.parse(aj); acEq = parseFloat(ac.equity || ac.portfolio_value || 0); acMg = parseFloat(ac.initial_margin || ac.margin_used || 0); } catch (_) {}
    const techS = ["AAPL","MSFT","GOOGL","GOOG","AMZN","META","NVDA","TSLA","AMD","INTC","CRM","ADBE","NFLX","AVGO","ORCL","QCOM","MU","NOW","SHOP","SQ","PLTR","SNOW","UBER"];
    const finS = ["JPM","BAC","GS","MS","WFC","C","BRK.B","AXP","V","MA","SCHW","BLK","COF","USB"];
    const cryS = ["BTCUSD","ETHUSD","SOLUSD","DOGEUSD","BTC/USD","ETH/USD","SOL/USD","DOGE/USD","BTCUSDT","ETHUSDT"];
    function cls(sym) { const s = sym.toUpperCase(); if (cryS.some(c => s.includes(c.replace("/","")))) return "crypto"; if (techS.includes(s)) return "tech"; if (finS.includes(s)) return "financial"; return "other"; }
    const SC = { "Market Crash (-20%)": {tech:-25,financial:-20,crypto:-40,other:-20}, "Correction (-10%)": {tech:-12,financial:-10,crypto:-20,other:-10}, "Rate Hike Shock": {tech:-5,financial:5,crypto:-3,other:-3}, "Crypto Winter": {tech:0,financial:0,crypto:-50,other:0} };
    win.contentElement.textContent = "";
    const ct = document.createElement("div"); ct.style.cssText = "padding:12px;font-family:monospace;font-size:13px;color:#ddd;overflow-y:auto;height:100%;";
    const br = document.createElement("div"); br.style.cssText = "display:flex;flex-wrap:wrap;gap:8px;margin-bottom:12px;";
    const rd = document.createElement("div");
    function runSc(nm, pm) {
      rd.textContent = ""; let tPnL = 0; const rows = [];
      for (const p of positions) { const sy = p.symbol||""; const mv = parseFloat(p.market_value||(parseFloat(p.qty||p.quantity||0)*parseFloat(p.current_price||p.avg_entry_price||0))||0); const sec = cls(sy); const sp = pm[sec]!==undefined?pm[sec]:(pm.other||0); const pnl = mv*(sp/100); tPnL += pnl; rows.push({sym:sy,mktVal:mv,sector:sec,scenarioPct:sp,pnl,newVal:mv+pnl}); }
      const pi = acEq > 0 ? (tPnL/acEq)*100 : 0; const ma = acEq > 0 && acMg > 0 ? ((acEq+tPnL)/acMg)*100 : 0; const md = ma > 0 && ma < 150;
      const pc = tPnL >= 0 ? "#4caf50" : "#f44336"; const mc = md ? "#f44336" : "#4caf50";
      const sm = document.createElement("div"); sm.style.cssText = "background:#111;border:1px solid #333;border-radius:6px;padding:10px;margin-bottom:12px;display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;";
      sm.innerHTML = `<div style="text-align:center;"><div style="color:#888;font-size:10px;">SCENARIO P&L</div><div style="color:${pc};font-size:16px;font-weight:bold;">${tPnL>=0?"+":""}$${tPnL.toFixed(2)}</div></div><div style="text-align:center;"><div style="color:#888;font-size:10px;">% PORTFOLIO</div><div style="color:${pc};font-size:16px;font-weight:bold;">${pi>=0?"+":""}${pi.toFixed(2)}%</div></div><div style="text-align:center;"><div style="color:#888;font-size:10px;">MARGIN LEVEL</div><div style="color:${mc};font-size:16px;font-weight:bold;">${acMg>0?ma.toFixed(0)+"%":"N/A"}${md?" DANGER":""}</div></div>`;
      rd.appendChild(sm);
      const tbl = document.createElement("table"); tbl.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;";
      tbl.innerHTML = `<thead><tr style="color:#888;border-bottom:1px solid #444;"><th style="text-align:left;padding:4px;">Symbol</th><th style="text-align:right;padding:4px;">Current Value</th><th style="text-align:right;padding:4px;">Scenario %</th><th style="text-align:right;padding:4px;">Scenario P&L</th><th style="text-align:right;padding:4px;">New Value</th></tr></thead>`;
      const tb = document.createElement("tbody");
      for (const r of rows) { const tr = document.createElement("tr"); tr.style.cssText = "border-bottom:1px solid #222;"; const c = r.pnl>=0?"#4caf50":"#f44336"; tr.innerHTML = `<td style="padding:4px;">${r.sym}<span style="color:#555;font-size:9px;"> ${r.sector}</span></td><td style="text-align:right;padding:4px;">$${r.mktVal.toFixed(2)}</td><td style="text-align:right;padding:4px;color:${c};">${r.scenarioPct>=0?"+":""}${r.scenarioPct}%</td><td style="text-align:right;padding:4px;color:${c};">${r.pnl>=0?"+":""}$${r.pnl.toFixed(2)}</td><td style="text-align:right;padding:4px;">$${r.newVal.toFixed(2)}</td>`; tb.appendChild(tr); }
      tbl.appendChild(tb); rd.appendChild(tbl);
    }
    for (const [nm, pm] of Object.entries(SC)) { const b = document.createElement("button"); b.textContent = nm; b.style.cssText = "background:#222;color:#ddd;border:1px solid #444;border-radius:4px;padding:6px 12px;cursor:pointer;font-family:monospace;font-size:11px;"; b.addEventListener("mouseenter",()=>{b.style.background="#333";}); b.addEventListener("mouseleave",()=>{b.style.background="#222";}); b.addEventListener("click",()=>runSc(nm,pm)); br.appendChild(b); }
    const cb = document.createElement("button"); cb.textContent = "Custom..."; cb.style.cssText = "background:#1a237e;color:#ddd;border:1px solid #3949ab;border-radius:4px;padding:6px 12px;cursor:pointer;font-family:monospace;font-size:11px;";
    cb.addEventListener("click", () => { rd.textContent = ""; const fm = document.createElement("div"); fm.style.cssText = "background:#111;border:1px solid #333;border-radius:6px;padding:12px;margin-bottom:12px;"; const ft = document.createElement("div"); ft.textContent = "Enter % change per symbol:"; ft.style.cssText = "color:#888;margin-bottom:8px;font-size:11px;"; fm.appendChild(ft); const inp = {};
      for (const p of positions) { const rw = document.createElement("div"); rw.style.cssText = "display:flex;align-items:center;gap:8px;margin-bottom:4px;"; const lb = document.createElement("span"); lb.textContent = p.symbol; lb.style.cssText = "width:80px;"; const ip = document.createElement("input"); ip.type = "number"; ip.value = "-10"; ip.style.cssText = "background:#222;color:#ddd;border:1px solid #444;border-radius:3px;padding:3px 6px;width:80px;font-family:monospace;"; const pl = document.createElement("span"); pl.textContent = "%"; pl.style.cssText = "color:#666;"; inp[p.symbol] = ip; rw.appendChild(lb); rw.appendChild(ip); rw.appendChild(pl); fm.appendChild(rw); }
      const ab = document.createElement("button"); ab.textContent = "Apply"; ab.style.cssText = "background:#2e7d32;color:#fff;border:none;border-radius:4px;padding:6px 16px;cursor:pointer;margin-top:8px;font-family:monospace;";
      ab.addEventListener("click", () => { const ps = {}; for (const p of positions) ps[p.symbol] = parseFloat(inp[p.symbol].value||0); rd.textContent = ""; let tPnL = 0; const rows = [];
        for (const p of positions) { const sy = p.symbol||""; const mv = parseFloat(p.market_value||(parseFloat(p.qty||p.quantity||0)*parseFloat(p.current_price||p.avg_entry_price||0))||0); const sp = ps[sy]||0; const pnl = mv*(sp/100); tPnL += pnl; rows.push({sym:sy,mktVal:mv,scenarioPct:sp,pnl,newVal:mv+pnl}); }
        const pi = acEq>0?(tPnL/acEq)*100:0; const ma = acEq>0&&acMg>0?((acEq+tPnL)/acMg)*100:0; const md = ma>0&&ma<150; const pc = tPnL>=0?"#4caf50":"#f44336"; const mc2 = md?"#f44336":"#4caf50";
        const sm = document.createElement("div"); sm.style.cssText = "background:#111;border:1px solid #333;border-radius:6px;padding:10px;margin-bottom:12px;display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;";
        sm.innerHTML = `<div style="text-align:center;"><div style="color:#888;font-size:10px;">SCENARIO P&L</div><div style="color:${pc};font-size:16px;font-weight:bold;">${tPnL>=0?"+":""}$${tPnL.toFixed(2)}</div></div><div style="text-align:center;"><div style="color:#888;font-size:10px;">% PORTFOLIO</div><div style="color:${pc};font-size:16px;font-weight:bold;">${pi>=0?"+":""}${pi.toFixed(2)}%</div></div><div style="text-align:center;"><div style="color:#888;font-size:10px;">MARGIN LEVEL</div><div style="color:${mc2};font-size:16px;font-weight:bold;">${acMg>0?ma.toFixed(0)+"%":"N/A"}${md?" DANGER":""}</div></div>`;
        rd.appendChild(sm); const tbl = document.createElement("table"); tbl.style.cssText = "width:100%;border-collapse:collapse;font-size:11px;"; tbl.innerHTML = `<thead><tr style="color:#888;border-bottom:1px solid #444;"><th style="text-align:left;padding:4px;">Symbol</th><th style="text-align:right;padding:4px;">Current Value</th><th style="text-align:right;padding:4px;">Scenario %</th><th style="text-align:right;padding:4px;">Scenario P&L</th><th style="text-align:right;padding:4px;">New Value</th></tr></thead>`; const tb = document.createElement("tbody");
        for (const r of rows) { const tr = document.createElement("tr"); tr.style.cssText = "border-bottom:1px solid #222;"; const c = r.pnl>=0?"#4caf50":"#f44336"; tr.innerHTML = `<td style="padding:4px;">${r.sym}</td><td style="text-align:right;padding:4px;">$${r.mktVal.toFixed(2)}</td><td style="text-align:right;padding:4px;color:${c};">${r.scenarioPct>=0?"+":""}${r.scenarioPct}%</td><td style="text-align:right;padding:4px;color:${c};">${r.pnl>=0?"+":""}$${r.pnl.toFixed(2)}</td><td style="text-align:right;padding:4px;">$${r.newVal.toFixed(2)}</td>`; tb.appendChild(tr); }
        tbl.appendChild(tb); rd.appendChild(tbl); }); fm.appendChild(ab); rd.appendChild(fm); });
    br.appendChild(cb); ct.appendChild(br); ct.appendChild(rd); win.appendElement(ct);
    const fs = Object.entries(SC)[0]; runSc(fs[0], fs[1]);
    log("RISKSIM loaded with " + positions.length + " positions", "ok");
  } catch (e) { win.contentElement.textContent = ""; win.setContent(`Failed to load risk sim: ${e}`); }
}

// ══════════════════════════════════════════════════════════════
// SMARTALERT — Statistical Anomaly Detection
// ══════════════════════════════════════════════════════════════
async function cmdSmartAlert() {
  if (!currentSymbol) { log("Load a chart first", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} \u2014 Anomaly Detection`, width: 620, height: 520 });
  win.contentElement.textContent = "";
  const ld = document.createElement("div"); ld.textContent = "Computing statistical anomalies..."; ld.style.cssText = "color:#888;padding:20px;"; win.appendElement(ld);
  try {
    let data = null; const ck = `${currentSymbol}:1Day`; const cc = barCache[ck];
    if (cc && cc.data && cc.data.length > 60) { data = cc.data; cc.lastAccess = Date.now(); }
    if (!data) { const j = await invoke("get_bars", { symbol: currentSymbol, timeframe: "1Day", limit: 500 }); const b = JSON.parse(j); if (b.length > 60) { data = b; barCache[ck] = { data: b, timestamp: Date.now(), lastAccess: Date.now() }; } }
    if (!data || data.length < 61) { win.contentElement.textContent = ""; win.setContent("Need at least 61 daily bars for anomaly detection."); return; }
    const n = data.length; const today = data[n-1];
    function zSc(v, m, s) { return s > 0 ? (v - m) / s : 0; }
    function mS(arr) { const m = arr.reduce((a,b)=>a+b,0)/arr.length; const v = arr.reduce((a,b)=>a+(b-m)*(b-m),0)/arr.length; return {mean:m,std:Math.sqrt(v)}; }
    function cExc(arr, th) { let c = 0; for (const v of arr) if (Math.abs(v) >= th) c++; return c; }
    const v60 = data.slice(n-61,n-1).map(b=>b.volume||0); const vSt = mS(v60); const tV = today.volume||0; const vZ = zSc(tV,vSt.mean,vSt.std);
    const rg60 = data.slice(n-61,n-1).map(b=>b.high-b.low); const rgSt = mS(rg60); const tRg = today.high-today.low; const rgZ = zSc(tRg,rgSt.mean,rgSt.std);
    const rt60 = []; for (let i=n-60;i<n;i++) rt60.push(data[i].close>0?(data[i].close-data[i-1].close)/data[i-1].close:0);
    const rtSt = mS(rt60.slice(0,-1)); const tRt = rt60[rt60.length-1]; const rtZ = zSc(tRt,rtSt.mean,rtSt.std);
    const fR = calcEhlersFisher(data, 32); let fZ = 0, fV = 0;
    if (fR.fisher.length > 0) { fV = fR.fisher[fR.fisher.length-1].value; const lb = Math.min(fR.fisher.length-1,200); const fVs = fR.fisher.slice(fR.fisher.length-1-lb,fR.fisher.length-1).map(f=>f.value); if (fVs.length > 10) { const fs = mS(fVs); fZ = zSc(fV,fs.mean,fs.std); } }
    const rsiR = calcRSI(data, 14); const rsiV = rsiR.length > 0 ? rsiR[rsiR.length-1].value : 50; const rsiE = rsiV < 20 || rsiV > 80;
    const kD = calcKAMA(data, 10); const aD = calcATR(data, 14); let pkZ = 0, pkV = 0;
    if (kD.length > 0 && aD.length > 0) { const lK = kD[kD.length-1].value; const lA = aD[aD.length-1].value; pkV = lA > 0 ? (today.close - lK) / lA : 0;
      const kM = new Map(kD.map(k=>[k.time,k.value])); const aM = new Map(aD.map(a=>[a.time,a.value])); const hV = []; const lb = Math.min(60,data.length-1);
      for (let i=n-1-lb;i<n-1;i++){const kv=kM.get(data[i].time),av=aM.get(data[i].time);if(kv!==undefined&&av!==undefined&&av>0)hV.push((data[i].close-kv)/av);}
      if (hV.length > 10) { const ps = mS(hV); pkZ = zSc(pkV, ps.mean, ps.std); } }
    const aVZ = []; for (let i=61;i<n;i++){const vs=data.slice(i-60,i).map(b=>b.volume||0);const s=mS(vs);if(s.std>0)aVZ.push(((data[i].volume||0)-s.mean)/s.std);}
    const vEC = cExc(aVZ, Math.abs(vZ)); const tBC = Math.min(aVZ.length, 200);
    const metrics = [
      {name:"Volume",value:tV.toLocaleString(),z:vZ,detail:`60d avg: ${vSt.mean.toFixed(0)}`},
      {name:"Range (H-L)",value:tRg.toFixed(4),z:rgZ,detail:`60d avg: ${rgSt.mean.toFixed(4)}`},
      {name:"Return",value:`${(tRt*100).toFixed(2)}%`,z:rtZ,detail:`60d avg: ${(rtSt.mean*100).toFixed(2)}%`},
      {name:"Fisher Transform",value:fV.toFixed(3),z:fZ,detail:`200-bar z-score`},
      {name:"RSI(14)",value:rsiV.toFixed(1),z:rsiE?(rsiV>80?3.0:-3.0):0,detail:rsiE?(rsiV>80?"OVERBOUGHT":"OVERSOLD"):"Normal range"},
      {name:"Price vs KAMA",value:`${pkV.toFixed(2)} ATR`,z:pkZ,detail:`Distance in ATR units`},
    ];
    const aC = metrics.filter(m=>Math.abs(m.z)>=2.0).length;
    win.contentElement.textContent = "";
    const ct = document.createElement("div"); ct.style.cssText = "padding:12px;font-family:monospace;font-size:13px;color:#ddd;overflow-y:auto;height:100%;";
    const sD = document.createElement("div"); const sC = aC===0?"#4caf50":aC<=2?"#ff9800":"#f44336";
    sD.style.cssText = `text-align:center;padding:10px;background:#111;border:1px solid ${sC};border-radius:6px;margin-bottom:14px;`;
    sD.innerHTML = `<span style="color:${sC};font-size:18px;font-weight:bold;">${aC} of 6</span><span style="color:#888;font-size:13px;"> metrics are unusual</span>${aC>0?`<br><span style="color:#ff9800;font-size:11px;">Heightened attention recommended</span>`:`<br><span style="color:#4caf50;font-size:11px;">All metrics within normal range</span>`}`;
    ct.appendChild(sD);
    const gr = document.createElement("div"); gr.style.cssText = "display:grid;grid-template-columns:1fr 1fr 1fr;gap:10px;margin-bottom:14px;";
    for (const m of metrics) { const az = Math.abs(m.z); let st, sc; if (az>=3){st="EXTREME";sc="#f44336";}else if(az>=2){st="UNUSUAL";sc="#ff9800";}else{st="NORMAL";sc="#4caf50";}
      const cd = document.createElement("div"); cd.style.cssText = `background:#1a1a1a;border:1px solid ${az>=2?sc:"#333"};border-radius:6px;padding:10px;text-align:center;`;
      const dt = document.createElement("span"); dt.style.cssText = `display:inline-block;width:8px;height:8px;border-radius:50%;background:${sc};margin-right:4px;`;
      const ne = document.createElement("div"); ne.style.cssText = "color:#888;font-size:10px;margin-bottom:4px;"; ne.textContent = m.name;
      const ve = document.createElement("div"); ve.style.cssText = "font-size:14px;font-weight:bold;color:#ddd;margin-bottom:2px;"; ve.textContent = m.value;
      const ze = document.createElement("div"); ze.style.cssText = `font-size:11px;color:${sc};margin-bottom:2px;`; ze.appendChild(dt); ze.appendChild(document.createTextNode(`z: ${m.z>=0?"+":""}${m.z.toFixed(2)} \u2014 ${st}`));
      const de = document.createElement("div"); de.style.cssText = "color:#555;font-size:9px;"; de.textContent = m.detail;
      cd.appendChild(ne); cd.appendChild(ve); cd.appendChild(ze); cd.appendChild(de); gr.appendChild(cd); }
    ct.appendChild(gr);
    if (Math.abs(vZ) >= 2) { const cx = document.createElement("div"); cx.style.cssText = "background:#111;border:1px solid #333;border-radius:6px;padding:10px;font-size:11px;color:#aaa;"; cx.innerHTML = `Volume is at <span style="color:#ff9800;font-weight:bold;">${Math.abs(vZ).toFixed(1)}\u03C3</span> \u2014 this has happened <span style="color:#fff;font-weight:bold;">${vEC}</span> times in the last ${tBC} bars`; ct.appendChild(cx); }
    win.appendElement(ct);
    log(`SMARTALERT for ${currentSymbol}: ${aC}/6 anomalies detected`, aC > 0 ? "warn" : "ok");
  } catch (e) { win.contentElement.textContent = ""; win.setContent(`Failed to compute anomalies: ${e}`); }
}

// ══════════════════════════════════════════════════════════════
// LADDER — Price Ladder / DOM Visualization
// ══════════════════════════════════════════════════════════════
function cmdLadder() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  let ladderInterval = null;
  const LEVELS = 20;
  const win = createWindow({
    title: `${currentSymbol} — Price Ladder`,
    width: 420, height: 550,
    onClose: () => { if (ladderInterval) { clearInterval(ladderInterval); ladderInterval = null; } },
  });
  win.contentElement.textContent = "";

  const container = document.createElement("div");
  container.style.cssText = "font-family:Consolas,monospace;font-size:11px;overflow-y:auto;max-height:calc(100% - 40px);";
  win.appendElement(container);

  const footer = document.createElement("div");
  footer.style.cssText = "padding:6px 8px;border-top:1px solid #333;font-size:10px;color:#888;display:flex;justify-content:space-between;";
  win.appendElement(footer);

  async function refresh() {
    let bids = [], asks = [], midPrice = 0;
    try {
      const json = await invokeQuiet("get_orderbook", { symbol: currentSymbol });
      const book = typeof json === "string" ? JSON.parse(json) : json;
      bids = Array.isArray(book.bids) ? book.bids : [];
      asks = Array.isArray(book.asks) ? book.asks : [];
      if (bids.length > 0 && asks.length > 0) {
        midPrice = (Math.max(...bids.map(b => b.price || 0)) + Math.min(...asks.map(a => a.price || Infinity))) / 2;
      }
    } catch (_) {
      // Orderbook not available, fall back to quote
      try {
        const qJson = await invokeQuiet("get_latest_quote", { symbol: currentSymbol });
        const q = typeof qJson === "string" ? JSON.parse(qJson) : qJson;
        const bid = q.bid || q.bp || 0;
        const ask = q.ask || q.ap || 0;
        if (bid > 0 && ask > 0) {
          midPrice = (bid + ask) / 2;
          bids = [{ price: bid, size: q.bidsz || q.bs || 1 }];
          asks = [{ price: ask, size: q.asksz || q.as || 1 }];
        }
      } catch (_) {}
    }

    if (midPrice <= 0 && lastPrice > 0) midPrice = lastPrice;
    if (midPrice <= 0) return;

    const dp = midPrice > 100 ? 2 : midPrice > 1 ? 4 : 6;
    const step = midPrice > 100 ? 0.01 : midPrice > 10 ? 0.005 : midPrice > 1 ? 0.001 : 0.0001;

    // Build level map
    const bidMap = {};
    const askMap = {};
    for (const b of bids) { const key = (b.price || 0).toFixed(dp); bidMap[key] = (bidMap[key] || 0) + (b.size || b.qty || 0); }
    for (const a of asks) { const key = (a.price || 0).toFixed(dp); askMap[key] = (askMap[key] || 0) + (a.size || a.qty || 0); }

    // Generate price levels centered on midPrice
    const levels = [];
    for (let i = LEVELS; i >= -LEVELS; i--) {
      const price = midPrice + i * step;
      const key = price.toFixed(dp);
      levels.push({ price, key, bidVol: bidMap[key] || 0, askVol: askMap[key] || 0 });
    }

    const allSizes = levels.map(l => Math.max(l.bidVol, l.askVol)).filter(v => v > 0);
    const maxSize = allSizes.length > 0 ? Math.max(...allSizes) : 1;
    const avgSize = allSizes.length > 0 ? allSizes.reduce((a, b) => a + b, 0) / allSizes.length : 1;
    const largeThreshold = avgSize * 2;

    container.textContent = "";
    for (const level of levels) {
      const row = document.createElement("div");
      row.style.cssText = "display:grid;grid-template-columns:1fr 80px 1fr;height:18px;align-items:center;border-bottom:1px solid #111;position:relative;";
      const isCurrentPrice = Math.abs(level.price - midPrice) < step * 0.5;
      if (isCurrentPrice) row.style.background = "rgba(255,235,59,0.15)";

      // Bid column (left)
      const bidCell = document.createElement("div");
      bidCell.style.cssText = "position:relative;text-align:right;padding-right:4px;height:100%;display:flex;align-items:center;justify-content:flex-end;";
      if (level.bidVol > 0) {
        const bar = document.createElement("div");
        const pct = (level.bidVol / maxSize) * 100;
        const bright = level.bidVol >= largeThreshold;
        bar.style.cssText = `position:absolute;right:0;top:0;height:100%;width:${pct}%;background:${bright ? "rgba(76,175,80,0.5)" : "rgba(76,175,80,0.25)"};`;
        bidCell.appendChild(bar);
        const txt = document.createElement("span");
        txt.style.cssText = `position:relative;z-index:1;color:${bright ? "#66ff66" : "#4caf50"};font-size:10px;`;
        txt.textContent = level.bidVol.toLocaleString();
        bidCell.appendChild(txt);
      }
      row.appendChild(bidCell);

      // Price column (center)
      const priceCell = document.createElement("div");
      priceCell.style.cssText = `text-align:center;font-weight:${isCurrentPrice ? "bold" : "normal"};color:${isCurrentPrice ? "#ffeb3b" : "#ccc"};font-size:10px;`;
      priceCell.textContent = level.price.toFixed(dp);
      row.appendChild(priceCell);

      // Ask column (right)
      const askCell = document.createElement("div");
      askCell.style.cssText = "position:relative;text-align:left;padding-left:4px;height:100%;display:flex;align-items:center;";
      if (level.askVol > 0) {
        const bar = document.createElement("div");
        const pct = (level.askVol / maxSize) * 100;
        const bright = level.askVol >= largeThreshold;
        bar.style.cssText = `position:absolute;left:0;top:0;height:100%;width:${pct}%;background:${bright ? "rgba(244,67,54,0.5)" : "rgba(244,67,54,0.25)"};`;
        askCell.appendChild(bar);
        const txt = document.createElement("span");
        txt.style.cssText = `position:relative;z-index:1;color:${bright ? "#ff6666" : "#f44336"};font-size:10px;`;
        txt.textContent = level.askVol.toLocaleString();
        askCell.appendChild(txt);
      }
      row.appendChild(askCell);

      container.appendChild(row);
    }

    // Footer: total bid vs ask imbalance
    const totalBid = levels.reduce((s, l) => s + l.bidVol, 0);
    const totalAsk = levels.reduce((s, l) => s + l.askVol, 0);
    const total = totalBid + totalAsk || 1;
    const bidPct = ((totalBid / total) * 100).toFixed(1);
    const askPct = ((totalAsk / total) * 100).toFixed(1);
    footer.innerHTML = `<span style="color:#4caf50">BID: ${totalBid.toLocaleString()} (${bidPct}%)</span><span style="color:#888">|</span><span style="color:#f44336">ASK: ${totalAsk.toLocaleString()} (${askPct}%)</span>`;
  }

  refresh();
  ladderInterval = setInterval(refresh, 2000);
}

// ══════════════════════════════════════════════════════════════
// CHAIN+ — Enhanced Options Chain Visualizer
// ══════════════════════════════════════════════════════════════
async function cmdChainPlus() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  const win = createWindow({ title: `${currentSymbol} — Enhanced Options Chain`, width: 750, height: 550 });
  win.contentElement.textContent = "";

  // Expiry selector toolbar
  const toolbar = document.createElement("div");
  toolbar.style.cssText = "display:flex;gap:6px;padding:6px;border-bottom:1px solid #333;align-items:center;flex-wrap:wrap;";
  const label = document.createElement("span");
  label.textContent = "Expiry:";
  label.style.cssText = "color:#888;font-size:10px;";
  const expiryInput = document.createElement("input");
  expiryInput.type = "date";
  expiryInput.style.cssText = "font-size:10px;background:#111;color:#ccc;border:1px solid #333;padding:3px;";
  const nextFri = new Date();
  nextFri.setDate(nextFri.getDate() + (5 - nextFri.getDay() + 7) % 7 + 7);
  expiryInput.value = nextFri.toISOString().split("T")[0];
  const loadBtn = document.createElement("button");
  loadBtn.textContent = "Load";
  loadBtn.style.cssText = "font-size:10px;padding:3px 10px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;";
  toolbar.appendChild(label);
  toolbar.appendChild(expiryInput);
  toolbar.appendChild(loadBtn);

  // Tab buttons
  const tabNames = ["Vol Smile", "OI Profile", "Vol Heatmap"];
  const tabBtns = [];
  let activeTab = 0;
  for (let i = 0; i < tabNames.length; i++) {
    const btn = document.createElement("button");
    btn.textContent = tabNames[i];
    btn.style.cssText = `font-size:10px;padding:3px 10px;border:1px solid #555;cursor:pointer;margin-left:${i === 0 ? "auto" : "0"};background:${i === 0 ? "#1a3a5c" : "#111"};color:${i === 0 ? "#8cf" : "#888"};`;
    btn.addEventListener("click", () => {
      activeTab = i;
      tabBtns.forEach((b, j) => { b.style.background = j === i ? "#1a3a5c" : "#111"; b.style.color = j === i ? "#8cf" : "#888"; });
      if (chainData) renderTab(chainData);
    });
    tabBtns.push(btn);
    toolbar.appendChild(btn);
  }
  win.appendElement(toolbar);

  const content = document.createElement("div");
  content.style.cssText = "padding:4px;font-size:10px;overflow-y:auto;max-height:460px;";
  content.textContent = "Select expiry and click Load.";
  win.appendElement(content);

  let chainData = null;

  function renderTab(chain) {
    content.textContent = "";
    if (activeTab === 0) renderVolSmile(chain);
    else if (activeTab === 1) renderOIProfile(chain);
    else renderVolHeatmap(chain);
  }

  function renderVolSmile(chain) {
    const calls = chain.filter(c => c.option_type === "call").sort((a, b) => a.strike - b.strike);
    const puts = chain.filter(c => c.option_type === "put").sort((a, b) => a.strike - b.strike);
    if (calls.length === 0 && puts.length === 0) { content.textContent = "No IV data available."; return; }

    const chartDiv = document.createElement("div");
    chartDiv.style.cssText = "width:100%;height:350px;";
    content.appendChild(chartDiv);

    const smileChart = createChart(chartDiv, {
      width: chartDiv.clientWidth || 700, height: 350,
      layout: { background: { color: "#000" }, textColor: "#888", fontFamily: "Consolas, monospace", attributionLogo: false },
      grid: { vertLines: { color: "#1a1a2e" }, horzLines: { color: "#1a1a2e" } },
      rightPriceScale: { borderColor: "#333" },
      timeScale: { borderColor: "#333", visible: false },
    });

    // Use whitespace-based custom x-axis: map strike index to time
    const allStrikes = [...new Set(chain.map(c => c.strike))].sort((a, b) => a - b);
    const strikeToIndex = {};
    allStrikes.forEach((s, i) => { strikeToIndex[s.toFixed(2)] = i; });

    if (calls.length > 0) {
      const callLine = smileChart.addLineSeries({ color: "#2196f3", lineWidth: 2, title: "Call IV" });
      const callData = calls.filter(c => c.implied_volatility > 0).map(c => ({
        time: strikeToIndex[c.strike.toFixed(2)] || 0,
        value: c.implied_volatility * 100,
      }));
      callLine.setData(callData);
    }
    if (puts.length > 0) {
      const putLine = smileChart.addLineSeries({ color: "#f44336", lineWidth: 2, title: "Put IV" });
      const putData = puts.filter(c => c.implied_volatility > 0).map(c => ({
        time: strikeToIndex[c.strike.toFixed(2)] || 0,
        value: c.implied_volatility * 100,
      }));
      putLine.setData(putData);
    }
    smileChart.timeScale().fitContent();

    // Legend
    const legend = document.createElement("div");
    legend.style.cssText = "padding:6px;font-size:10px;color:#888;text-align:center;";
    legend.innerHTML = '<span style="color:#2196f3">Call IV</span> / <span style="color:#f44336">Put IV</span> — X: Strike Index (lowest to highest), Y: IV %';
    content.appendChild(legend);

    const ro = new ResizeObserver(() => { if (chartDiv.clientWidth > 0) smileChart.applyOptions({ width: chartDiv.clientWidth }); });
    ro.observe(chartDiv);
  }

  function renderOIProfile(chain) {
    const allStrikes = [...new Set(chain.map(c => c.strike))].sort((a, b) => a - b);
    const calls = chain.filter(c => c.option_type === "call");
    const puts = chain.filter(c => c.option_type === "put");

    // Calculate max pain (strike with most total OI expiring worthless)
    let maxPainStrike = 0, maxPainOI = 0;
    for (const strike of allStrikes) {
      const callOI = calls.filter(c => c.strike === strike).reduce((s, c) => s + (c.open_interest || 0), 0);
      const putOI = puts.filter(c => c.strike === strike).reduce((s, c) => s + (c.open_interest || 0), 0);
      const total = callOI + putOI;
      if (total > maxPainOI) { maxPainOI = total; maxPainStrike = strike; }
    }

    const maxOI = Math.max(...chain.map(c => c.open_interest || 0), 1);
    const container = document.createElement("div");
    container.style.cssText = "overflow-y:auto;max-height:400px;";

    if (maxPainStrike > 0) {
      const mpDiv = document.createElement("div");
      mpDiv.style.cssText = "padding:6px;text-align:center;font-size:11px;border-bottom:1px solid #333;";
      mpDiv.innerHTML = `Max Pain: <span style="color:#ff9800;font-weight:bold">$${maxPainStrike.toFixed(2)}</span> (Total OI: ${maxPainOI.toLocaleString()})`;
      container.appendChild(mpDiv);
    }

    for (const strike of allStrikes) {
      const callOI = calls.filter(c => c.strike === strike).reduce((s, c) => s + (c.open_interest || 0), 0);
      const putOI = puts.filter(c => c.strike === strike).reduce((s, c) => s + (c.open_interest || 0), 0);
      const isMaxPain = strike === maxPainStrike;

      const row = document.createElement("div");
      row.style.cssText = `display:grid;grid-template-columns:1fr 70px 1fr;height:20px;align-items:center;border-bottom:1px solid #111;${isMaxPain ? "background:rgba(255,152,0,0.1);border:1px solid #ff980044;" : ""}`;

      // Call OI bar (left, green, pointing right)
      const callCell = document.createElement("div");
      callCell.style.cssText = "position:relative;height:100%;display:flex;align-items:center;justify-content:flex-end;padding-right:4px;";
      if (callOI > 0) {
        const bar = document.createElement("div");
        bar.style.cssText = `position:absolute;right:0;top:2px;height:calc(100% - 4px);width:${(callOI / maxOI) * 100}%;background:rgba(76,175,80,0.35);border-radius:2px;`;
        callCell.appendChild(bar);
        const txt = document.createElement("span");
        txt.style.cssText = "position:relative;z-index:1;color:#4caf50;font-size:9px;";
        txt.textContent = callOI.toLocaleString();
        callCell.appendChild(txt);
      }
      row.appendChild(callCell);

      // Strike (center)
      const strikeCell = document.createElement("div");
      strikeCell.style.cssText = `text-align:center;font-weight:${isMaxPain ? "bold" : "normal"};color:${isMaxPain ? "#ff9800" : "#ccc"};font-size:10px;`;
      strikeCell.textContent = strike.toFixed(2);
      row.appendChild(strikeCell);

      // Put OI bar (right, red, pointing right)
      const putCell = document.createElement("div");
      putCell.style.cssText = "position:relative;height:100%;display:flex;align-items:center;padding-left:4px;";
      if (putOI > 0) {
        const bar = document.createElement("div");
        bar.style.cssText = `position:absolute;left:0;top:2px;height:calc(100% - 4px);width:${(putOI / maxOI) * 100}%;background:rgba(244,67,54,0.35);border-radius:2px;`;
        putCell.appendChild(bar);
        const txt = document.createElement("span");
        txt.style.cssText = "position:relative;z-index:1;color:#f44336;font-size:9px;";
        txt.textContent = putOI.toLocaleString();
        putCell.appendChild(txt);
      }
      row.appendChild(putCell);

      container.appendChild(row);
    }
    content.appendChild(container);

    const legend = document.createElement("div");
    legend.style.cssText = "padding:6px;font-size:10px;color:#888;text-align:center;";
    legend.innerHTML = '<span style="color:#4caf50">Call OI</span> | Strike | <span style="color:#f44336">Put OI</span>';
    content.appendChild(legend);
  }

  function renderVolHeatmap(chain) {
    const allStrikes = [...new Set(chain.map(c => c.strike))].sort((a, b) => a - b);
    const expiries = [...new Set(chain.map(c => c.expiration_date || c.expiry || ""))].filter(Boolean).sort();

    if (expiries.length === 0) {
      // Single-expiry: show strike x type grid
      const maxVol = Math.max(...chain.map(c => c.volume || 0), 1);
      const table = document.createElement("table");
      table.style.cssText = "width:100%;border-collapse:collapse;font-size:10px;";
      const thead = document.createElement("thead");
      const hr = document.createElement("tr");
      for (const h of ["Strike", "Call Vol", "Put Vol"]) {
        const th = document.createElement("th");
        th.style.cssText = "padding:4px;color:#888;border-bottom:1px solid #333;";
        th.textContent = h;
        hr.appendChild(th);
      }
      thead.appendChild(hr);
      table.appendChild(thead);
      const tbody = document.createElement("tbody");
      for (const strike of allStrikes) {
        const callVol = chain.filter(c => c.strike === strike && c.option_type === "call").reduce((s, c) => s + (c.volume || 0), 0);
        const putVol = chain.filter(c => c.strike === strike && c.option_type === "put").reduce((s, c) => s + (c.volume || 0), 0);
        const tr = document.createElement("tr");
        const tdStrike = document.createElement("td");
        tdStrike.style.cssText = "padding:3px;text-align:center;color:#ccc;border-bottom:1px solid #111;";
        tdStrike.textContent = strike.toFixed(2);
        tr.appendChild(tdStrike);
        for (const vol of [callVol, putVol]) {
          const td = document.createElement("td");
          const intensity = maxVol > 0 ? vol / maxVol : 0;
          const g = Math.round(80 + intensity * 175);
          td.style.cssText = `padding:3px;text-align:center;border-bottom:1px solid #111;background:rgba(${Math.round(intensity * 20)},${g},${Math.round(intensity * 20)},${0.1 + intensity * 0.5});color:#ccc;`;
          td.textContent = vol > 0 ? vol.toLocaleString() : "-";
          tr.appendChild(td);
        }
        tbody.appendChild(tr);
      }
      table.appendChild(tbody);
      content.appendChild(table);
      return;
    }

    // Multi-expiry heatmap: rows = strikes, columns = expiry dates
    const maxVol = Math.max(...chain.map(c => c.volume || 0), 1);
    const wrapper = document.createElement("div");
    wrapper.style.cssText = "overflow:auto;max-height:400px;";
    const table = document.createElement("table");
    table.style.cssText = "border-collapse:collapse;font-size:9px;";
    const thead = document.createElement("thead");
    const hr = document.createElement("tr");
    const thCorner = document.createElement("th");
    thCorner.style.cssText = "padding:3px;color:#888;border:1px solid #222;position:sticky;left:0;background:#000;z-index:1;";
    thCorner.textContent = "Strike";
    hr.appendChild(thCorner);
    for (const exp of expiries) {
      const th = document.createElement("th");
      th.style.cssText = "padding:3px;color:#888;border:1px solid #222;white-space:nowrap;";
      th.textContent = exp.slice(5); // MM-DD
      hr.appendChild(th);
    }
    thead.appendChild(hr);
    table.appendChild(thead);
    const tbody = document.createElement("tbody");
    for (const strike of allStrikes) {
      const tr = document.createElement("tr");
      const tdStrike = document.createElement("td");
      tdStrike.style.cssText = "padding:3px;text-align:center;color:#ccc;border:1px solid #222;position:sticky;left:0;background:#000;z-index:1;";
      tdStrike.textContent = strike.toFixed(2);
      tr.appendChild(tdStrike);
      for (const exp of expiries) {
        const vol = chain.filter(c => c.strike === strike && (c.expiration_date === exp || c.expiry === exp)).reduce((s, c) => s + (c.volume || 0), 0);
        const td = document.createElement("td");
        const intensity = maxVol > 0 ? vol / maxVol : 0;
        const g = Math.round(80 + intensity * 175);
        td.style.cssText = `padding:3px;text-align:center;border:1px solid #222;min-width:40px;background:rgba(${Math.round(intensity * 20)},${g},${Math.round(intensity * 20)},${0.05 + intensity * 0.55});color:${intensity > 0.3 ? "#fff" : "#888"};`;
        td.textContent = vol > 0 ? vol.toLocaleString() : "-";
        tr.appendChild(td);
      }
      tbody.appendChild(tr);
    }
    table.appendChild(tbody);
    wrapper.appendChild(table);
    content.appendChild(wrapper);
  }

  loadBtn.addEventListener("click", async () => {
    content.textContent = "Loading options chain...";
    try {
      const json = await invoke("get_options", { symbol: currentSymbol, expiry: expiryInput.value });
      const chain = typeof json === "string" ? JSON.parse(json) : json;
      if (!Array.isArray(chain) || chain.length === 0) { content.textContent = "No options data."; return; }
      chainData = chain;
      renderTab(chain);
    } catch (e) { content.textContent = `Error: ${e}`; }
  });
}

// ══════════════════════════════════════════════════════════════
// SPREAD+ — Live Bid-Ask Spread Monitor
// ══════════════════════════════════════════════════════════════
function cmdSpreadMonitor() {
  if (!currentSymbol) { log("No symbol loaded", "warn"); return; }
  let spreadInterval = null;
  const ROLLING_SIZE = 50;
  const spreadHistory = []; // { time, spread }
  let maxSpread = 0;

  const win = createWindow({
    title: `${currentSymbol} — Spread Monitor`,
    width: 650, height: 480,
    onClose: () => { if (spreadInterval) { clearInterval(spreadInterval); spreadInterval = null; } },
  });
  win.contentElement.textContent = "";

  const chartDiv = document.createElement("div");
  chartDiv.style.cssText = "width:100%;height:300px;";
  win.appendElement(chartDiv);

  const statsDiv = document.createElement("div");
  statsDiv.style.cssText = "padding:8px;display:grid;grid-template-columns:1fr 1fr 1fr 1fr;gap:8px;font-family:Consolas,monospace;font-size:11px;border-top:1px solid #333;";
  win.appendElement(statsDiv);

  const spreadChart = createChart(chartDiv, {
    width: chartDiv.clientWidth || 620, height: 300,
    layout: { background: { color: "#000" }, textColor: "#888", fontFamily: "Consolas, monospace", attributionLogo: false },
    grid: { vertLines: { color: "#111" }, horzLines: { color: "#111" } },
    rightPriceScale: { borderColor: "#333" },
    timeScale: { borderColor: "#333", timeVisible: true, secondsVisible: true },
  });

  const spreadLine = spreadChart.addLineSeries({ color: "#00bcd4", lineWidth: 2, title: "Spread", lastValueVisible: true });
  const avgLine = spreadChart.addLineSeries({ color: "#ffffff", lineWidth: 1, lineStyle: 2, title: "Avg Spread" });
  const sdLine = spreadChart.addLineSeries({ color: "#f44336", lineWidth: 1, lineStyle: 2, title: "+2 SD" });

  let sampleIndex = 0;

  function updateStats() {
    const n = spreadHistory.length;
    if (n === 0) return;
    const current = spreadHistory[n - 1].spread;
    const window50 = spreadHistory.slice(-ROLLING_SIZE);
    const mean = window50.reduce((s, h) => s + h.spread, 0) / window50.length;
    const variance = window50.reduce((s, h) => s + (h.spread - mean) ** 2, 0) / window50.length;
    const sd = Math.sqrt(variance);
    const warnLevel = mean + 2 * sd;
    const latestPrice = lastPrice > 0 ? lastPrice : 1;
    const spreadPct = (current / latestPrice * 100).toFixed(4);

    statsDiv.textContent = "";
    const stats = [
      ["Current", `$${current.toFixed(4)}`, "#00bcd4"],
      ["Avg (50)", `$${mean.toFixed(4)}`, "#fff"],
      ["Max", `$${maxSpread.toFixed(4)}`, "#ff9800"],
      ["Spread %", `${spreadPct}%`, "#888"],
    ];
    for (const [label, value, color] of stats) {
      const cell = document.createElement("div");
      cell.style.cssText = "text-align:center;";
      const lbl = document.createElement("div");
      lbl.style.cssText = "color:#666;font-size:9px;";
      lbl.textContent = label;
      const val = document.createElement("div");
      val.style.cssText = `color:${color};font-weight:bold;`;
      val.textContent = value;
      cell.appendChild(lbl);
      cell.appendChild(val);
      statsDiv.appendChild(cell);
    }

    if (current > warnLevel && n > ROLLING_SIZE) {
      log(`SPREAD ALERT: ${currentSymbol} spread $${current.toFixed(4)} exceeds 2 SD ($${warnLevel.toFixed(4)})`, "warn");
    }
  }

  async function pollSpread() {
    try {
      const json = await invokeQuiet("get_latest_quote", { symbol: currentSymbol });
      const q = typeof json === "string" ? JSON.parse(json) : json;
      const bid = q.bid || q.bp || 0;
      const ask = q.ask || q.ap || 0;
      if (bid <= 0 || ask <= 0) return;
      const spread = ask - bid;
      if (spread > maxSpread) maxSpread = spread;

      sampleIndex++;
      spreadHistory.push({ time: sampleIndex, spread });
      if (spreadHistory.length > 500) spreadHistory.shift();

      // Update chart data
      spreadLine.setData(spreadHistory.map(h => ({ time: h.time, value: h.spread })));

      // Rolling average and SD
      const window50 = spreadHistory.slice(-ROLLING_SIZE);
      const mean = window50.reduce((s, h) => s + h.spread, 0) / window50.length;
      const variance = window50.reduce((s, h) => s + (h.spread - mean) ** 2, 0) / window50.length;
      const sd = Math.sqrt(variance);

      avgLine.setData(spreadHistory.map(h => ({ time: h.time, value: mean })));
      sdLine.setData(spreadHistory.map(h => ({ time: h.time, value: mean + 2 * sd })));

      spreadChart.timeScale().scrollToRealTime();
      updateStats();
    } catch (_) {}
  }

  pollSpread();
  spreadInterval = setInterval(pollSpread, 2000);

  const ro = new ResizeObserver(() => { if (chartDiv.clientWidth > 0) spreadChart.applyOptions({ width: chartDiv.clientWidth }); });
  ro.observe(chartDiv);
}

// ══════════════════════════════════════════════════════════════
// WEBHOOK — Custom Webhook Alert Endpoints
// ══════════════════════════════════════════════════════════════
const WEBHOOK_STORAGE_KEY = "typhoon_webhooks";

function loadWebhooks() {
  try { return JSON.parse(localStorage.getItem(WEBHOOK_STORAGE_KEY) || "[]"); } catch { return []; }
}

function saveWebhooks(hooks) {
  localStorage.setItem(WEBHOOK_STORAGE_KEY, JSON.stringify(hooks));
}

function fireWebhooks(event, data) {
  const hooks = loadWebhooks();
  for (const hook of hooks) {
    if (hook.events && hook.events.includes(event)) {
      const payload = { event, timestamp: new Date().toISOString(), ...data };
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 5000);
      fetch(hook.url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
        signal: controller.signal,
      }).then(() => {
        log(`Webhook "${hook.name}" fired (${event})`, "ok");
      }).catch(err => {
        log(`Webhook "${hook.name}" failed: ${err}`, "error");
      }).finally(() => clearTimeout(timeout));
    }
  }
}

function cmdWebhook() {
  const win = createWindow({ title: "Webhook Alert Endpoints", width: 550, height: 500 });
  win.contentElement.textContent = "";

  const EVENT_TYPES = ["price_alert", "order_fill", "signal", "unusual_volume"];
  const EVENT_LABELS = { price_alert: "Price Alert", order_fill: "Order Fill", signal: "Signal Change", unusual_volume: "Unusual Volume" };

  function render() {
    win.contentElement.textContent = "";
    const hooks = loadWebhooks();

    // Header
    const header = document.createElement("div");
    header.style.cssText = "padding:8px;border-bottom:1px solid #333;display:flex;justify-content:space-between;align-items:center;";
    const title = document.createElement("span");
    title.style.cssText = "color:#ccc;font-size:12px;font-weight:bold;";
    title.textContent = `Webhooks (${hooks.length})`;
    const addBtn = document.createElement("button");
    addBtn.textContent = "+ Add Webhook";
    addBtn.style.cssText = "font-size:10px;padding:4px 12px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;border-radius:3px;";
    addBtn.addEventListener("click", () => renderForm());
    header.appendChild(title);
    header.appendChild(addBtn);
    win.appendElement(header);

    // List
    const list = document.createElement("div");
    list.style.cssText = "overflow-y:auto;max-height:calc(100% - 50px);padding:4px;";

    if (hooks.length === 0) {
      const empty = document.createElement("div");
      empty.style.cssText = "color:#666;padding:20px;text-align:center;font-size:11px;";
      empty.textContent = "No webhooks configured. Click '+ Add Webhook' to get started.";
      list.appendChild(empty);
    }

    for (let i = 0; i < hooks.length; i++) {
      const hook = hooks[i];
      const card = document.createElement("div");
      card.style.cssText = "background:#111;border:1px solid #333;border-radius:4px;padding:8px;margin-bottom:6px;";

      const topRow = document.createElement("div");
      topRow.style.cssText = "display:flex;justify-content:space-between;align-items:center;margin-bottom:4px;";
      const name = document.createElement("span");
      name.style.cssText = "color:#ccc;font-weight:bold;font-size:11px;";
      name.textContent = hook.name;
      const btnGroup = document.createElement("div");
      btnGroup.style.cssText = "display:flex;gap:4px;";

      const testBtn = document.createElement("button");
      testBtn.textContent = "Test";
      testBtn.style.cssText = "font-size:9px;padding:2px 8px;background:#1a3a1a;color:#4caf50;border:1px solid #333;cursor:pointer;border-radius:2px;";
      testBtn.addEventListener("click", () => {
        const payload = {
          event: "test",
          symbol: currentSymbol || "TEST",
          price: lastPrice || 100.00,
          message: "Test webhook from TyphooN Terminal",
          timestamp: new Date().toISOString(),
        };
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 5000);
        fetch(hook.url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
          signal: controller.signal,
        }).then(r => {
          log(`Webhook test "${hook.name}": ${r.status} ${r.statusText}`, r.ok ? "ok" : "warn");
        }).catch(err => {
          log(`Webhook test "${hook.name}" failed: ${err}`, "error");
        }).finally(() => clearTimeout(timeout));
      });

      const editBtn = document.createElement("button");
      editBtn.textContent = "Edit";
      editBtn.style.cssText = "font-size:9px;padding:2px 8px;background:#1a1a3a;color:#8cf;border:1px solid #333;cursor:pointer;border-radius:2px;";
      editBtn.addEventListener("click", () => renderForm(i));

      const delBtn = document.createElement("button");
      delBtn.textContent = "Delete";
      delBtn.style.cssText = "font-size:9px;padding:2px 8px;background:#3a1a1a;color:#f44336;border:1px solid #333;cursor:pointer;border-radius:2px;";
      delBtn.addEventListener("click", () => {
        const all = loadWebhooks();
        all.splice(i, 1);
        saveWebhooks(all);
        log(`Webhook "${hook.name}" deleted`, "ok");
        render();
      });

      btnGroup.appendChild(testBtn);
      btnGroup.appendChild(editBtn);
      btnGroup.appendChild(delBtn);
      topRow.appendChild(name);
      topRow.appendChild(btnGroup);
      card.appendChild(topRow);

      const urlDiv = document.createElement("div");
      urlDiv.style.cssText = "color:#666;font-size:9px;margin-bottom:4px;word-break:break-all;";
      urlDiv.textContent = hook.url;
      card.appendChild(urlDiv);

      const eventsDiv = document.createElement("div");
      eventsDiv.style.cssText = "display:flex;gap:4px;flex-wrap:wrap;";
      for (const evt of (hook.events || [])) {
        const tag = document.createElement("span");
        tag.style.cssText = "font-size:9px;padding:1px 6px;background:#1a1a2e;color:#8cf;border-radius:8px;border:1px solid #333;";
        tag.textContent = EVENT_LABELS[evt] || evt;
        eventsDiv.appendChild(tag);
      }
      card.appendChild(eventsDiv);

      list.appendChild(card);
    }
    win.appendElement(list);
  }

  function renderForm(editIndex) {
    win.contentElement.textContent = "";
    const hooks = loadWebhooks();
    const existing = editIndex !== undefined ? hooks[editIndex] : null;

    const form = document.createElement("div");
    form.style.cssText = "padding:12px;font-size:11px;";

    const heading = document.createElement("div");
    heading.style.cssText = "color:#ccc;font-size:13px;font-weight:bold;margin-bottom:12px;";
    heading.textContent = existing ? "Edit Webhook" : "Add Webhook";
    form.appendChild(heading);

    // Name
    const nameLabel = document.createElement("label");
    nameLabel.style.cssText = "color:#888;font-size:10px;display:block;margin-bottom:2px;";
    nameLabel.textContent = "Name";
    const nameInput = document.createElement("input");
    nameInput.style.cssText = "width:100%;padding:6px;background:#111;color:#ccc;border:1px solid #333;margin-bottom:8px;font-size:11px;box-sizing:border-box;";
    nameInput.value = existing ? existing.name : "";
    nameInput.placeholder = "e.g. Discord Bot";
    form.appendChild(nameLabel);
    form.appendChild(nameInput);

    // URL
    const urlLabel = document.createElement("label");
    urlLabel.style.cssText = "color:#888;font-size:10px;display:block;margin-bottom:2px;";
    urlLabel.textContent = "Webhook URL";
    const urlInput = document.createElement("input");
    urlInput.style.cssText = "width:100%;padding:6px;background:#111;color:#ccc;border:1px solid #333;margin-bottom:8px;font-size:11px;box-sizing:border-box;";
    urlInput.value = existing ? existing.url : "";
    urlInput.placeholder = "https://...";
    form.appendChild(urlLabel);
    form.appendChild(urlInput);

    // Event checkboxes
    const evLabel = document.createElement("label");
    evLabel.style.cssText = "color:#888;font-size:10px;display:block;margin-bottom:6px;";
    evLabel.textContent = "Events";
    form.appendChild(evLabel);

    const checkboxes = {};
    const cbContainer = document.createElement("div");
    cbContainer.style.cssText = "display:grid;grid-template-columns:1fr 1fr;gap:4px;margin-bottom:12px;";
    for (const evt of EVENT_TYPES) {
      const wrap = document.createElement("label");
      wrap.style.cssText = "color:#ccc;font-size:10px;display:flex;align-items:center;gap:4px;cursor:pointer;";
      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = existing ? (existing.events || []).includes(evt) : false;
      checkboxes[evt] = cb;
      wrap.appendChild(cb);
      wrap.appendChild(document.createTextNode(EVENT_LABELS[evt]));
      cbContainer.appendChild(wrap);
    }
    form.appendChild(cbContainer);

    // Buttons
    const btnRow = document.createElement("div");
    btnRow.style.cssText = "display:flex;gap:8px;";
    const saveBtn = document.createElement("button");
    saveBtn.textContent = existing ? "Update" : "Save";
    saveBtn.style.cssText = "padding:6px 20px;background:#0f3460;color:#8cf;border:1px solid #555;cursor:pointer;font-size:11px;border-radius:3px;";
    saveBtn.addEventListener("click", () => {
      const name = nameInput.value.trim();
      const url = urlInput.value.trim();
      if (!name || !url) { log("Webhook name and URL are required", "warn"); return; }
      if (!url.startsWith("https://")) { log("Webhook URL must start with https://", "warn"); return; }
      if (/localhost|127\.0\.0\.1|0\.0\.0\.0|192\.168\.|10\.|172\.16\./.test(url)) { log("Webhook URL must not point to local/private networks (SSRF prevention)", "warn"); return; }
      const events = EVENT_TYPES.filter(e => checkboxes[e].checked);
      if (events.length === 0) { log("Select at least one event type", "warn"); return; }
      const all = loadWebhooks();
      const entry = { name, url, events };
      if (editIndex !== undefined) { all[editIndex] = entry; } else { all.push(entry); }
      saveWebhooks(all);
      log(`Webhook "${name}" ${existing ? "updated" : "added"}`, "ok");
      render();
    });

    const cancelBtn = document.createElement("button");
    cancelBtn.textContent = "Cancel";
    cancelBtn.style.cssText = "padding:6px 20px;background:#222;color:#888;border:1px solid #444;cursor:pointer;font-size:11px;border-radius:3px;";
    cancelBtn.addEventListener("click", () => render());

    btnRow.appendChild(saveBtn);
    btnRow.appendChild(cancelBtn);
    form.appendChild(btnRow);

    win.appendElement(form);
  }

  render();
}

async function checkWatchlistSMA200Alerts() {
  if (!window._watchlistSymbols || window._watchlistSymbols.length === 0) return;
  const now = Date.now();
  for (const sym of window._watchlistSymbols) {
    const cacheKey = `sma200alert:${sym}`;
    if (watchlistSMA200Cache[cacheKey] && (now - watchlistSMA200Cache[cacheKey].ts) < 300000) continue; // 5min cache
    try {
      const barsJson = await invoke("get_bars", { symbol: sym, timeframe: "1Day", limit: 210 });
      const bars = JSON.parse(barsJson);
      if (bars.length < 201) continue;
      const closes = bars.map(b => b.close);
      const sma200 = closes.slice(-200).reduce((a, b) => a + b, 0) / 200;
      const price = closes[closes.length - 1];
      const prevPrice = closes[closes.length - 2];
      const prevAbove = prevPrice > sma200;
      const nowAbove = price > sma200;
      if (prevAbove !== nowAbove) {
        const direction = nowAbove ? "CROSSED ABOVE" : "CROSSED BELOW";
        log(`WATCHLIST ALERT: ${sym} ${direction} SMA200 ($${sma200.toFixed(2)}) — Price: $${price.toFixed(2)}`, "warn");
        try { new Notification(`${sym} ${direction} SMA200`, { body: `Price: $${price.toFixed(2)}, SMA200: $${sma200.toFixed(2)}` }); } catch (_) {}
      }
      watchlistSMA200Cache[cacheKey] = { ts: now, above: nowAbove };
    } catch (_) {}
  }
}
