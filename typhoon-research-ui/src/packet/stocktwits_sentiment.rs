use super::context::SymbolResearchContext;
use std::fmt::Write as _;

pub fn write_symbol_stocktwits_sentiment_section(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    let Ok(Some(snapshot)) =
        typhoon_engine::core::research::get_stocktwits_sentiment(ctx.conn, sym_upper)
    else {
        return;
    };
    if snapshot.message_count == 0 {
        return;
    }

    let _ = writeln!(
        p,
        "### Social Sentiment — StockTwits ({}, as of {})",
        sym_upper, snapshot.fetched_at
    );
    let _ = writeln!(
        p,
        "- Messages: {} | Bullish: {} | Bearish: {} | Neutral: {} | Bull/Bear: {:.2} | 24h velocity: {}",
        snapshot.message_count,
        snapshot.bullish,
        snapshot.bearish,
        snapshot.neutral,
        snapshot.bull_bear_ratio,
        snapshot.velocity_24h
    );
    let _ = writeln!(p);
    let _ = writeln!(
        p,
        "| Time | User | Sentiment | Likes | Reshares | Message |"
    );
    let _ = writeln!(p, "|---|---:|---:|---:|---:|---|");
    for msg in snapshot.top_messages.iter().take(5) {
        let body = msg.body.replace('|', "\\|").replace('\n', " ");
        let _ = writeln!(
            p,
            "| {} | {} | {} | {} | {} | {} |",
            msg.created_at, msg.username, msg.sentiment, msg.like_count, msg.reshare_count, body
        );
    }
    let _ = writeln!(p);
}
