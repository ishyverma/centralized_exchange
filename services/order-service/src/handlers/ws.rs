use actix_web::{web, HttpRequest, HttpResponse};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::engine_client::EngineClient;

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    engine: web::Data<EngineClient>,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let subscriptions = Arc::new(Mutex::new(HashSet::<String>::new()));
    let subs = subscriptions.clone();
    let engine_clone = engine.get_ref().clone();

    actix_web::rt::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000));
        loop {
            tokio::select! {
                msg = msg_stream.recv() => {
                    match msg {
                        Some(Ok(actix_ws::Message::Text(text))) => {
                            if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&text) {
                                let method = cmd.get("method").and_then(|m| m.as_str()).unwrap_or("");
                                match method {
                                    "SUBSCRIBE" => {
                                        if let Some(params) = cmd.get("params").and_then(|p| p.as_array()) {
                                            let mut subs = subs.lock().await;
                                            for param in params {
                                                if let Some(stream_name) = param.as_str() {
                                                    subs.insert(stream_name.to_string());
                                                }
                                            }
                                        }
                                        if let Some(result) = cmd.get("params") {
                                            let ack = serde_json::json!({
                                                "id": cmd.get("id"),
                                                "status": 200,
                                                "result": result,
                                            });
                                            let _ = session.text(ack.to_string()).await;
                                        }
                                    }
                                    "UNSUBSCRIBE" => {
                                        if let Some(params) = cmd.get("params").and_then(|p| p.as_array()) {
                                            let mut subs = subs.lock().await;
                                            for param in params {
                                                if let Some(stream_name) = param.as_str() {
                                                    subs.remove(stream_name);
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(Ok(actix_ws::Message::Ping(bytes))) => {
                            let _ = session.pong(&bytes).await;
                        }
                        Some(Ok(actix_ws::Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
                _ = interval.tick() => {
                    let current_subs = subs.lock().await.clone();
                    if current_subs.is_empty() {
                        continue;
                    }
                    for stream_name in &current_subs {
                        if let Some(symbol) = stream_name.strip_prefix("depth.") {
                            if let Some(state) = engine_clone.get_depth(symbol) {
                                let data = serde_json::json!({
                                    "stream": stream_name,
                                    "data": {
                                        "e": "depthUpdate",
                                        "E": std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_millis() as u64,
                                        "s": symbol.to_uppercase(),
                                        "b": state.bids.iter().map(|l| vec![l.price.to_string(), l.quantity.to_string()]).collect::<Vec<_>>(),
                                        "a": state.asks.iter().map(|l| vec![l.price.to_string(), l.quantity.to_string()]).collect::<Vec<_>>(),
                                    }
                                });
                                if session.text(data.to_string()).await.is_err() {
                                    return;
                                }
                            }
                        } else if let Some(symbol) = stream_name.strip_prefix("ticker.") {
                            if let Some(ticker) = engine_clone.get_book_ticker(symbol) {
                                let data = serde_json::json!({
                                    "stream": stream_name,
                                    "data": {
                                        "e": "ticker",
                                        "E": std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_millis() as u64,
                                        "s": symbol.to_uppercase(),
                                        "b": ticker.bid_price.to_string(),
                                        "B": ticker.bid_qty.to_string(),
                                        "a": ticker.ask_price.to_string(),
                                        "A": ticker.ask_qty.to_string(),
                                    }
                                });
                                if session.text(data.to_string()).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(response)
}
