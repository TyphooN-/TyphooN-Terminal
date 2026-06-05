/// Client-side filter parsed from the News window's Search field. The
/// search text is interpreted in three modes, auto-detected from its
/// content, so the user doesn't need a separate "Symbol:" input or a
/// mode toggle:
///
///   * empty                  → [`SearchFilterMode::All`] — show every cached article
///   * `/<pattern>/`          → [`SearchFilterMode::Regex`] — case-insensitive headline match
///   * `TNDM, GDC, CC` (CSV)  → [`SearchFilterMode::Symbols`] — match article.symbol OR any ticker
///
/// The FTS broker keyword search stays available via the dedicated "FTS
/// Search" button next to the field so a user can still ask for the
/// literal substring "TNDM" in body text rather than tagging.
pub(super) enum SearchFilterMode {
    All,
    Symbols(Vec<String>),
    Regex {
        pattern: String,
        compiled: regex::Regex,
    },
    InvalidRegex(String),
}

impl SearchFilterMode {
    pub(super) fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Self::All;
        }
        // Regex mode: `/pattern/` (case-insensitive headline match).
        // Strip the slashes, compile, fall back to InvalidRegex when the
        // pattern itself is malformed so the UI can show why the filter
        // isn't biting instead of silently returning everything.
        if trimmed.starts_with('/') && trimmed.ends_with('/') && trimmed.len() >= 3 {
            let pattern = trimmed[1..trimmed.len() - 1].to_string();
            return match regex::RegexBuilder::new(&pattern)
                .case_insensitive(true)
                .build()
            {
                Ok(compiled) => Self::Regex { pattern, compiled },
                Err(e) => Self::InvalidRegex(e.to_string()),
            };
        }
        // Symbol CSV mode: comma-separated tokens that look like tickers.
        // A "ticker-shaped" token is short (≤6 chars), alphanumeric +
        // dot/dash/slash (covers BRK.B, TSLA, BTC/USD, etc.). If every
        // comma-separated token passes the shape check, treat the whole
        // thing as a symbol list; otherwise fall through to FTS keyword
        // semantics (which the FTS Search button explicitly invokes).
        let candidates: Vec<&str> = trimmed
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !candidates.is_empty()
            && candidates.iter().all(|c| {
                c.len() <= 8
                    && c.chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '/')
            })
        {
            let syms: Vec<String> = candidates.iter().map(|s| s.to_ascii_uppercase()).collect();
            return Self::Symbols(syms);
        }
        // Bare keyword that isn't a symbol-CSV or regex: show all and
        // wait for the user to press the FTS Search button (which
        // submits to the broker). Falling back to All here means the
        // user typing a partial keyword sees the full list rather than
        // an empty one — and the FTS button is right next to the field.
        Self::All
    }

    /// True when the given article passes this filter.
    pub(super) fn matches(&self, a: &typhoon_engine::core::news::NewsArticle) -> bool {
        match self {
            Self::All | Self::InvalidRegex(_) => true,
            Self::Symbols(syms) => {
                // Match article.symbol OR any tagged ticker, case-
                // insensitive. The article symbol is already upper in
                // most providers; we normalise both sides to be safe.
                let primary = a.symbol.trim().to_ascii_uppercase();
                if !primary.is_empty() && syms.iter().any(|s| s == &primary) {
                    return true;
                }
                for t in &a.tickers {
                    let t_up = t.trim().to_ascii_uppercase();
                    if !t_up.is_empty() && syms.iter().any(|s| s == &t_up) {
                        return true;
                    }
                }
                false
            }
            Self::Regex { compiled, .. } => compiled.is_match(&a.headline),
        }
    }
}
