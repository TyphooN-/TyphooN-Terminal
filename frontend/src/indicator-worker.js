// indicator-worker.js — Web Worker for off-thread indicator computation
// Receives bar data, computes indicators via Wasm or JS fallback, posts results back.
// This prevents main thread blocking on large datasets (50K+ bars).

let wasmModule = null;

// Try to load Wasm in the worker context
async function initWasm() {
  try {
    const mod = await import("./wasm_indicators.js");
    await mod.default();
    wasmModule = mod;
    postMessage({ type: "wasm_ready" });
  } catch (e) {
    postMessage({ type: "wasm_fallback", error: e.message });
  }
}

// Pack bars to flat f64 array for Wasm
function packBars(bars) {
  const flat = new Float64Array(bars.length * 5);
  for (let i = 0; i < bars.length; i++) {
    flat[i * 5] = bars[i].o;
    flat[i * 5 + 1] = bars[i].h;
    flat[i * 5 + 2] = bars[i].l;
    flat[i * 5 + 3] = bars[i].c;
    flat[i * 5 + 4] = bars[i].v || 0;
  }
  return flat;
}

// JS fallback implementations (same algorithms as main thread)
function jsSMA(bars, period) {
  const result = [];
  for (let i = period - 1; i < bars.length; i++) {
    let sum = 0;
    for (let j = i - period + 1; j <= i; j++) sum += bars[j].c;
    result.push(sum / period);
  }
  return result;
}

function jsEMA(bars, period) {
  if (bars.length < period) return [];
  const k = 2 / (period + 1);
  // SMA bootstrap for first period bars
  let sum = 0;
  for (let i = 0; i < period; i++) sum += bars[i].c;
  let ema = sum / period;
  const result = [ema];
  for (let i = period; i < bars.length; i++) {
    ema = bars[i].c * k + ema * (1 - k);
    result.push(ema);
  }
  return result;
}

function jsKAMA(bars, period, fastP = 2, slowP = 30) {
  const fastSC = 2 / (fastP + 1);
  const slowSC = 2 / (slowP + 1);
  const result = [];
  if (bars.length < period + 1) return result;
  let kama = bars[period - 1].c;
  for (let i = period; i < bars.length; i++) {
    const signal = Math.abs(bars[i].c - bars[i - period].c);
    let noise = 0;
    for (let j = i - period + 1; j <= i; j++) noise += Math.abs(bars[j].c - bars[j - 1].c);
    const er = noise !== 0 ? signal / noise : 0;
    const ssc = er * (fastSC - slowSC) + slowSC;
    kama = ssc * ssc * (bars[i].c - kama) + kama;
    result.push(kama);
  }
  return result;
}

function jsRSI(bars, period) {
  if (bars.length < period + 1) return [];
  let gains = 0, losses = 0;
  for (let i = 1; i <= period; i++) {
    const d = bars[i].c - bars[i - 1].c;
    if (d > 0) gains += d; else losses -= d;
  }
  let avgGain = gains / period, avgLoss = losses / period;
  const result = [];
  for (let i = period; i < bars.length; i++) {
    if (i > period) {
      const d = bars[i].c - bars[i - 1].c;
      avgGain = (avgGain * (period - 1) + (d > 0 ? d : 0)) / period;
      avgLoss = (avgLoss * (period - 1) + (d < 0 ? -d : 0)) / period;
    }
    const rs = avgLoss === 0 ? 100 : avgGain / avgLoss;
    result.push(100 - 100 / (1 + rs));
  }
  return result;
}

// ── Fisher Transform (off-thread) ─────────────────────────
function jsFisher(bars, period) {
  if (bars.length < period + 1) return { fisher: [], signal: [], colors: [] };
  const fisher = [], signal = [], colors = [];
  let prevSmoothed = 0, prevFisher = 0;
  const medians = bars.map(d => (d.h + d.l) / 2);
  for (let i = period; i < bars.length; i++) {
    let maxM = -Infinity, minM = Infinity;
    for (let j = i - period + 1; j <= i; j++) {
      if (medians[j] > maxM) maxM = medians[j];
      if (medians[j] < minM) minM = medians[j];
    }
    const range = maxM - minM;
    const normalized = range > 0 ? (medians[i] - minM) / range : 0.5;
    const os = 2.0 * (normalized - 0.5);
    let smoothed = 0.5 * os + 0.5 * prevSmoothed;
    smoothed = Math.max(-0.999, Math.min(0.999, smoothed));
    const ft = 0.25 * Math.log((1 + smoothed) / (1 - smoothed)) + 0.5 * prevFisher;
    const sig = prevFisher;
    const color = ft > sig ? 1 : ft < sig ? -1 : 0; // 1=bull, -1=bear, 0=neutral
    fisher.push(ft);
    signal.push(sig);
    colors.push(color);
    prevSmoothed = smoothed;
    prevFisher = ft;
  }
  return { fisher, signal, colors, startIdx: period };
}

// ── BetterVolume (off-thread) ─────────────────────────────
function jsBetterVolume(bars, lookback) {
  if (!lookback) lookback = 20;
  const result = [];
  for (let i = 0; i < bars.length; i++) {
    const vol = bars[i].v || 0;
    const range = bars[i].h - bars[i].l;
    const isUp = bars[i].c >= bars[i].o;
    // Compute average volume over lookback
    let avgVol = 0;
    const lb = Math.min(i, lookback);
    for (let j = i - lb; j < i; j++) avgVol += (bars[j].v || 0);
    avgVol = lb > 0 ? avgVol / lb : vol;
    // Color logic (matches BetterVolume.mqh)
    let colorFlag = 0; // 0=gray
    if (vol > avgVol * 1.5 && isUp) colorFlag = 1; // green - high vol up
    else if (vol > avgVol * 1.5 && !isUp) colorFlag = -1; // red - high vol down
    else if (vol < avgVol * 0.5) colorFlag = 2; // yellow - low vol
    else if (range > 0 && (bars[i].c - bars[i].o) / range > 0.7) colorFlag = 3; // cyan - wide spread up
    else if (range > 0 && (bars[i].o - bars[i].c) / range > 0.7) colorFlag = -2; // magenta - wide spread down
    result.push({ value: vol, color: colorFlag });
  }
  return result;
}

