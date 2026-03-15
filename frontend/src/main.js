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

import { createChart, CrosshairMode, PriceLineSource } from "lightweight-charts";

const { invoke } = window.__TAURI__.core;

// ── State ───────────────────────────────────────────────────

let chart = null;
let candleSeries = null;
let slLine = null;
let tpLine = null;
let currentSymbol = "";
let currentTimeframe = "1Hour";

// ── Chart Setup ─────────────────────────────────────────────

function initChart() {
  const container = document.getElementById("chart-container");
  chart = createChart(container, {
    width: container.clientWidth,
    height: container.clientHeight,
    layout: {
      background: { color: "#1a1a2e" },
      textColor: "#d1d4dc",
      fontFamily: "Consolas, Courier New, monospace",
    },
    grid: {
      vertLines: { color: "#2B2B43" },
      horzLines: { color: "#2B2B43" },
    },
    crosshair: { mode: CrosshairMode.Normal },
    rightPriceScale: {
      borderColor: "#2B2B43",
    },
    timeScale: {
      borderColor: "#2B2B43",
      timeVisible: true,
    },
  });

  candleSeries = chart.addCandlestickSeries({
    upColor: "#4caf50",
    downColor: "#f44336",
    borderDownColor: "#f44336",
    borderUpColor: "#4caf50",
    wickDownColor: "#f44336",
    wickUpColor: "#4caf50",
  });

  // Resize with window
  const ro = new ResizeObserver((entries) => {
    for (const entry of entries) {
      chart.resize(entry.contentRect.width, entry.contentRect.height);
    }
  });
  ro.observe(container);
}

// ── SL/TP Lines (draggable price lines on chart) ────────────

function createSLLine(price) {
  removeSLLine();
  slLine = candleSeries.createPriceLine({
    price: price,
    color: "#f44336",
    lineWidth: 2,
    lineStyle: 0, // solid
    axisLabelVisible: true,
    title: "SL",
    draggable: true,
  });
}

function createTPLine(price) {
  removeTPLine();
  tpLine = candleSeries.createPriceLine({
    price: price,
    color: "#4caf50",
    lineWidth: 2,
    lineStyle: 0,
    axisLabelVisible: true,
    title: "TP",
    draggable: true,
  });
}

function removeSLLine() {
  if (slLine) {
    candleSeries.removePriceLine(slLine);
    slLine = null;
  }
}

function removeTPLine() {
  if (tpLine) {
    candleSeries.removePriceLine(tpLine);
    tpLine = null;
  }
}

function getSLPrice() {
  return slLine ? slLine.options().price : null;
}

function getTPPrice() {
  return tpLine ? tpLine.options().price : null;
}

// ── Load Chart Data ─────────────────────────────────────────

async function loadChart(symbol, timeframe) {
  try {
    const barsJson = await invoke("get_bars", {
      symbol,
      timeframe,
      limit: 500,
    });
    const bars = JSON.parse(barsJson);

    const chartData = bars.map((b) => ({
      time: Math.floor(new Date(b.timestamp).getTime() / 1000),
      open: b.open,
      high: b.high,
      low: b.low,
      close: b.close,
    }));

    candleSeries.setData(chartData);
    chart.timeScale().fitContent();
    currentSymbol = symbol;
    currentTimeframe = timeframe;
  } catch (e) {
    console.error("Failed to load chart:", e);
  }
}

// ── Dashboard Update ────────────────────────────────────────

async function updateDashboard() {
  try {
    const acctJson = await invoke("get_account");
    const acct = JSON.parse(acctJson);

    document.getElementById("account-info").textContent =
      `Eq: $${Number(acct.equity).toLocaleString()} | BP: $${Number(acct.buying_power).toLocaleString()}`;

    document.getElementById("info-equity").textContent =
      `Eq: $${Number(acct.equity).toFixed(2)}`;
    document.getElementById("info-balance").textContent =
      `Bal: $${Number(acct.balance).toFixed(2)}`;

    const ml = acct.initial_margin > 0.01
      ? (acct.equity / acct.initial_margin * 100).toFixed(1)
      : "999.0";
    document.getElementById("info-margin").textContent = `ML: ${ml}%`;

    // Update positions
    const posJson = await invoke("get_positions");
    const positions = JSON.parse(posJson);

    let totalPL = 0;
    let posText = "Position: —";
    for (const p of positions) {
      if (p.symbol === currentSymbol) {
        totalPL = p.unrealized_pl;
        posText = `${p.side === "long" ? "Long" : "Short"} ${p.qty} lots`;
      }
    }

    document.getElementById("info-position").textContent = posText;
    const plEl = document.getElementById("info-pl");
    plEl.textContent = `P/L: $${totalPL.toFixed(2)}`;
    plEl.className = `dash-row ${totalPL >= 0 ? "positive" : "negative"}`;
  } catch (e) {
    // Not connected yet — silent
  }
}

// ── Button Handlers ─────────────────────────────────────────

