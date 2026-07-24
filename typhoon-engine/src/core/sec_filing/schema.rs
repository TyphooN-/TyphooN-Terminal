use rusqlite::Connection;
use std::path::Path;

// ── Helper: open a WAL connection ───────────────────────────────────

pub(super) fn open_conn(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("SQLite open failed: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
        .map_err(|e| format!("Pragma failed: {e}"))?;
    // SEC scraping writes on this dedicated connection — independent of the
    // `SqliteCache` write connection the UI thread and bar-sync share. Under WAL a
    // single writer holds the lock at a time, so WITHOUT a busy timeout an SEC write
    // that collides with a UI/bar-sync write fails *instantly* with SQLITE_BUSY —
    // silently dropping filings — and thrashes the lock. Retrying for 5s lets SEC and
    // the shared writer interleave politely, while WAL keeps the UI's *reads* (a
    // separate `read_conn`) unblocked throughout. This is what decouples SEC writes
    // from the render path and lets a broad scrape run during heavy market-data sync.
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| format!("busy_timeout failed: {e}"))?;
    Ok(conn)
}

// ── SQLite Schema ───────────────────────────────────────────────────

pub fn create_sec_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS sec_filings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ticker TEXT NOT NULL,
            form_type TEXT NOT NULL,
            accession_number TEXT UNIQUE NOT NULL,
            filing_date TEXT NOT NULL,
            url TEXT NOT NULL,
            company_name TEXT DEFAULT '',
            importance_score INTEGER DEFAULT 50,
            category TEXT DEFAULT 'OTHER',
            summary TEXT DEFAULT '',
            insider_flag BOOLEAN DEFAULT FALSE,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_sec_ticker_date ON sec_filings(ticker, filing_date DESC);
        CREATE INDEX IF NOT EXISTS idx_sec_form ON sec_filings(form_type);
        -- Global recency. `idx_sec_ticker_date` is ticker-leading, so it cannot
        -- serve the un-tickered `ORDER BY filing_date DESC LIMIT n` the scanner's
        -- background snapshot runs every cycle: without this index SQLite did a
        -- full SCAN plus a TEMP B-TREE sort over the whole table (1M+ rows after a
        -- broad EDGAR scrape) on every refresh, which is both a recurring stall and
        -- the reason the query was fragile enough to fail under scrape contention
        -- and leave the Filings tab empty.
        CREATE INDEX IF NOT EXISTS idx_sec_filing_date ON sec_filings(filing_date DESC);

        CREATE TABLE IF NOT EXISTS sec_insider_trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ticker TEXT NOT NULL,
            accession_number TEXT NOT NULL,
            insider_name TEXT NOT NULL,
            insider_title TEXT DEFAULT '',
            transaction_date TEXT NOT NULL,
            transaction_type TEXT NOT NULL,
            shares REAL DEFAULT 0,
            price REAL DEFAULT 0,
            aggregate_value REAL DEFAULT 0,
            is_officer BOOLEAN DEFAULT FALSE,
            is_director BOOLEAN DEFAULT FALSE,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_insider_ticker ON sec_insider_trades(ticker, transaction_date DESC);

        CREATE TABLE IF NOT EXISTS sec_filing_alerts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ticker TEXT NOT NULL,
            alert_type TEXT NOT NULL,
            message TEXT NOT NULL,
            filing_accession TEXT,
            importance INTEGER DEFAULT 50,
            created_at INTEGER NOT NULL,
            dismissed BOOLEAN DEFAULT FALSE,
            dismissed_reason TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_alerts_created ON sec_filing_alerts(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_alerts_ticker ON sec_filing_alerts(ticker);

        CREATE TABLE IF NOT EXISTS sec_scrape_index (
            ticker TEXT PRIMARY KEY,
            last_scrape_date TEXT,
            filing_count INTEGER DEFAULT 0,
            cik TEXT
        );
    ").map_err(|e| format!("Failed to create SEC tables: {e}"))?;

    // Schema migration: add updated_at column (last-modified tracking)
    let _ = conn.execute(
        "ALTER TABLE sec_scrape_index ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        [],
    );

    // Schema migration: filing content storage (indefinite, growing database)
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN content_fetched BOOLEAN DEFAULT FALSE",
        [],
    );
    // Retry bookkeeping for SEC content hydration. Without this, permanently
    // unfetchable/blocked documents stay at the front of the queue and the
    // background worker can refetch the same failing batch forever.
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN content_fetch_attempts INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN content_last_attempt_at INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN content_last_error TEXT DEFAULT ''",
        [],
    );
    // Retry bookkeeping for Form 4 insider parsing, mirroring the content
    // columns above. Insider trades were only ever parsed for filings inserted
    // during the current scrape pass, so a Form 4 that failed (and until the
    // `xslF345X0*` URL fix, every single one did) was never revisited. The
    // backfill worker needs somewhere to record attempts, or a Form 4 that
    // genuinely reports no transactions would be refetched forever.
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN insider_parsed BOOLEAN DEFAULT FALSE",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN insider_parse_attempts INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN insider_last_attempt_at INTEGER DEFAULT 0",
        [],
    );
    // Partial index for the backfill queue: without it, finding un-parsed
    // Form 4s means a full scan of a 1M+ row table on every background cycle.
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sec_insider_backfill
         ON sec_filings(filing_date DESC)
         WHERE insider_flag = TRUE AND COALESCE(insider_parsed, FALSE) = FALSE",
        [],
    );
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sec_filing_content (
            accession_number TEXT PRIMARY KEY,
            content_plain TEXT NOT NULL,
            content_size INTEGER DEFAULT 0,
            fetched_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sec_keyword_watchlist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            keyword TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL
        );
    ",
    )
    .map_err(|e| format!("Failed to create SEC content tables: {e}"))?;

    // FTS5 full-text search index (porter stemming + unicode)
    let _ = conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS sec_fts USING fts5(
            accession_number, ticker, form_type, company_name, content,
            tokenize='porter unicode61'
        );
    ",
    );

    Ok(())
}
