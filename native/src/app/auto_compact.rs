//! Auto-compaction scheduler for the bar/KV cache.
//!
//! Policy is locked in by ADR-205. This module owns the gating logic and the
//! AC-power probe; the scheduler tick lives in the main update loop and dispatches
//! `BrokerCmd::CompactStorage` once the gate passes.
//!
//! Gate (all must hold):
//! - User has not disabled auto-compact in Storage Manager.
//! - At least the configured cadence has elapsed since the last successful run.
//! - Local time is within the configured idle window (default daily 04:00–05:00).
//! - The engine has been idle for ≥ `IDLE_THRESHOLD` (no UI input, no compact in flight).
//! - The host is on AC power (best-effort; non-Linux assumes AC).
//! - At least the configured row threshold is below the target zstd level.

use chrono::{Datelike, TimeZone, Timelike};

/// Target zstd level for periodic compaction.
pub const TARGET_LEVEL: i32 = 22;

/// Minimum days between automated runs. Live ingestion stores bars at a fast
/// zstd level for responsiveness; daily idle compaction restores cold rows to
/// the archival target without taxing foreground sync or chart interaction.
pub const CADENCE_DAYS: i64 = 1;

/// User must have been idle this long before we start a run.
pub const IDLE_THRESHOLD_SECS: u64 = 300;

/// Skip the run if fewer rows than this are below TARGET_LEVEL — not worth the wake-up.
pub const UNCOMPACTED_THRESHOLD: i64 = 100;

/// Default idle window: weekday-of-week (0 = Sunday … 6 = Saturday) and the
/// inclusive hour range in local time during which a run is allowed to start.
pub const DEFAULT_WINDOW_WEEKDAY: u32 = 0; // Sunday
pub const DEFAULT_WINDOW_HOUR_START: u32 = 4; // 04:00 local
pub const DEFAULT_WINDOW_HOUR_END: u32 = 5; // 05:00 local (exclusive upper bound)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    pub cadence_days: i64,
    pub window_weekday: u32,
    pub window_hour_start: u32,
    pub window_hour_end: u32,
    pub uncompacted_threshold: i64,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            cadence_days: CADENCE_DAYS,
            window_weekday: DEFAULT_WINDOW_WEEKDAY,
            window_hour_start: DEFAULT_WINDOW_HOUR_START,
            window_hour_end: DEFAULT_WINDOW_HOUR_END,
            uncompacted_threshold: UNCOMPACTED_THRESHOLD,
        }
    }
}

/// Cadence presets surfaced to the user. The underlying schedule still
/// stores `cadence_days`, so a custom interval can be dialled in via the
/// numeric DragValue; this enum is just the dropdown taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CadencePreset {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Custom,
}

impl CadencePreset {
    pub fn from_days(days: i64) -> Self {
        match days {
            1 => Self::Daily,
            7 => Self::Weekly,
            30 => Self::Monthly,
            365 => Self::Yearly,
            _ => Self::Custom,
        }
    }

    pub fn to_days(self, current: i64) -> i64 {
        match self {
            Self::Daily => 1,
            Self::Weekly => 7,
            Self::Monthly => 30,
            Self::Yearly => 365,
            Self::Custom => current.clamp(1, 365),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::Monthly => "Monthly",
            Self::Yearly => "Yearly",
            Self::Custom => "Custom",
        }
    }
}

