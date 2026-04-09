use axum::{
    Router,
    extract::{ConnectInfo, State, WebSocketUpgrade, ws},
    response::IntoResponse,
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
use tokio::sync::{broadcast, mpsc, Mutex};
use tower_http::services::ServeDir;
use typhoon_web_protocol::{WebCmd, WebMsg};

// ── Security constants ──────────────────────────────────────────────
/// Maximum concurrent WebSocket clients.
const MAX_CLIENTS: usize = 10;
/// Maximum concurrent connections per IP address.
const MAX_PER_IP: usize = 3;
/// Maximum WebSocket text message size (64 KB — WebCmd JSON is tiny).
const MAX_WS_MSG_SIZE: usize = 64 * 1024;
/// Rate limit: maximum commands per second per client.
const MAX_CMDS_PER_SEC: u32 = 20;
/// Authentication timeout: client must send Auth within this many seconds.
const AUTH_TIMEOUT_SECS: u64 = 10;

/// Shared state for the web server.
pub struct WebServerState {
    /// Send WebCmds to the native app relay loop.
    pub cmd_tx: mpsc::UnboundedSender<WebCmd>,
    /// Broadcast channel the native app publishes WebMsgs to.
    pub msg_tx: broadcast::Sender<WebMsg>,
    /// Passphrase for client authentication (PBKDF2-hashed comparison).
    pub passphrase: String,
}

struct AppState {
    cmd_tx: mpsc::UnboundedSender<WebCmd>,
    msg_tx: broadcast::Sender<WebMsg>,
    passphrase: String,
    /// Track connected client count and per-IP counts.
    connections: Mutex<ConnectionTracker>,
}

struct ConnectionTracker {
    total: usize,
    per_ip: HashMap<std::net::IpAddr, usize>,
}

impl ConnectionTracker {
    fn new() -> Self {
        Self { total: 0, per_ip: HashMap::new() }
    }

    fn try_add(&mut self, ip: std::net::IpAddr) -> bool {
        if self.total >= MAX_CLIENTS {
            return false;
        }
        let count = self.per_ip.entry(ip).or_insert(0);
        if *count >= MAX_PER_IP {
            return false;
        }
        self.total += 1;
        *count += 1;
        true
    }

    fn remove(&mut self, ip: std::net::IpAddr) {
        self.total = self.total.saturating_sub(1);
        if let Some(count) = self.per_ip.get_mut(&ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.per_ip.remove(&ip);
            }
        }
    }
}

/// Start the HTTPS web server on `port`, serving the WASM bundle from `wasm_dir`.
///
/// `cert_pem` and `key_pem` are PEM-encoded TLS certificate and private key
/// (generated via rcgen in the engine, same pattern as LAN sync).
pub fn start_web_server(
    rt: &tokio::runtime::Handle,
    state: WebServerState,
    port: u16,
    wasm_dir: PathBuf,
    cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
) {
    let app_state = Arc::new(AppState {
        cmd_tx: state.cmd_tx,
        msg_tx: state.msg_tx,
        passphrase: state.passphrase,
        connections: Mutex::new(ConnectionTracker::new()),
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health_handler))
        .fallback_service(ServeDir::new(wasm_dir))
        .with_state(app_state);

    rt.spawn(async move {
        let tls_config = match RustlsConfig::from_pem(cert_pem, key_pem).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Web server TLS config failed: {e}");
                return;
            }
        };

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("Web server listening on https://0.0.0.0:{port}");

        if let Err(e) = axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
        {
            tracing::error!("Web server error: {e}");
        }
    });
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Enforce connection limits before upgrading
    let ip = addr.ip();
    let mut conns = state.connections.lock().await;
    if !conns.try_add(ip) {
        drop(conns);
        tracing::warn!("Web client rejected from {ip}: connection limit reached");
        return (axum::http::StatusCode::SERVICE_UNAVAILABLE, "Too many connections")
            .into_response();
    }
    drop(conns);

    ws.max_message_size(MAX_WS_MSG_SIZE)
        .on_upgrade(move |socket| run_websocket_session(socket, state, ip))
        .into_response()
}

