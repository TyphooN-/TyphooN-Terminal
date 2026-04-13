//! LAN Sync — Encrypted WebSocket (TLS) cache synchronization between TyphooN Terminal instances.
//!
//! Server mode: serves bar cache data to connecting clients over local network.
//! Client mode: connects to a server, syncs missing/outdated cache entries.
//! Transport: wss:// (TLS encrypted) with ephemeral self-signed certificate.
//! Auth: PBKDF2-derived shared secret + HMAC-SHA256 challenge-response.

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::core::cache::SqliteCache;

/// Maximum key length from network input (16 KB).
const MAX_KEY_LEN: usize = 16384;
/// Maximum data length from network input (256 MB — reasonable for bar data).
const MAX_DATA_LEN: usize = 256 * 1024 * 1024;
/// Maximum WebSocket message size (512 MB).
const MAX_WS_MESSAGE_SIZE: usize = 512 * 1024 * 1024;

fn ws_config() -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    let mut config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
    config.max_message_size = Some(MAX_WS_MESSAGE_SIZE);
    config.max_frame_size = Some(MAX_WS_MESSAGE_SIZE);
    config
}

// ── TLS Certificate Generation ────────────────────────────────────

/// Generate an ephemeral self-signed TLS certificate for LAN sync server.
/// Returns (PEM certificate, PEM private key, SHA-256 fingerprint hex) for native-tls.
pub fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>, String), String> {
    let certified_key = rcgen::generate_simple_self_signed(vec!["typhoon-lan-sync".into(), "localhost".into()])
        .map_err(|e| format!("Certificate generation failed: {e}"))?;
    let cert_pem = certified_key.cert.pem().into_bytes();
    let key_pem = certified_key.signing_key.serialize_pem().into_bytes();
    let fingerprint = compute_sha256_fingerprint(&certified_key.cert.der().to_vec());
    Ok((cert_pem, key_pem, fingerprint))
}

/// Compute SHA-256 fingerprint of DER-encoded certificate bytes. Returns lowercase hex string.
fn compute_sha256_fingerprint(der_bytes: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(der_bytes);
    hex_encode(&hash)
}

/// Build a native-tls TLS acceptor from PEM cert + key.
fn build_tls_acceptor(cert_pem: &[u8], key_pem: &[u8]) -> Result<native_tls::TlsAcceptor, String> {
    let identity = native_tls::Identity::from_pkcs8(cert_pem, key_pem)
        .map_err(|e| format!("TLS identity failed: {e}"))?;
    native_tls::TlsAcceptor::new(identity)
        .map_err(|e| format!("TLS acceptor build failed: {e}"))
}

/// Build a native-tls TLS connector that accepts any certificate (for LAN self-signed).
fn build_tls_connector() -> Result<native_tls::TlsConnector, String> {
    native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true) // LAN self-signed cert
        .danger_accept_invalid_hostnames(true) // LAN IP addresses
        .build()
        .map_err(|e| format!("TLS connector build failed: {e}"))
}

// ── Protocol Messages ──────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SyncMessage {
    AuthChallenge { challenge: String },
    Auth { response: String },
    AuthOk,
    AuthFail { reason: String },
    RequestMeta,
    /// Incremental metadata request — only entries updated since since_ts.
    RequestMetaSince { since_ts: i64 },
    Metadata { entries: Vec<CacheMeta> },
    RequestEntries { keys: Vec<String> },
    EntryData { key: String, data: String, timestamp: i64 },
    BatchComplete { count: usize },
    IncrementalUpdate { key: String, data: String, timestamp: i64 },
    Ping,
    Pong,
    // ── DARWIN data sync (opt-in) ──
    /// Request DARWIN snapshot (deals, positions, equity) for all accounts.
    RequestDarwinData,
    /// Server response: serialized DARWIN data (JSON blob, zstd + base64 encoded).
    DarwinData { data: String, accounts: usize, deals: usize, positions: usize },
    /// Server stats pushed to client on connect and periodically.
    SyncStats { bytes_sent: u64, bytes_received: u64, entries_synced: usize, uptime_secs: u64 },
    /// Request KV cache entries (fundamentals, news, SEC, FRED, etc.)
    /// since_ts: only return entries with timestamp > since_ts (0 = full sync).
    RequestKvData { since_ts: i64 },
    /// KV batch complete marker
    KvBatchComplete { count: usize },
    /// Client requests server to execute a data fetch and return results.
    /// cmd: command name (e.g. "SEC_SCRAPE", "FUNDAMENTALS", "FINNHUB_NEWS", "KRAKEN_BACKFILL")
    /// args: JSON-encoded arguments
    RemoteRequest { cmd: String, args: String },
    /// Server response to RemoteRequest — triggers a re-sync of affected data.
    RemoteRequestDone { cmd: String, message: String },
    // ── Generic table sync ──
    /// Client requests bulk sync of SQLite tables (by whitelist name).
    /// Each entry is (table_name, since_ts). since_ts=0 means full sync.
    RequestTableSync { tables: Vec<(String, i64)> },
    /// Server sends one table's rows as zstd-compressed + base64-encoded JSON.
    TableSyncData { table: String, rows_json: String },
    /// Server signals all requested tables have been sent.
    TableSyncDone,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheMeta {
    pub key: String,
    pub timestamp: i64,
    pub bar_count: Option<i64>,
}

// ── Syncable Tables (whitelist) ────────────────────────────────────

const SYNCABLE_TABLES: &[&str] = &[
    "darwin_equity_snapshots",
    "sec_filings",
    "sec_insider_trades",
    "sec_filing_alerts",
    "sec_scrape_index",
    "sec_filing_content",
    "fundamentals",
    "quarterly_financials",
    "institutional_holders",
    "research_news",
];

/// Returns the CREATE TABLE statement for a syncable table (whitelist only).
fn create_table_sql(table: &str) -> Option<&'static str> {
    match table {
        "darwin_equity_snapshots" => Some(
            "CREATE TABLE IF NOT EXISTS darwin_equity_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                darwin_ticker TEXT NOT NULL,
                closed_balance REAL NOT NULL DEFAULT 0,
                unrealized_pnl REAL NOT NULL DEFAULT 0,
                floating_equity REAL NOT NULL DEFAULT 0,
                open_position_count INTEGER NOT NULL DEFAULT 0
            )"
        ),
        "sec_filings" => Some(
            "CREATE TABLE IF NOT EXISTS sec_filings (
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
            )"
        ),
        "sec_insider_trades" => Some(
            "CREATE TABLE IF NOT EXISTS sec_insider_trades (
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
            )"
        ),
        "sec_filing_alerts" => Some(
            "CREATE TABLE IF NOT EXISTS sec_filing_alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ticker TEXT NOT NULL,
                alert_type TEXT NOT NULL,
                message TEXT NOT NULL,
                filing_accession TEXT,
                importance INTEGER DEFAULT 50,
                created_at INTEGER NOT NULL,
                dismissed BOOLEAN DEFAULT FALSE,
                dismissed_reason TEXT
            )"
        ),
        "sec_scrape_index" => Some(
            "CREATE TABLE IF NOT EXISTS sec_scrape_index (
                ticker TEXT PRIMARY KEY,
                last_scrape_date TEXT,
                filing_count INTEGER DEFAULT 0,
                cik TEXT,
                updated_at INTEGER NOT NULL DEFAULT 0
            )"
        ),
        "sec_filing_content" => Some(
            "CREATE TABLE IF NOT EXISTS sec_filing_content (
                accession_number TEXT PRIMARY KEY,
                content_plain TEXT NOT NULL,
                content_size INTEGER DEFAULT 0,
                fetched_at INTEGER NOT NULL
            )"
        ),
        "fundamentals" => Some(
            "CREATE TABLE IF NOT EXISTS fundamentals (
                symbol TEXT PRIMARY KEY,
                cik TEXT,
                company_name TEXT NOT NULL DEFAULT '',
                sector TEXT NOT NULL DEFAULT '',
                industry TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                market_cap REAL,
                enterprise_value REAL,
                total_debt REAL,
                cash_and_equivalents REAL,
                shares_outstanding REAL,
                stock_price REAL,
                mcap_ev_ratio REAL,
                next_earnings_date TEXT,
                previous_earnings_date TEXT,
                next_ex_dividend_date TEXT,
                next_dividend_payment_date TEXT,
                last_dividend_payment_date TEXT,
                is_dividend_stock INTEGER NOT NULL DEFAULT 0,
                dividend_yield REAL,
                pe_ratio REAL,
                forward_pe REAL,
                peg_ratio REAL,
                price_to_book REAL,
                price_to_sales REAL,
                ev_to_ebitda REAL,
                profit_margin REAL,
                operating_margin REAL,
                roe REAL,
                roa REAL,
                beta REAL,
                short_ratio REAL,
                short_percent_of_float REAL,
                last_updated TEXT NOT NULL DEFAULT '',
                updated_at INTEGER NOT NULL DEFAULT 0
            )"
        ),
        "quarterly_financials" => Some(
            "CREATE TABLE IF NOT EXISTS quarterly_financials (
                symbol TEXT NOT NULL,
                period_end TEXT NOT NULL,
                total_revenue REAL,
                net_income REAL,
                free_cash_flow REAL,
                gross_profit REAL,
                operating_income REAL,
                ebitda REAL,
                eps REAL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (symbol, period_end)
            )"
        ),
        "institutional_holders" => Some(
            "CREATE TABLE IF NOT EXISTS institutional_holders (
                symbol TEXT NOT NULL,
                holder_name TEXT NOT NULL,
                shares INTEGER NOT NULL DEFAULT 0,
                pct_held REAL NOT NULL DEFAULT 0.0,
                value REAL NOT NULL DEFAULT 0.0,
                date_reported TEXT NOT NULL DEFAULT '',
                updated_at INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (symbol, holder_name)
            )"
        ),
        "research_news" => Some(
            "CREATE TABLE IF NOT EXISTS research_news (
                url_hash TEXT PRIMARY KEY,
                symbol TEXT NOT NULL DEFAULT '',
                source TEXT NOT NULL DEFAULT '',
                provider TEXT NOT NULL DEFAULT '',
                headline TEXT NOT NULL DEFAULT '',
                summary TEXT NOT NULL DEFAULT '',
                url TEXT NOT NULL DEFAULT '',
                published_at INTEGER NOT NULL DEFAULT 0,
                image_url TEXT NOT NULL DEFAULT '',
                sentiment TEXT NOT NULL DEFAULT '',
                sentiment_score REAL NOT NULL DEFAULT 0.0,
                tickers_json TEXT NOT NULL DEFAULT '[]',
                categories_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL DEFAULT 0
            )"
        ),
        _ => None,
    }
}

