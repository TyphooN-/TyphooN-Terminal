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
  entry.innerHTML = `<span class="log-time">${time}</span>${msg}`;
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

// ── Indicator Series ─────────────────────────────────────────

let indicatorSeries = {}; // key → series object

function clearIndicators() {
  for (const [key, series] of Object.entries(indicatorSeries)) {
    chart.removeSeries(series);
  }
  indicatorSeries = {};
}

function calcSMA(data, period) {
  const result = [];
  for (let i = period - 1; i < data.length; i++) {
    let sum = 0;
    for (let j = i - period + 1; j <= i; j++) sum += data[j].close;
    result.push({ time: data[i].time, value: sum / period });
  }
  return result;
}

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

function calcRSI(data, period) {
  const result = [];
  let gains = 0, losses = 0;
  for (let i = 1; i <= period; i++) {
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

function calcBollinger(data, period) {
  const upper = [], lower = [], mid = [];
  for (let i = period - 1; i < data.length; i++) {
    let sum = 0, sumSq = 0;
    for (let j = i - period + 1; j <= i; j++) { sum += data[j].close; sumSq += data[j].close ** 2; }
    const mean = sum / period;
    const std = Math.sqrt(sumSq / period - mean ** 2);
    mid.push({ time: data[i].time, value: mean });
    upper.push({ time: data[i].time, value: mean + 2 * std });
    lower.push({ time: data[i].time, value: mean - 2 * std });
  }
  return { upper, lower, mid };
}

function calcATR(data, period) {
  const result = [];
  const trs = [];
  for (let i = 1; i < data.length; i++) {
    const tr = Math.max(data[i].high - data[i].low, Math.abs(data[i].high - data[i - 1].close), Math.abs(data[i].low - data[i - 1].close));
    trs.push(tr);
  }
  let atr = trs.slice(0, period).reduce((a, b) => a + b, 0) / period;
  for (let i = period; i < trs.length; i++) {
    atr = (atr * (period - 1) + trs[i]) / period;
    result.push({ time: data[i + 1].time, value: atr });
  }
  return result;
}

function calcKAMA(data, period) {
  // Kaufman Adaptive Moving Average
  const fastSC = 2 / (2 + 1);   // fast EMA constant
  const slowSC = 2 / (30 + 1);  // slow EMA constant
  const result = [];
  if (data.length < period + 1) return result;
  let kama = data[period].close;
  for (let i = period; i < data.length; i++) {
    const direction = Math.abs(data[i].close - data[i - period].close);
    let volatility = 0;
    for (let j = i - period + 1; j <= i; j++) {
      volatility += Math.abs(data[j].close - data[j - 1].close);
    }
    const er = volatility !== 0 ? direction / volatility : 0;
    const sc = (er * (fastSC - slowSC) + slowSC) ** 2;
    kama = kama + sc * (data[i].close - kama);
    result.push({ time: data[i].time, value: kama });
  }
  return result;
}

function calcPrevCandleLevels(data) {
  // Previous candle high/low as horizontal markers on current bar
  const highs = [], lows = [];
  for (let i = 1; i < data.length; i++) {
    highs.push({ time: data[i].time, value: data[i - 1].high });
    lows.push({ time: data[i].time, value: data[i - 1].low });
  }
  return { highs, lows };
}

function calcATRProjection(data, period) {
  // ATR projected from current close as upper/lower bands
  const atrValues = [];
  for (let i = 1; i < data.length; i++) {
    const tr = Math.max(
      data[i].high - data[i].low,
      Math.abs(data[i].high - data[i - 1].close),
      Math.abs(data[i].low - data[i - 1].close)
    );
    atrValues.push(tr);
  }
  if (atrValues.length < period) return { upper: [], lower: [] };

  let atr = atrValues.slice(0, period).reduce((a, b) => a + b, 0) / period;
  const upper = [], lower = [];
  for (let i = period; i < atrValues.length; i++) {
    atr = (atr * (period - 1) + atrValues[i]) / period;
    const idx = i + 1; // offset by 1 since atrValues starts at data[1]
    if (idx < data.length) {
      upper.push({ time: data[idx].time, value: data[idx].close + atr });
      lower.push({ time: data[idx].time, value: data[idx].close - atr });
    }
  }
  return { upper, lower };
}

function applyIndicators(chartData) {
  clearIndicators();
  const checkboxes = document.querySelectorAll("#indicator-list input[type=checkbox]:checked");

  for (const cb of checkboxes) {
    const ind = cb.dataset.ind;
    const period = parseInt(cb.dataset.period) || 14;
    const key = `${ind}_${period}`;

    if (ind === "sma" && chartData.length > period) {
      const s = chart.addLineSeries({ color: "#ffeb3b", lineWidth: 1, title: `SMA${period}` });
      s.setData(calcSMA(chartData, period));
      indicatorSeries[key] = s;
    } else if (ind === "ema" && chartData.length > period) {
      const colors = { 50: "#2196f3", 200: "#ff9800", 20: "#e91e63" };
      const s = chart.addLineSeries({ color: colors[period] || "#fff", lineWidth: 1, title: `EMA${period}` });
      s.setData(calcEMA(chartData, period));
      indicatorSeries[key] = s;
    } else if (ind === "bollinger" && chartData.length > period) {
      const bb = calcBollinger(chartData, period);
      const su = chart.addLineSeries({ color: "#9c27b0", lineWidth: 1, lineStyle: 2, title: "BB+" });
      const sl = chart.addLineSeries({ color: "#9c27b0", lineWidth: 1, lineStyle: 2, title: "BB-" });
      su.setData(bb.upper); sl.setData(bb.lower);
      indicatorSeries[key + "_u"] = su;
      indicatorSeries[key + "_l"] = sl;
    } else if (ind === "volume") {
      const s = chart.addHistogramSeries({
        color: "#26a69a", priceFormat: { type: "volume" },
        priceScaleId: "volume",
      });
      chart.priceScale("volume").applyOptions({ scaleMargins: { top: 0.8, bottom: 0 } });
      s.setData(chartData.map(d => ({ time: d.time, value: d.volume || 0, color: d.close >= d.open ? "#26a69a80" : "#ef535080" })));
      indicatorSeries[key] = s;
    }
    else if (ind === "kama" && chartData.length > period) {
      const s = chart.addLineSeries({ color: "#e91e63", lineWidth: 2, title: `KAMA${period}` });
      s.setData(calcKAMA(chartData, period));
      indicatorSeries[key] = s;
    } else if (ind === "prev-levels" && chartData.length > 1) {
      const pcl = calcPrevCandleLevels(chartData);
      const sh = chart.addLineSeries({ color: "#ffeb3b", lineWidth: 1, lineStyle: 2, title: "PrevH", lastValueVisible: false, priceLineVisible: false });
      const sl2 = chart.addLineSeries({ color: "#ffeb3b", lineWidth: 1, lineStyle: 2, title: "PrevL", lastValueVisible: false, priceLineVisible: false });
      sh.setData(pcl.highs); sl2.setData(pcl.lows);
      indicatorSeries[key + "_h"] = sh;
      indicatorSeries[key + "_l"] = sl2;
    } else if (ind === "atr-proj" && chartData.length > period + 1) {
      const atrp = calcATRProjection(chartData, period);
      const su = chart.addLineSeries({ color: "#00bcd4", lineWidth: 1, lineStyle: 1, title: "ATR+", lastValueVisible: false });
      const sl3 = chart.addLineSeries({ color: "#00bcd4", lineWidth: 1, lineStyle: 1, title: "ATR-", lastValueVisible: false });
      su.setData(atrp.upper); sl3.setData(atrp.lower);
      indicatorSeries[key + "_u"] = su;
      indicatorSeries[key + "_l"] = sl3;
    }
    // RSI, MACD, ATR would need a separate pane — coming soon
    else if (ind === "rsi" || ind === "macd" || ind === "atr" || ind === "vwap") {
      log(`${ind.toUpperCase()} indicator requires separate pane — coming soon`, "warn");
    }
  }
}

// ── Bar Cache ───────────────────────────────────────────────

const barCache = {}; // "SYMBOL:TF" → { data: [], timestamp: Date }
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes

function getCacheKey(symbol, tf) { return `${symbol}:${tf}`; }

// ── Load Chart Data ─────────────────────────────────────────

let liveBarInterval = null;

async function loadChart(symbol, timeframe) {
  const loading = document.getElementById("loading-indicator");
  loading.classList.remove("hidden");
  loading.textContent = "Loading...";

  try {
    const limit = parseInt(document.getElementById("bar-count").value) || 1000;
    const cacheKey = getCacheKey(symbol, timeframe);
    let bars;

    // Check cache
    const cached = barCache[cacheKey];
    if (cached && (Date.now() - cached.timestamp) < CACHE_TTL_MS) {
      bars = cached.data;
      log(`${symbol} @ ${timeframe}: ${bars.length} bars from cache`, "info");
    } else {
      const barsJson = await invoke("get_bars", { symbol, timeframe, limit });
      bars = JSON.parse(barsJson);
      barCache[cacheKey] = { data: bars, timestamp: Date.now() };
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
      return;
    }

    candleSeries.setData(chartData);
    chart.timeScale().fitContent();
    currentSymbol = symbol;
    currentTimeframe = timeframe;
    if (chartData.length > 0) lastPrice = chartData[chartData.length - 1].close;

    // Apply indicators
    applyIndicators(chartData);

    log(`${symbol} @ ${timeframe}: ${chartData.length} bars, last=$${lastPrice}`, "ok");
    setText("connect-status-bar", `${symbol} — ${chartData.length} bars`);
    loading.classList.add("hidden");

    // Start live bar polling (update latest bar every 10s)
    if (liveBarInterval) clearInterval(liveBarInterval);
    liveBarInterval = setInterval(() => updateLatestBar(symbol, timeframe), 10000);
  } catch (e) {
    log(`Chart load failed for ${symbol} @ ${timeframe}: ${e}`, "error");
    setText("connect-status-bar", `Chart error: ${e}`);
    loading.classList.add("hidden");
  }
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
          item.innerHTML = `<span class="sym">${sym}</span><span class="name">${name}</span>`;
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

// ── Init ────────────────────────────────────────────────────

document.addEventListener("DOMContentLoaded", () => {
  initChart();
  setupLogPanel();
  setupIndicatorPanel();
  setupAutocomplete();
  setupButtons();
  setupKeyboard();
  setupConnect();
});
