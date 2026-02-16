//! WebSocket routes for real-time updates.
//!
//! This module provides the WebSocket endpoint for real-time communication
//! between clients and the server.

use axum::{
    extract::{Extension, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use uuid::Uuid;

use peilbeheer_core::WsMessage;

use crate::websocket_service::WebSocketServer;

/// WebSocket upgrade endpoint.
///
/// Clients connect to this endpoint to receive real-time updates.
///
/// Example:
/// ```javascript
/// const ws = new WebSocket('ws://localhost:3000/api/ws');
/// ws.onmessage = (event) => {
///     const msg = JSON.parse(event.data);
///     console.log('Received:', msg);
/// };
/// ```
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(server): Extension<Arc<WebSocketServer>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, server))
}

/// Handle a WebSocket connection after upgrade.
async fn handle_websocket(mut socket: WebSocket, server: Arc<WebSocketServer>) {
    let client_id = Uuid::new_v4().to_string();

    tracing::info!("WebSocket client connecting: {}", client_id);

    // Send welcome message
    let welcome = WsMessage::welcome(server.server_id().to_string());
    if let Ok(json) = welcome.to_json() {
        let _ = socket.send(Message::Text(json.into())).await;
    }

    // Add client with default subscriptions
    server.add_client(client_id.clone(), None, None).await;

    // Create a broadcast receiver to get messages from the server
    let mut rx = server.broadcaster().subscribe();

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Task to send messages to the client
    let client_id_send = client_id.clone();
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = msg.to_json() {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Task to receive messages from the client
    let server_recv = server.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Handle client messages (subscribe, unsubscribe, ping, etc.)
                    if let Ok(ws_msg) = WsMessage::from_json(&text) {
                        handle_client_message(&server_recv, &client_id_send, ws_msg).await;
                    }
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {
            tracing::info!("WebSocket sender task completed: {}", client_id);
        }
        _ = recv_task => {
            tracing::info!("WebSocket receiver task completed: {}", client_id);
        }
    }

    // Clean up
    server.remove_client(&client_id).await;
    tracing::info!("WebSocket client disconnected: {}", client_id);
}

/// Handle a message received from a client.
async fn handle_client_message(server: &WebSocketServer, client_id: &str, msg: WsMessage) {
    match msg {
        WsMessage::Ping { .. } => {
            // Respond with pong
            let pong = WsMessage::pong();
            tracing::trace!("Sending pong to {}", client_id);
        }
        WsMessage::Data { payload, .. } => {
            // Handle client data messages (e.g., subscribe requests)
            if let Some(action) = payload.get("action").and_then(|v| v.as_str()) {
                match action {
                    "subscribe" => {
                        if let Some(channels) = payload.get("channels").and_then(|v| v.as_array()) {
                            for ch in channels {
                                if let Some(ch_str) = ch.as_str() {
                                    let _ = server.subscribe_client(client_id, ch_str).await;
                                }
                            }
                        }
                    }
                    "unsubscribe" => {
                        if let Some(channels) = payload.get("channels").and_then(|v| v.as_array()) {
                            for ch in channels {
                                if let Some(ch_str) = ch.as_str() {
                                    let _ = server.unsubscribe_client(client_id, ch_str).await;
                                }
                            }
                        }
                    }
                    _ => {
                        tracing::debug!("Unknown action: {}", action);
                    }
                }
            }
        }
        _ => {
            tracing::trace!("Unhandled WebSocket message: {:?}", msg);
        }
    }
}

/// Get WebSocket server status.
pub async fn ws_status(
    Extension(server): Extension<Arc<WebSocketServer>>,
) -> impl IntoResponse {
    let client_count = server.client_count().await;
    let server_id = server.server_id().to_string();

    axum::Json(serde_json::json!({
        "server_id": server_id,
        "connected_clients": client_count,
        "uptime": chrono::Utc::now().to_rfc3339(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welcome_message_serialization() {
        let msg = WsMessage::welcome("test-server".to_string());
        let json = msg.to_json().unwrap();
        assert!(json.contains("welcome"));
        assert!(json.contains("test-server"));
    }
}
