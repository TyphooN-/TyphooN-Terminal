use super::context::SymbolResearchContext;
use std::fmt::Write as _;

/// ADR-117 Reddit lane: keyless mention counts + engagement across the
/// finance subreddits, with post provenance. Deliberately no derived
/// buy/sell signal — Reddit has no user-tagged sentiment.
pub fn write_symbol_reddit_mentions_section(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    let Ok(Some(snapshot)) =
        typhoon_engine::core::research::get_reddit_mentions(ctx.conn, sym_upper)
    else {
        return;
    };
    if snapshot.mentions_24h == 0 {
        return;
    }

    let _ = writeln!(
        p,
        "### Social Sentiment — Reddit mentions ({}, as of {})",
        sym_upper, snapshot.fetched_at
    );
    let _ = writeln!(
        p,
        "- Mentions (24h, r/wallstreetbets + r/stocks + r/investing + r/StockMarket): {} | Score Σ: {} | Comments Σ: {}",
        snapshot.mentions_24h, snapshot.score_sum_24h, snapshot.comments_sum_24h
    );
    let _ = writeln!(
        p,
        "- Raw counts + provenance only; Reddit carries no bull/bear tags."
    );
    let _ = writeln!(p);
    let _ = writeln!(p, "| Subreddit | Score | Comments | Title |");
    let _ = writeln!(p, "|---|---:|---:|---|");
    for post in snapshot.top_posts.iter().take(5) {
        let title = post.title.replace('|', "\\|").replace('\n', " ");
        let _ = writeln!(
            p,
            "| r/{} | {} | {} | {} |",
            post.subreddit, post.score, post.num_comments, title
        );
    }
    let _ = writeln!(p);
}
