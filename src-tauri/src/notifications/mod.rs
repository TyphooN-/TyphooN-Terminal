//! Notification systems — Discord webhook and logging.
//!
//! Port of BroadcastDiscordAnnouncement from TyphooN EA.

use reqwest::Client;
use serde_json::json;
use tracing::{error, info};

/// Send a message to a Discord webhook.
///
/// Port of BroadcastDiscordAnnouncement from MQL5:
/// - JSON-safe escaping (backslash, quote, newline, tab)
/// - POST to webhook URL
/// - Checks HTTP 200/204 for success
pub async fn send_discord(webhook_url: &str, message: &str) -> Result<(), String> {
    if webhook_url.is_empty() || !webhook_url.starts_with("https://discord.com/api/webhooks/") {
        return Err("Invalid Discord webhook URL".to_string());
    }
    // Discord max message length is 2000 chars
    if message.len() > 2000 {
        return Err("Message too long (max 2000 chars)".to_string());
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;
    let body = json!({ "content": message });

    let resp = client
        .post(webhook_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Discord POST failed: {e}"))?;

    let status = resp.status().as_u16();
    if status == 200 || status == 204 {
        info!("Discord notification sent: {}", &message[..message.len().min(80)]);
        Ok(())
    } else {
        let err = format!("Discord returned HTTP {status}");
        error!("{err}");
        Err(err)
    }
}
