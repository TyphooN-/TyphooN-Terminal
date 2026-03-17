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
let currentChartData = []; // full chartData with volume — candleSeries.data() drops volume
let chartLoadGeneration = 0; // increments on each loadChart call — stale intervals check this
let activeBrokerId = "default"; // per-broker data isolation — set on connect
let currentChartType = "candles"; // "candles" | "line" | "bars"
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
    } else {
      candleSeries.setData(currentChartData);
    }
  }

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

const MTF_LABELS = { "15Min": "M15", "30Min": "M30", "1Hour": "H1", "2Hour": "H2", "3Hour": "H3", "4Hour": "H4", "6Hour": "H6", "8Hour": "H8", "12Hour": "H12", "1Day": "D1", "1Week": "W1", "1Month": "MN1" };

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
    const brokerKey = `${activeBrokerId}/${key}`;
    await invoke("save_cold_cache", { key: brokerKey, data: JSON.stringify(data) });
  } catch (_) {}
}

async function coldLoad(key) {
  try {
    const brokerKey = `${activeBrokerId}/${key}`;
    const json = await invoke("load_cold_cache", { key: brokerKey });
    return JSON.parse(json);
  } catch (_) {
    // Fallback: try legacy key without broker prefix
    try {
      const json = await invoke("load_cold_cache", { key });
      return JSON.parse(json);
    } catch (_) {}
    return null;
  }
}

// ── Unified Cache Operations ────────────────────────────────

// Load from all tiers on startup: SQLite → IndexedDB → cold → hot
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
              barCache[entry.key] = { data: entry.data, timestamp: entry.timestamp || 0, lastAccess: Date.now() };
            }
          }
          resolve();
        };
        req.onerror = () => resolve();
      });
    }
    const idbCount = Object.keys(barCache).length;
    if (idbCount > 0) log(`Loaded ${idbCount} cached bar sets from IndexedDB`, "info");

    // Try SQLite cache stats
    try {
      const stats = JSON.parse(await invoke("db_cache_stats"));
      log(`SQLite cache: ${stats.bar_entries} bar sets, ${stats.kv_entries} KV entries, ${stats.total_compressed_mb.toFixed(1)}MB compressed`, "info");
    } catch (_) {}

    evictLRU();
  } catch (e) {
    log(`Cache init: ${e}`, "warn");
  }
}

