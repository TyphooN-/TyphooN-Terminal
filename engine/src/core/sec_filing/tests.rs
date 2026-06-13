use super::*;
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    create_sec_tables(&conn).unwrap();
    conn
}

fn insert_filing(conn: &Connection, ticker: &str, form_type: &str, accession: &str, date: &str) {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
            "INSERT INTO sec_filings (ticker, form_type, accession_number, filing_date, url, company_name, importance_score, category, summary, insider_flag, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![ticker, form_type, accession, date,
                    format!("https://sec.gov/test/{accession}"), "Test Corp",
                    compute_importance(form_type, false, false),
                    categorize_form(form_type), "", form_type == "4", now],
        ).unwrap();
}

fn insert_insider_trade(
    conn: &Connection,
    ticker: &str,
    accession: &str,
    name: &str,
    txn_type: &str,
    date: &str,
    shares: f64,
    price: f64,
) {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
            "INSERT INTO sec_insider_trades (ticker, accession_number, insider_name, insider_title, transaction_date, transaction_type, shares, price, aggregate_value, is_officer, is_director, created_at)
             VALUES (?1, ?2, ?3, 'CEO', ?4, ?5, ?6, ?7, ?8, TRUE, FALSE, ?9)",
            params![ticker, accession, name, date, txn_type, shares, price, shares * price, now],
        ).unwrap();
}

fn insert_alert(
    conn: &Connection,
    ticker: &str,
    alert_type: &str,
    message: &str,
    dismissed: bool,
) -> i64 {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
            "INSERT INTO sec_filing_alerts (ticker, alert_type, message, filing_accession, importance, created_at, dismissed, dismissed_reason)
             VALUES (?1, ?2, ?3, 'acc-001', 50, ?4, ?5, '')",
            params![ticker, alert_type, message, now, dismissed],
        ).unwrap();
    conn.last_insert_rowid()
}

// ── create_sec_tables ──────────────────────────────────────────

#[test]
fn create_tables_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    create_sec_tables(&conn).unwrap();
    create_sec_tables(&conn).unwrap(); // second call should not fail
}

// ── compute_importance ─────────────────────────────────────────

#[test]
fn compute_importance_base_scores() {
    assert_eq!(compute_importance("10-K", false, false), 40);
    assert_eq!(compute_importance("10-Q", false, false), 30);
    assert_eq!(compute_importance("8-K", false, false), 35);
    assert_eq!(compute_importance("4", false, false), 25);
}

#[test]
fn compute_importance_insider_sell_boost() {
    let base = compute_importance("4", false, false);
    let with_sell = compute_importance("4", true, false);
    assert_eq!(with_sell, base + 15);
}

#[test]
fn compute_importance_capped_at_100() {
    // 15-12B has base 85, + 15 for insider sell = 100 (capped)
    assert_eq!(compute_importance("15-12B", true, false), 100);
}

#[test]
fn compute_importance_unknown_form() {
    assert_eq!(compute_importance("UNKNOWN-FORM", false, false), 10);
}

// ── importance_and_category ────────────────────────────────────

#[test]
fn categorize_form_categories() {
    assert_eq!(categorize_form("10-K"), "EARNINGS");
    assert_eq!(categorize_form("10-Q"), "EARNINGS");
    assert_eq!(categorize_form("SC 13D"), "ACTIVIST");
    assert_eq!(categorize_form("15-12B"), "DELISTING");
    assert_eq!(categorize_form("4"), "INSIDER_ACTIVITY");
    assert_eq!(categorize_form("S-3"), "DILUTION");
    assert_eq!(categorize_form("CORRESP"), "SEC_SCRUTINY");
    assert_eq!(categorize_form("RANDOM"), "OTHER");
}

// ── is_equity_symbol ───────────────────────────────────────────

#[test]
fn is_equity_symbol_valid() {
    assert!(is_equity_symbol("AAPL"));
    assert!(is_equity_symbol("MSFT"));
    assert!(is_equity_symbol("GOOG"));
    assert!(is_equity_symbol("A")); // single letter tickers exist
}

