/// All SEC filing types we track — comprehensive coverage for trading signals.
pub(super) const RELEVANT_FORMS: &[&str] = &[
    // Core financials
    "10-K", "10-Q", "20-F", "20-F/A", "8-K", // Amended (restated = red flag)
    "10-K/A", "10-Q/A", "8-K/A", // Late filing (distress signal)
    "NT 10-K", "NT 10-Q", // Insider trades
    "4", "3", "5", "144", // Proxy/governance
    "DEF 14A", "DEFA14A", "PREM14A",
    // Shareholder disclosures (activist/institutional)
    "SC 13D", "SC 13D/A", "SC 13G", "SC 13G/A", "13F-HR", // Offerings/dilution / registrations
    "S-1", "S-3", "S-4", "S-8", "424B5", "424B2",
    "424B4", // Foreign issuer / specialized reports
    "6-K", "SD", // M&A
    "SC TO-T", "SC TO-I", "SC 14D9", // Deregistration (delisting risk)
    "15-12B", "15-12G",  // SEC scrutiny
    "CORRESP", // Employee plans
    "11-K",
];

// ── Importance Scoring ──────────────────────────────────────────────

pub fn compute_importance(form_type: &str, is_insider_sell: bool, _is_late: bool) -> i32 {
    let (base, _cat) = importance_and_category(form_type);
    let mut score = base;
    if is_insider_sell {
        score += 15;
    }
    score.min(100)
}

/// Returns (importance_score, category) for a form type.
fn importance_and_category(form_type: &str) -> (i32, &'static str) {
    match form_type {
        "15-12B" | "15-12G" => (85, "DELISTING"),
        "SC TO-T" | "SC TO-I" | "SC 14D9" => (80, "ACQUISITION"),
        "10-K/A" | "10-Q/A" | "8-K/A" => (75, "AMENDED"),
        "NT 10-K" | "NT 10-Q" => (75, "LATE_FILING"),
        "SC 13D" | "SC 13D/A" => (70, "ACTIVIST"),
        "PREM14A" => (70, "ACQUISITION"),
        "424B5" | "424B2" | "424B4" => (65, "DILUTION"),
        "S-3" => (60, "DILUTION"),
        "CORRESP" => (45, "SEC_SCRUTINY"),
        "10-K" | "20-F" => (40, "EARNINGS"),
        "S-1" | "S-4" => (40, "OFFERING"),
        "8-K" => (35, "MATERIAL_EVENT"),
        "SC 13G" | "SC 13G/A" => (35, "INSTITUTIONAL"),
        "DEFA14A" => (35, "GOVERNANCE"),
        "10-Q" => (30, "EARNINGS"),
        "13F-HR" => (30, "INSTITUTIONAL"),
        "4" => (25, "INSIDER_ACTIVITY"),
        "3" | "5" => (20, "INSIDER_ACTIVITY"),
        "DEF 14A" => (20, "GOVERNANCE"),
        "11-K" => (15, "GOVERNANCE"),
        _ => (10, "OTHER"),
    }
}

pub(super) fn categorize_form(form_type: &str) -> &'static str {
    importance_and_category(form_type).1
}
