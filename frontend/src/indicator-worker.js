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
