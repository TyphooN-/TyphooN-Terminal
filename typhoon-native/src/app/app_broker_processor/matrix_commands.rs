use super::prelude::*;

fn encode_matrix_room_path(room_id: &str, encode_hash: bool) -> String {
    let mut encoded = String::with_capacity(room_id.len());
    for ch in room_id.chars() {
        match ch {
            '!' => encoded.push_str("%21"),
            '#' if encode_hash => encoded.push_str("%23"),
            ':' => encoded.push_str("%3A"),
            _ => encoded.push(ch),
        }
    }
    encoded
}

pub(super) fn handle_matrix_command(
    cmd: BrokerCmd,
    broker_msg_tx_clone: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) {
    match cmd {
        BrokerCmd::MatrixJoinRoom {
            room_id,
            access_token,
        } => {
            let client = reqwest::Client::new();
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                // Encode the room ID/alias path segment. matrix.org requires %21/%23/%3A
                // for the URL path portion.
                let encoded_room = encode_matrix_room_path(&room_id, true);
                // server_name hint — derived from the suffix of the room id/alias.
                // For room IDs like "!abc:matrix.org" the Matrix spec recommends passing
                // ?server_name=matrix.org so your homeserver knows which federation peer
                // to ask about the room. Without it, homeservers that haven't yet
                // resolved the room return "M_UNKNOWN: No known servers".
                let server_name = room_id
                    .rsplit(':')
                    .next()
                    .unwrap_or("matrix.org")
                    .to_string();
                let url = format!(
                    "https://matrix.org/_matrix/client/v3/join/{}?server_name={}",
                    encoded_room, server_name
                );
                match client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", access_token))
                    .json(&serde_json::json!({}))
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        let _ =
                            msg_tx.send(BrokerMsg::JsonResult("MatrixJoined".into(), "ok".into()));
                    }
                    Ok(resp) => {
                        let text = resp.text().await.unwrap_or_default();
                        // M_ALREADY_JOINED is fine
                        if text.contains("already") {
                            let _ = msg_tx.send(BrokerMsg::JsonResult(
                                "MatrixJoined".into(),
                                "already".into(),
                            ));
                        } else {
                            let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix join: {}", text)));
                        }
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix join: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::MatrixFetchMessages {
            room_id,
            access_token,
        } => {
            let client = reqwest::Client::new();
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let encoded_room = encode_matrix_room_path(&room_id, false);
                let url = format!(
                    "https://matrix.org/_matrix/client/r0/rooms/{}/messages?dir=b&limit=50",
                    encoded_room
                );
                let mut req = client
                    .get(&url)
                    .header("User-Agent", "TyphooN-Terminal/1.0");
                if !access_token.is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", access_token));
                }
                match req.send().await {
                    Ok(resp) => {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            let mut msgs = json["chunk"]
                                .as_array()
                                .map_or_else(Vec::new, |chunk| Vec::with_capacity(chunk.len()));
                            if let Some(chunk) = json["chunk"].as_array() {
                                for ev in chunk.iter().rev() {
                                    if ev["type"].as_str() == Some("m.room.message") {
                                        let sender =
                                            ev["sender"].as_str().unwrap_or("?").to_string();
                                        let ts = ev["origin_server_ts"].as_i64().unwrap_or(0);
                                        let dt = chrono::DateTime::from_timestamp(ts / 1000, 0)
                                            .map(|d| d.format("%H:%M").to_string())
                                            .unwrap_or_default();
                                        let body = ev["content"]["body"]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string();
                                        msgs.push((sender, dt, body));
                                    }
                                }
                            }
                            let text = serde_json::to_string(&msgs).unwrap_or_default();
                            let _ =
                                msg_tx.send(BrokerMsg::JsonResult("MatrixMessages".into(), text));
                        }
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix: {}", e)));
                    }
                }
            });
        }
        BrokerCmd::MatrixSendImage {
            room_id,
            access_token,
            file_path,
        } => {
            let client = reqwest::Client::new();
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                // Step 1: Read file
                let data = match tokio::fs::read(&file_path).await {
                    Ok(d) => d,
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Read screenshot: {e}")));
                        return;
                    }
                };
                let filename = file_path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| "screenshot.webp".into());
                let content_type = if filename.ends_with(".webp") {
                    "image/webp"
                } else {
                    "image/png"
                };

                // Step 2: Upload to Matrix content repository
                let upload_url = format!(
                    "https://matrix.org/_matrix/media/r0/upload?filename={}",
                    filename
                );
                match client
                    .post(&upload_url)
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", content_type)
                    .body(data.clone())
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            let mxc_url = json["content_uri"].as_str().unwrap_or("").to_string();
                            if mxc_url.is_empty() {
                                let _ = msg_tx.send(BrokerMsg::Error(
                                    "Matrix upload: no content_uri returned".into(),
                                ));
                                return;
                            }
                            // Step 3: Send m.image message
                            let txn_id =
                                format!("typhoon_img_{}", chrono::Utc::now().timestamp_millis());
                            let encoded_room = encode_matrix_room_path(&room_id, false);
                            let send_url = format!(
                                "https://matrix.org/_matrix/client/r0/rooms/{}/send/m.room.message/{}",
                                encoded_room, txn_id
                            );
                            let msg_body = serde_json::json!({
                                "msgtype": "m.image",
                                "body": filename,
                                "url": mxc_url,
                                "info": { "mimetype": content_type, "size": data.len() },
                            });
                            match client
                                .put(&send_url)
                                .header("Authorization", format!("Bearer {}", access_token))
                                .json(&msg_body)
                                .send()
                                .await
                            {
                                Ok(r) if r.status().is_success() => {
                                    let _ = msg_tx.send(BrokerMsg::JsonResult(
                                        "MatrixSent".into(),
                                        "image shared".into(),
                                    ));
                                    let _ = msg_tx.send(BrokerMsg::OrderResult(
                                        "Screenshot shared to community chat".into(),
                                    ));
                                }
                                Ok(r) => {
                                    let text = r.text().await.unwrap_or_default();
                                    let _ = msg_tx.send(BrokerMsg::Error(format!(
                                        "Matrix send image: {}",
                                        text
                                    )));
                                }
                                Err(e) => {
                                    let _ =
                                        msg_tx.send(BrokerMsg::Error(format!("Matrix send: {e}")));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix upload: {e}")));
                    }
                }
            });
        }
        BrokerCmd::MatrixSendMessage {
            room_id,
            access_token,
            body,
        } => {
            let client = reqwest::Client::new();
            let msg_tx = broker_msg_tx_clone.clone();
            tokio::spawn(async move {
                let txn_id = format!("typhoon_{}", chrono::Utc::now().timestamp_millis());
                let encoded_room = encode_matrix_room_path(&room_id, false);
                let url = format!(
                    "https://matrix.org/_matrix/client/r0/rooms/{}/send/m.room.message/{}",
                    encoded_room, txn_id
                );
                let msg_body = serde_json::json!({
                    "msgtype": "m.text",
                    "body": body,
                });
                match client
                    .put(&url)
                    .header("Authorization", format!("Bearer {}", access_token))
                    .json(&msg_body)
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            let _ = msg_tx
                                .send(BrokerMsg::JsonResult("MatrixSent".into(), "ok".into()));
                        } else {
                            let text = resp.text().await.unwrap_or_default();
                            let _ = msg_tx
                                .send(BrokerMsg::Error(format!("Matrix send failed: {}", text)));
                        }
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BrokerMsg::Error(format!("Matrix send: {}", e)));
                    }
                }
            });
        }
        _ => unreachable!("non-Matrix command routed to Matrix handler"),
    }
}
