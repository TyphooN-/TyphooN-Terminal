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

// Tauri v2 invoke — lazy access to avoid race with __TAURI__ injection
function invoke(cmd, args) {
  if (!window.__TAURI__ || !window.__TAURI__.core) {
    return Promise.reject("Tauri not loaded yet");
  }
  return window.__TAURI__.core.invoke(cmd, args);
}

// ── State ───────────────────────────────────────────────────

let chart = null;
let candleSeries = null;
let slLine = null;
let tpLine = null;
let currentSymbol = "";
let currentTimeframe = "1Hour";
let lastPrice = 0;

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
    rightPriceScale: { borderColor: "#2B2B43" },
    timeScale: { borderColor: "#2B2B43", timeVisible: true },
  });

  candleSeries = chart.addCandlestickSeries({
    upColor: "#4caf50",
    downColor: "#f44336",
    borderDownColor: "#f44336",
    borderUpColor: "#4caf50",
    wickDownColor: "#f44336",
    wickUpColor: "#4caf50",
  });

  new ResizeObserver((entries) => {
    for (const entry of entries) {
      chart.resize(entry.contentRect.width, entry.contentRect.height);
    }
  }).observe(container);
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

// ── Load Chart Data ─────────────────────────────────────────

async function loadChart(symbol, timeframe) {
  try {
    const barsJson = await invoke("get_bars", { symbol, timeframe, limit: 500 });
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
    if (chartData.length > 0) lastPrice = chartData[chartData.length - 1].close;
  } catch (e) {
    console.error("Failed to load chart:", e);
  }
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
    "1Min": 60, "5Min": 300, "15Min": 900, "1Hour": 3600,
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

// ── Button Handlers ─────────────────────────────────────────

function setupButtons() {
  document.getElementById("btn-load-chart").addEventListener("click", () => {
    const symbol = document.getElementById("symbol-input").value.trim();
    const tf = document.getElementById("timeframe-select").value;
    if (symbol) loadChart(symbol, tf);
  });

  document.getElementById("symbol-input").addEventListener("keydown", (e) => {
    if (e.key === "Enter") document.getElementById("btn-load-chart").click();
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
      status.textContent = `Connected! [${typeLabel}] Equity: $${Number(acct.equity).toFixed(2)}`;
      status.style.color = "#8f8";
      setTimeout(() => modal.classList.add("hidden"), 800);

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

// ── Init ────────────────────────────────────────────────────

document.addEventListener("DOMContentLoaded", () => {
  initChart();
  setupButtons();
  setupKeyboard();
  setupConnect();
});