impl Schedule {
    pub fn sanitized(self) -> Self {
        let cadence_days = self.cadence_days.clamp(1, 365);
        let window_weekday = self.window_weekday.min(6);
        let window_hour_start = self.window_hour_start.min(23);
        let mut window_hour_end = self.window_hour_end.clamp(1, 24);
        if window_hour_end <= window_hour_start {
            window_hour_end = (window_hour_start + 1).min(24);
        }
        let uncompacted_threshold = self.uncompacted_threshold.clamp(1, 1_000_000);
        Self {
            cadence_days,
            window_weekday,
            window_hour_start,
            window_hour_end,
            uncompacted_threshold,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GateInputs {
    pub enabled: bool,
    pub schedule: Schedule,
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
    let schedule = inputs.schedule.sanitized();
    if !inputs.enabled {
        return skip("auto-compact disabled in Storage Manager");
    }
    if inputs.in_progress {
        return skip("compact already running");
    }
    let cadence_ms: i64 = schedule.cadence_days * 24 * 60 * 60 * 1000;
    if inputs.last_run_ms > 0 && (inputs.now_ms - inputs.last_run_ms) < cadence_ms {
        let days_remaining =
            ((cadence_ms - (inputs.now_ms - inputs.last_run_ms)) / 86_400_000).max(0);
        return skip(&format!(
            "last run too recent (~{}d remaining)",
            days_remaining
        ));
    }
    // Sub-weekly cadences (Daily / Custom < 7d) skip the weekday gate so a
    // daily preset doesn't get pinned to whichever weekday the schedule still
    // has stored. Weekly / Monthly / Yearly keep their weekday so the user
    // can still steer the run to a low-traffic day.
    if schedule.cadence_days >= 7 && inputs.local_weekday != schedule.window_weekday {
        return skip(&format!(
            "outside idle window (expected {})",
            weekday_label(schedule.window_weekday)
        ));
    }
    if inputs.local_hour < schedule.window_hour_start
        || inputs.local_hour >= schedule.window_hour_end
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
    if inputs.uncompacted_count < schedule.uncompacted_threshold {
        return skip(&format!(
            "only {} uncompacted rows (< {})",
            inputs.uncompacted_count, schedule.uncompacted_threshold
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

pub fn next_eligible_time_ms(schedule: Schedule, last_run_ms: i64) -> i64 {
    next_eligible_time_ms_at(schedule, last_run_ms, chrono::Local::now())
}

fn next_eligible_time_ms_at(
    schedule: Schedule,
    last_run_ms: i64,
    now: chrono::DateTime<chrono::Local>,
) -> i64 {
    let schedule = schedule.sanitized();
    let now_ms = now.timestamp_millis();
    let cadence_ready_ms = if last_run_ms > 0 {
        last_run_ms.saturating_add(schedule.cadence_days * 86_400_000)
    } else {
        now_ms
    };
    let weekday_gated = schedule.cadence_days >= 7;

    if now_ms >= cadence_ready_ms
        && (!weekday_gated || now.weekday().num_days_from_sunday() == schedule.window_weekday)
        && now.hour() >= schedule.window_hour_start
        && now.hour() < schedule.window_hour_end
    {
        return now_ms;
    }

    let today = now.date_naive();
    for day_offset in 0..=370 {
        let Some(date) = today.checked_add_signed(chrono::Duration::days(day_offset)) else {
            continue;
        };
        if weekday_gated && date.weekday().num_days_from_sunday() != schedule.window_weekday {
            continue;
        }
        let Some(window_start_ms) = local_window_boundary_ms(date, schedule.window_hour_start)
        else {
            continue;
        };
        let Some(window_end_ms) = local_window_boundary_ms(date, schedule.window_hour_end) else {
            continue;
        };
        let candidate_ms = window_start_ms.max(now_ms).max(cadence_ready_ms);
        if candidate_ms < window_end_ms {
            return candidate_ms;
        }
    }

    now_ms.max(cadence_ready_ms)
}

fn local_window_boundary_ms(date: chrono::NaiveDate, hour: u32) -> Option<i64> {
    let (date, hour) = if hour >= 24 {
        (date.checked_add_signed(chrono::Duration::days(1))?, 0)
    } else {
        (date, hour)
    };
    let naive = date.and_hms_opt(hour, 0, 0)?;
    match chrono::Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Some(dt.timestamp_millis()),
        chrono::LocalResult::Ambiguous(a, b) => Some(a.min(b).timestamp_millis()),
        chrono::LocalResult::None => None,
    }
}

pub fn weekday_label(weekday: u32) -> &'static str {
    match weekday {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        6 => "Sat",
        _ => "Sun",
    }
}

pub fn schedule_summary(schedule: Schedule) -> String {
    let schedule = schedule.sanitized();
    let preset = CadencePreset::from_days(schedule.cadence_days);
    let cadence_label: String = match preset {
        CadencePreset::Custom => format!("every {}d", schedule.cadence_days),
        _ => preset.label().to_string(),
    };
    if schedule.cadence_days >= 7 {
        format!(
            "{} {} {:02}:00-{:02}:00, >= {} rows",
            cadence_label,
            weekday_label(schedule.window_weekday),
            schedule.window_hour_start,
            schedule.window_hour_end,
            schedule.uncompacted_threshold
        )
    } else {
        format!(
            "{} {:02}:00-{:02}:00, >= {} rows",
            cadence_label,
            schedule.window_hour_start,
            schedule.window_hour_end,
            schedule.uncompacted_threshold
        )
    }
}

fn skip(reason: &str) -> GateDecision {
    GateDecision {
        run: false,
        reason: reason.to_string(),
    }
}

/// Best-effort AC-power probe.
///
/// - **Linux:** walks `/sys/class/power_supply/`, returns true if any `Mains` entry
///   is online OR if no battery is present (desktop).
/// - **Windows:** calls `GetSystemPowerStatus` and reads `ACLineStatus`. `1`
///   = AC online, `0` = on battery, `255` = unknown (treated as AC since most
///   Windows trading rigs are wall-powered).
/// - **macOS:** shells out to `pmset -g batt` and treats `AC Power` as online.
/// - **Other:** returns true (assume AC) because unknown desktops should not be
///   blocked from scheduled compaction.
pub fn on_ac_power() -> bool {
    #[cfg(target_os = "linux")]
    {
        on_ac_power_linux()
    }
    #[cfg(target_os = "windows")]
    {
        on_ac_power_windows()
    }
    #[cfg(target_os = "macos")]
    {
        on_ac_power_macos()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
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

#[cfg(target_os = "macos")]
fn on_ac_power_macos() -> bool {
    let output = match std::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output()
    {
        Ok(output) => output,
        Err(_) => return true,
    };
    if !output.status.success() {
        return true;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    if text.contains("AC Power") {
        true
    } else if text.contains("Battery Power") {
        false
    } else {
        true
    }
}

#[cfg(target_os = "windows")]
fn on_ac_power_windows() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetSystemPowerStatus(&mut status as *mut _) };
    if ok == 0 {
        return true;
    }
    match status.ACLineStatus {
        0 => false, // running on battery
        1 => true,  // running on AC
        _ => true,  // unknown (255) — assume AC for desktop trading boxes
    }
}

/// Compute (local_weekday, local_hour) for the current moment using chrono::Local.
pub fn local_weekday_hour_now() -> (u32, u32) {
    let now = chrono::Local::now();
    (now.weekday().num_days_from_sunday(), now.hour())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn base() -> GateInputs {
        GateInputs {
            enabled: true,
            schedule: Schedule::default(),
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
        i.last_run_ms = i.now_ms - (Schedule::default().cadence_days - 1) * 86_400_000;
        assert!(!evaluate_gate(&i).run);
    }

    #[test]
    fn gate_passes_after_cadence() {
        let mut i = base();
        i.last_run_ms = i.now_ms - (Schedule::default().cadence_days + 1) * 86_400_000;
        assert!(evaluate_gate(&i).run);
    }

    #[test]
    fn weekly_gate_skips_off_weekday() {
        let mut i = base();
        i.schedule.cadence_days = 7;
        i.local_weekday = (DEFAULT_WINDOW_WEEKDAY + 1) % 7;
        assert!(!evaluate_gate(&i).run);
    }

    #[test]
    fn daily_default_ignores_weekday_gate() {
        let mut i = base();
        i.local_weekday = (DEFAULT_WINDOW_WEEKDAY + 1) % 7;
        assert!(evaluate_gate(&i).run);
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

    #[test]
    fn gate_uses_custom_schedule() {
        let mut i = base();
        i.schedule = Schedule {
            cadence_days: 1,
            window_weekday: 2,
            window_hour_start: 8,
            window_hour_end: 10,
            uncompacted_threshold: 5,
        };
        i.local_weekday = 2;
        i.local_hour = 8;
        i.uncompacted_count = 5;
        assert!(evaluate_gate(&i).run);
    }

    #[test]
    fn schedule_sanitizes_invalid_bounds() {
        let s = Schedule {
            cadence_days: 0,
            window_weekday: 12,
            window_hour_start: 24,
            window_hour_end: 1,
            uncompacted_threshold: 0,
        }
        .sanitized();
        assert_eq!(s.cadence_days, 1);
        assert_eq!(s.window_weekday, 6);
        assert_eq!(s.window_hour_start, 23);
        assert_eq!(s.window_hour_end, 24);
        assert_eq!(s.uncompacted_threshold, 1);
    }

    fn local_dt(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        min: u32,
    ) -> chrono::DateTime<chrono::Local> {
        match chrono::Local.with_ymd_and_hms(year, month, day, hour, min, 0) {
            chrono::LocalResult::Single(dt) => dt,
            chrono::LocalResult::Ambiguous(a, b) => a.min(b),
            chrono::LocalResult::None => panic!("test local datetime does not exist"),
        }
    }

    #[test]
    fn next_eligible_can_be_inside_current_window_after_cadence_wait() {
        let schedule = Schedule::default();
        let now = local_dt(2026, 5, 3, DEFAULT_WINDOW_HOUR_START, 0);
        let last_run_ms = now.timestamp_millis() - schedule.cadence_days * 86_400_000 + 30 * 60_000;
        let next = next_eligible_time_ms_at(schedule, last_run_ms, now);
        assert_eq!(next, now.timestamp_millis() + 30 * 60_000);
    }

    #[test]
    fn next_eligible_skips_to_next_week_when_weekly_cadence_misses_window() {
        let schedule = Schedule {
            cadence_days: 7,
            ..Schedule::default()
        };
        let now = local_dt(2026, 5, 3, DEFAULT_WINDOW_HOUR_START, 0);
        let last_run_ms =
            now.timestamp_millis() - schedule.cadence_days * 86_400_000 + 2 * 60 * 60_000;
        let next = next_eligible_time_ms_at(schedule, last_run_ms, now);
        assert!(next >= now.timestamp_millis() + 6 * 86_400_000);
    }
}
