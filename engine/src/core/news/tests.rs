use super::*;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory");
    create_news_tables(&conn).expect("create tables");
    conn
}

#[test]
fn hash_is_stable_and_lowercases_url() {
    let a = NewsArticle::compute_hash("https://Example.com/News/Article?Id=1");
    let b = NewsArticle::compute_hash("HTTPS://EXAMPLE.COM/NEWS/ARTICLE?ID=1");
    assert_eq!(a, b);
    assert_eq!(a.len(), 64);
}

#[test]
fn with_hash_populates_hash_field() {
    let a = NewsArticle {
        url: "https://example.com/a".into(),
        ..Default::default()
    }
    .with_hash();
    assert!(!a.url_hash.is_empty());
}

#[test]
fn upsert_and_get_roundtrip() {
    let conn = mem_conn();
    let article = NewsArticle {
        symbol: "AAPL".into(),
        source: "FMP".into(),
        headline: "Apple reports record Q4".into(),
        summary: "AAPL beat estimates...".into(),
        url: "https://example.com/apple-q4".into(),
        published_at: 1_700_000_000,
        sentiment: "bullish".into(),
        sentiment_score: 0.7,
        tickers: vec!["AAPL".into()],
        ..Default::default()
    }
    .with_hash();

    upsert_news(&conn, &article).unwrap();
    let got = get_news_by_symbol(&conn, "AAPL", 10).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].headline, "Apple reports record Q4");
    assert_eq!(got[0].sentiment, "bullish");
}

#[test]
fn upsert_dedup_by_url_hash() {
    let conn = mem_conn();
    let mut a = NewsArticle {
        symbol: "MSFT".into(),
        source: "GDELT".into(),
        headline: "Original headline".into(),
        url: "https://example.com/msft".into(),
        published_at: 1,
        ..Default::default()
    }
    .with_hash();
    upsert_news(&conn, &a).unwrap();

    // Update with new headline, same URL — should merge.
    a.headline = "Updated headline".into();
    upsert_news(&conn, &a).unwrap();

    let rows = get_news_by_symbol(&conn, "MSFT", 10).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].headline, "Updated headline");
}

#[test]
fn fts_search_matches_headline_and_summary() {
    let conn = mem_conn();
    let a = NewsArticle {
        symbol: "TSLA".into(),
        source: "FMP".into(),
        headline: "Tesla beats delivery target".into(),
        summary: "EV maker delivered record number of vehicles.".into(),
        url: "https://example.com/tesla".into(),
        published_at: 1_700_000_000,
        ..Default::default()
    }
    .with_hash();
    upsert_news(&conn, &a).unwrap();

    let hit = search_news(&conn, "delivery", 10).unwrap();
    assert_eq!(hit.len(), 1);

    let hit2 = search_news(&conn, "vehicles", 10).unwrap();
    assert_eq!(hit2.len(), 1);

    let miss = search_news(&conn, "zebra", 10).unwrap();
    assert_eq!(miss.len(), 0);
}

#[test]
fn cached_news_queries_hide_sec_filings() {
    let conn = mem_conn();
    let filing = NewsArticle {
        symbol: "AAPL".into(),
        source: "SEC".into(),
        headline: "10-Q filed".into(),
        summary: "Quarterly report".into(),
        url: "https://sec.gov/aapl-10q".into(),
        published_at: 200,
        ..Default::default()
    }
    .with_hash();
    let story = NewsArticle {
        symbol: "AAPL".into(),
        source: "YahooRSS".into(),
        headline: "Apple rallies on product news".into(),
        summary: "Market story".into(),
        url: "https://example.com/aapl-news".into(),
        published_at: 100,
        ..Default::default()
    }
    .with_hash();
    upsert_news(&conn, &filing).unwrap();
    upsert_news(&conn, &story).unwrap();

    let by_symbol = get_news_by_symbol(&conn, "AAPL", 10).unwrap();
    assert_eq!(by_symbol.len(), 1);
    assert_eq!(by_symbol[0].source, "YahooRSS");

    let all = get_news_by_symbol(&conn, "", 10).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].source, "YahooRSS");

    let filing_search = search_news(&conn, "Quarterly", 10).unwrap();
    assert!(filing_search.is_empty());
}

