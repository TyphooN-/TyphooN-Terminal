//! Runtime environment definitions for compiled MQL5 indicators.
//!
//! Defines the WASM import functions that the compiled indicator expects:
//! - Bar data access (iOpen, iHigh, iLow, iClose, iVolume, iBars)
//! - Math functions (abs, sqrt, log, max, min)
//! - Buffer write (set_buffer)
//!
//! The frontend JS provides these as WASM imports when instantiating
//! the compiled indicator module.

/// JavaScript import object template for compiled MQL5 indicators.
/// The frontend uses this to create the WASM import object.
pub const JS_RUNTIME_TEMPLATE: &str = r#"
// MQL5 Runtime — WASM import object for compiled indicators.
// Provides bar data access + math functions.
// `bars` is a Float64Array of [O,H,L,C,V, O,H,L,C,V, ...].
function createMql5Runtime(bars, numBars) {
  const getBar = (shift) => {
    const idx = numBars - 1 - shift; // MQL5 shift: 0=current, 1=previous
    if (idx < 0 || idx >= numBars) return 0.0;
    return idx;
  };
  return {
    env: {
      iBars: () => numBars,
      iOpen:   (shift) => { const i = getBar(shift); return bars[i * 5]; },
      iHigh:   (shift) => { const i = getBar(shift); return bars[i * 5 + 1]; },
      iLow:    (shift) => { const i = getBar(shift); return bars[i * 5 + 2]; },
      iClose:  (shift) => { const i = getBar(shift); return bars[i * 5 + 3]; },
      iVolume: (shift) => { const i = getBar(shift); return bars[i * 5 + 4]; },
      math_abs:  (x) => Math.abs(x),
      math_sqrt: (x) => Math.sqrt(x),
      math_log:  (x) => Math.log(x),
      math_max:  (a, b) => Math.max(a, b),
      math_min:  (a, b) => Math.min(a, b),
      set_buffer: (barIdx, value) => {
        // Store in output buffer — caller provides the buffer array
        if (barIdx >= 0 && barIdx < numBars) {
          outputBuffer[barIdx] = value;
        }
      },
    },
  };
}
"#;