// ── ATR Projection (off-thread) ───────────────────────────
function jsATR(bars, period) {
  if (bars.length < period + 2) return [];
  const trs = [];
  for (let i = 1; i < bars.length; i++) {
    trs.push(Math.max(bars[i].h - bars[i].l, Math.abs(bars[i].h - bars[i-1].c), Math.abs(bars[i].l - bars[i-1].c)));
  }
  if (trs.length < period) return [];
  let atr = 0;
  for (let i = 0; i < period; i++) atr += trs[i];
  atr /= period;
  const result = [];
  for (let i = period; i < trs.length; i++) {
    atr = (atr * (period - 1) + trs[i]) / period;
    result.push(atr);
  }
  return result;
}

// Handle messages from main thread
self.onmessage = function(e) {
  const { id, type, bars, period, fastP, slowP } = e.data;

  try {
    let values;
    const flat = wasmModule ? packBars(bars) : null;

    switch (type) {
      case "sma":
        values = wasmModule ? Array.from(wasmModule.wasm_sma(flat, period)) : jsSMA(bars, period);
        break;
      case "ema":
        values = wasmModule ? Array.from(wasmModule.wasm_ema(flat, period)) : jsEMA(bars, period);
        break;
      case "kama":
        values = wasmModule ? Array.from(wasmModule.wasm_kama(flat, period, fastP || 2, slowP || 30)) : jsKAMA(bars, period, fastP || 2, slowP || 30);
        break;
      case "rsi":
        values = wasmModule ? Array.from(wasmModule.wasm_rsi(flat, period)) : jsRSI(bars, period);
        break;
      case "fisher":
        values = jsFisher(bars, period || 32);
        break;
      case "bettervolume":
        values = jsBetterVolume(bars, period || 20);
        break;
      case "atr":
        values = wasmModule ? Array.from(wasmModule.wasm_atr(flat, period)) : jsATR(bars, period);
        break;
      // Batch: compute multiple indicators at once for a grid cell
      case "grid_cell": {
        const sma200 = bars.length > 200 ? (wasmModule ? Array.from(wasmModule.wasm_sma(flat, 200)) : jsSMA(bars, 200)) : [];
        const sma100 = bars.length > 100 ? (wasmModule ? Array.from(wasmModule.wasm_sma(flat, 100)) : jsSMA(bars, 100)) : [];
        const kama = bars.length > 11 ? (wasmModule ? Array.from(wasmModule.wasm_kama(flat, 10, 2, 30)) : jsKAMA(bars, 10, 2, 30)) : [];
        const atrVals = bars.length > 15 ? (wasmModule ? Array.from(wasmModule.wasm_atr(flat, 14)) : jsATR(bars, 14)) : [];
        const fisher = bars.length > 32 ? jsFisher(bars, 32) : null;
        const bv = bars.length > 22 ? jsBetterVolume(bars, 20) : null;
        values = { sma200, sma100, kama, atr: atrVals, fisher, bettervolume: bv };
        break;
      }
      // Full batch: compute ANY requested indicators at once (for main chart)
      case "batch": {
        const { indicators } = e.data;
        const results = {};
        for (const ind of indicators) {
          try {
            const p = ind.period || 14;
            switch (ind.type) {
              case "sma":
                results[ind.key] = bars.length > p ? (wasmModule ? Array.from(wasmModule.wasm_sma(flat, p)) : jsSMA(bars, p)) : [];
                break;
              case "ema":
                results[ind.key] = bars.length > p ? (wasmModule ? Array.from(wasmModule.wasm_ema(flat, p)) : jsEMA(bars, p)) : [];
                break;
              case "kama":
                results[ind.key] = bars.length > p + 1 ? (wasmModule ? Array.from(wasmModule.wasm_kama(flat, p, ind.fastP || 2, ind.slowP || 30)) : jsKAMA(bars, p, ind.fastP || 2, ind.slowP || 30)) : [];
                break;
              case "rsi":
                results[ind.key] = bars.length > p + 1 ? (wasmModule ? Array.from(wasmModule.wasm_rsi(flat, p)) : jsRSI(bars, p)) : [];
                break;
              case "atr":
                results[ind.key] = bars.length > p + 2 ? (wasmModule ? Array.from(wasmModule.wasm_atr(flat, p)) : jsATR(bars, p)) : [];
                break;
              case "fisher":
                results[ind.key] = bars.length > (p || 32) + 1 ? jsFisher(bars, p || 32) : null;
                break;
              case "bettervolume":
                results[ind.key] = bars.length > 22 ? jsBetterVolume(bars, p || 20) : null;
                break;
              default:
                results[ind.key] = null;
            }
          } catch (err) {
            results[ind.key] = null;
          }
        }
        values = results;
        break;
      }
      default:
        values = [];
    }

    postMessage({ id, type: "result", values });
  } catch (err) {
    postMessage({ id, type: "error", error: err.message });
  }
};

// Initialize Wasm on worker start
initWasm();