#[test]
fn is_equity_symbol_invalid() {
    assert!(!is_equity_symbol("")); // empty
    assert!(!is_equity_symbol("EUR/USD")); // forex with slash
    assert!(!is_equity_symbol("XAUUSD")); // gold
    assert!(!is_equity_symbol("XAGUSD")); // silver
    assert!(!is_equity_symbol("XNGUSD")); // natural gas
    assert!(!is_equity_symbol("TOOLONG")); // > 5 chars
    assert!(!is_equity_symbol("AB123")); // contains digits
}

#[test]
fn is_equity_symbol_boundary_cases() {
    assert!(is_equity_symbol("ABCDE")); // exactly 5 chars (max)
    assert!(!is_equity_symbol("ABCDEF")); // 6 chars (too long)
    assert!(is_equity_symbol("A")); // single letter
    assert!(!is_equity_symbol("XBRUSD")); // XBR prefix
    assert!(!is_equity_symbol("XTIUSD")); // XTI prefix
}

#[test]
fn normalize_sec_equity_symbol_strips_kraken_xstock_suffixes() {
    assert_eq!(normalize_sec_equity_symbol("WOK.EQ"), Some("WOK".into()));
    assert_eq!(normalize_sec_equity_symbol("baby.eq"), Some("BABY".into()));
    assert_eq!(normalize_sec_equity_symbol("AAPL"), Some("AAPL".into()));
    assert_eq!(normalize_sec_equity_symbol("BTC/USD"), None);
    assert_eq!(normalize_sec_equity_symbol("TOOLONG.EQ"), None);
}

#[test]
fn scoped_sec_symbols_preserve_caller_priority_order() {
    assert_eq!(
        normalize_sec_equity_symbols_preserving_order([
            "WOK.EQ", "AAPL", "WOK", "BTC/USD", "baby.eq"
        ]),
        vec!["WOK".to_string(), "AAPL".to_string(), "BABY".to_string()]
    );
}

#[test]
fn bar_cache_key_parsing_3_part() {
    // Canonical bar-cache key shape: `<source>:SYM:TF`.
    let key = "kraken-equities:MSFT:1Day";
    let parts: Vec<&str> = key.split(':').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[1], "MSFT");
    assert!(is_equity_symbol(&parts[1].to_uppercase()));
}

#[test]
fn bar_cache_key_filters_timeframe_not_symbol() {
    // Ensure we don't accidentally extract timeframe as symbol.
    let key = "kraken-equities:SLV:15Min";
    let parts: Vec<&str> = key.split(':').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[1], "SLV");
    assert!(is_equity_symbol(&parts[1].to_uppercase()));
    // "15Min" should NOT pass as equity.
    assert!(!is_equity_symbol("15MIN"));
}

#[test]
fn collect_equity_symbols_from_kv_blob_extracts_watchlist_and_positions() {
    let json = serde_json::json!([
        "TNDM",
        {"symbol": "wok"},
        {"symbol": "WOK.EQ"},
        {"ticker": "BABY.EQ"},
        {"ticker": "POM"},
        {"side": "BUY"},
        {"symbol": "BTC/USD"},
        {"underlying_symbol": "ARAY"}
    ]);
    let compressed = zstd::encode_all(json.to_string().as_bytes(), 3).unwrap();
    let mut symbols = std::collections::HashSet::new();
    collect_equity_symbols_from_kv_blob(&compressed, &mut symbols);
    assert!(symbols.contains("TNDM"));
    assert!(symbols.contains("WOK"));
    assert!(symbols.contains("BABY"));
    assert!(symbols.contains("POM"));
    assert!(symbols.contains("ARAY"));
    assert!(!symbols.contains("BUY"));
    assert!(!symbols.contains("BTC/USD"));
}

// ── extract_xml_value ──────────────────────────────────────────

#[test]
fn extract_xml_value_simple() {
    let xml = "<ownershipDocument><rptOwnerName>John Doe</rptOwnerName></ownershipDocument>";
    assert_eq!(
        extract_xml_value(xml, "rptOwnerName"),
        Some("John Doe".to_string())
    );
}

#[test]
fn extract_xml_value_nested_value_tag() {
    let xml = "<transactionShares><value>10000</value></transactionShares>";
    assert_eq!(
        extract_xml_value(xml, "transactionShares"),
        Some("10000".to_string())
    );
}