#[test]
fn news_scrape_index_gates_repeated_fetches() {
    let conn = mem_conn();
    assert!(!news_cache_is_fresh(&conn, "AAPL", 30 * 60, 1).unwrap());

    let article = NewsArticle {
        symbol: "AAPL".into(),
        source: "YahooRSS".into(),
        headline: "Apple product news".into(),
        url: "https://example.com/aapl-product".into(),
        published_at: 1_700_000_000,
        ..Default::default()
    }
    .with_hash();
    upsert_news(&conn, &article).unwrap();
    assert_eq!(mark_news_scraped(&conn, "AAPL").unwrap(), 1);
    assert!(news_cache_is_fresh(&conn, "aapl", 30 * 60, 1).unwrap());
    assert!(!news_cache_is_fresh(&conn, "aapl", 30 * 60, 2).unwrap());
    let fresh = fresh_news_symbols(&conn, &["aapl".into(), "MSFT".into()], 30 * 60, 1).unwrap();
    assert!(fresh.contains("AAPL"));
    assert!(!fresh.contains("MSFT"));
}

#[test]
fn purge_removes_old_articles() {
    let conn = mem_conn();
    let old = NewsArticle {
        symbol: "A".into(),
        url: "https://example.com/old".into(),
        published_at: 100,
        ..Default::default()
    }
    .with_hash();
    let fresh = NewsArticle {
        symbol: "A".into(),
        url: "https://example.com/new".into(),
        published_at: 999_999_999,
        ..Default::default()
    }
    .with_hash();
    upsert_news(&conn, &old).unwrap();
    upsert_news(&conn, &fresh).unwrap();

    let removed = purge_older_than(&conn, 1000).unwrap();
    assert_eq!(removed, 1);
    let remaining = get_news_by_symbol(&conn, "A", 10).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].url, "https://example.com/new");
}

#[test]
fn parse_gdelt_ts_valid() {
    let t = parse_gdelt_ts("20260413T142030Z");
    assert!(t > 1_700_000_000);
}

#[test]
fn parse_av_ts_valid() {
    let t = parse_av_ts("20260413T142030");
    assert!(t > 1_700_000_000);
}

#[test]
fn parse_iso_ts_variants() {
    assert!(parse_iso_ts("2026-04-13T14:20:30Z") > 0);
    assert!(parse_iso_ts("2026-04-13 14:20:30") > 0);
    assert_eq!(parse_iso_ts(""), 0);
}

#[test]
fn strip_html_removes_tags_and_decodes_entities() {
    let s = strip_html("<a href='x'>Hello</a> &amp; <b>world</b>");
    assert_eq!(s, "Hello & world");
}

#[test]
fn rss_item_parser_extracts_fields() {
    let rss = r#"
        <rss><channel>
            <item>
                <title><![CDATA[Apple rallies on earnings]]></title>
                <link>https://example.com/apple</link>
                <description>Apple beat expectations...</description>
                <pubDate>Mon, 13 Apr 2026 14:20:30 GMT</pubDate>
            </item>
            <item>
                <title>Microsoft cloud growth</title>
                <link>https://example.com/msft</link>
                <description>Azure posted 30% growth</description>
                <pubDate>Mon, 13 Apr 2026 10:00:00 GMT</pubDate>
            </item>
        </channel></rss>
        "#;
    let items = parse_rss_items(rss, "AAPL", "YahooRSS");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].headline, "Apple rallies on earnings");
    assert_eq!(items[0].url, "https://example.com/apple");
    assert_eq!(items[1].headline, "Microsoft cloud growth");
}

#[test]
fn atom_parser_extracts_link_from_href() {
    let atom = r#"
        <feed>
            <entry>
                <title>10-Q filed</title>
                <link href="https://sec.gov/a.htm" rel="alternate"/>
                <summary>Quarterly report</summary>
                <updated>2026-04-13T14:20:30Z</updated>
            </entry>
        </feed>
        "#;
    let items = parse_atom_items(atom, "AAPL", "SEC");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].url, "https://sec.gov/a.htm");
    assert_eq!(items[0].headline, "10-Q filed");
}

#[test]
fn is_us_symbol_heuristic() {
    assert!(is_us_symbol("AAPL"));
    assert!(is_us_symbol("T"));
    assert!(is_us_symbol("BRK.A"));
    assert!(!is_us_symbol(""));
    assert!(!is_us_symbol("EURUSD"));
    assert!(!is_us_symbol("BTC/USD"));
}

#[test]
fn crypto_base_for_symbol_recognises_pair_forms() {
    assert_eq!(crypto_base_for_symbol("BTC/USD").as_deref(), Some("BTC"));
    assert_eq!(crypto_base_for_symbol("eth-usd").as_deref(), Some("ETH"));
    assert_eq!(crypto_base_for_symbol("SOLUSDT").as_deref(), Some("SOL"));
    assert_eq!(crypto_base_for_symbol("BTC").as_deref(), Some("BTC"));
    // lowercase still works
    assert_eq!(crypto_base_for_symbol("doge/usd").as_deref(), Some("DOGE"));
}

