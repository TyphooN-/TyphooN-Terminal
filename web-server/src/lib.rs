use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws},
    response::IntoResponse,
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;
use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{broadcast, mpsc};
use tower_http::services::ServeDir;
use typhoon_web_protocol::{WebCmd, WebMsg};

/// Shared state for the web server.
pub struct WebServerState {
    /// Send WebCmds to the native app relay loop.
    pub cmd_tx: mpsc::UnboundedSender<WebCmd>,
    /// Broadcast channel the native app publishes WebMsgs to.
    pub msg_tx: broadcast::Sender<WebMsg>,
}

struct AppState {
    cmd_tx: mpsc::UnboundedSender<WebCmd>,
    msg_tx: broadcast::Sender<WebMsg>,
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
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
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
            .serve(app.into_make_service())
            .await
        {
            tracing::error!("Web server error: {e}");
        }
    });
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(socket: ws::WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut msg_rx = state.msg_tx.subscribe();
    let cmd_tx = state.cmd_tx.clone();

    tracing::info!("Web client connected");

    // Task: forward WebMsgs from broadcast → WebSocket client
    let send_task = tokio::spawn(async move {
        while let Ok(web_msg) = msg_rx.recv().await {
            let json = match serde_json::to_string(&web_msg) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if sender.send(ws::Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Read WebCmds from WebSocket client → forward to native app
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            ws::Message::Text(text) => {
                if let Ok(cmd) = serde_json::from_str::<WebCmd>(&text) {
                    let _ = cmd_tx.send(cmd);
                }
            }
            ws::Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    tracing::info!("Web client disconnected");
}