#[test]
fn extract_xml_value_missing_tag() {
    let xml = "<document><name>Test</name></document>";
    assert_eq!(extract_xml_value(xml, "missing"), None);
}

#[test]
fn extract_xml_value_empty_content() {
    let xml = "<officerTitle></officerTitle>";
    assert_eq!(extract_xml_value(xml, "officerTitle"), None);
}

// ── extract_transactions ───────────────────────────────────────

#[test]
fn extract_transactions_non_derivative() {
    let xml = r#"
        <ownershipDocument>
            <nonDerivativeTransaction>
                <transactionCode>S</transactionCode>
                <transactionShares><value>5000</value></transactionShares>
                <transactionPricePerShare><value>150.50</value></transactionPricePerShare>
                <transactionDate><value>2024-03-15</value></transactionDate>
            </nonDerivativeTransaction>
        </ownershipDocument>
        "#;
    let txns = extract_transactions(xml);
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0].code, "S");
    assert_eq!(txns[0].shares, 5000.0);
    assert!((txns[0].price - 150.50).abs() < 0.01);
    assert_eq!(txns[0].date, "2024-03-15");
}

#[test]
fn extract_transactions_multiple() {
    let xml = r#"
        <doc>
            <nonDerivativeTransaction>
                <transactionCode>P</transactionCode>
                <transactionShares><value>1000</value></transactionShares>
                <transactionPricePerShare><value>100.00</value></transactionPricePerShare>
                <transactionDate><value>2024-01-01</value></transactionDate>
            </nonDerivativeTransaction>
            <nonDerivativeTransaction>
                <transactionCode>S</transactionCode>
                <transactionShares><value>2000</value></transactionShares>
                <transactionPricePerShare><value>110.00</value></transactionPricePerShare>
                <transactionDate><value>2024-01-02</value></transactionDate>
            </nonDerivativeTransaction>
        </doc>
        "#;
    let txns = extract_transactions(xml);
    assert_eq!(txns.len(), 2);
    assert_eq!(txns[0].code, "P");
    assert_eq!(txns[1].code, "S");
}

#[test]
fn extract_transactions_empty_body() {
    let txns = extract_transactions("<doc>nothing relevant here</doc>");
    assert!(txns.is_empty());
}

#[test]
fn extract_transactions_derivative() {
    let xml = r#"
        <doc>
            <derivativeTransaction>
                <transactionCode>A</transactionCode>
                <transactionShares><value>3000</value></transactionShares>
                <transactionPricePerShare><value>0</value></transactionPricePerShare>
                <transactionDate><value>2024-06-01</value></transactionDate>
            </derivativeTransaction>
        </doc>
        "#;
    let txns = extract_transactions(xml);
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0].code, "A");
}

// ── get_recent_filings ─────────────────────────────────────────

#[test]
fn get_recent_filings_all() {
    let conn = setup_test_db();
    insert_filing(&conn, "AAPL", "10-K", "acc-001", "2024-03-01");
    insert_filing(&conn, "AAPL", "10-Q", "acc-002", "2024-06-01");
    insert_filing(&conn, "MSFT", "8-K", "acc-003", "2024-05-15");

    let filings = get_recent_filings(&conn, None, 100).unwrap();
    assert_eq!(filings.len(), 3);
    // Ordered by filing_date DESC
    assert_eq!(filings[0].filing_date, "2024-06-01");
    assert_eq!(filings[1].filing_date, "2024-05-15");
}

#[test]
fn get_recent_filings_filtered_by_ticker() {
    let conn = setup_test_db();
    insert_filing(&conn, "AAPL", "10-K", "acc-001", "2024-03-01");
    insert_filing(&conn, "MSFT", "10-Q", "acc-002", "2024-06-01");

    let filings = get_recent_filings(&conn, Some("AAPL"), 100).unwrap();
    assert_eq!(filings.len(), 1);
    assert_eq!(filings[0].ticker, "AAPL");
}

