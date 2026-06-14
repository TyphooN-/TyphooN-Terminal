use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_investigation_question_and_return_path(
        &self,
        p: &mut String,
        user_question: &str,
    ) {
        use std::fmt::Write as _;
        // Closing question
        let _ = writeln!(p, "---");
        let _ = writeln!(p, "## Question");
        if user_question.trim().is_empty() {
            let _ = writeln!(
                p,
                "Using only the data above, write a concise investment research note on \
                each symbol covering: (1) valuation vs sector peers, (2) financial trajectory from the \
                quarterly data, (3) balance-sheet / solvency notes, (4) SEC filing activity and insider \
                sentiment, (5) volatility regime and risk profile, and (6) a neutral-to-directional \
                takeaway. Flag any data gaps you'd want filled in to refine the view."
            );
        } else {
            let _ = writeln!(p, "{}", user_question.trim());
        }

        // ── Return Path: instruct the agent to emit a structured
        //    ingest block so TyphooN can absorb the web-search findings
        //    back into the local cache. ──
        let _ = writeln!(p);
        let _ = writeln!(p, "---");
        let _ = writeln!(p, "## Return Path — Web Research Ingest");
        let _ = writeln!(p);
        let _ = writeln!(
            p,
            "If you consulted any web sources to answer the above, **please emit a \
            fenced ingest block at the very end of your reply** so TyphooN-Terminal can cache \
            your findings. Use this exact format:"
        );
        let _ = writeln!(p);
        let _ = writeln!(p, "```");
        let _ = writeln!(p, "===TYPHOON_INGEST===");
        let _ = writeln!(p, "[");
        let _ = writeln!(
            p,
            "  {{\"symbol\": \"TICKER\", \"title\": \"article headline\", \"url\": \"https://...\","
        );
        let _ = writeln!(
            p,
            "   \"source\": \"Reuters|Bloomberg|WSJ|...\", \"published_at\": \"YYYY-MM-DD\","
        );
        let _ = writeln!(
            p,
            "   \"summary\": \"2-3 sentence takeaway\", \"agent\": \"claude|gemini|chatgpt|...\","
        );
        let _ = writeln!(
            p,
            "   \"body\": \"full article text if you actually fetched the source (optional)\"}},"
        );
        let _ = writeln!(p, "  ...");
        let _ = writeln!(p, "]");
        let _ = writeln!(p, "===END_INGEST===");
        let _ = writeln!(p, "```");
        let _ = writeln!(p);
        let _ = writeln!(
            p,
            "Rules: (1) one object per distinct article, (2) include every symbol from \
            the research packet that the article references, (3) each article may appear once per symbol \
            (dedup by URL is handled on ingest), (4) the `summary` field should be YOUR synthesis, not a \
            raw copy-paste, (5) the `body` field is optional — populate it with the full article text \
            ONLY when you actually fetched and read the source (e.g. via web_search/browse), since \
            TyphooN-Terminal can already hydrate bodies for the public web; the value of `body` here is \
            for paywalled or hard-to-fetch content where the terminal's own fetcher would 4xx, (6) \
            missing fields are OK — the parser will skip entries without a symbol or url but keep the rest."
        );
    }
}