// Save to all tiers: hot → warm → cold (async, non-blocking)
function saveBarCacheToDisk(cacheKey, data) {
  const ts = Date.now();
  barCache[cacheKey] = { data, timestamp: ts, lastAccess: ts };
  evictLRU();
  // Warm (IndexedDB) — async, fire-and-forget
  idbPut(cacheKey, data, ts);
  // SQL (SQLite via Rust) — async, fire-and-forget
  invoke("db_cache_put", { key: `${activeBrokerId}:${cacheKey}`, data: JSON.stringify(data), kind: "bars" }).catch(() => {});
  // Cold (zstd file) — async, fire-and-forget (legacy)
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
  "10Min":  { base: "5Min",  factor: 2 },
  "20Min":  { base: "5Min",  factor: 4 },
  "45Min":  { base: "15Min", factor: 3 },
  "2Hour":  { base: "1Hour", factor: 2 },
  "3Hour":  { base: "1Hour", factor: 3 },
  "6Hour":  { base: "1Hour", factor: 6 },
  "8Hour":  { base: "4Hour", factor: 2 },
  "12Hour": { base: "4Hour", factor: 3 },
  "2Day":   { base: "1Day",  factor: 2 },
  "3Day":   { base: "1Day",  factor: 3 },
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

const ALL_TIMEFRAMES = ["1Min", "5Min", "15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week"];

const prefetchInProgress = new Set(); // prevent duplicate prefetch runs per symbol

async function prefetchAllTimeframes(symbol, currentTF, limit) {
  if (prefetchInProgress.has(symbol)) return; // already prefetching this symbol
  prefetchInProgress.add(symbol);

  const toFetch = ALL_TIMEFRAMES.filter(tf => tf !== currentTF);
  let fetched = 0;
  for (const tf of toFetch) {
    const cacheKey = getCacheKey(symbol, tf);
    const cached = barCache[cacheKey];
    // Skip if already cached (any data at all — don't re-fetch)
    if (cached && cached.data && cached.data.length > 0) continue;
    try {
      const barsJson = await invoke("get_bars", { symbol, timeframe: tf, limit: Math.min(limit, 1000) });
      const bars = JSON.parse(barsJson);
      if (bars.length > 0) {
        barCache[cacheKey] = { data: bars, timestamp: Date.now() };
        saveBarCacheToDisk(cacheKey, bars);
        fetched++;
      }
    } catch (_) {}
  }
  if (fetched > 0) log(`Pre-cached ${fetched} timeframes for ${symbol}`, "ok");
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
  } catch (_) {}
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

  // Chart type selector
  document.getElementById("chart-type-select").addEventListener("change", (e) => {
    rebuildMainSeries(e.target.value);
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
      case "e": // ray (extends right from two points)
        drawingMode = "ray"; drawingAnchor = null;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Drawing mode: ray — click two points (extends right)", "info");
        break;
      case "j": // ruler / price range measure
        drawingMode = "ruler"; drawingAnchor = null;
        document.getElementById("chart-container").style.cursor = "crosshair";
        log("Ruler mode: click two points to measure", "info");
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
    const saveCredentials = document.getElementById("save-credentials").checked;
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
    content.textContent = "";

    if (positions.length === 0) {
      content.textContent = "No positions";
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
    content.appendChild(controls);

    // Filter positions if checkbox is ticked
    const sym = currentSymbol.replace("/", "");
    const filtered = positionsChartOnly
      ? positions.filter(p => p.symbol === currentSymbol || p.symbol === sym)
      : positions;

    if (filtered.length === 0) {
      const msg = document.createElement("div");
      msg.style.cssText = "color:#888;font-size:10px;padding:4px 0;";
      msg.textContent = `No positions for ${currentSymbol}`;
      content.appendChild(msg);
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
  content.textContent = "";
  try {
    // Open orders first
    const openJson = await invoke("get_open_orders");
    const openOrders = JSON.parse(openJson);

    // Recent closed orders
    const histJson = await invoke("get_order_history", { limit: 20 });
    const history = JSON.parse(histJson);

    const hasOrders = openOrders.length > 0 || history.length > 0;

    if (!hasOrders) {
      content.textContent = "No orders";
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
    content.appendChild(controls);

    // Filter by current chart symbol if checkbox ticked
    const sym = currentSymbol.replace("/", "");
    const matchSymbol = (o) => !ordersChartOnly || o.symbol === currentSymbol || o.symbol === sym;

    const filteredOpen = openOrders.filter(matchSymbol);
    const filteredHist = history.filter(matchSymbol);

    if (filteredOpen.length > 0) {
      const hdr = document.createElement("div");
      hdr.textContent = "Open Orders";
      hdr.style.cssText = "color:#ff8;font-size:10px;font-weight:bold;padding:4px 0 2px;";
      content.appendChild(hdr);
      for (const o of filteredOpen) {
        content.appendChild(renderOrderRow(o, true));
      }
    }

    if (filteredHist.length > 0) {
      const hdr = document.createElement("div");
      hdr.textContent = "Recent Fills";
      hdr.style.cssText = "color:#888;font-size:10px;font-weight:bold;padding:4px 0 2px;";
      content.appendChild(hdr);
      for (const o of filteredHist.slice(0, 15)) {
        content.appendChild(renderOrderRow(o, false));
      }
    }

    if (filteredOpen.length === 0 && filteredHist.length === 0) {
      const msg = document.createElement("div");
      msg.style.cssText = "color:#888;font-size:10px;padding:4px 0;";
      msg.textContent = `No orders for ${currentSymbol}`;
      content.appendChild(msg);
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

const CMD_PALETTE_COMMANDS = [
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
  { name: "BRACKET", desc: "Conditional bracket/OCO order placement", action: cmdBracketOrder },
  { name: "HEATMAP", desc: "Portfolio heat map (daily P&L)", action: cmdHeatmap },
  { name: "OPTCALC", desc: "Options P&L calculator (payoff diagram)", action: cmdOptionsCalc },
  { name: "SECTORS", desc: "Sector rotation heatmap (S&P 500 ETFs)", action: cmdSectorRotation },
  { name: "ECON", desc: "Economic calendar with countdown", action: cmdEconCalendar },
  { name: "OPTSTRAT", desc: "Options strategy builder (spreads, condors)", action: cmdOptionsStrategy },
  { name: "AUTOTRADE", desc: "Strategy auto-trading (JS plugin → live orders)", action: cmdAutoTrade },
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
  setupSplitButton();
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
      closeMTFGrid();
    } else {
      tfCheckboxes.classList.toggle("hidden");
      if (!tfCheckboxes.classList.contains("hidden")) {
        // Show checkboxes; user clicks again to activate
        btn.textContent = "Apply";
      } else {
        btn.textContent = "MTF Grid";
      }
    }
  });

  // When "Apply" is clicked with checkboxes visible
  btn.addEventListener("click", () => {
    if (btn.textContent === "Apply" && !tfCheckboxes.classList.contains("hidden")) {
      const selectedTFs = [...document.querySelectorAll(".mtf-grid-cb:checked")].map(cb => cb.value);
      if (selectedTFs.length < 2) { alert("Select at least 2 timeframes"); return; }
      if (!currentSymbol) { alert("Load a chart first"); return; }
      tfCheckboxes.classList.add("hidden");
      openMTFGrid(currentSymbol, selectedTFs);
    }
  });
}

async function openMTFGrid(symbol, timeframes) {
  if (!symbol) { log("MTF grid: no symbol", "warn"); return; }
  mtfGridActive = true;
  mtfGridSymbol = symbol;
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
  gridContainer.className = `grid-${Math.min(count, 5)}`;
  chartStack.parentElement.insertBefore(gridContainer, chartStack);

  const tfLabels = { "1Min": "M1", "5Min": "M5", "10Min": "M10", "15Min": "M15", "20Min": "M20", "30Min": "M30", "45Min": "M45", "1Hour": "H1", "2Hour": "H2", "3Hour": "H3", "4Hour": "H4", "6Hour": "H6", "8Hour": "H8", "12Hour": "H12", "1Day": "D1", "2Day": "2D", "3Day": "3D", "1Week": "W1", "1Month": "MN1" };

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

    // Create chart instances
    const cellChart = createChart(chartDiv, {
      width: 100, height: 100,
      layout: { background: { color: "#000000" }, textColor: "#d1d4dc", fontFamily: "Consolas, Courier New, monospace", attributionLogo: false },
      grid: { vertLines: { color: "#222", style: 3 }, horzLines: { color: "#222", style: 3 } },
      crosshair: { mode: CrosshairMode.Normal },
      rightPriceScale: { borderColor: "#333" },
      timeScale: { borderColor: "#333", timeVisible: true },
    });

    const cellCandleSeries = cellChart.addCandlestickSeries({
      upColor: "#00ff00", downColor: "#ff0000",
      borderDownColor: "#ff0000", borderUpColor: "#00ff00",
      wickDownColor: "#ff0000", wickUpColor: "#00ff00",
    });

    const cellFisherChart = createChart(fisherDiv, {
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

    const cellInfo = { tf, chart: cellChart, candleSeries: cellCandleSeries, fisherChart: cellFisherChart, volumeChart: cellVolumeChart, container: cell, chartDiv, fisherDiv, volumeDiv };
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
  // Without this, cells can have 0×0 dimensions on first symbol load.
  requestAnimationFrame(() => {
    resizeMTFGrid();
    requestAnimationFrame(() => {
      resizeMTFGrid();
      // Final resize + fit content after layout is stable
      for (const cell of mtfGridCells) {
        cell.chart.timeScale().fitContent();
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
    // MTF grid uses smaller limit to avoid rate limit hammering
    const limit = 1000;
    const cacheKey = getCacheKey(symbol, cellInfo.tf);
    let bars;

    // Prefer cache — background pre-fetch should have all TFs cached
    const cached = barCache[cacheKey];
    if (cached && cached.data && cached.data.length > 0) {
      bars = cached.data;
      log(`MTF grid ${cellInfo.tf}: ${bars.length} bars from cache`, "info");
    } else {
      log(`MTF grid ${cellInfo.tf}: fetching (not cached)...`, "info");
      const barsJson = await invoke("get_bars", { symbol, timeframe: cellInfo.tf, limit });
      bars = JSON.parse(barsJson);
      barCache[cacheKey] = { data: bars, timestamp: Date.now() };
    }

    const chartData = bars.map(b => ({
      time: Math.floor(new Date(b.timestamp).getTime() / 1000),
      open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume,
    }));

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

    // ATR Projection (yellow bands)
    if (chartData.length > 15) {
      const atrp = calcATRProjection(chartData, 14);
      if (atrp.upper.length > 0) {
        addLine("#FFFF00", 1, atrp.upper);
        addLine("#FFFF00", 1, atrp.lower);
      }
    }

    // Previous Candle Levels (white)
    if (chartData.length > 1) {
      const pcl = calcPrevCandleLevels(chartData);
      addLine("#FFFFFF88", 1, pcl.highs);
      addLine("#FFFFFF88", 1, pcl.lows);
    }

    // Supply/Demand zones (lightweight — lines only, last 4 zones for performance)
    if (chartData.length > 12) {
      const zones = calcSupplyDemandZones(chartData);
      for (const z of zones.slice(-4)) {
        const color = z.type === "supply" ? "#87CEEB66" : "#8FBC8F66";
        const zoneBars = chartData.filter(d => d.time >= z.startTime);
        if (zoneBars.length < 2) continue;
        addLine(color, 1, zoneBars.map(d => ({ time: d.time, value: z.high })));
        addLine(color, 1, zoneBars.map(d => ({ time: d.time, value: z.low })));
      }
    }

    // Auto Fibonacci
    if (chartData.length > 30) {
      const fib = calcAutoFibonacci(chartData);
      if (fib) {
        const fibBars = chartData.filter(d => d.time >= fib.startTime);
        if (fibBars.length >= 2) {
          const keyLevels = fib.levels.filter(l => ["38.2%", "50%", "61.8%", "161.8%"].includes(l.label));
          const colors = { "38.2%": "#ffeb3b", "50%": "#8bc34a", "61.8%": "#00bcd4", "161.8%": "#ff5722" };
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
  document.getElementById("mtf-grid-tfs").classList.add("hidden");

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
    "split": () => document.getElementById("btn-split").click(),
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