/// Returns the timestamp column name for incremental sync, if available.
/// Tables without a usable timestamp column return None and fall back to full sync.
fn table_timestamp_column(table: &str) -> Option<&'static str> {
    match table {
        "sec_filings" => Some("created_at"),
        "sec_insider_trades" => Some("created_at"),
        "sec_filing_alerts" => Some("created_at"),
        "sec_filing_content" => Some("fetched_at"),
        "darwin_equity_snapshots" => Some("timestamp"),
        "fundamentals" => Some("updated_at"),
        "quarterly_financials" => Some("updated_at"),
        "institutional_holders" => Some("updated_at"),
        "sec_scrape_index" => Some("updated_at"),
        "research_news" => Some("updated_at"),
        _ => None,
    }
}

/// Export rows from a table as JSON, optionally filtered by timestamp.
/// If since_ts > 0 and the table has a timestamp column, only rows newer than since_ts are returned.
/// Falls back to full export if since_ts == 0 or no timestamp column exists.
fn export_table_as_json_since(conn: &rusqlite::Connection, table: &str, since_ts: i64) -> Result<(String, usize), String> {
    if !SYNCABLE_TABLES.contains(&table) {
        return Err(format!("Table '{}' not in whitelist", table));
    }

    let ts_col = table_timestamp_column(table);
    let use_filter = since_ts > 0 && ts_col.is_some();
    let sql = if let (true, Some(col)) = (use_filter, ts_col) {
        format!("SELECT * FROM {} WHERE {} > ?1", table, col)
    } else {
        format!("SELECT * FROM {}", table)
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare SELECT from {table}: {e}"))?;
    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count).map(|i| stmt.column_name(i).unwrap_or("?").to_string()).collect();

    // Use query() with manual row iteration to avoid closure type mismatch
    let mut rows = if use_filter {
        stmt.query(rusqlite::params![since_ts]).map_err(|e| format!("Query {table}: {e}"))?
    } else {
        stmt.query([]).map_err(|e| format!("Query {table}: {e}"))?
    };

    let mut arr = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("Row iter {table}: {e}"))? {
        let mut map = serde_json::Map::new();
        for (i, name) in col_names.iter().enumerate() {
            let val: rusqlite::types::Value = row.get(i).map_err(|e| format!("Get col {name}: {e}"))?;
            let json_val = match val {
                rusqlite::types::Value::Null => serde_json::Value::Null,
                rusqlite::types::Value::Integer(n) => serde_json::Value::Number(n.into()),
                rusqlite::types::Value::Real(f) => serde_json::Value::Number(
                    serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into())
                ),
                rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
                rusqlite::types::Value::Blob(b) => serde_json::Value::String(
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &b)
                ),
            };
            map.insert(name.clone(), json_val);
        }
        arr.push(serde_json::Value::Object(map));
    }

    let count = arr.len();
    let json = serde_json::to_string(&arr).map_err(|e| format!("Serialize {table}: {e}"))?;
    Ok((json, count))
}

/// Import JSON rows into a table. Creates the table if it doesn't exist.
/// Uses INSERT OR REPLACE to handle duplicates.
fn import_table_from_json(conn: &rusqlite::Connection, table: &str, json: &str) -> Result<usize, String> {
    // Create table if needed
    let ddl = create_table_sql(table).ok_or_else(|| format!("No DDL for table '{}'", table))?;
    conn.execute_batch(ddl).map_err(|e| format!("Create table {table}: {e}"))?;

    let rows: Vec<serde_json::Value> = serde_json::from_str(json)
        .map_err(|e| format!("Parse JSON for {table}: {e}"))?;
    if rows.is_empty() { return Ok(0); }

    // Get column names from first row
    let first = rows[0].as_object().ok_or("Expected JSON object row")?;
    // Filter out AUTOINCREMENT 'id' column — let SQLite assign new IDs on import
    // to avoid UNIQUE constraint conflicts across different databases.
    let has_autoincrement_id = ddl.contains("id INTEGER PRIMARY KEY AUTOINCREMENT");
    let col_names: Vec<String> = first.keys()
        .filter(|k| !(has_autoincrement_id && *k == "id"))
        .cloned()
        .collect();
    let placeholders: Vec<&str> = col_names.iter().map(|_| "?").collect();
    let sql = format!(
        "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
        table,
        col_names.join(", "),
        placeholders.join(", ")
    );

    let mut count = 0usize;
    // Safety: rollback any dangling transaction from a previous failed import.
    // unchecked_transaction() fails with "cannot start a transaction within a transaction"
    // if a prior BEGIN didn't get committed or rolled back.
    let _ = conn.execute_batch("ROLLBACK");
    let tx = conn.unchecked_transaction().map_err(|e| format!("Begin tx for {table}: {e}"))?;
    {
        let mut stmt = tx.prepare(&sql).map_err(|e| format!("Prepare INSERT for {table}: {e}"))?;
        // PERF: reuse the param buffer across rows — was allocating two Vecs
        // (values + refs) for every row during multi-thousand-row imports.
        let mut params: Vec<rusqlite::types::Value> = Vec::with_capacity(col_names.len());
        for row in &rows {
            if let Some(obj) = row.as_object() {
                params.clear();
                for col in col_names.iter() {
                    let v = match obj.get(col) {
                        Some(serde_json::Value::Null) | None => rusqlite::types::Value::Null,
                        Some(serde_json::Value::Number(n)) => {
                            if let Some(i) = n.as_i64() {
                                rusqlite::types::Value::Integer(i)
                            } else if let Some(f) = n.as_f64() {
                                rusqlite::types::Value::Real(f)
                            } else {
                                rusqlite::types::Value::Null
                            }
                        }
                        Some(serde_json::Value::String(s)) => rusqlite::types::Value::Text(s.clone()),
                        Some(serde_json::Value::Bool(b)) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
                        Some(other) => rusqlite::types::Value::Text(other.to_string()),
                    };
                    params.push(v);
                }
                match stmt.execute(rusqlite::params_from_iter(params.iter())) {
                    Ok(_) => count += 1,
                    Err(e) => tracing::warn!("LAN sync: insert into {table} failed: {e}"),
                }
            }
        }
    }
    tx.commit().map_err(|e| format!("Commit {table}: {e}"))?;
    Ok(count)
}

// ── Key Derivation ─────────────────────────────────────────────────

/// Derive a 32-byte shared secret from passphrase using PBKDF2-HMAC-SHA256.
fn derive_secret(passphrase: &str) -> [u8; 32] {
    use pbkdf2::pbkdf2_hmac;
    use sha2::Sha256;
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), b"typhoon-lan-sync", 100_000, &mut key);
    key
}

