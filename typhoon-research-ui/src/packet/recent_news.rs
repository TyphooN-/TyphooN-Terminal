use std::fmt::Write as _;

/// Render recent news articles for a symbol. ADR-125 Phase 1 step 3 — a free
/// function over the engine `NewsArticle` slice (no `TyphooNApp`); the caller
/// fetches the articles from the DB.
pub fn write_symbol_recent_news_section(
    p: &mut String,
    articles: &[typhoon_engine::core::news::NewsArticle],
) {
    write_symbol_recent_news_section_with_impact(p, articles, &std::collections::HashMap::new());
}

/// Variant carrying `day_move_by_date` (YYYY-MM-DD → daily close-over-close
/// change %) so each headline shows the symbol's move on its publication day —
/// the ADR-116 "headline intraday impact" column. An empty map degrades to the
/// plain rendering.
pub fn write_symbol_recent_news_section_with_impact(
    p: &mut String,
    articles: &[typhoon_engine::core::news::NewsArticle],
    day_move_by_date: &std::collections::HashMap<String, f64>,
) {
    // Recent news (research_news + news crate — most relevant wins).
    // Bodies are included when the hydrator has fetched them so
    // the LLM has actual article text to ground its analysis,
    // not just headlines. Each body is truncated to ~1500
    // chars (lede + first few paragraphs) to keep the packet
    // token-budget under control even with 8 articles.
    const NEWS_BODY_CHAR_LIMIT: usize = 1500;
    if articles.is_empty() {
        return;
    }

    let bodies_present = articles.iter().filter(|a| !a.body.is_empty()).count();
    let _ = writeln!(
        p,
        "### Recent News ({} of {}, {} with full body)",
        articles.len(),
        articles.len(),
        bodies_present
    );
    for (idx, a) in articles.iter().enumerate() {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(a.published_at, 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "—".into());
        let sent = if a.sentiment.is_empty() {
            "—".to_string()
        } else {
            a.sentiment.clone()
        };
        let src = if a.provider.is_empty() {
            a.source.as_str()
        } else {
            a.provider.as_str()
        };
        let _ = writeln!(
            p,
            "**Article {}** — {} — {} — sentiment: {}",
            idx + 1,
            dt,
            src,
            sent
        );
        let _ = writeln!(p, "Headline: {}", a.headline);
        if let Some(mv) = day_move_by_date.get(&dt) {
            let _ = writeln!(p, "Day move: {:+.2}% (close vs prior close on {})", mv, dt);
        }
        if !a.url.is_empty() {
            let _ = writeln!(p, "URL: {}", a.url);
        }
        // Prefer body (hydrated full text) over summary
        // (provider-supplied blurb). When neither is
        // present the row degrades to a headline-only
        // entry, same as the previous table format.
        let text = if !a.body.is_empty() {
            a.body.as_str()
        } else {
            a.summary.as_str()
        };
        if !text.is_empty() {
            let label = if !a.body.is_empty() {
                "Body"
            } else {
                "Summary"
            };
            // Char-aware truncate so multi-byte UTF-8
            // (em-dashes, smart quotes, accented
            // letters) doesn't slice a code point.
            let truncated: String = if text.chars().count() > NEWS_BODY_CHAR_LIMIT {
                let mut buf = text.chars().take(NEWS_BODY_CHAR_LIMIT).collect::<String>();
                buf.push('…');
                buf
            } else {
                text.to_string()
            };
            let _ = writeln!(p, "{}: {}", label, truncated);
        }
        let _ = writeln!(p);
    }
}
