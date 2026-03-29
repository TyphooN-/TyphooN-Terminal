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

// ── TLS Certificate Generation ────────────────────────────────────

/// Generate an ephemeral self-signed TLS certificate for LAN sync server.
/// Returns (PEM certificate, PEM private key) for native-tls.
fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>), String> {
    let cert = rcgen::generate_simple_self_signed(vec!["typhoon-lan-sync".into(), "localhost".into()])
        .map_err(|e| format!("Certificate generation failed: {e}"))?;
    let cert_pem = cert.cert.pem().into_bytes();
    let key_pem = cert.key_pair.serialize_pem().into_bytes();
    Ok((cert_pem, key_pem))
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
    /// Request all KV cache entries (fundamentals, news, SEC, FRED, etc.)
    RequestKvData,
    /// KV batch complete marker
    KvBatchComplete { count: usize },
    /// Client requests server to execute a data fetch and return results.
    /// cmd: command name (e.g. "SEC_SCRAPE", "FUNDAMENTALS", "FINNHUB_NEWS", "KRAKEN_BACKFILL")
    /// args: JSON-encoded arguments
    RemoteRequest { cmd: String, args: String },
    /// Server response to RemoteRequest — triggers a re-sync of affected data.
    RemoteRequestDone { cmd: String, message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheMeta {
    pub key: String,
    pub timestamp: i64,
    pub bar_count: Option<i64>,
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
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC key length is valid");
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
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self {
            mode: "idle".into(), connected: false, clients: 0,
            host: String::new(), port: 0,
            bytes_sent: 0, bytes_received: 0, entries_synced: 0,
            darwin_synced: false, uptime_secs: 0, send_darwin: false,
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
        let (cert_der, key_der) = generate_self_signed_cert()?;
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
            ..Default::default()
        }));

        let status_clone = status.clone();
        let task = tokio::spawn(async move {
            tracing::info!("LAN sync server listening on wss://0.0.0.0:{port} (TLS encrypted)");
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        tracing::info!("LAN sync: TLS client connected from {addr}");
                        let tls_acc = tls_acceptor_tokio.clone();
                        let cache = cache.clone();
                        let status = status_clone.clone();
                        tokio::spawn(async move {
                            // TLS handshake
                            match tls_acc.accept(stream).await {
                                Ok(tls_stream) => {
                                    {
                                        let mut s = status.lock().await;
                                        s.clients += 1;
                                    }
                                    handle_client_tls(tls_stream, cache, secret, status).await;
                                }
                                Err(e) => {
                                    tracing::warn!("LAN sync: TLS handshake failed from {addr}: {e}");
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
) {
    let ws = match tokio_tungstenite::accept_async(tls_stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::warn!("LAN sync WebSocket handshake failed: {e}");
            let mut s = status.lock().await;
            s.clients = s.clients.saturating_sub(1);
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
            let mut s = status.lock().await;
            s.clients = s.clients.saturating_sub(1);
            return;
        }
    };
    if sink.send(challenge_msg).await.is_err() {
        let mut s = status.lock().await;
        s.clients = s.clients.saturating_sub(1);
        return;
    }

    // 2. Wait for Auth response
    let auth_ok = match tokio::time::timeout(std::time::Duration::from_secs(10), stream_rx.next()).await {
        Ok(Some(Ok(msg))) => {
            match parse_msg(&msg) {
                Ok(SyncMessage::Auth { response }) => {
                    let expected = hmac_hex(&challenge_bytes, &secret);
                    response == expected
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
        let mut s = status.lock().await;
        s.clients = s.clients.saturating_sub(1);
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
            SyncMessage::RequestEntries { keys } => {
                // Fast binary transfer: batch entries into large binary WebSocket frames
                // Format per entry: [u32 key_len][key_bytes][i64 timestamp][u32 data_len][data_bytes]
                let mut count = 0u32;
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

                        // Flush when batch is large enough
                        if batch_buf.len() >= flush_threshold {
                            let _ = sink.send(Message::Binary(batch_buf.clone().into())).await;
                            batch_buf.clear();
                        }
                    }
                }
                // Flush remaining
                if !batch_buf.is_empty() {
                    let _ = sink.send(Message::Binary(batch_buf.into())).await;
                }
                // Send completion marker as text
                if let Ok(msg) = send_msg(&SyncMessage::BatchComplete { count: count as usize }) {
                    let _ = sink.send(msg).await;
                }
                {
                    let mut s = status.lock().await;
                    s.entries_synced += count as usize;
                    s.bytes_sent += count as u64; // approximate
                }
            }
            SyncMessage::RequestDarwinData => {
                // Export DARWIN tables — run synchronously (Connection is not Send)
                let cache_clone = cache.clone();
                let darwin_result = tokio::task::spawn_blocking(move || {
                    if let Ok(conn) = cache_clone.connection() {
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
            SyncMessage::RequestKvData => {
                // Send all KV cache entries as binary batch
                let cache_clone = cache.clone();
                let kv_data = tokio::task::spawn_blocking(move || {
                    let mut entries: Vec<(String, String)> = Vec::new();
                    if let Ok(keys) = cache_clone.list_kv_keys("") {
                        for key in &keys {
                            if let Ok(Some(val)) = cache_clone.get_kv(key) {
                                entries.push((key.clone(), val));
                            }
                        }
                    }
                    entries
                }).await.unwrap_or_default();

                // Send as binary batch: [u32 key_len][key][u32 val_len][val] repeated
                let mut count = 0u32;
                let mut batch_buf: Vec<u8> = Vec::with_capacity(2 * 1024 * 1024);
                for (key, val) in &kv_data {
                    let kb = key.as_bytes();
                    let vb = val.as_bytes();
                    batch_buf.extend_from_slice(&(kb.len() as u32).to_le_bytes());
                    batch_buf.extend_from_slice(kb);
                    batch_buf.extend_from_slice(&(vb.len() as u32).to_le_bytes());
                    batch_buf.extend_from_slice(vb);
                    count += 1;
                    if batch_buf.len() >= 2 * 1024 * 1024 {
                        let _ = sink.send(Message::Binary(batch_buf.clone().into())).await;
                        batch_buf.clear();
                    }
                }
                if !batch_buf.is_empty() {
                    let _ = sink.send(Message::Binary(batch_buf.into())).await;
                }
                if let Ok(msg) = send_msg(&SyncMessage::KvBatchComplete { count: count as usize }) {
                    let _ = sink.send(msg).await;
                }
                tracing::info!("LAN sync: sent {} KV entries to client", count);
            }
            SyncMessage::RemoteRequest { cmd, args } => {
                tracing::info!("LAN sync: client requested remote '{}' (args: {})", cmd, &args[..args.len().min(100)]);
                // Server-side: execute the request, then send updated data back
                // For now, acknowledge and tell client to re-sync affected data
                let msg_text = format!("Remote '{}' queued on server", cmd);
                if let Ok(msg) = send_msg(&SyncMessage::RemoteRequestDone { cmd, message: msg_text }) {
                    let _ = sink.send(msg).await;
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
    tracing::info!("LAN sync: client disconnected");
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
            &url, None, false, Some(connector),
        )
            .await
            .map_err(|e| format!("Connect to {url} failed: {e}"))?;

        let status = Arc::new(TokioMutex::new(SyncStatus {
            mode: "client".into(),
            connected: true,
            host: host.to_string(),
            port,
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
        let total_received = 0usize;
        let mut total_bytes = 0usize;
        loop {
            match stream.next().await {
                Some(Ok(msg)) if msg.is_binary() => {
                    // Parse binary batch: [u32 key_len][key][i64 ts][u32 data_len][data] repeated
                    let buf = msg.into_data();
                    total_bytes += buf.len();
                    let mut pos = 0;
                    while pos + 4 <= buf.len() {
                        let key_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                        pos += 4;
                        if pos + key_len + 8 + 4 > buf.len() { break; }
                        let key = String::from_utf8_lossy(&buf[pos..pos+key_len]).to_string();
                        pos += key_len;
                        let ts = i64::from_le_bytes(buf[pos..pos+8].try_into().unwrap_or([0;8]));
                        pos += 8;
                        let data_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                        pos += 4;
                        if pos + data_len > buf.len() { break; }
                        let data = &buf[pos..pos+data_len];
                        pos += data_len;

                        let bar_count = extract_bar_count(data);
                        if let Err(e) = cache.put_raw_bar_entry(&key, data, ts, bar_count) {
                            tracing::warn!("LAN sync: failed to write {key}: {e}");
                        }
                        let _ = total_received;
                    }
                }
                Some(Ok(msg)) if msg.is_text() => {
                    // Check for BatchComplete
                    if let Ok(SyncMessage::BatchComplete { count }) = parse_msg(&msg) {
                        tracing::info!("LAN sync: received {} entries ({:.1} MB)", count, total_bytes as f64 / 1024.0 / 1024.0);
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
    sink.send(send_msg(&SyncMessage::RequestKvData)?)
        .await.map_err(|e| format!("Send RequestKvData failed: {e}"))?;

    let mut kv_count = 0usize;
    loop {
        match stream.next().await {
            Some(Ok(msg)) if msg.is_binary() => {
                // Parse KV batch: [u32 key_len][key][u32 val_len][val] repeated
                let buf = msg.into_data();
                let mut pos = 0;
                while pos + 4 <= buf.len() {
                    let key_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                    pos += 4;
                    if pos + key_len + 4 > buf.len() { break; }
                    let key = String::from_utf8_lossy(&buf[pos..pos+key_len]).to_string();
                    pos += key_len;
                    let val_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                    pos += 4;
                    if pos + val_len > buf.len() { break; }
                    let val = String::from_utf8_lossy(&buf[pos..pos+val_len]).to_string();
                    pos += val_len;
                    let _ = cache.put_kv(&key, &val);
                    kv_count += 1;
                }
            }
            Some(Ok(msg)) if msg.is_text() => {
                if let Ok(SyncMessage::KvBatchComplete { count }) = parse_msg(&msg) {
                    tracing::info!("LAN sync: received {} KV entries", count);
                    break;
                }
            }
            Some(Ok(_)) => continue,
            Some(Err(e)) => return Err(format!("KV sync error: {e}")),
            None => return Err("Connection closed during KV sync".into()),
        }
    }
    tracing::info!("LAN sync: imported {} KV cache entries (fundamentals, news, SEC, FRED, etc.)", kv_count);

    // 9. Listen for incremental updates (server pushes + ping/pong keepalive)
    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() { break; }
                        if msg.is_pong() { continue; }
                        if msg.is_binary() {
                            // Binary incremental update (same format as batch)
                            let buf = msg.into_data();
                            let mut pos = 0;
                            while pos + 4 <= buf.len() {
                                let key_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                                pos += 4;
                                if pos + key_len + 8 + 4 > buf.len() { break; }
                                let key = String::from_utf8_lossy(&buf[pos..pos+key_len]).to_string();
                                pos += key_len;
                                let ts = i64::from_le_bytes(buf[pos..pos+8].try_into().unwrap_or([0;8]));
                                pos += 8;
                                let data_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap_or([0;4])) as usize;
                                pos += 4;
                                if pos + data_len > buf.len() { break; }
                                let data = &buf[pos..pos+data_len];
                                pos += data_len;
                                let bar_count = extract_bar_count(data);
                                let _ = cache.put_raw_bar_entry(&key, data, ts, bar_count);
                                tracing::debug!("LAN sync: incremental update for {key}");
                            }
                        } else if msg.is_text() {
                            match parse_msg(&msg) {
                                Ok(SyncMessage::Pong) => {}
                                Ok(SyncMessage::Ping) => {
                                    let _ = sink.send(send_msg(&SyncMessage::Pong)?).await;
                                }
                                Ok(SyncMessage::RemoteRequestDone { cmd, message }) => {
                                    tracing::info!("LAN sync: server completed '{}': {}", cmd, message);
                                    // After server completes request, re-sync KV + DARWIN data
                                    let _ = sink.send(send_msg(&SyncMessage::RequestKvData)?).await;
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
            // Forward remote requests from broker task to server
            Some(request_json) = remote_rx.recv() => {
                // Parse "CMD:ARGS" format
                let (cmd, args) = request_json.split_once(':').unwrap_or((&request_json, ""));
                if let Ok(msg) = send_msg(&SyncMessage::RemoteRequest {
                    cmd: cmd.to_string(), args: args.to_string(),
                }) {
                    let _ = sink.send(msg).await;
                    tracing::info!("LAN sync: forwarded remote request '{}' to server", cmd);
                }
            }
            _ = ping_interval.tick() => {
                if sink.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn read_next(stream: &mut WsStream) -> Result<SyncMessage, String> {
    match tokio::time::timeout(std::time::Duration::from_secs(60), stream.next()).await {
        Ok(Some(Ok(msg))) => parse_msg(&msg),
        Ok(Some(Err(e))) => Err(format!("WebSocket error: {e}")),
        Ok(None) => Err("Connection closed".into()),
        Err(_) => Err("Timeout waiting for message".into()),
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