/// Compute HMAC-SHA256(challenge_bytes, secret) and return hex string.
fn hmac_hex(challenge: &[u8], secret: &[u8; 32]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = match HmacSha256::new_from_slice(secret) {
        Ok(m) => m,
        Err(_) => return String::new(), // 32-byte key can't fail, but don't panic
    };
    mac.update(challenge);
    let result = mac.finalize().into_bytes();
    hex_encode(&result)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn send_msg(msg: &SyncMessage) -> Result<Message, String> {
    let json = serde_json::to_string(msg).map_err(|e| format!("Serialize failed: {e}"))?;
    Ok(Message::Text(json.into()))
}

fn parse_msg(msg: &Message) -> Result<SyncMessage, String> {
    match msg {
        Message::Text(txt) => {
            serde_json::from_str(txt.as_ref()).map_err(|e| format!("Parse failed: {e}"))
        }
        _ => Err("Expected text message".into()),
    }
}

// ── Sync Status ────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncStatus {
    pub mode: String,           // "server", "client", "idle"
    pub connected: bool,
    pub clients: usize,         // server: number of connected clients
    pub host: String,           // client: server host
    pub port: u16,
    pub bytes_sent: u64,        // total bytes sent
    pub bytes_received: u64,    // total bytes received
    pub entries_synced: usize,  // bar entries synced
    pub darwin_synced: bool,    // whether DARWIN data has been synced
    pub uptime_secs: u64,       // seconds since start
    pub send_darwin: bool,      // server: opt-in to send DARWIN data to clients
    pub cert_fingerprint: String, // SHA-256 fingerprint of the TLS certificate (hex)
    pub client_ips: Vec<String>, // server: list of connected client IP addresses
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self {
            mode: "idle".into(), connected: false, clients: 0,
            host: String::new(), port: 0,
            bytes_sent: 0, bytes_received: 0, entries_synced: 0,
            darwin_synced: false, uptime_secs: 0, send_darwin: false,
            cert_fingerprint: String::new(), client_ips: Vec::new(),
        }
    }
}

// ── Server ─────────────────────────────────────────────────────────

pub struct LanSyncServer {
    task: Option<tokio::task::JoinHandle<()>>,
    status: Arc<TokioMutex<SyncStatus>>,
}

impl LanSyncServer {
    pub async fn start(
        cache: Arc<SqliteCache>,
        port: u16,
        passphrase: &str,
    ) -> Result<Self, String> {
        let secret = derive_secret(passphrase);

        // Generate ephemeral self-signed TLS certificate
        let (cert_der, key_der, cert_fingerprint) = generate_self_signed_cert()?;
        let tls_acceptor = build_tls_acceptor(&cert_der, &key_der)?;
        let tls_acceptor_tokio = tokio_native_tls::TlsAcceptor::from(tls_acceptor);

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .map_err(|e| format!("Bind failed on port {port}: {e}"))?;

        let status = Arc::new(TokioMutex::new(SyncStatus {
            mode: "server".into(),
            connected: true,
            clients: 0,
            host: "0.0.0.0".into(),
            port,
            cert_fingerprint: cert_fingerprint.clone(),
            ..Default::default()
        }));

        let status_clone = status.clone();
        let task = tokio::spawn(async move {
            tracing::info!("LAN sync server listening on wss://0.0.0.0:{port} (TLS encrypted, fingerprint: {cert_fingerprint})");
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        tracing::info!("LAN sync: TLS client connected from {addr}");
                        let tls_acc = tls_acceptor_tokio.clone();
                        let cache = cache.clone();
                        let status = status_clone.clone();
                        tokio::spawn(async move {
                            // TLS handshake with 30s timeout
                            let client_ip = addr.ip().to_string();
                            let tls_result = tokio::time::timeout(
                                std::time::Duration::from_secs(30),
                                tls_acc.accept(stream),
                            ).await;
                            match tls_result {
                                Ok(Ok(tls_stream)) => {
                                    // Update status under lock, then write KV outside lock to avoid I/O contention
                                    let ips_json = {
                                        let mut s = status.lock().await;
                                        s.clients += 1;
                                        s.client_ips.push(client_ip.clone());
                                        serde_json::to_string(&s.client_ips).unwrap_or_default()
                                    }; // lock dropped here — before I/O
                                    let _ = cache.put_kv("lan:server:clients", &ips_json);
                                    handle_client_tls(tls_stream, cache, secret, status, &client_ip).await;
                                }
                                Ok(Err(e)) => {
                                    tracing::warn!("LAN sync: TLS handshake failed from {addr}: {e}");
                                }
                                Err(_) => {
                                    tracing::warn!("LAN sync: TLS handshake timeout from {addr}");
                                }
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!("LAN sync accept error: {e}");
                    }
                }
            }
        });

        Ok(Self { task: Some(task), status })
    }

    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            tracing::info!("LAN sync server stopped");
        }
    }

    pub async fn status(&self) -> SyncStatus {
        self.status.lock().await.clone()
    }
}