#[test]
fn crypto_base_for_symbol_rejects_equities() {
    // Equity tickers that happen to overlap a coin format must not match.
    assert!(crypto_base_for_symbol("AAPL").is_none());
    assert!(crypto_base_for_symbol("SPY").is_none());
    assert!(crypto_base_for_symbol("BRK.A").is_none());
    assert!(crypto_base_for_symbol("").is_none());
}

#[test]
fn article_mentions_crypto_matches_ticker_or_name() {
    assert!(article_mentions_crypto("BTC pumps 5%", "", "BTC"));
    assert!(article_mentions_crypto(
        "Bitcoin hits new ATH",
        "spot inflows surge",
        "BTC"
    ));
    assert!(!article_mentions_crypto("Apple beats earnings", "", "BTC"));
}

#[test]
fn get_news_empty_symbol_returns_all() {
    let conn = mem_conn();
    let a1 = NewsArticle {
        symbol: "A".into(),
        url: "https://example.com/1".into(),
        published_at: 100,
        headline: "h1".into(),
        ..Default::default()
    }
    .with_hash();
    let a2 = NewsArticle {
        symbol: "B".into(),
        url: "https://example.com/2".into(),
        published_at: 200,
        headline: "h2".into(),
        ..Default::default()
    }
    .with_hash();
    upsert_news(&conn, &a1).unwrap();
    upsert_news(&conn, &a2).unwrap();
    let all = get_news_by_symbol(&conn, "", 10).unwrap();
    assert_eq!(all.len(), 2);
    // Descending by published_at
    assert_eq!(all[0].headline, "h2");
}

#[test]
fn upsert_batch_counts_rows() {
    let conn = mem_conn();
    let articles: Vec<NewsArticle> = (0..5)
        .map(|i| {
            NewsArticle {
                symbol: "X".into(),
                url: format!("https://example.com/{i}"),
                published_at: 1000 + i,
                ..Default::default()
            }
            .with_hash()
        })
        .collect();
    let n = upsert_news_batch(&conn, &articles).unwrap();
    assert_eq!(n, 5);
}

#[test]
fn extract_article_prefers_caas_body_over_page_chrome() {
    // Yahoo Finance-style page with a left nav full of menu links and
    // a `<div class="caas-body">` containing the actual article. The
    // extractor must return the article body, not the nav text.
    let html = r#"<html><head>
            <meta property="og:image" content="https://yimg.com/hero.jpg">
            </head><body>
            <nav>
                <ul>
                    <li>Today's news</li><li>US</li><li>Politics</li>
                    <li>World</li><li>Weather</li><li>Climate change</li>
                    <li>Health</li><li>Science</li><li>Originals</li>
                    <li>Newsletters</li><li>Games</li><li>Life</li>
                </ul>
            </nav>
            <header><h1>Yahoo Finance</h1></header>
            <div class="caas-body">
                <p>WORK Medical Technology Group Ltd. (NASDAQ: WOK) shares are
                trending on Wednesday.</p>
                <p>WOK shares spiked 69.67% to $11.30 in after-hours trading
                on Tuesday after the Hangzhou-based medical device supplier
                disclosed a strategic cooperation agreement with Shanghai
                Novabioplus Biotechnology Co., Ltd.</p>
                <p>The deal is centered on a "BioToken" framework.</p>
            </div>
            <footer>Copyright Yahoo</footer>
            <aside class="related">Related articles: foo, bar, baz</aside>
        </body></html>"#;
    let (body, image) = extract_article_with_image(html);
    // The article body must be present.
    assert!(body.contains("WORK Medical Technology"));
    assert!(body.contains("BioToken"));
    // The nav menu, header, footer, and related-articles aside must
    // be stripped — the old extractor kept them all.
    assert!(!body.contains("Today's news"));
    assert!(!body.contains("Politics"));
    assert!(!body.contains("Copyright Yahoo"));
    assert!(!body.contains("Related articles"));
    // og:image meta tag should be picked up.
    assert_eq!(image, "https://yimg.com/hero.jpg");
}

