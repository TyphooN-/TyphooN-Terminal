//! Notification systems — Discord webhook, Pushover, and ntfy.
//!
//! Port of BroadcastDiscordAnnouncement from TyphooN EA,
//! plus push notification providers.

use reqwest::Client;
use serde_json::json;
use std::sync::OnceLock;
use tracing::{error, info};

/// Shared HTTP client for all notification providers.
/// Reuses TCP connections across calls instead of creating a new client each time.
fn notification_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(2)
            .build()
            .expect("Failed to build notification HTTP client")
    })
}

/// Send a message to a Discord webhook.
///
/// Port of BroadcastDiscordAnnouncement from MQL5:
/// - JSON-safe escaping (backslash, quote, newline, tab)
/// - POST to webhook URL
/// - Checks HTTP 200/204 for success
pub async fn send_discord(webhook_url: &str, message: &str) -> Result<(), String> {
    // Strict webhook URL validation: must be exactly discord.com, no path traversal
    if webhook_url.is_empty() { return Err("Empty webhook URL".to_string()); }
    if !webhook_url.starts_with("https://discord.com/api/webhooks/") {
        return Err("Invalid Discord webhook URL".to_string());
    }
    if webhook_url.contains("..") || webhook_url.contains('@') {
        return Err("Invalid characters in webhook URL".to_string());
    }
    // Validate no redirect/hostname tricks (e.g. discord.com.evil.com)
    let after_scheme = webhook_url.strip_prefix("https://").unwrap_or("");
    if !after_scheme.starts_with("discord.com/") {
        return Err("Webhook URL hostname must be exactly discord.com".to_string());
    }
    // Discord max message length is 2000 chars
    if message.len() > 2000 {
        return Err("Message too long (max 2000 chars)".to_string());
    }

    let client = notification_client();
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

    let client = notification_client();

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

    let client = notification_client();

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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Discord validation tests ---

    #[tokio::test]
    async fn discord_empty_webhook_returns_err() {
        let result = send_discord("", "hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn discord_invalid_webhook_returns_err() {
        let result = send_discord("https://example.com/not-a-webhook", "hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn discord_message_too_long_returns_err() {
        let long_msg = "a".repeat(2001);
        let result =
            send_discord("https://discord.com/api/webhooks/123/abc", &long_msg).await;
        assert!(result.is_err());
    }

    // --- Pushover validation tests ---

    #[tokio::test]
    async fn pushover_empty_token_returns_err() {
        let result = send_pushover("", "user123", "hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pushover_token_too_long_returns_err() {
        let long_token = "a".repeat(65);
        let result = send_pushover(&long_token, "user123", "hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pushover_empty_user_returns_err() {
        let result = send_pushover("token123", "", "hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pushover_empty_message_returns_err() {
        let result = send_pushover("token123", "user123", "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pushover_message_too_long_returns_err() {
        let long_msg = "a".repeat(1025);
        let result = send_pushover("token123", "user123", &long_msg).await;
        assert!(result.is_err());
    }

    // --- ntfy validation tests ---

    #[tokio::test]
    async fn ntfy_empty_topic_returns_err() {
        let result = send_ntfy("", "hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ntfy_topic_too_long_returns_err() {
        let long_topic = "a".repeat(129);
        let result = send_ntfy(&long_topic, "hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ntfy_invalid_topic_chars_returns_err() {
        assert!(send_ntfy("has spaces", "hello").await.is_err());
        assert!(send_ntfy("special!@#", "hello").await.is_err());
        assert!(send_ntfy("path/slash", "hello").await.is_err());
    }

    #[tokio::test]
    async fn ntfy_empty_message_returns_err() {
        let result = send_ntfy("valid-topic", "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ntfy_message_too_long_returns_err() {
        let long_msg = "a".repeat(4097);
        let result = send_ntfy("valid-topic", &long_msg).await;
        assert!(result.is_err());
    }
}