/// Handle a TLS-encrypted client connection.
async fn handle_client_tls(
    tls_stream: tokio_native_tls::TlsStream<tokio::net::TcpStream>,
    cache: Arc<SqliteCache>,
    secret: [u8; 32],
    status: Arc<TokioMutex<SyncStatus>>,
    client_ip: &str,
) {
    // Helper: clean up client count + IP on any early exit
    let cleanup_client = |status: &Arc<TokioMutex<SyncStatus>>, cache: &Arc<SqliteCache>, ip: &str| {
        let status = status.clone();
        let cache = cache.clone();
        let ip = ip.to_string();
        async move {
            // Update status under lock, write KV outside lock to avoid I/O contention
            let ips_json = {
                let mut s = status.lock().await;
                s.clients = s.clients.saturating_sub(1);
                s.client_ips.retain(|i| i != &ip);
                serde_json::to_string(&s.client_ips).unwrap_or_default()
            }; // lock dropped here
            let _ = cache.put_kv("lan:server:clients", &ips_json);
        }
    };

    let ws = match tokio_tungstenite::accept_async_with_config(tls_stream, Some(ws_config())).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::warn!("LAN sync WebSocket handshake failed: {e}");
            cleanup_client(&status, &cache, client_ip).await;
            return;
        }
    };

    let (mut sink, mut stream_rx) = ws.split();

    // 1. Send AuthChallenge
    let challenge_bytes: [u8; 32] = rand::random();
    let challenge_hex = hex_encode(&challenge_bytes);
    let challenge_msg = match send_msg(&SyncMessage::AuthChallenge { challenge: challenge_hex.clone() }) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("LAN sync: failed to serialize AuthChallenge: {e}");
            cleanup_client(&status, &cache, client_ip).await;
            return;
        }
    };
    if sink.send(challenge_msg).await.is_err() {
        cleanup_client(&status, &cache, client_ip).await;
        return;
    }

    // 2. Wait for Auth response
    let auth_ok = match tokio::time::timeout(std::time::Duration::from_secs(10), stream_rx.next()).await {
        Ok(Some(Ok(msg))) => {
            match parse_msg(&msg) {
                Ok(SyncMessage::Auth { response }) => {
                    let expected = hmac_hex(&challenge_bytes, &secret);
                    // Constant-time comparison to prevent timing attacks
                    response.len() == expected.len() && response.as_bytes().iter().zip(expected.as_bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
                }
                _ => false,
            }
        }
        _ => false,
    };

    if !auth_ok {
        if let Ok(msg) = send_msg(&SyncMessage::AuthFail { reason: "Invalid credentials".into() }) {
            let _ = sink.send(msg).await;
        }
        cleanup_client(&status, &cache, client_ip).await;
        return;
    }
    if let Ok(msg) = send_msg(&SyncMessage::AuthOk) {
        let _ = sink.send(msg).await;
    }
    tracing::info!("LAN sync: client authenticated");

    // 3. Main message loop
    while let Some(Ok(msg)) = stream_rx.next().await {
        if msg.is_close() { break; }
        if msg.is_ping() {
            let _ = sink.send(Message::Pong(msg.into_data())).await;
            continue;
        }
        let parsed = match parse_msg(&msg) {
            Ok(m) => m,
            Err(_) => continue,
        };

        match parsed {
            SyncMessage::RequestMeta => {
                let meta = build_cache_meta(&cache);
                if let Ok(msg) = send_msg(&SyncMessage::Metadata { entries: meta }) {
                    let _ = sink.send(msg).await;
                }
            }
            SyncMessage::RequestMetaSince { since_ts } => {
                // Delta metadata: only entries updated since since_ts
                let entries = cache.get_cache_meta_since(since_ts).unwrap_or_default();
                let meta: Vec<CacheMeta> = entries.into_iter().map(|(key, ts, bc)| {
                    CacheMeta { key, timestamp: ts, bar_count: Some(bc) }
                }).collect();
                if meta.is_empty() {
                    tracing::trace!("LAN sync: meta delta — 0 changed entries since {since_ts}");
                } else {
                    tracing::debug!("LAN sync: meta delta — {} changed entries since {since_ts}", meta.len());
                }
                if let Ok(msg) = send_msg(&SyncMessage::Metadata { entries: meta }) {
                    let _ = sink.send(msg).await;
                }
            }
            SyncMessage::RequestEntries { keys } => {
                // Fast binary transfer: batch entries into large binary WebSocket frames
                // Format per entry: [u32 key_len][key_bytes][i64 timestamp][u32 data_len][data_bytes]
                let mut count = 0u32;
                let mut bytes_total = 0u64;
                let mut batch_buf: Vec<u8> = Vec::with_capacity(4 * 1024 * 1024); // 4MB batch buffer
                let flush_threshold = 2 * 1024 * 1024; // flush every 2MB

                for key in &keys {
                    if let Ok(Some((data, ts))) = cache.get_raw_bar_entry(key) {
                        let key_bytes = key.as_bytes();
                        batch_buf.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
                        batch_buf.extend_from_slice(key_bytes);
                        batch_buf.extend_from_slice(&ts.to_le_bytes());
                        batch_buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
                        batch_buf.extend_from_slice(&data);
                        count += 1;

                        // Flush when batch is large enough (swap to avoid clone)
                        if batch_buf.len() >= flush_threshold {
                            bytes_total += batch_buf.len() as u64;
                            let send_buf = std::mem::replace(&mut batch_buf, Vec::with_capacity(4 * 1024 * 1024));
                            let _ = sink.send(Message::Binary(send_buf.into())).await;
                        }
                    }
                }
                // Flush remaining
                if !batch_buf.is_empty() {
                    bytes_total += batch_buf.len() as u64;
                    let _ = sink.send(Message::Binary(batch_buf.into())).await;
                }
                // Send completion marker as text
                if let Ok(msg) = send_msg(&SyncMessage::BatchComplete { count: count as usize }) {
                    let _ = sink.send(msg).await;
                }
                {
                    let mut s = status.lock().await;
                    s.entries_synced += count as usize;
                    s.bytes_sent += bytes_total;
                }
            }
            SyncMessage::RequestDarwinData => {
                // Export DARWIN tables via read connection (doesn't block writes)
                let cache_clone = cache.clone();
                let darwin_result = tokio::task::spawn_blocking(move || {
                    if let Ok(conn) = cache_clone.read_connection() {
                        crate::core::darwin::export_darwin_data(&conn).ok()
                    } else { None }
                }).await.ok().flatten();

                if let Some((json, n_acct, n_deals, n_pos)) = darwin_result {
                    let compressed = zstd::encode_all(json.as_bytes(), 3).unwrap_or_else(|_| json.into_bytes());
                    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &compressed);
                    if let Ok(msg) = send_msg(&SyncMessage::DarwinData {
                        data: encoded, accounts: n_acct, deals: n_deals, positions: n_pos,
                    }) {
                        let _ = sink.send(msg).await;
                    }
                    tracing::info!("LAN sync: sent DARWIN data ({} accounts, {} deals, {} positions)", n_acct, n_deals, n_pos);
                } else {
                    tracing::warn!("LAN sync: DARWIN export failed or cache unavailable");
                }
            }
            SyncMessage::RequestKvData { since_ts } => {
                // Send KV cache entries as binary batch (incremental if since_ts > 0)
                // NEVER sync machine-local config keys: LAN topology and credentials are
                // per-machine and must not overwrite client settings.
                let kv_local_keys: &[&str] = &[
                    "lan:server_enabled", "lan:client_enabled",
                    "lan:server_ip", "lan:sync_port",
                ];
                let is_skip_key = |k: &str| {
                    kv_local_keys.iter().any(|&lk| k == lk)
                        || k.starts_with("cred:")
                        || k.starts_with("quote:")            // 851 individual bid/ask entries — huge churn
                        || k.starts_with("darwin:daily_returns")  // huge (100KB+), client computes locally
                        || k.starts_with("darwin:correlations")   // N×N matrix, client computes locally
                        || k.starts_with("darwin:exposure")       // client computes locally
                        || k == "darwin:insider_trades"            // large, client has SEC data via table sync
                        || k == "client:demand"                   // per-machine demand list
                        || k.starts_with("lan:")                  // all LAN config is per-machine
                };
                let cache_clone = cache.clone();
                // Send KV values as compressed blobs (skip server-side decompression).
                // Client uses put_kv_compressed() to store directly — saves CPU + bandwidth.
                // Binary format: [u32 key_len][key][u32 blob_len][compressed_blob] repeated
                let kv_data: Vec<(String, Vec<u8>)> = tokio::task::spawn_blocking(move || {
                    if since_ts > 0 {
                        match cache_clone.list_kv_entries_since(since_ts) {
                            Ok(entries) => {
                                entries.into_iter()
                                    .filter(|(key, _, _)| !is_skip_key(key))
                                    .map(|(key, compressed, _ts)| (key, compressed))
                                    .collect()
                            }
                            Err(e) => {
                                tracing::warn!("LAN sync: list_kv_entries_since failed: {e}");
                                Vec::new()
                            }
                        }
                    } else {
                        // Full sync: send compressed blobs directly
                        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
                        if let Ok(all) = cache_clone.list_kv_entries_since(0) {
                            for (key, compressed, _ts) in all {
                                if is_skip_key(&key) { continue; }
                                entries.push((key, compressed));
                            }
                        }
                        entries
                    }
                }).await.unwrap_or_default();

                // Send as binary batch: [u32 key_len][key][u32 blob_len][compressed_blob] repeated
                let mut count = 0u32;
                let mut batch_buf: Vec<u8> = Vec::with_capacity(2 * 1024 * 1024);
                for (key, blob) in &kv_data {
                    let kb = key.as_bytes();
                    let vb = blob.as_slice();
                    batch_buf.extend_from_slice(&(kb.len() as u32).to_le_bytes());
                    batch_buf.extend_from_slice(kb);
                    batch_buf.extend_from_slice(&(vb.len() as u32).to_le_bytes());
                    batch_buf.extend_from_slice(vb);
                    count += 1;
                    if batch_buf.len() >= 2 * 1024 * 1024 {
                        let send_buf = std::mem::replace(&mut batch_buf, Vec::with_capacity(2 * 1024 * 1024));
                        let _ = sink.send(Message::Binary(send_buf.into())).await;
                    }
                }
                if !batch_buf.is_empty() {
                    let _ = sink.send(Message::Binary(batch_buf.into())).await;
                }
                if let Ok(msg) = send_msg(&SyncMessage::KvBatchComplete { count: count as usize }) {
                    let _ = sink.send(msg).await;
                }
                if count > 0 {
                    tracing::info!("LAN sync: sent {} KV entries to client (since_ts={})", count, since_ts);
                } else {
                    tracing::debug!("LAN sync: KV sync — 0 entries changed since ts={}", since_ts);
                }
            }
            SyncMessage::RemoteRequest { cmd, args } => {
                // Whitelist allowed remote commands — reject unknown commands
                const ALLOWED_REMOTE_CMDS: &[&str] = &[
                    "SEC_SCRAPE", "FUNDAMENTALS", "FUNDAMENTALS_ONE",
                    "KRAKEN_BACKFILL", "CRYPTOCOMPARE",
                    "MT5_SYNC", "DARWIN_IMPORT", "FETCH_BARS",
                    "FINNHUB_NEWS", "ECON_CALENDAR", "CONGRESS_TRADES", "FRED_DATA",
                    "SEC_FILING", "EVSCRAPE",
                ];
                if !ALLOWED_REMOTE_CMDS.contains(&cmd.as_str()) {
                    tracing::warn!("LAN sync: rejected unknown remote command '{}'", cmd);
                    let msg_text = format!("Rejected: '{}' not in allowed command list", cmd);
                    if let Ok(msg) = send_msg(&SyncMessage::RemoteRequestDone { cmd, message: msg_text }) {
                        let _ = sink.send(msg).await;
                    }
                } else {
                    tracing::info!("LAN sync: client requested remote '{}' (args: {})", cmd, &args[..args.len().min(100)]);
                    // Append to remote command queue. Multiple commands can arrive
                    // rapidly (e.g., FETCH_BARS for 9 timeframes). Use append_to_queue
                    // for O(1) inserts instead of the old read-decompress-append-recompress-write
                    // pattern which was O(n²) under burst load.
                    let new_entry = serde_json::json!({ "cmd": cmd, "args": args });
                    let entry_json = serde_json::to_string(&new_entry).unwrap_or_default();
                    let _ = cache.append_to_queue("lan:remote_queue", &entry_json);
                    let msg_text = format!("Remote '{}' accepted — executing on server", cmd);
                    if let Ok(msg) = send_msg(&SyncMessage::RemoteRequestDone { cmd, message: msg_text }) {
                        let _ = sink.send(msg).await;
                    }
                }
            }
            SyncMessage::RequestTableSync { tables } => {
                // Generic table sync: export each requested table as zstd-compressed JSON
                // tables is Vec<(table_name, since_ts)> — since_ts=0 means full sync
                let cache_clone = cache.clone();
                let table_results = tokio::task::spawn_blocking(move || {
                    let mut results: Vec<(String, String, usize)> = Vec::new();
                    if let Ok(conn) = cache_clone.read_connection() {
                        for (tbl, since_ts) in &tables {
                            if !SYNCABLE_TABLES.contains(&tbl.as_str()) {
                                tracing::warn!("LAN sync: table '{}' not in whitelist, skipping", tbl);
                                continue;
                            }
                            match export_table_as_json_since(&conn, tbl, *since_ts) {
                                Ok((json, row_count)) => {
                                    let compressed = zstd::encode_all(json.as_bytes(), 3)
                                        .unwrap_or_else(|_| json.into_bytes());
                                    let encoded = base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD, &compressed,
                                    );
                                    results.push((tbl.clone(), encoded, row_count));
                                }
                                Err(e) => {
                                    tracing::warn!("LAN sync: export table '{}' failed: {}", tbl, e);
                                }
                            }
                        }
                    }
                    results
                }).await.unwrap_or_default();

                let mut sent_count = 0usize;
                for (tbl, encoded, row_count) in &table_results {
                    if *row_count == 0 { continue; } // skip empty tables
                    if let Ok(msg) = send_msg(&SyncMessage::TableSyncData {
                        table: tbl.clone(),
                        rows_json: encoded.clone(),
                    }) {
                        let _ = sink.send(msg).await;
                    }
                    tracing::info!("LAN sync: sent table '{}' ({} rows)", tbl, row_count);
                    sent_count += 1;
                }
                if let Ok(msg) = send_msg(&SyncMessage::TableSyncDone) {
                    let _ = sink.send(msg).await;
                }
                if sent_count > 0 {
                    tracing::info!("LAN sync: sent {} table(s) to client", sent_count);
                }
            }
            SyncMessage::Ping => {
                if let Ok(msg) = send_msg(&SyncMessage::Pong) {
                    let _ = sink.send(msg).await;
                }
            }
            _ => {}
        }
    }

    let mut s = status.lock().await;
    s.clients = s.clients.saturating_sub(1);
    s.client_ips.retain(|ip| ip != client_ip);
    // Persist updated client list
    let ips_json = serde_json::to_string(&s.client_ips).unwrap_or_default();
    let _ = cache.put_kv("lan:server:clients", &ips_json);
    tracing::info!("LAN sync: client {} disconnected", client_ip);
}