#[test]
fn get_recent_filings_respects_limit() {
    let conn = setup_test_db();
    for i in 0..10 {
        insert_filing(
            &conn,
            "AAPL",
            "10-Q",
            &format!("acc-{i:03}"),
            &format!("2024-{:02}-01", i + 1),
        );
    }

    let filings = get_recent_filings(&conn, None, 3).unwrap();
    assert_eq!(filings.len(), 3);
}

#[test]
fn get_recent_filings_empty_db() {
    let conn = setup_test_db();
    let filings = get_recent_filings(&conn, None, 100).unwrap();
    assert!(filings.is_empty());
}

#[test]
fn get_unfetched_filings_skips_recent_failures() {
    let conn = setup_test_db();
    insert_filing(&conn, "AAPL", "10-Q", "acc-ok", "2024-06-01");
    insert_filing(&conn, "MSFT", "10-Q", "acc-failed", "2024-06-02");

    mark_filing_content_fetch_failed(&conn, "acc-failed", "HTTP 403").unwrap();

    let filings = get_unfetched_filings(&conn, 10).unwrap();
    assert_eq!(filings.len(), 1);
    assert_eq!(filings[0].accession_number, "acc-ok");
}

#[test]
fn get_unfetched_filings_stops_after_attempt_cap() {
    let conn = setup_test_db();
    insert_filing(&conn, "AAPL", "10-Q", "acc-ok", "2024-06-01");
    insert_filing(&conn, "MSFT", "10-Q", "acc-capped", "2024-06-02");
    let old_attempt = chrono::Utc::now().timestamp() - 7 * 60 * 60;
    conn.execute(
        "UPDATE sec_filings
             SET content_fetch_attempts = 3,
                 content_last_attempt_at = ?2,
                 content_last_error = 'HTTP 403 Forbidden'
             WHERE accession_number = ?1",
        params!["acc-capped", old_attempt],
    )
    .unwrap();

    let filings = get_unfetched_filings(&conn, 10).unwrap();
    assert_eq!(filings.len(), 1);
    assert_eq!(filings[0].accession_number, "acc-ok");
}

#[test]
fn get_unfetched_filings_prioritizes_recent_filings_before_old_high_importance() {
    let conn = setup_test_db();
    insert_filing(&conn, "OLD", "10-K/A", "acc-old", "2016-11-07");
    insert_filing(&conn, "NEW", "8-K", "acc-new", "2024-06-02");

    let filings = get_unfetched_filings(&conn, 2).unwrap();
    assert_eq!(filings[0].accession_number, "acc-new");
    assert_eq!(filings[1].accession_number, "acc-old");
}

#[test]
fn store_filing_content_clears_retry_state() {
    let conn = setup_test_db();
    insert_filing(&conn, "AAPL", "10-Q", "acc-001", "2024-06-01");
    mark_filing_content_fetch_failed(&conn, "acc-001", "HTTP 403").unwrap();

    store_filing_content(&conn, "acc-001", "AAPL", "10-Q", "Apple", "risk factors").unwrap();

    let (fetched, attempts, err): (bool, i64, String) = conn
        .query_row(
            "SELECT content_fetched, content_fetch_attempts, content_last_error \
                 FROM sec_filings WHERE accession_number='acc-001'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert!(fetched);
    assert_eq!(attempts, 0);
    assert!(err.is_empty());
    assert!(get_unfetched_filings(&conn, 10).unwrap().is_empty());
}

// ── get_insider_trades ─────────────────────────────────────────

#[test]
fn get_insider_trades_recent() {
    let conn = setup_test_db();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    insert_insider_trade(
        &conn, "AAPL", "acc-001", "Tim Cook", "S", &today, 10000.0, 195.0,
    );
    insert_insider_trade(
        &conn,
        "AAPL",
        "acc-002",
        "Jeff Williams",
        "P",
        &today,
        5000.0,
        190.0,
    );

    let trades = get_insider_trades(&conn, Some("AAPL"), 30).unwrap();
    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].insider_name, "Tim Cook");
}

#[test]
fn get_insider_trades_all_tickers() {
    let conn = setup_test_db();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    insert_insider_trade(
        &conn, "AAPL", "acc-001", "Tim Cook", "S", &today, 10000.0, 195.0,
    );
    insert_insider_trade(
        &conn,
        "MSFT",
        "acc-002",
        "Satya Nadella",
        "S",
        &today,
        5000.0,
        420.0,
    );

    let trades = get_insider_trades(&conn, None, 30).unwrap();
    assert_eq!(trades.len(), 2);
}

