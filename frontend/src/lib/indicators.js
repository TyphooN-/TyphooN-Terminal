// indicators.js — All indicator calculation functions (pure math, no DOM)
// These are the JS fallback implementations. Wasm-accelerated versions wrap these.

export function calcSMA(data, period) {
  const result = [];
  for (let i = period - 1; i < data.length; i++) {
    let sum = 0;
    for (let j = i - period + 1; j <= i; j++) sum += data[j].close;
    result.push({ time: data[i].time, value: sum / period });
  }
  return result;
}

export function calcEMA(data, period) {
  const k = 2 / (period + 1);
  const result = [];
  let ema = data[0].close;
  for (let i = 0; i < data.length; i++) {
    ema = data[i].close * k + ema * (1 - k);
    if (i >= period - 1) result.push({ time: data[i].time, value: ema });
  }
  return result;
}

export function calcKAMA(data, period = 10, fastP = 2, slowP = 30) {
  const fastSC = 2.0 / (fastP + 1.0);
  const slowSC = 2.0 / (slowP + 1.0);
  const result = [];
  if (data.length < period + 1) return result;
  let kama = data[period].close;
  for (let i = period; i < data.length; i++) {
    const signal = Math.abs(data[i].close - data[i - period].close);
    let noise = 0;
    for (let j = i - period + 1; j <= i; j++) {
      noise += Math.abs(data[j].close - data[j - 1].close);
    }
    const er = noise !== 0 ? signal / noise : 0;
    const ssc = er * (fastSC - slowSC) + slowSC;
    kama = ssc * ssc * (data[i].close - kama) + kama;
    result.push({ time: data[i].time, value: kama });
  }
  return result;
}

export function calcRSI(data, period) {
  if (data.length < period + 1) return [];
  let gains = 0, losses = 0;
  for (let i = 1; i <= period; i++) {
    const d = data[i].close - data[i - 1].close;
    if (d > 0) gains += d; else losses -= d;
  }
  let avgGain = gains / period, avgLoss = losses / period;
  const result = [];
  for (let i = period; i < data.length; i++) {
    if (i > period) {
      const d = data[i].close - data[i - 1].close;
      avgGain = (avgGain * (period - 1) + (d > 0 ? d : 0)) / period;
      avgLoss = (avgLoss * (period - 1) + (d < 0 ? -d : 0)) / period;
    }
    const rs = avgLoss === 0 ? 100 : avgGain / avgLoss;
    result.push({ time: data[i].time, value: 100 - 100 / (1 + rs) });
  }
  return result;
}

export function calcATR(data, period) {
  const result = [];
  if (data.length < period + 2) return result;
  const trs = [];
  for (let i = 1; i < data.length; i++) {
    trs.push(Math.max(
      data[i].high - data[i].low,
      Math.abs(data[i].high - data[i - 1].close),
      Math.abs(data[i].low - data[i - 1].close)
    ));
  }
  let atr = trs.slice(0, period).reduce((a, b) => a + b, 0) / period;
  for (let i = period; i < trs.length; i++) {
    atr = (atr * (period - 1) + trs[i]) / period;
    result.push({ time: data[i + 1].time, value: atr });
  }
  return result;
}

export function calcDEMA(data, period) {
  const ema1 = calcEMA(data, period);
  if (ema1.length < period) return [];
  const ema2 = calcEMA(ema1.map((d, i) => ({ time: d.time, close: d.value, high: d.value, low: d.value, open: d.value })), period);
  const offset = ema1.length - ema2.length;
  return ema2.map((d, i) => ({ time: d.time, value: 2 * ema1[i + offset].value - d.value }));
}
