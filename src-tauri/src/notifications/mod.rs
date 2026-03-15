//! Notification systems — Discord webhook, Pushover, and ntfy.
//!
//! Port of BroadcastDiscordAnnouncement from TyphooN EA,
//! plus push notification providers.

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

/// Send a push notification via Pushover (https://pushover.net/api).
///
/// Requires a Pushover application token and user key.
pub async fn send_pushover(token: &str, user: &str, message: &str) -> Result<(), String> {
    if token.is_empty() || token.len() > 64 {
        return Err("Invalid Pushover token".to_string());
    }
    if user.is_empty() || user.len() > 64 {
        return Err("Invalid Pushover user key".to_string());
    }
    if message.is_empty() || message.len() > 1024 {
        return Err("Message must be 1-1024 chars".to_string());
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let body = json!({
        "token": token,
        "user": user,
        "message": message,
        "title": "TyphooN Terminal",
    });

    let resp = client
        .post("https://api.pushover.net/1/messages.json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Pushover POST failed: {e}"))?;

    let status = resp.status().as_u16();
    if status == 200 {
        info!("Pushover notification sent: {}", &message[..message.len().min(80)]);
        Ok(())
    } else {
        let err = format!("Pushover returned HTTP {status}");
        error!("{err}");
        Err(err)
    }
}

/// Send a push notification via ntfy.sh (https://ntfy.sh).
///
/// Simple POST to ntfy.sh/<topic> with message as body.
pub async fn send_ntfy(topic: &str, message: &str) -> Result<(), String> {
    if topic.is_empty() || topic.len() > 128 {
        return Err("Invalid ntfy topic".to_string());
    }
    // Validate topic: alphanumeric, hyphens, underscores only
    if !topic.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid ntfy topic characters".to_string());
    }
    if message.is_empty() || message.len() > 4096 {
        return Err("Message must be 1-4096 chars".to_string());
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp = client
        .post(format!("https://ntfy.sh/{topic}"))
        .header("Title", "TyphooN Terminal")
        .body(message.to_string())
        .send()
        .await
        .map_err(|e| format!("ntfy POST failed: {e}"))?;

    let status = resp.status().as_u16();
    if status == 200 {
        info!("ntfy notification sent to {topic}: {}", &message[..message.len().min(80)]);
        Ok(())
    } else {
        let err = format!("ntfy returned HTTP {status}");
        error!("{err}");
        Err(err)
    }
}
