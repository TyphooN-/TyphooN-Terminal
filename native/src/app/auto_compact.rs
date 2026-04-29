//! Auto-compaction scheduler for the bar/KV cache.
//!
//! Policy is locked in by ADR-205. This module owns the gating logic and the
//! AC-power probe; the scheduler tick lives in the main update loop and dispatches
//! `BrokerCmd::CompactStorage` once the gate passes.
//!
//! Gate (all must hold):
//! - User has not disabled auto-compact in Storage Manager.
//! - At least `CADENCE_DAYS` have elapsed since the last successful run.
//! - Local time is within the configured idle window (default Sunday 04:00–05:00).
//! - The engine has been idle for ≥ `IDLE_THRESHOLD` (no UI input, no compact in flight).
//! - The host is on AC power (best-effort; non-Linux assumes AC).
//! - At least `UNCOMPACTED_THRESHOLD` rows are below the target zstd level.

use chrono::{Datelike, Timelike};

/// Target zstd level for periodic compaction.
pub const TARGET_LEVEL: i32 = 22;

/// Minimum days between automated runs.
pub const CADENCE_DAYS: i64 = 7;

/// User must have been idle this long before we start a run.
pub const IDLE_THRESHOLD_SECS: u64 = 300;

/// Skip the run if fewer rows than this are below TARGET_LEVEL — not worth the wake-up.
pub const UNCOMPACTED_THRESHOLD: i64 = 100;

/// Default idle window: weekday-of-week (0 = Sunday … 6 = Saturday) and the
/// inclusive hour range in local time during which a run is allowed to start.
pub const DEFAULT_WINDOW_WEEKDAY: u32 = 0; // Sunday
pub const DEFAULT_WINDOW_HOUR_START: u32 = 4; // 04:00 local
pub const DEFAULT_WINDOW_HOUR_END: u32 = 5; // 05:00 local (exclusive upper bound)

#[derive(Debug, Clone)]
pub struct GateInputs {
    pub enabled: bool,
    pub last_run_ms: i64,
    pub now_ms: i64,
    pub local_weekday: u32,
    pub local_hour: u32,
    pub idle_for_secs: u64,
    pub on_ac: bool,
    pub uncompacted_count: i64,
    pub in_progress: bool,
}

#[derive(Debug, Clone)]
pub struct GateDecision {
    pub run: bool,
    pub reason: String,
}

pub fn evaluate_gate(inputs: &GateInputs) -> GateDecision {
    if !inputs.enabled {
        return skip("auto-compact disabled in Storage Manager");
    }
    if inputs.in_progress {
        return skip("compact already running");
    }
    let cadence_ms: i64 = CADENCE_DAYS * 24 * 60 * 60 * 1000;
    if inputs.last_run_ms > 0 && (inputs.now_ms - inputs.last_run_ms) < cadence_ms {
        let days_remaining =
            ((cadence_ms - (inputs.now_ms - inputs.last_run_ms)) / 86_400_000).max(0);
        return skip(&format!("last run too recent (~{}d remaining)", days_remaining));
    }
    if inputs.local_weekday != DEFAULT_WINDOW_WEEKDAY {
        return skip("outside idle window (wrong weekday)");
    }
    if inputs.local_hour < DEFAULT_WINDOW_HOUR_START || inputs.local_hour >= DEFAULT_WINDOW_HOUR_END
    {
        return skip("outside idle window (wrong hour)");
    }
    if inputs.idle_for_secs < IDLE_THRESHOLD_SECS {
        return skip(&format!(
            "user activity within last {}s",
            IDLE_THRESHOLD_SECS
        ));
    }
    if !inputs.on_ac {
        return skip("running on battery");
    }
    if inputs.uncompacted_count < UNCOMPACTED_THRESHOLD {
        return skip(&format!(
            "only {} uncompacted rows (< {})",
            inputs.uncompacted_count, UNCOMPACTED_THRESHOLD
        ));
    }
    GateDecision {
        run: true,
        reason: format!(
            "idle window + {} rows below zstd-{}",
            inputs.uncompacted_count, TARGET_LEVEL
        ),
    }
}