#[test]
fn extract_article_picks_semantic_article_tag() {
    let html = r#"<html><body>
            <nav><a href="/">Home</a><a href="/news">News</a></nav>
            <article>
                <h1>Breaking</h1>
                <p>The first paragraph of a thousand chars at minimum so we
                cross the MIN threshold. ABCDEFGHIJKLMNOPQRSTUVWXYZ
                ABCDEFGHIJKLMNOPQRSTUVWXYZ ABCDEFGHIJKLMNOPQRSTUVWXYZ
                ABCDEFGHIJKLMNOPQRSTUVWXYZ ABCDEFGHIJKLMNOPQRSTUVWXYZ
                ABCDEFGHIJKLMNOPQRSTUVWXYZ ABCDEFGHIJKLMNOPQRSTUVWXYZ.</p>
            </article>
            <footer>Site footer</footer>
        </body></html>"#;
    let (body, _) = extract_article_with_image(html);
    assert!(body.contains("Breaking"));
    assert!(body.contains("first paragraph"));
    assert!(!body.contains("Home"));
    assert!(!body.contains("Site footer"));
}

#[test]
fn extract_article_falls_back_to_body_when_no_container() {
    // No <article>, no .caas-body, etc. — the extractor falls back to
    // <body> but still drops the nav/header/footer chrome thanks to
    // the drop-selector pass.
    let html = r#"<html><body>
            <nav>Top nav with lots of menu items everywhere</nav>
            <header>Page header that should not appear</header>
            <div class="content-wrapper">
                <p>This is the article content rendered in a generic div
                wrapper that the extractor's selector list doesn't
                explicitly know about, so the fallback path runs and at
                least skips the obvious chrome around it ABCDEFGHIJKLMN
                ABCDEFGHIJKLMN ABCDEFGHIJKLMN ABCDEFGHIJKLMN.</p>
            </div>
            <footer>Page footer that should also not appear</footer>
        </body></html>"#;
    let (body, _) = extract_article_with_image(html);
    assert!(body.contains("article content"));
    assert!(!body.contains("Top nav"));
    assert!(!body.contains("Page header"));
    assert!(!body.contains("Page footer"));
}

#[test]
fn extract_article_drops_script_and_style_blocks() {
    let html = r#"<html><body>
            <article>
                <script>alert('xss');</script>
                <style>body { display: none; }</style>
                <p>Real paragraph text that is long enough to clear the
                 MIN threshold so the article selector wins. Padding
                 padding padding padding padding padding padding padding
                 padding padding padding padding padding padding.</p>
            </article>
        </body></html>"#;
    let (body, _) = extract_article_with_image(html);
    assert!(body.contains("Real paragraph"));
    assert!(!body.contains("alert"));
    assert!(!body.contains("display: none"));
}

#[test]
fn extract_image_uses_twitter_card_fallback() {
    let html = r#"<html><head>
            <meta name="twitter:image" content="https://cdn.example.com/twitter.jpg">
            </head><body><p>Hi.</p></body></html>"#;
    let (_, image) = extract_article_with_image(html);
    assert_eq!(image, "https://cdn.example.com/twitter.jpg");
}

#[test]
fn extract_image_ignores_non_absolute_urls() {
    // Relative or javascript: URIs must not be returned as a hero image.
    let html = r#"<html><head>
            <meta property="og:image" content="/local/image.jpg">
            <meta name="twitter:image" content="javascript:alert(1)">
            </head><body><p>Hi.</p></body></html>"#;
    let (_, image) = extract_article_with_image(html);
    assert_eq!(image, "");
}

#[test]
fn normalize_headline_strips_publisher_pipe_suffix() {
    let h1 = "Dads club Colchester says it is the antidote to manosphere";
    let h2 =
        "Dads club Colchester says it is the antidote to manosphere | Clacton and Frinton Gazette";
    let h3 =
        "Dads club Colchester says it is the antidote to manosphere | Maldon and Burnham Standard";
    let h4 = "Dads club Colchester says it is the antidote to manosphere - Halstead Gazette";
    let n1 = normalize_headline_for_dedup(h1);
    let n2 = normalize_headline_for_dedup(h2);
    let n3 = normalize_headline_for_dedup(h3);
    let n4 = normalize_headline_for_dedup(h4);
    assert_eq!(n1, n2);
    assert_eq!(n2, n3);
    assert_eq!(n3, n4);
    assert!(n1.contains("dads club colchester"));
    assert!(!n1.contains("gazette"));
    assert!(!n1.contains("standard"));
}

