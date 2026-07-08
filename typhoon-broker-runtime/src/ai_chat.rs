use std::sync::Arc;

use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg};
use typhoon_engine::core::cache::SqliteCache;

pub fn handle_ai_chat_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    shared_cache_broker: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    match cmd {
        BrokerCmd::AiChat {
            provider,
            api_key,
            message,
            history,
            system,
            model,
        } => {
            let client = reqwest::Client::new();
            let msg_tx = broker_msg_tx_clone.clone();
            let shared_cache_broker = shared_cache_broker.clone();
            tokio::spawn(async move {
                // The base system prompt: trading assistant + research packet if supplied.
                let base_system = "You are a trading assistant inside TyphooN-Terminal. \
When the question touches recent news, sentiment, or prices, combine the research packet \
(if provided) with your own live web search and cite the sources you rely on."
                    .to_string();
                let full_system = match &system {
                    Some(packet) if !packet.is_empty() => format!(
                        "{base_system}\n\n=== RESEARCH PACKET ===\n{packet}\n=== END RESEARCH PACKET ==="
                    ),
                    _ => base_system,
                };

                // Build the message chain (history + new user turn).
                let msgs: Vec<serde_json::Value> = history.iter()
                    .map(|(is_user, text)| serde_json::json!({"role": if *is_user { "user" } else { "assistant" }, "content": text}))
                    .chain(std::iter::once(serde_json::json!({"role": "user", "content": message})))
                    .collect();

                // ── cross-client AI response cache lookup ──
                // Compute deterministic hash over the full prompt tuple and check
                // the AI response cache before spending tokens. On hit, emit the
                // cached response and skip the HTTP call entirely.
                use typhoon_engine::core::ai_response_cache as arc_cache;
                let cache_provider_tag = match provider.as_str() {
                    "claude" => "claude_http",
                    other => other,
                };
                let cache_model = model.clone().unwrap_or_else(|| match provider.as_str() {
                    "claude" => "claude-fable-5".to_string(),
                    "openai" => "gpt-5.1".into(),
                    "gemini" => typhoon_engine::core::ai_sessions::DEFAULT_GEMINI_CLI_MODEL.into(),
                    "grok" => "grok-4.1".into(),
                    "mistral" => "mistral-large-latest".into(),
                    "perplexity" => "sonar-pro".into(),
                    "local" => "llama3.2".into(),
                    _ => "unknown".into(),
                });
                let prompt_hash = arc_cache::hash_ai_prompt(
                    cache_provider_tag,
                    &cache_model,
                    &full_system,
                    &history,
                    &message,
                );
                let cache_snapshot = shared_cache_broker.read().ok().and_then(|g| g.clone());
                if let Some(cache) = cache_snapshot.as_ref() {
                    if let Ok(Some(hit)) = arc_cache::lookup_response(cache, &prompt_hash) {
                        let _ = msg_tx.send(BrokerMsg::JsonResult("AiChat".into(), hit.response));
                        return;
                    }
                }

                if provider == "claude" {
                    // Anthropic uses its own API format (not OpenAI-compatible).
                    // `system` goes in its own top-level field, not as a role.
                    let anth_model = model
                        .clone()
                        .unwrap_or_else(|| "claude-fable-5".to_string());
                    let body = serde_json::json!({
                        "model": anth_model,
                        "max_tokens": 4096,
                        "system": full_system,
                        "messages": msgs,
                    });
                    match client
                        .post("https://api.anthropic.com/v1/messages")
                        .header("x-api-key", &api_key)
                        .header("anthropic-version", "2023-06-01")
                        .header("content-type", "application/json")
                        .json(&body)
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            let text = resp
                                .json::<serde_json::Value>()
                                .await
                                .ok()
                                .and_then(|j| {
                                    j["content"][0]["text"].as_str().map(|s| s.to_string())
                                })
                                .unwrap_or_else(|| "(no response)".into());
                            // record the fresh response in the AI response cache.
                            if text != "(no response)" {
                                if let Some(cache) = cache_snapshot.as_ref() {
                                    let preview: String = message.chars().take(400).collect();
                                    let host = std::env::var("HOSTNAME").unwrap_or_default();
                                    let _ = arc_cache::upsert_response(
                                        cache,
                                        &arc_cache::AiResponseCacheEntry {
                                            prompt_hash: prompt_hash.clone(),
                                            provider: cache_provider_tag.to_string(),
                                            model: anth_model.clone(),
                                            prompt_preview: preview,
                                            response: text.clone(),
                                            token_count_prompt: arc_cache::estimate_tokens(
                                                &full_system,
                                            ) + arc_cache::estimate_tokens(
                                                &message,
                                            ),
                                            token_count_completion: arc_cache::estimate_tokens(
                                                &text,
                                            ),
                                            created_at: 0,
                                            updated_at: 0,
                                            hit_count: 0,
                                            source_client: host,
                                        },
                                    );
                                }
                            }
                            let _ = msg_tx.send(BrokerMsg::JsonResult("AiChat".into(), text));
                        }
                        Err(e) => {
                            let _ = msg_tx.send(BrokerMsg::Error(format!("Claude API: {}", e)));
                        }
                    }
                } else {
                    // OpenAI-compatible endpoint (GPT, Gemini, Grok, Mistral, Perplexity, Ollama)
                    let (url, default_model, auth_header) = match provider.as_str() {
                        "openai" => (
                            "https://api.openai.com/v1/chat/completions",
                            "gpt-5.1",
                            format!("Bearer {}", api_key),
                        ),
                        "gemini" => (
                            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
                            typhoon_engine::core::ai_sessions::DEFAULT_GEMINI_CLI_MODEL,
                            format!("Bearer {}", api_key),
                        ),
                        "grok" => (
                            "https://api.x.ai/v1/chat/completions",
                            "grok-4.1",
                            format!("Bearer {}", api_key),
                        ),
                        "mistral" => (
                            "https://api.mistral.ai/v1/chat/completions",
                            "mistral-large-latest",
                            format!("Bearer {}", api_key),
                        ),
                        "perplexity" => (
                            "https://api.perplexity.ai/chat/completions",
                            "sonar-pro",
                            format!("Bearer {}", api_key),
                        ),
                        "local" => {
                            // Ollama / LM Studio: local OpenAI-compatible server
                            let local_url = if api_key.starts_with("http") {
                                api_key.as_str()
                            } else {
                                "http://localhost:11434"
                            };
                            (
                                if local_url.contains("11434") {
                                    "http://localhost:11434/v1/chat/completions"
                                } else {
                                    "http://localhost:1234/v1/chat/completions"
                                },
                                "llama3.2",
                                String::new(),
                            )
                        }
                        _ => (
                            "https://api.openai.com/v1/chat/completions",
                            "gpt-5.1",
                            format!("Bearer {}", api_key),
                        ),
                    };
                    let effective_model =
                        model.clone().unwrap_or_else(|| default_model.to_string());
                    let mut all =
                        vec![serde_json::json!({"role": "system", "content": full_system})];
                    all.extend(msgs);
                    let body = serde_json::json!({"model": effective_model, "messages": all, "max_tokens": 4096});
                    let mut req = client
                        .post(url)
                        .header("content-type", "application/json")
                        .json(&body);
                    if !auth_header.is_empty() {
                        req = req.header("Authorization", &auth_header);
                    }
                    match req.send().await {
                        Ok(resp) => {
                            let text = resp
                                .json::<serde_json::Value>()
                                .await
                                .ok()
                                .and_then(|j| {
                                    j["choices"][0]["message"]["content"]
                                        .as_str()
                                        .map(|s| s.to_string())
                                })
                                .unwrap_or_else(|| "(no response)".into());
                            // record the fresh response in the AI response cache.
                            if text != "(no response)" {
                                if let Some(cache) = cache_snapshot.as_ref() {
                                    let preview: String = message.chars().take(400).collect();
                                    let host = std::env::var("HOSTNAME").unwrap_or_default();
                                    let _ = arc_cache::upsert_response(
                                        cache,
                                        &arc_cache::AiResponseCacheEntry {
                                            prompt_hash: prompt_hash.clone(),
                                            provider: cache_provider_tag.to_string(),
                                            model: effective_model.clone(),
                                            prompt_preview: preview,
                                            response: text.clone(),
                                            token_count_prompt: arc_cache::estimate_tokens(
                                                &full_system,
                                            ) + arc_cache::estimate_tokens(
                                                &message,
                                            ),
                                            token_count_completion: arc_cache::estimate_tokens(
                                                &text,
                                            ),
                                            created_at: 0,
                                            updated_at: 0,
                                            hit_count: 0,
                                            source_client: host,
                                        },
                                    );
                                }
                            }
                            let _ = msg_tx.send(BrokerMsg::JsonResult("AiChat".into(), text));
                        }
                        Err(e) => {
                            let _ =
                                msg_tx.send(BrokerMsg::Error(format!("{} API: {}", provider, e)));
                        }
                    }
                }
            });
        }
        _ => unreachable!("non-AI chat command routed to AI chat handler"),
    }
}