#[test]
fn get_insider_trades_old_excluded() {
    let conn = setup_test_db();
    // Insert a trade from 60 days ago
    let old_date = (chrono::Utc::now() - chrono::Duration::days(60))
        .format("%Y-%m-%d")
        .to_string();
    insert_insider_trade(
        &conn, "AAPL", "acc-001", "Tim Cook", "S", &old_date, 10000.0, 195.0,
    );

    let trades = get_insider_trades(&conn, None, 30).unwrap();
    assert!(trades.is_empty());
}

// ── get_filing_alerts / dismiss_alert ──────────────────────────

#[test]
fn get_filing_alerts_undismissed() {
    let conn = setup_test_db();
    insert_alert(&conn, "AAPL", "LATE_FILING", "AAPL: Late filing", false);
    insert_alert(&conn, "MSFT", "ACTIVIST", "MSFT: Activist position", false);
    insert_alert(&conn, "GOOG", "RESTATEMENT", "GOOG: dismissed", true);

    let active = get_filing_alerts(&conn, false).unwrap();
    assert_eq!(active.len(), 2);

    let dismissed = get_filing_alerts(&conn, true).unwrap();
    assert_eq!(dismissed.len(), 1);
    assert_eq!(dismissed[0].ticker, "GOOG");
}

#[test]
fn dismiss_alert_works() {
    let conn = setup_test_db();
    let id = insert_alert(&conn, "AAPL", "LATE_FILING", "AAPL: Late filing", false);

    // Before dismiss
    let active = get_filing_alerts(&conn, false).unwrap();
    assert_eq!(active.len(), 1);

    dismiss_alert(&conn, id, "Reviewed and not material").unwrap();

    // After dismiss
    let active = get_filing_alerts(&conn, false).unwrap();
    assert!(active.is_empty());

    let dismissed = get_filing_alerts(&conn, true).unwrap();
    assert_eq!(dismissed.len(), 1);
    assert_eq!(dismissed[0].dismissed_reason, "Reviewed and not material");
}

#[test]
fn dismiss_alert_nonexistent_id() {
    let conn = setup_test_db();
    // Should succeed (UPDATE affects 0 rows, no error)
    dismiss_alert(&conn, 99999, "no such alert").unwrap();
}

// ── get_filing_alerts field mapping ────────────────────────────

#[test]
fn filing_alert_fields_populated() {
    let conn = setup_test_db();
    insert_alert(&conn, "TSLA", "DILUTION_RISK", "TSLA: Shelf reg", false);

    let alerts = get_filing_alerts(&conn, false).unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].ticker, "TSLA");
    assert_eq!(alerts[0].alert_type, "DILUTION_RISK");
    assert_eq!(alerts[0].message, "TSLA: Shelf reg");
    assert_eq!(alerts[0].filing_accession, "acc-001");
    assert!(!alerts[0].dismissed);
}

// ── get_recent_filings field mapping ───────────────────────────

#[test]
fn filing_fields_populated() {
    let conn = setup_test_db();
    insert_filing(&conn, "NVDA", "SC 13D", "acc-activist-001", "2024-07-01");

    let filings = get_recent_filings(&conn, Some("NVDA"), 10).unwrap();
    assert_eq!(filings.len(), 1);
    assert_eq!(filings[0].ticker, "NVDA");
    assert_eq!(filings[0].form_type, "SC 13D");
    assert_eq!(filings[0].category, "ACTIVIST");
    assert_eq!(filings[0].importance_score, 70);
    assert!(!filings[0].insider_flag);
}

#[test]
fn decode_html_entities_handles_named_set() {
    assert_eq!(
        decode_html_entities("Tom &amp; Jerry &lt;love&gt; &quot;cheese&quot;"),
        "Tom & Jerry <love> \"cheese\""
    );
    assert_eq!(decode_html_entities("don&apos;t"), "don't");
    assert_eq!(decode_html_entities("a&nbsp;b"), "a b");
}