fn build_cache_meta(cache: &SqliteCache) -> Vec<CacheMeta> {
    match cache.get_all_cache_meta() {
        Ok(map) => {
            let now = chrono::Utc::now().timestamp();
            map.into_iter().map(|(key, (age_secs, bar_count))| {
                CacheMeta {
                    key,
                    timestamp: now - age_secs, // convert age back to absolute timestamp
                    bar_count: Some(bar_count),
                }
            }).collect()
        }
        Err(e) => {
            tracing::warn!("LAN sync: failed to read cache meta: {e}");
            Vec::new()
        }
    }
}

// ── Client ─────────────────────────────────────────────────────────

pub struct LanSyncClient {
    task: Option<tokio::task::JoinHandle<()>>,
    status: Arc<TokioMutex<SyncStatus>>,
}

impl LanSyncClient {
    pub async fn connect(
        cache: Arc<SqliteCache>,
        host: &str,
        port: u16,
        passphrase: &str,
    ) -> Result<(Self, tokio::sync::mpsc::UnboundedSender<String>), String> {
        let secret = derive_secret(passphrase);
        let url = format!("wss://{host}:{port}");

        // Build TLS connector that accepts self-signed certs (LAN only)
        let tls_connector = build_tls_connector()?;
        let connector = tokio_tungstenite::Connector::NativeTls(tls_connector);

        let (ws, _) = tokio_tungstenite::connect_async_tls_with_config(
            &url, Some(ws_config()), false, Some(connector),
        )
            .await
            .map_err(|e| format!("Connect to {url} failed: {e}"))?;

        // Log peer certificate fingerprint for diagnostics (no pinning).
        // The server generates a new ephemeral self-signed cert on every startup,
        // so TOFU pinning would break on every normal server restart. Authentication
        // is handled by the PBKDF2-HMAC-SHA256 passphrase challenge — the TLS layer
        // provides transport encryption, not identity verification.
        let peer_fingerprint = match ws.get_ref() {
            tokio_tungstenite::MaybeTlsStream::NativeTls(tls_stream) => {
                match tls_stream.get_ref().peer_certificate() {
                    Ok(Some(cert)) => cert.to_der().ok().map(|der| compute_sha256_fingerprint(&der)),
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(ref fp) = peer_fingerprint {
            tracing::info!("LAN sync: server certificate fingerprint: {fp}");
        }

        let status = Arc::new(TokioMutex::new(SyncStatus {
            mode: "client".into(),
            connected: true,
            host: host.to_string(),
            port,
            cert_fingerprint: peer_fingerprint.unwrap_or_default(),
            ..Default::default()
        }));

        let (mut sink, mut stream_rx) = ws.split();

        // 1. Wait for AuthChallenge
        let challenge_bytes = match tokio::time::timeout(std::time::Duration::from_secs(10), stream_rx.next()).await {
            Ok(Some(Ok(msg))) => {
                match parse_msg(&msg) {
                    Ok(SyncMessage::AuthChallenge { challenge }) => {
                        hex_decode(&challenge).ok_or("Invalid challenge hex")?
                    }
                    _ => return Err("Expected AuthChallenge".into()),
                }
            }
            _ => return Err("Timeout waiting for AuthChallenge".into()),
        };

        // 2. Send Auth response
        let response = hmac_hex(&challenge_bytes, &secret);
        sink.send(send_msg(&SyncMessage::Auth { response })?)
            .await
            .map_err(|e| format!("Send auth failed: {e}"))?;

        // 3. Wait for AuthOk
        match tokio::time::timeout(std::time::Duration::from_secs(10), stream_rx.next()).await {
            Ok(Some(Ok(msg))) => {
                match parse_msg(&msg) {
                    Ok(SyncMessage::AuthOk) => {}
                    Ok(SyncMessage::AuthFail { reason }) => {
                        return Err(format!("Auth failed: {reason}"));
                    }
                    _ => return Err("Unexpected message after auth".into()),
                }
            }
            _ => return Err("Timeout waiting for auth result".into()),
        }

        tracing::info!("LAN sync: connected to {url}, authenticated");

        // Channel for sending remote requests from broker task → LAN sync WebSocket
        let (remote_tx, remote_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let status_clone = status.clone();
        let task = tokio::spawn(async move {
            if let Err(e) = client_sync_loop(&cache, &mut sink, &mut stream_rx, remote_rx).await {
                tracing::warn!("LAN sync client error: {e}");
            }
            let mut s = status_clone.lock().await;
            s.connected = false;
            tracing::info!("LAN sync: client disconnected");
        });

        Ok((Self { task: Some(task), status }, remote_tx))
    }

    /// Wait for the client sync task to finish (disconnect or error).
    /// Used by auto-reconnect loop to detect when reconnection is needed.
    pub async fn wait(mut self) {
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            tracing::info!("LAN sync client disconnected");
        }
    }

    pub async fn status(&self) -> SyncStatus {
        self.status.lock().await.clone()
    }
}

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;
type WsStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

async fn client_sync_loop(
    cache: &Arc<SqliteCache>,
    sink: &mut WsSink,
    stream: &mut WsStream,
    mut remote_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
) -> Result<(), String> {
    // 4. Send RequestMeta
    sink.send(send_msg(&SyncMessage::RequestMeta)?)
        .await
        .map_err(|e| format!("Send RequestMeta failed: {e}"))?;

    // 5. Receive Metadata
    let remote_meta = match read_next(stream).await? {
        SyncMessage::Metadata { entries } => entries,
        other => return Err(format!("Expected Metadata, got {:?}", other)),
    };

    // 6. Diff against local cache
    let local_meta = match cache.get_all_cache_meta() {
        Ok(m) => m,
        Err(e) => return Err(format!("Local cache meta failed: {e}")),
    };

    let now = chrono::Utc::now().timestamp();
    let mut needed: Vec<String> = Vec::new();
    for entry in &remote_meta {
        let local_ts = local_meta.get(&entry.key).map(|(age, _)| now - age);
        match local_ts {
            Some(ts) if ts >= entry.timestamp => {} // local is same or newer
            _ => needed.push(entry.key.clone()),     // missing or outdated
        }
    }

    if needed.is_empty() {
        tracing::info!("LAN sync: local cache is up to date ({} entries checked)", remote_meta.len());
    } else {
        tracing::info!("LAN sync: requesting {} entries from server", needed.len());

        // 7. Request missing entries
        sink.send(send_msg(&SyncMessage::RequestEntries { keys: needed })?)
            .await
            .map_err(|e| format!("Send RequestEntries failed: {e}"))?;

        // 8. Receive entries until BatchComplete
        // Server sends binary frames (fast, no base64/JSON overhead) + text BatchComplete
        let mut total_received = 0usize;
        let mut total_bytes = 0usize;
        loop {
            match stream.next().await {
                Some(Ok(msg)) if msg.is_binary() => {
                    // Parse binary batch: [u32 key_len][key][i64 ts][u32 data_len][data] repeated
                    let buf = msg.into_data();
                    total_bytes += buf.len();
                    let mut pos = 0;
                    while pos + 4 <= buf.len() {
                        let prev_pos = pos;
                        let key_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                        pos += 4;
                        if key_len == 0 || key_len > MAX_KEY_LEN { tracing::warn!("LAN sync: key_len {key_len} invalid"); break; }
                        if pos + key_len + 8 + 4 > buf.len() { break; }
                        let key = String::from_utf8_lossy(&buf[pos..pos+key_len]).to_string();
                        pos += key_len;
                        let ts = i64::from_le_bytes(buf[pos..pos+8].try_into().unwrap_or([0;8]));
                        pos += 8;
                        let data_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                        pos += 4;
                        if data_len > MAX_DATA_LEN { tracing::warn!("LAN sync: data_len {data_len} exceeds limit"); break; }
                        if pos + data_len > buf.len() { break; }
                        let data = &buf[pos..pos+data_len];
                        pos += data_len;

                        let bar_count = extract_bar_count(data);
                        if let Err(e) = cache.put_raw_bar_entry(&key, data, ts, bar_count) {
                            tracing::warn!("LAN sync: failed to write {key}: {e}");
                        }
                        total_received += 1;
                        if pos == prev_pos { tracing::warn!("LAN sync: binary parse stalled at pos {pos}"); break; }
                    }
                }
                Some(Ok(msg)) if msg.is_text() => {
                    // Check for BatchComplete
                    if let Ok(SyncMessage::BatchComplete { count }) = parse_msg(&msg) {
                        if total_received != count {
                            tracing::warn!("LAN sync: batch count mismatch — server sent {count}, received {total_received}");
                        }
                        tracing::info!("LAN sync: received {total_received} entries ({:.1} MB)", total_bytes as f64 / 1024.0 / 1024.0);
                        break;
                    }
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(format!("WebSocket error: {e}")),
                None => return Err("Connection closed during sync".into()),
            }
        }
    }

    // 8b. Request DARWIN data (accounts, deals, positions)
    sink.send(send_msg(&SyncMessage::RequestDarwinData)?)
        .await.map_err(|e| format!("Send RequestDarwinData failed: {e}"))?;

    match read_next(stream).await? {
        SyncMessage::DarwinData { data, accounts, deals, positions } => {
            // Decompress and import
            let compressed = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data)
                .map_err(|e| format!("Base64 decode DARWIN data failed: {e}"))?;
            let json_bytes = zstd::decode_all(std::io::Cursor::new(&compressed))
                .unwrap_or_else(|_| compressed.clone());
            let json = String::from_utf8_lossy(&json_bytes);

            let cache_clone = cache.clone();
            let json_owned = json.to_string();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(conn) = cache_clone.connection() {
                    match crate::core::darwin::import_darwin_data(&conn, &json_owned) {
                        Ok((na, nd, np)) => {
                            tracing::info!("LAN sync: imported DARWIN data — {} accounts, {} deals, {} positions", na, nd, np);
                        }
                        Err(e) => { tracing::warn!("LAN sync: DARWIN import failed: {e}"); }
                    }
                }
            }).await;
            tracing::info!("LAN sync: DARWIN data received ({} accounts, {} deals, {} positions)", accounts, deals, positions);
        }
        other => { tracing::warn!("LAN sync: expected DarwinData, got {:?}", other); }
    }

    // 8c. Request KV cache entries (fundamentals, news, SEC filings, FRED, etc.)
    // Incremental: send last known KV sync timestamp; 0 = full sync
    let kv_since_ts = cache.get_sync_ts("kv_cache");
    let kv_local_count = cache.kv_count();
    tracing::info!("LAN sync: requesting KV data (since_ts={}, local_count={})", kv_since_ts, kv_local_count);
    sink.send(send_msg(&SyncMessage::RequestKvData { since_ts: kv_since_ts })?)
        .await.map_err(|e| format!("Send RequestKvData failed: {e}"))?;

    let mut kv_count = 0usize;
    loop {
        match stream.next().await {
            Some(Ok(msg)) if msg.is_binary() => {
                // Parse KV batch: [u32 key_len][key][u32 val_len][val] repeated
                let buf = msg.into_data();
                let mut pos = 0;
                while pos + 4 <= buf.len() {
                    let prev_pos = pos;
                    let key_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                    pos += 4;
                    if key_len == 0 || key_len > MAX_KEY_LEN { tracing::warn!("LAN sync: KV key_len {key_len} invalid"); break; }
                    if pos + key_len + 4 > buf.len() { break; }
                    let key = String::from_utf8_lossy(&buf[pos..pos+key_len]).to_string();
                    pos += key_len;
                    let val_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                    pos += 4;
                    if val_len > MAX_DATA_LEN { tracing::warn!("LAN sync: KV val_len {val_len} exceeds limit"); break; }
                    if pos + val_len > buf.len() { break; }
                    let blob = &buf[pos..pos+val_len];
                    pos += val_len;
                    let _ = cache.put_kv_compressed(&key, blob);
                    kv_count += 1;
                    if pos == prev_pos { break; }
                }
            }
            Some(Ok(msg)) if msg.is_text() => {
                if let Ok(SyncMessage::KvBatchComplete { count }) = parse_msg(&msg) {
                    tracing::info!("LAN sync: received {} KV entries (incremental since_ts={})", count, kv_since_ts);
                    break;
                }
            }
            Some(Ok(_)) => continue,
            Some(Err(e)) => return Err(format!("KV sync error: {e}")),
            None => return Err("Connection closed during KV sync".into()),
        }
    }
    // Safety: if incremental returned 0 but client table is empty, do full re-sync
    if kv_count == 0 && kv_since_ts > 0 && kv_local_count == 0 {
        tracing::warn!("LAN sync: KV incremental returned 0 rows but local is empty — triggering full sync");
        sink.send(send_msg(&SyncMessage::RequestKvData { since_ts: 0 })?)
            .await.map_err(|e| format!("Send RequestKvData (full) failed: {e}"))?;
        loop {
            match stream.next().await {
                Some(Ok(msg)) if msg.is_binary() => {
                    let buf = msg.into_data();
                    let mut pos = 0;
                    while pos + 4 <= buf.len() {
                        let prev_pos = pos;
                        let key_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                        pos += 4;
                        if key_len == 0 || key_len > MAX_KEY_LEN { break; }
                        if pos + key_len + 4 > buf.len() { break; }
                        let key = String::from_utf8_lossy(&buf[pos..pos+key_len]).to_string();
                        pos += key_len;
                        let val_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                        pos += 4;
                        if val_len > MAX_DATA_LEN { break; }
                        if pos + val_len > buf.len() { break; }
                        let blob = &buf[pos..pos+val_len];
                        pos += val_len;
                        let _ = cache.put_kv_compressed(&key, blob);
                        kv_count += 1;
                        if pos == prev_pos { break; }
                    }
                }
                Some(Ok(msg)) if msg.is_text() => {
                    if let Ok(SyncMessage::KvBatchComplete { count }) = parse_msg(&msg) {
                        tracing::info!("LAN sync: full KV re-sync received {} entries", count);
                        break;
                    }
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(format!("KV full sync error: {e}")),
                None => return Err("Connection closed during KV full sync".into()),
            }
        }
    }
    // Update sync_state timestamp after successful KV import
    let new_kv_ts = chrono::Utc::now().timestamp();
    let _ = cache.set_sync_ts("kv_cache", new_kv_ts);
    tracing::info!("LAN sync: imported {} KV cache entries (fundamentals, news, SEC, FRED, etc.)", kv_count);

    // 8d. Request generic table sync (SEC, fundamentals, equity snapshots, etc.)
    // Build incremental request: each table gets its last sync timestamp
    let table_requests: Vec<(String, i64)> = SYNCABLE_TABLES.iter().map(|tbl| {
        let since_ts = cache.get_sync_ts(&format!("table:{}", tbl));
        tracing::info!("LAN sync: table '{}' since_ts={}", tbl, since_ts);
        (tbl.to_string(), since_ts)
    }).collect();
    sink.send(send_msg(&SyncMessage::RequestTableSync {
        tables: table_requests,
    })?)
        .await.map_err(|e| format!("Send RequestTableSync failed: {e}"))?;

    // Track which tables need full re-sync (incremental returned 0 but local is empty)
    let mut tables_needing_full_sync: Vec<String> = Vec::new();
    let mut table_count = 0usize;
    loop {
        match read_next(stream).await? {
            SyncMessage::TableSyncData { table, rows_json } => {
                // Validate table name against whitelist (defense in depth — server is trusted but verify)
                if !SYNCABLE_TABLES.contains(&table.as_str()) {
                    tracing::warn!("LAN sync client: server sent non-whitelisted table '{}', skipping", table);
                    table_count += 1;
                    continue;
                }
                // Decompress zstd + base64
                let compressed = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &rows_json)
                    .map_err(|e| format!("Base64 decode table '{}': {e}", table))?;
                let json_bytes = zstd::decode_all(std::io::Cursor::new(&compressed))
                    .unwrap_or_else(|_| compressed.clone());
                let json = String::from_utf8_lossy(&json_bytes).to_string();

                // Check if this was an incremental sync that returned empty
                let tbl_since_ts = cache.get_sync_ts(&format!("table:{}", table));
                let rows: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap_or_default();
                let row_count = rows.len();

                if row_count == 0 && tbl_since_ts > 0 {
                    // Check if local table is empty — if so, need full sync
                    let local_count = {
                        let cache_clone = cache.clone();
                        let tbl = table.clone();
                        tokio::task::spawn_blocking(move || {
                            if let Ok(conn) = cache_clone.connection() {
                                conn.query_row(
                                    &format!("SELECT COUNT(*) FROM {}", tbl),
                                    [], |r| r.get::<_, i64>(0)
                                ).unwrap_or(-1) // -1 means table doesn't exist
                            } else { 0 }
                        }).await.unwrap_or(0)
                    };
                    if local_count <= 0 {
                        tracing::warn!("LAN sync: table '{}' incremental returned 0 but local is empty — will full sync", table);
                        tables_needing_full_sync.push(table.clone());
                    } else {
                        tracing::info!("LAN sync: table '{}' up to date ({} local rows)", table, local_count);
                    }
                } else {
                    let cache_clone = cache.clone();
                    let tbl = table.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Ok(conn) = cache_clone.connection() {
                            match import_table_from_json(&conn, &tbl, &json) {
                                Ok(n) => tracing::info!("LAN sync: imported {} rows into '{}'", n, tbl),
                                Err(e) => tracing::warn!("LAN sync: import '{}' failed: {}", tbl, e),
                            }
                        }
                    }).await;
                }
                // Update sync_state for this table
                let new_ts = chrono::Utc::now().timestamp();
                let _ = cache.set_sync_ts(&format!("table:{}", table), new_ts);
                table_count += 1;
            }
            SyncMessage::TableSyncDone => {
                tracing::info!("LAN sync: table sync complete ({} tables)", table_count);
                break;
            }
            other => {
                tracing::warn!("LAN sync: expected TableSyncData/TableSyncDone, got {:?}", other);
                break;
            }
        }
    }

    // Full re-sync for tables that returned 0 rows but have empty local data
    if !tables_needing_full_sync.is_empty() {
        tracing::info!("LAN sync: triggering full sync for {} table(s): {:?}", tables_needing_full_sync.len(), tables_needing_full_sync);
        let full_sync_requests: Vec<(String, i64)> = tables_needing_full_sync.iter()
            .map(|tbl| (tbl.clone(), 0i64))
            .collect();
        sink.send(send_msg(&SyncMessage::RequestTableSync {
            tables: full_sync_requests,
        })?)
            .await.map_err(|e| format!("Send RequestTableSync (full) failed: {e}"))?;

        loop {
            match read_next(stream).await? {
                SyncMessage::TableSyncData { table, rows_json } => {
                    let compressed = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &rows_json)
                        .map_err(|e| format!("Base64 decode table '{}': {e}", table))?;
                    let json_bytes = zstd::decode_all(std::io::Cursor::new(&compressed))
                        .unwrap_or_else(|_| compressed.clone());
                    let json = String::from_utf8_lossy(&json_bytes).to_string();

                    let cache_clone = cache.clone();
                    let tbl = table.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Ok(conn) = cache_clone.connection() {
                            match import_table_from_json(&conn, &tbl, &json) {
                                Ok(n) if n > 0 => tracing::info!("LAN sync: full re-sync imported {} rows into '{}'", n, tbl),
                                Ok(_) => {}
                                Err(e) => tracing::warn!("LAN sync: full re-sync import '{}' failed: {}", tbl, e),
                            }
                        }
                    }).await;
                    let new_ts = chrono::Utc::now().timestamp();
                    let _ = cache.set_sync_ts(&format!("table:{}", table), new_ts);
                }
                SyncMessage::TableSyncDone => {
                    tracing::debug!("LAN sync: full table re-sync complete");
                    break;
                }
                other => {
                    tracing::warn!("LAN sync: full re-sync unexpected: {:?}", other);
                    break;
                }
            }
        }
    }

    // 9. Listen for incremental updates + periodic re-sync
    // State flag: when true, binary frames are KV data (key+val); when false, bar data (key+ts+data)
    let mut expecting_kv_binary = false;
    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));
    // Periodic re-sync: pull new bars/KV/tables every 60s.
    // With hash-based KV dedup on the server, most re-syncs send zero entries.
    // 60s is sufficient since positions/watchlist update infrequently.
    let mut resync_interval = tokio::time::interval(std::time::Duration::from_secs(60));
    resync_interval.tick().await; // skip the first immediate tick (initial sync just completed)
    let mut resync_count: u64 = 0;
    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() { break; }
                        if msg.is_pong() { continue; }
                        if msg.is_binary() {
                            let buf = msg.into_data();
                            if expecting_kv_binary {
                                // KV binary: [u32 key_len][key][u32 val_len][val] repeated
                                let mut pos = 0;
                                while pos + 4 <= buf.len() {
                                    let prev_pos = pos;
                                    let key_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                                    pos += 4;
                                    if key_len == 0 || key_len > MAX_KEY_LEN { break; }
                                    if pos + key_len + 4 > buf.len() { break; }
                                    let key = String::from_utf8_lossy(&buf[pos..pos+key_len]).to_string();
                                    pos += key_len;
                                    let val_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                                    pos += 4;
                                    if val_len > MAX_DATA_LEN { break; }
                                    if pos + val_len > buf.len() { break; }
                                    let blob = &buf[pos..pos+val_len];
                                    pos += val_len;
                                    let _ = cache.put_kv_compressed(&key, blob);
                                    if pos == prev_pos { break; }
                                }
                            } else {
                                // Bar binary: [u32 key_len][key][i64 ts][u32 data_len][data] repeated
                                let mut pos = 0;
                                while pos + 4 <= buf.len() {
                                    let prev_pos = pos;
                                    let key_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                                    pos += 4;
                                    if key_len == 0 || key_len > MAX_KEY_LEN { tracing::warn!("LAN sync: incremental key_len {key_len} invalid"); break; }
                                    if pos + key_len + 8 + 4 > buf.len() { break; }
                                    let key = String::from_utf8_lossy(&buf[pos..pos+key_len]).to_string();
                                    pos += key_len;
                                    let ts = i64::from_le_bytes(buf[pos..pos+8].try_into().unwrap_or([0;8]));
                                    pos += 8;
                                    let data_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                                    pos += 4;
                                    if data_len > MAX_DATA_LEN { tracing::warn!("LAN sync: incremental data_len {data_len} exceeds limit"); break; }
                                    if pos + data_len > buf.len() { break; }
                                    let data = &buf[pos..pos+data_len];
                                    pos += data_len;
                                    let bar_count = extract_bar_count(data);
                                    let _ = cache.put_raw_bar_entry(&key, data, ts, bar_count);
                                    tracing::debug!("LAN sync: incremental update for {key}");
                                    if pos == prev_pos { break; }
                                }
                            }
                        } else if msg.is_text() {
                            match parse_msg(&msg) {
                                Ok(SyncMessage::Pong) => {}
                                Ok(SyncMessage::Ping) => {
                                    let _ = sink.send(send_msg(&SyncMessage::Pong)?).await;
                                }
                                Ok(SyncMessage::RemoteRequestDone { cmd, message }) => {
                                    tracing::info!("LAN sync: server completed '{}': {}", cmd, message);
                                    // Re-sync research tables incrementally
                                    let table_requests: Vec<(String, i64)> = SYNCABLE_TABLES.iter().map(|tbl| {
                                        let since_ts = cache.get_sync_ts(&format!("table:{}", tbl));
                                        (tbl.to_string(), since_ts)
                                    }).collect();
                                    let _ = sink.send(send_msg(&SyncMessage::RequestTableSync {
                                        tables: table_requests,
                                    })?).await;
                                    // Re-sync KV data incrementally — set flag so binary frames parse as KV
                                    expecting_kv_binary = true;
                                    let kv_since = cache.get_sync_ts("kv_cache");
                                    let _ = sink.send(send_msg(&SyncMessage::RequestKvData { since_ts: kv_since })?).await;
                                    // NOTE: DARWIN deals NOT re-synced here — analytics come via KV.
                                    tracing::info!("LAN sync: incremental re-sync triggered after '{}' completion", cmd);
                                }
                                Ok(SyncMessage::TableSyncData { table, rows_json }) => {
                                    // Decompress zstd + base64 (same as initial sync)
                                    if let Ok(compressed) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &rows_json) {
                                        let json_bytes = zstd::decode_all(std::io::Cursor::new(&compressed))
                                            .unwrap_or_else(|_| compressed.clone());
                                        let json = String::from_utf8_lossy(&json_bytes).to_string();
                                        if let Ok(conn) = cache.connection() {
                                            match import_table_from_json(&conn, &table, &json) {
                                                Ok(n) if n > 0 => tracing::info!("LAN sync: re-sync imported {n} rows into {table}"),
                                                Ok(_) => {} // 0 rows — don't log
                                                Err(e) => tracing::warn!("LAN sync: table re-import {table} failed: {e}"),
                                            }
                                        }
                                    } else {
                                        tracing::warn!("LAN sync: table re-sync base64 decode failed for {table}");
                                    }
                                    // Update sync_state for this table
                                    let new_ts = chrono::Utc::now().timestamp();
                                    let _ = cache.set_sync_ts(&format!("table:{}", table), new_ts);
                                }
                                Ok(SyncMessage::TableSyncDone) => {
                                    tracing::debug!("LAN sync: table re-sync cycle complete");
                                }
                                Ok(SyncMessage::KvBatchComplete { count }) => {
                                    // KV re-sync complete — update sync_state, reset binary mode
                                    expecting_kv_binary = false;
                                    let new_ts = chrono::Utc::now().timestamp();
                                    let _ = cache.set_sync_ts("kv_cache", new_ts);
                                    if count > 0 {
                                        tracing::info!("LAN sync: KV re-sync received {} entries", count);
                                    } else {
                                        tracing::debug!("LAN sync: KV re-sync received 0 entries");
                                    }
                                }
                                Ok(SyncMessage::DarwinData { data, accounts: _, deals: _, positions: _ }) => {
                                    // Decode: base64 → zstd decompress → JSON (same as initial sync)
                                    if let Ok(compressed) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data) {
                                        let json_bytes = zstd::decode_all(std::io::Cursor::new(&compressed))
                                            .unwrap_or_else(|_| compressed.clone());
                                        let json = String::from_utf8_lossy(&json_bytes);
                                        if let Ok(conn) = cache.connection() {
                                            match crate::core::darwin::import_darwin_data(&conn, &json) {
                                                Ok((a, d, p)) => tracing::info!("LAN sync: DARWIN re-sync: {a} accounts, {d} deals, {p} positions"),
                                                Err(e) => tracing::warn!("LAN sync: DARWIN re-import failed: {e}"),
                                            }
                                        }
                                    } else {
                                        tracing::warn!("LAN sync: DARWIN re-sync base64 decode failed");
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!("LAN sync: stream error: {e}");
                        break;
                    }
                    None => break,
                }
            }
            // Forward remote requests from broker task to server (or handle client-side resyncs)
            Some(request_json) = remote_rx.recv() => {
                let (cmd, args) = request_json.split_once(':').unwrap_or((&request_json, ""));
                match cmd {
                    "RESYNC_BARS" => {
                        // Client-side: re-request all bar metadata and entries from server
                        tracing::info!("LAN sync: resync bars requested — sending RequestMeta");
                        let _ = sink.send(send_msg(&SyncMessage::RequestMeta)?).await;
                    }
                    "RESYNC_DARWIN" => {
                        // Client-side: re-request DARWIN data from server
                        tracing::info!("LAN sync: resync DARWIN requested");
                        let _ = sink.send(send_msg(&SyncMessage::RequestDarwinData)?).await;
                    }
                    _ => {
                        // Forward to server as RemoteRequest
                        if let Ok(msg) = send_msg(&SyncMessage::RemoteRequest {
                            cmd: cmd.to_string(), args: args.to_string(),
                        }) {
                            let _ = sink.send(msg).await;
                            tracing::info!("LAN sync: forwarded remote request '{}' to server", cmd);
                        }
                    }
                }
            }
            _ = ping_interval.tick() => {
                if sink.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
            _ = resync_interval.tick() => {
                resync_count += 1;
                // Periodic re-sync: pull only CHANGED data from server.
                tracing::trace!("LAN sync: periodic re-sync #{resync_count}");

                // Bars: delta metadata (only entries updated since last sync)
                let bar_since = cache.get_sync_ts("bar_cache");
                let _ = cache.set_sync_ts("bar_cache", chrono::Utc::now().timestamp());
                let _ = sink.send(send_msg(&SyncMessage::RequestMetaSince { since_ts: bar_since })?).await;

                // KV: incremental since last sync timestamp
                expecting_kv_binary = true;
                let kv_since = cache.get_sync_ts("kv_cache");
                let _ = sink.send(send_msg(&SyncMessage::RequestKvData { since_ts: kv_since })?).await;

                // Research tables: only every 5th cycle (~5 min) — tables change rarely
                if resync_count % 5 == 0 {
                    let table_requests: Vec<(String, i64)> = SYNCABLE_TABLES.iter().map(|tbl| {
                        let since_ts = cache.get_sync_ts(&format!("table:{}", tbl));
                        (tbl.to_string(), since_ts)
                    }).collect();
                    let _ = sink.send(send_msg(&SyncMessage::RequestTableSync { tables: table_requests })?).await;
                }
                // NOTE: DARWIN deal import removed from periodic resync.
                // All DARWIN analytics (positions, VaR, exposure, etc.) now sync via KV cache.
                // The 45K deal import was 20+ seconds of CPU and produced wrong results on client.
                // DARWIN deals only imported during initial sync (for table completeness).
            }
        }
    }

    Ok(())
}

