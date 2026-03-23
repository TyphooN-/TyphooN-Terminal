//! LAN Sync — WebSocket-based cache synchronization between TyphooN Terminal instances.
//!
//! Server mode: serves bar cache data to connecting clients over local network.
//! Client mode: connects to a server, syncs missing/outdated cache entries.
//! Auth: PBKDF2-derived shared secret + HMAC-SHA256 challenge-response.

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::core::cache::SqliteCache;

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
    pub mode: String,      // "server", "client", "idle"
    pub connected: bool,
    pub clients: usize,    // server: number of connected clients
    pub host: String,      // client: server host
    pub port: u16,
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self { mode: "idle".into(), connected: false, clients: 0, host: String::new(), port: 0 }
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
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .map_err(|e| format!("Bind failed on port {port}: {e}"))?;

        let status = Arc::new(TokioMutex::new(SyncStatus {
            mode: "server".into(),
            connected: true,
            clients: 0,
            host: "0.0.0.0".into(),
            port,
        }));

        let status_clone = status.clone();
        let task = tokio::spawn(async move {
            tracing::info!("LAN sync server listening on 0.0.0.0:{port}");
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        tracing::info!("LAN sync: client connected from {addr}");
                        let cache = cache.clone();
                        let status = status_clone.clone();
                        {
                            let mut s = status.lock().await;
                            s.clients += 1;
                        }
                        tokio::spawn(handle_client(stream, cache, secret, status));
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

async fn handle_client(
    stream: tokio::net::TcpStream,
    cache: Arc<SqliteCache>,
    secret: [u8; 32],
    status: Arc<TokioMutex<SyncStatus>>,
) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
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
                let mut count = 0;
                for key in &keys {
                    if let Ok(Some((data, ts))) = cache.get_raw_bar_entry(key) {
                        let encoded = base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &data,
                        );
                        if let Ok(msg) = send_msg(&SyncMessage::EntryData {
                            key: key.clone(),
                            data: encoded,
                            timestamp: ts,
                        }) {
                            let _ = sink.send(msg).await;
                        }
                        count += 1;
                    }
                }
                if let Ok(msg) = send_msg(&SyncMessage::BatchComplete { count }) {
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
    ) -> Result<Self, String> {
        let secret = derive_secret(passphrase);
        let url = format!("ws://{host}:{port}");

        let (ws, _) = tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| format!("Connect to {url} failed: {e}"))?;

        let status = Arc::new(TokioMutex::new(SyncStatus {
            mode: "client".into(),
            connected: true,
            clients: 0,
            host: host.to_string(),
            port,
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

        let status_clone = status.clone();
        let task = tokio::spawn(async move {
            if let Err(e) = client_sync_loop(&cache, &mut sink, &mut stream_rx).await {
                tracing::warn!("LAN sync client error: {e}");
            }
            let mut s = status_clone.lock().await;
            s.connected = false;
            tracing::info!("LAN sync: client disconnected");
        });

        Ok(Self { task: Some(task), status })
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
        loop {
            match read_next(stream).await? {
                SyncMessage::EntryData { key, data, timestamp } => {
                    let decoded = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        &data,
                    ).map_err(|e| format!("Base64 decode failed for {key}: {e}"))?;

                    // Get bar_count from the compressed data by decompressing header
                    let bar_count = extract_bar_count(&decoded);

                    if let Err(e) = cache.put_raw_bar_entry(&key, &decoded, timestamp, bar_count) {
                        tracing::warn!("LAN sync: failed to write {key}: {e}");
                    }
                }
                SyncMessage::BatchComplete { count } => {
                    tracing::info!("LAN sync: received {count} entries");
                    break;
                }
                other => {
                    tracing::warn!("LAN sync: unexpected message during transfer: {:?}", other);
                }
            }
        }
    }

    // 9. Listen for incremental updates (server pushes + ping/pong keepalive)
    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() { break; }
                        if msg.is_pong() { continue; }
                        match parse_msg(&msg) {
                            Ok(SyncMessage::IncrementalUpdate { key, data, timestamp }) => {
                                let decoded = base64::Engine::decode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &data,
                                ).map_err(|e| format!("Base64 decode failed: {e}"))?;
                                let bar_count = extract_bar_count(&decoded);
                                let _ = cache.put_raw_bar_entry(&key, &decoded, timestamp, bar_count);
                                tracing::debug!("LAN sync: incremental update for {key}");
                            }
                            Ok(SyncMessage::Pong) => {}
                            Ok(SyncMessage::Ping) => {
                                let _ = sink.send(send_msg(&SyncMessage::Pong)?).await;
                            }
                            _ => {}
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!("LAN sync: stream error: {e}");
                        break;
                    }
                    None => break,
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
