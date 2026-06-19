use serde::{Deserialize, Serialize};

// ── Data Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecFiling {
    pub id: i64,
    pub ticker: String,
    pub form_type: String,
    pub accession_number: String,
    pub filing_date: String,
    pub url: String,
    pub company_name: String,
    pub importance_score: i32,
    pub category: String,
    pub summary: String,
    pub insider_flag: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsiderTrade {
    pub id: i64,
    pub ticker: String,
    pub accession_number: String,
    pub insider_name: String,
    pub insider_title: String,
    pub transaction_date: String,
    pub transaction_type: String,
    pub shares: f64,
    pub price: f64,
    pub aggregate_value: f64,
    pub is_officer: bool,
    pub is_director: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilingAlert {
    pub id: i64,
    pub ticker: String,
    pub alert_type: String,
    pub message: String,
    pub filing_accession: String,
    pub importance: i32,
    pub created_at: i64,
    pub dismissed: bool,
    pub dismissed_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeStats {
    pub tickers_scanned: usize,
    pub new_filings: usize,
    pub new_insider_trades: usize,
    pub new_alerts: usize,
    pub errors: Vec<String>,
}

/// A filing parsed from the SEC JSON but not yet inserted.
#[derive(Debug, Clone)]
pub(super) struct PendingFiling {
    pub(super) ticker: String,
    pub(super) form_type: String,
    pub(super) accession_number: String,
    pub(super) filing_date: String,
    pub(super) url: String,
    pub(super) company_name: String,
    pub(super) importance_score: i32,
    pub(super) category: String,
    pub(super) insider_flag: bool,
    pub(super) is_late: bool,
}

#[derive(Debug, Clone)]
pub enum DiffChunk {
    Same(String),
    Added(String),
    Removed(String),
}

#[derive(Debug, Clone, Default)]
pub struct FilingSection {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Default)]
pub struct FilingSummary {
    /// Short one-line headline (e.g., "8-K — Item 2.02 Results of Operations").
    pub headline: String,
    /// Key bullets (2-8 entries), already trimmed for display.
    pub bullets: Vec<String>,
    /// Section extracts (title → body paragraph). Rendered collapsible in GUI.
    pub sections: Vec<FilingSection>,
}