#[test]
fn decode_html_entities_handles_decimal_numeric() {
    // NBSP via numeric form normalises to a regular space.
    assert_eq!(decode_html_entities("a&#160;b"), "a b");
    // Ballot box ☐ — was leaking through as raw `[&#9744;]` before the fix.
    assert_eq!(decode_html_entities("[&#9744;]"), "[\u{2610}]");
    // Apostrophe via &#39;
    assert_eq!(decode_html_entities("Tom&#39;s"), "Tom's");
}

#[test]
fn decode_html_entities_handles_hex_numeric() {
    assert_eq!(decode_html_entities("&#x2610;"), "\u{2610}");
    assert_eq!(decode_html_entities("&#X2610;"), "\u{2610}");
    assert_eq!(decode_html_entities("&#xa0;"), " ");
}

#[test]
fn decode_html_entities_leaves_unknown_entities_alone() {
    // Unknown entity body: keep the literal `&` and let the rest fall
    // through as text — losing data is worse than printing the
    // unrecognised entity verbatim.
    assert_eq!(decode_html_entities("&unknown;"), "&unknown;");
    // Bare `&` with no following `;` within the lookahead window.
    assert_eq!(decode_html_entities("a & b"), "a & b");
    // Invalid numeric entity stays literal.
    assert_eq!(decode_html_entities("&#notanumber;"), "&#notanumber;");
}

#[test]
fn decode_html_entities_preserves_multibyte_chars() {
    assert_eq!(decode_html_entities("café &amp; thé"), "café & thé");
    assert_eq!(
        decode_html_entities("日本語 &#160; テキスト"),
        "日本語   テキスト"
    );
}

#[test]
fn polish_filing_text_strips_pipe_only_table_rows() {
    let input = "Real content\n| | | |\n|  |  |\nMore content";
    let polished = polish_filing_text(input);
    assert_eq!(polished, "Real content\nMore content");
}

#[test]
fn polish_filing_text_strips_nbsp_only_lines() {
    // `&#160;` decoded to spaces, leaving the line visually empty.
    let input = "Section A\n&#160; &#160; &#160;\nSection B";
    let polished = polish_filing_text(input);
    assert_eq!(polished, "Section A\nSection B");
}

#[test]
fn polish_filing_text_decodes_entities_inside_real_content() {
    let input = "Tom&#39;s [&#9744;] checkbox at AT&amp;T";
    let polished = polish_filing_text(input);
    assert_eq!(polished, "Tom's [\u{2610}] checkbox at AT&T");
}

#[test]
fn polish_filing_text_trims_per_line_whitespace() {
    let input = "  leading spaces\nno spaces\n   ";
    let polished = polish_filing_text(input);
    assert_eq!(polished, "leading spaces\nno spaces");
}

#[test]
fn strip_html_to_text_decodes_numeric_entities_end_to_end() {
    // Reproduce the rendered-output bug from the user-reported screenshot.
    let html = r#"<p>Check this box: [&#9744;] before &#160; signing</p>"#;
    let stripped = strip_html_to_text(html);
    assert!(stripped.contains("\u{2610}"), "ballot box must be decoded");
    assert!(
        !stripped.contains("&#"),
        "no raw numeric entities should remain"
    );
}

#[test]
fn strip_html_to_text_filters_pipe_only_table_rows_from_real_filing_shape() {
    // The 8-K table-of-checkboxes pattern that showed up in the
    // screenshot: every row collapses to ` | | | | `.
    let html = "<p>Header</p><table>\
            <tr><td>&#160;</td><td>&#160;</td><td>&#160;</td><td>&#160;</td></tr>\
            <tr><td>Real cell</td><td>&#160;</td><td>Other cell</td></tr>\
            </table>";
    let stripped = strip_html_to_text(html);
    assert!(stripped.contains("Header"));
    assert!(stripped.contains("Real cell"));
    assert!(stripped.contains("Other cell"));
    // The all-NBSP row must not show up as `| | | |`.
    assert!(
        !stripped
            .lines()
            .any(|l| l.trim().chars().all(|c| c == '|' || c.is_whitespace()))
    );
}