#[test]
fn normalize_headline_preserves_short_titles_with_pipes() {
    // Don't decapitate "Apple | Q3 earnings" — the prefix is too
    // short to be a publisher name.
    let h = "Apple | Q3 earnings";
    let n = normalize_headline_for_dedup(h);
    assert_eq!(n, "apple | q3 earnings");
}

#[test]
fn group_articles_collapses_same_story_across_sources() {
    let mk = |url: &str, headline: &str, ts: i64| {
        NewsArticle {
            url: url.into(),
            headline: headline.into(),
            published_at: ts,
            ..Default::default()
        }
        .with_hash()
    };
    let articles = vec![
        mk("u1", "Dads club Colchester | A Gazette", 100),
        mk("u2", "Apple beats Q3 estimates", 200),
        mk("u3", "Dads club Colchester | B Standard", 110),
        mk("u4", "Dads club Colchester - C News", 120),
        mk("u5", "Tesla rises", 300),
    ];
    let groups = group_articles_by_headline(&articles);
    // 3 distinct stories: dads (3 sources), apple, tesla
    assert_eq!(groups.len(), 3);
    // Find the dads group — primary should be the newest (ts=120, idx=3)
    let dads = groups
        .iter()
        .find(|(p, _)| articles[*p].headline.contains("Dads"))
        .expect("dads group present");
    assert_eq!(dads.0, 3, "primary should be newest (idx 3)");
    assert_eq!(dads.1.len(), 2, "two alternates for the dads story");
}

#[test]
fn count_all_articles_returns_total() {
    let conn = mem_conn();
    assert_eq!(count_all_articles(&conn).unwrap(), 0);
    for i in 0..5 {
        let a = NewsArticle {
            symbol: "AAPL".into(),
            url: format!("https://example.com/{i}"),
            published_at: 1_700_000_000 + i,
            ..Default::default()
        }
        .with_hash();
        upsert_news(&conn, &a).unwrap();
    }
    assert_eq!(count_all_articles(&conn).unwrap(), 5);
}

#[test]
fn search_news_falls_back_for_comma_separated_terms() {
    let conn = mem_conn();
    let a = NewsArticle {
        symbol: "TNDM".into(),
        headline: "TNDM Tandem Diabetes reports results".into(),
        summary: "Insulin pump maker update".into(),
        url: "https://example.com/tndm".into(),
        published_at: 1_700_000_000,
        tickers: vec!["TNDM".into()],
        ..Default::default()
    }
    .with_hash();
    upsert_news(&conn, &a).unwrap();

    let rows = search_news(&conn, "TNDM, GDC", 10).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].symbol, "TNDM");
}

#[test]
fn group_articles_preserves_singletons() {
    let mk = |url: &str, headline: &str| {
        NewsArticle {
            url: url.into(),
            headline: headline.into(),
            ..Default::default()
        }
        .with_hash()
    };
    let articles = vec![mk("u1", "Unique A"), mk("u2", "Unique B")];
    let groups = group_articles_by_headline(&articles);
    assert_eq!(groups.len(), 2);
    for (_, alts) in &groups {
        assert!(alts.is_empty());
    }
}

#[test]
fn count_older_than_matches_purge_count() {
    let conn = mem_conn();
    let now = chrono::Utc::now().timestamp();
    // 5 articles spanning a year of ages.
    let ages_days: [i64; 5] = [1, 30, 100, 200, 400];
    for (i, age) in ages_days.iter().enumerate() {
        let a = NewsArticle {
            symbol: "AAPL".into(),
            url: format!("https://example.com/a{i}"),
            published_at: now - age * 86_400,
            ..Default::default()
        }
        .with_hash();
        upsert_news(&conn, &a).unwrap();
    }
    // Older than 90 days → ages 100, 200, 400 = 3 articles.
    let cutoff_90 = now - 90 * 86_400;
    assert_eq!(count_articles_older_than(&conn, cutoff_90).unwrap(), 3);
    // Older than 365 days → only the 400-day-old one.
    let cutoff_365 = now - 365 * 86_400;
    assert_eq!(count_articles_older_than(&conn, cutoff_365).unwrap(), 1);
    // Older than 1000 days → none.
    let cutoff_1000 = now - 1000 * 86_400;
    assert_eq!(count_articles_older_than(&conn, cutoff_1000).unwrap(), 0);
    // After purging at cutoff_90, the count should match what purge
    // reported, and a fresh count_older_than should return 0.
    let purged = purge_older_than(&conn, cutoff_90).unwrap();
    assert_eq!(purged, 3);
    assert_eq!(count_articles_older_than(&conn, cutoff_90).unwrap(), 0);
}