fn skip(reason: &str) -> GateDecision {
    GateDecision {
        run: false,
        reason: reason.to_string(),
    }
}

/// Best-effort AC-power probe. On Linux, walks `/sys/class/power_supply/` and
/// returns true if any `Mains` entry is online, OR if no battery is present
/// (desktop). On other platforms returns true — most trading rigs are wall-powered
/// and the gating is conservative enough that a false-positive here is fine.
pub fn on_ac_power() -> bool {
    #[cfg(target_os = "linux")]
    {
        on_ac_power_linux()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

#[cfg(target_os = "linux")]
fn on_ac_power_linux() -> bool {
    use std::fs;
    let dir = match fs::read_dir("/sys/class/power_supply") {
        Ok(d) => d,
        Err(_) => return true,
    };
    let mut found_battery = false;
    for entry in dir.flatten() {
        let path = entry.path();
        let kind = fs::read_to_string(path.join("type")).unwrap_or_default();
        match kind.trim() {
            "Mains" => {
                let online = fs::read_to_string(path.join("online")).unwrap_or_default();
                if online.trim() == "1" {
                    return true;
                }
            }
            "Battery" => {
                found_battery = true;
            }
            _ => {}
        }
    }
    !found_battery
}

/// Compute (local_weekday, local_hour) for the current moment using chrono::Local.
pub fn local_weekday_hour_now() -> (u32, u32) {
    let now = chrono::Local::now();
    (now.weekday().num_days_from_sunday(), now.hour())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> GateInputs {
        GateInputs {
            enabled: true,
            last_run_ms: 0,
            now_ms: 1_800_000_000_000,
            local_weekday: DEFAULT_WINDOW_WEEKDAY,
            local_hour: DEFAULT_WINDOW_HOUR_START,
            idle_for_secs: IDLE_THRESHOLD_SECS,
            on_ac: true,
            uncompacted_count: UNCOMPACTED_THRESHOLD + 1,
            in_progress: false,
        }
    }

    #[test]
    fn gate_passes_with_default_inputs() {
        let d = evaluate_gate(&base());
        assert!(d.run, "expected pass: {}", d.reason);
    }

    #[test]
    fn gate_skips_when_disabled() {
        let mut i = base();
        i.enabled = false;
        assert!(!evaluate_gate(&i).run);
    }

    #[test]
    fn gate_skips_when_already_running() {
        let mut i = base();
        i.in_progress = true;
        assert!(!evaluate_gate(&i).run);
    }

    #[test]
    fn gate_skips_within_cadence() {
        let mut i = base();
        i.last_run_ms = i.now_ms - (CADENCE_DAYS - 1) * 86_400_000;
        assert!(!evaluate_gate(&i).run);
    }

    #[test]
    fn gate_passes_after_cadence() {
        let mut i = base();
        i.last_run_ms = i.now_ms - (CADENCE_DAYS + 1) * 86_400_000;
        assert!(evaluate_gate(&i).run);
    }

    #[test]
    fn gate_skips_off_weekday() {
        let mut i = base();
        i.local_weekday = (DEFAULT_WINDOW_WEEKDAY + 1) % 7;
        assert!(!evaluate_gate(&i).run);
    }

    #[test]
    fn gate_skips_off_hour() {
        let mut i = base();
        i.local_hour = DEFAULT_WINDOW_HOUR_END;
        assert!(!evaluate_gate(&i).run);
    }

    #[test]
    fn gate_skips_when_user_active() {
        let mut i = base();
        i.idle_for_secs = IDLE_THRESHOLD_SECS - 1;
        assert!(!evaluate_gate(&i).run);
    }

    #[test]
    fn gate_skips_on_battery() {
        let mut i = base();
        i.on_ac = false;
        assert!(!evaluate_gate(&i).run);
    }

    #[test]
    fn gate_skips_under_threshold() {
        let mut i = base();
        i.uncompacted_count = UNCOMPACTED_THRESHOLD - 1;
        assert!(!evaluate_gate(&i).run);
    }
}