async fn read_next(stream: &mut WsStream) -> Result<SyncMessage, String> {
    // 5-minute timeout: DARWIN export for 45K+ deals requires serialization + zstd
    // compression + base64 encoding which can take 60-120s on large databases.
    // The previous 60s timeout caused "Timeout waiting for message" during initial sync.
    match tokio::time::timeout(std::time::Duration::from_secs(300), stream.next()).await {
        Ok(Some(Ok(msg))) => parse_msg(&msg),
        Ok(Some(Err(e))) => Err(format!("WebSocket error: {e}")),
        Ok(None) => Err("Connection closed".into()),
        Err(_) => Err("Timeout waiting for message (5min)".into()),
    }
}

/// Extract bar_count from a compressed blob by decompressing just enough to read the header.
/// Returns 0 if extraction fails (non-binary format, etc.).
fn extract_bar_count(compressed: &[u8]) -> i64 {
    match zstd::decode_all(std::io::Cursor::new(compressed)) {
        Ok(decompressed) => {
            if decompressed.len() >= 8 && &decompressed[0..4] == b"TTBR" {
                u32::from_le_bytes(
                    decompressed[4..8].try_into().unwrap_or([0; 4])
                ) as i64
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // ── SyncMessage serialization ─────────────────────────────────

    #[test]
    fn ping_serializes_to_tagged_json() {
        let json = serde_json::to_string(&SyncMessage::Ping).unwrap();
        assert_eq!(json, r#"{"type":"Ping"}"#);
    }

    #[test]
    fn auth_challenge_roundtrips() {
        let msg = SyncMessage::AuthChallenge {
            challenge: "abc123def456".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SyncMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            SyncMessage::AuthChallenge { challenge } => {
                assert_eq!(challenge, "abc123def456");
            }
            other => panic!("Expected AuthChallenge, got {other:?}"),
        }
    }

    #[test]
    fn remote_request_serializes_with_cmd_and_args() {
        let msg = SyncMessage::RemoteRequest {
            cmd: "SEC_SCRAPE".into(),
            args: r#"{"ticker":"AAPL"}"#.into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "RemoteRequest");
        assert_eq!(v["cmd"], "SEC_SCRAPE");
        assert_eq!(v["args"], r#"{"ticker":"AAPL"}"#);
    }

    // ── CacheMeta serialization ───────────────────────────────────

    #[test]
    fn cache_meta_roundtrips() {
        let meta = CacheMeta {
            key: "EURUSD_H1".into(),
            timestamp: 1700000000,
            bar_count: Some(5000),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: CacheMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key, "EURUSD_H1");
        assert_eq!(parsed.timestamp, 1700000000);
        assert_eq!(parsed.bar_count, Some(5000));
    }

    // ── derive_secret ─────────────────────────────────────────────

    #[test]
    fn derive_secret_is_deterministic() {
        let a = derive_secret("my-passphrase");
        let b = derive_secret("my-passphrase");
        assert_eq!(a, b);
        // Must not be all zeros
        assert_ne!(a, [0u8; 32]);
    }

    #[test]
    fn derive_secret_differs_for_different_passphrases() {
        let a = derive_secret("passphrase-one");
        let b = derive_secret("passphrase-two");
        assert_ne!(a, b);
    }

    // ── hmac_hex ──────────────────────────────────────────────────

    #[test]
    fn hmac_hex_is_consistent() {
        let secret = derive_secret("test-secret");
        let h1 = hmac_hex(b"challenge-data", &secret);
        let h2 = hmac_hex(b"challenge-data", &secret);
        assert_eq!(h1, h2);
        // HMAC-SHA256 produces 64 hex chars
        assert_eq!(h1.len(), 64);
        // All chars are lowercase hex
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ── hex_encode / hex_decode ───────────────────────────────────

    #[test]
    fn hex_encode_decode_roundtrip() {
        let original: Vec<u8> = (0u8..=255).collect();
        let encoded = hex_encode(&original);
        let decoded = hex_decode(&encoded).expect("valid hex should decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn hex_decode_rejects_invalid_hex() {
        // Odd length
        assert!(hex_decode("abc").is_none());
        // Invalid hex chars
        assert!(hex_decode("zzzz").is_none());
    }
}