async fn run_websocket_session(socket: ws::WebSocket, state: Arc<AppState>, client_ip: std::net::IpAddr) {
    let (mut sender, mut receiver) = socket.split();

    tracing::info!("Web client connected from {client_ip}");

    // ── Phase 1: Authentication ─────────────────────────────────────
    let authenticated = match tokio::time::timeout(
        std::time::Duration::from_secs(AUTH_TIMEOUT_SECS),
        wait_for_auth(&mut receiver, &state.passphrase),
    )
    .await
    {
        Ok(Ok(true)) => {
            // AuthResult has a single bool field — serialization cannot realistically fail,
            // but we handle the Err arm anyway rather than .unwrap() per ADR-082.
            let payload = serde_json::to_string(&WebMsg::AuthResult { ok: true })
                .unwrap_or_else(|_| r#"{"AuthResult":{"ok":true}}"#.to_string());
            let _ = sender.send(ws::Message::Text(payload.into())).await;
            true
        }
        _ => {
            let payload = serde_json::to_string(&WebMsg::AuthResult { ok: false })
                .unwrap_or_else(|_| r#"{"AuthResult":{"ok":false}}"#.to_string());
            let _ = sender.send(ws::Message::Text(payload.into())).await;
            false
        }
    };

    if !authenticated {
        tracing::warn!("Web client from {client_ip} failed authentication");
        state.connections.lock().await.remove(client_ip);
        return;
    }

    tracing::info!("Web client from {client_ip} authenticated");

    // ── Phase 2: Authenticated message loop ─────────────────────────
    let mut msg_rx = state.msg_tx.subscribe();
    let cmd_tx = state.cmd_tx.clone();

    // Task: forward WebMsgs from broadcast → WebSocket client
    let send_task = tokio::spawn(async move {
        loop {
            match msg_rx.recv().await {
                Ok(web_msg) => {
                    let json = match serde_json::to_string(&web_msg) {
                        Ok(j) => j,
                        Err(_) => continue,
                    };
                    if sender.send(ws::Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Web client {client_ip} lagged {n} messages");
                    // Continue — client will get the next message
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Rate limiter state
    let mut cmd_count: u32 = 0;
    let mut window_start = Instant::now();

    // Read WebCmds from WebSocket client → forward to native app
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            ws::Message::Text(text) => {
                // Rate limiting
                let now = Instant::now();
                if now.duration_since(window_start).as_secs() >= 1 {
                    cmd_count = 0;
                    window_start = now;
                }
                cmd_count += 1;
                if cmd_count > MAX_CMDS_PER_SEC {
                    tracing::warn!("Web client {client_ip} rate limited");
                    continue;
                }

                // Parse and validate
                match serde_json::from_str::<WebCmd>(&text) {
                    Ok(WebCmd::Auth { .. }) => {
                        // Already authenticated, ignore duplicate auth
                    }
                    Ok(cmd) => {
                        // Validate symbol/timeframe fields
                        if let WebCmd::GetBars { ref symbol, ref timeframe } = cmd {
                            if !typhoon_web_protocol::is_valid_symbol(symbol)
                                || !typhoon_web_protocol::is_valid_timeframe(timeframe)
                            {
                                tracing::warn!("Web client {client_ip} sent invalid symbol/timeframe");
                                continue;
                            }
                        }
                        if let WebCmd::GetWatchlistQuotes { ref symbols } = cmd {
                            if symbols.len() > typhoon_web_protocol::MAX_WATCHLIST_SYMBOLS
                                || symbols.iter().any(|s| !typhoon_web_protocol::is_valid_symbol(s))
                            {
                                tracing::warn!("Web client {client_ip} sent invalid watchlist request");
                                continue;
                            }
                        }
                        let _ = cmd_tx.send(cmd);
                    }
                    Err(_) => {
                        tracing::debug!("Web client {client_ip} sent invalid command");
                    }
                }
            }
            ws::Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    state.connections.lock().await.remove(client_ip);
    tracing::info!("Web client from {client_ip} disconnected");
}

/// Wait for the first message to be an Auth command with correct passphrase.
async fn wait_for_auth(
    receiver: &mut futures_util::stream::SplitStream<ws::WebSocket>,
    expected_passphrase: &str,
) -> Result<bool, ()> {
    while let Some(Ok(msg)) = receiver.next().await {
        if let ws::Message::Text(text) = msg {
            return match serde_json::from_str::<WebCmd>(&text) {
                Ok(WebCmd::Auth { passphrase }) => {
                    Ok(passphrase == expected_passphrase)
                }
                _ => Ok(false),
            };
        }
    }
    Err(())
}