function setupButtons() {
  // Load chart
  document.getElementById("btn-load-chart").addEventListener("click", () => {
    const symbol = document.getElementById("symbol-input").value.trim();
    const tf = document.getElementById("timeframe-select").value;
    if (symbol) loadChart(symbol, tf);
  });

  // Enter to load chart
  document.getElementById("symbol-input").addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      document.getElementById("btn-load-chart").click();
    }
  });

  // Buy Lines: SL = lowest visible, TP = highest visible
  document.getElementById("btn-buy-lines").addEventListener("click", () => {
    const data = candleSeries.data();
    if (!data || data.length === 0) return;
    // Use last 50 bars or all visible
    const recent = data.slice(-50);
    const low = Math.min(...recent.map((d) => d.low));
    const high = Math.max(...recent.map((d) => d.high));
    createSLLine(low);
    createTPLine(high);
  });

  // Sell Lines: SL = highest visible, TP = lowest visible
  document.getElementById("btn-sell-lines").addEventListener("click", () => {
    const data = candleSeries.data();
    if (!data || data.length === 0) return;
    const recent = data.slice(-50);
    const low = Math.min(...recent.map((d) => d.low));
    const high = Math.max(...recent.map((d) => d.high));
    createSLLine(high);
    createTPLine(low);
  });

  // Destroy Lines
  document.getElementById("btn-destroy-lines").addEventListener("click", () => {
    removeSLLine();
    removeTPLine();
  });

  // Open Trade — uses SL/TP lines + order mode to calculate lots
  document.getElementById("btn-trade").addEventListener("click", async () => {
    const sl = getSLPrice();
    const tp = getTPPrice();
    if (!sl || !tp || !currentSymbol) {
      alert("Set SL and TP lines first, and load a chart.");
      return;
    }
    // Determine direction from TP/SL relationship
    const side = tp > sl ? "buy" : "sell";
    // TODO: Calculate lots from risk config via Rust backend
    // For now, prompt for lot size
    const qty = prompt(`${side.toUpperCase()} ${currentSymbol}\nSL: ${sl}\nTP: ${tp}\n\nQuantity:`);
    if (!qty || isNaN(qty)) return;

    try {
      const result = await invoke("place_order", {
        symbol: currentSymbol,
        qty: parseFloat(qty),
        side,
      });
      console.log("Order result:", result);
      updateDashboard();
    } catch (e) {
      alert(`Order failed: ${e}`);
    }
  });

  // Close All
  document.getElementById("btn-close-all").addEventListener("click", async () => {
    if (!confirm("Close ALL positions on " + currentSymbol + "?")) return;
    try {
      await invoke("close_position", { symbol: currentSymbol, qty: null });
      updateDashboard();
    } catch (e) {
      alert(`Close failed: ${e}`);
    }
  });

  // Close Partial — close smallest lot
  document.getElementById("btn-close-partial").addEventListener("click", async () => {
    const qty = prompt("Qty to close on " + currentSymbol + ":");
    if (!qty || isNaN(qty)) return;
    try {
      await invoke("close_position", { symbol: currentSymbol, qty: parseFloat(qty) });
      updateDashboard();
    } catch (e) {
      alert(`Close partial failed: ${e}`);
    }
  });

  // Set SL / Set TP — modify existing positions (TODO: implement via Alpaca bracket orders)
  document.getElementById("btn-set-sl").addEventListener("click", () => {
    const sl = getSLPrice();
    if (sl) console.log("Set SL:", sl); // TODO: modify position SL
  });

  document.getElementById("btn-set-tp").addEventListener("click", () => {
    const tp = getTPPrice();
    if (tp) console.log("Set TP:", tp); // TODO: modify position TP
  });

  // Martingale toggle
  document.getElementById("btn-martingale").addEventListener("click", () => {
    const btn = document.getElementById("btn-martingale");
    const modes = ["MG: OFF", "MG: LONG", "MG: SHORT", "MG: UNWIND"];
    const current = modes.indexOf(btn.textContent);
    const next = (current + 1) % modes.length;
    btn.textContent = modes[next];
    // TODO: sync with Rust backend MartingaleState
  });

  // Open MG
  document.getElementById("btn-open-mg").addEventListener("click", async () => {
    if (!currentSymbol) return;
    // TODO: full Open MG flow via Rust backend
    alert("Open MG: Not yet implemented — will calculate safe gross and place hedge/bias pairs");
  });
}

// ── Keyboard Shortcuts ──────────────────────────────────────

function setupKeyboard() {
  document.addEventListener("keydown", (e) => {
    // Don't trigger shortcuts when typing in inputs
    if (e.target.tagName === "INPUT" || e.target.tagName === "SELECT") return;

    switch (e.key) {
      case "b":
        document.getElementById("btn-buy-lines").click();
        break;
      case "s":
        document.getElementById("btn-sell-lines").click();
        break;
      case "d":
        document.getElementById("btn-destroy-lines").click();
        break;
      case "t":
        document.getElementById("btn-trade").click();
        break;
      case "Escape":
        removeSLLine();
        removeTPLine();
        break;
    }
  });
}

// ── Init ────────────────────────────────────────────────────

document.addEventListener("DOMContentLoaded", () => {
  initChart();
  setupButtons();
  setupKeyboard();

  // Dashboard refresh every 2 seconds
  setInterval(updateDashboard, 2000);
});
