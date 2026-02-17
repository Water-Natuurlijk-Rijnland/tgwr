//! WebSocket service for real-time updates.
//!
//! This module provides WebSocket connection management, message broadcasting,
//! and client subscription handling for real-time updates.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use peilbeheer_core::{alert::AlertSeverity, WsAlertSeverity, WsMessage};

/// Maximum WebSocket message size (16MB)
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;
/// Default heartbeat interval in seconds
const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Connected client information.
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub subscriptions: HashSet<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// WebSocket server state.
#[derive(Clone)]
pub struct WebSocketServer {
    clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    broadcaster: broadcast::Sender<WsMessage>,
    server_id: String,
}

impl WebSocketServer {
    /// Create a new WebSocket server.
    pub fn new() -> Self {
        let (broadcaster, _) = broadcast::channel(1000);

        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcaster,
            server_id: Uuid::new_v4().to_string(),
        }
    }

    /// Get the server ID.
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// Get a clone of the broadcaster for sending messages.
    pub fn broadcaster(&self) -> broadcast::Sender<WsMessage> {
        self.broadcaster.clone()
    }

    /// Add a new client connection.
    pub async fn add_client(
        &self,
        client_id: String,
        user_id: Option<String>,
        username: Option<String>,
    ) {
        let mut clients = self.clients.write().await;
        clients.insert(client_id.clone(), ClientInfo {
            id: client_id,
            user_id,
            username,
            subscriptions: HashSet::from_iter(vec!["system".to_string(), "alerts".to_string()]),
            connected_at: chrono::Utc::now(),
        });
    }

    /// Remove a client connection.
    pub async fn remove_client(&self, client_id: &str) {
        let mut clients = self.clients.write().await;
        clients.remove(client_id);
        tracing::info!("WebSocket client disconnected: {}", client_id);
    }

    /// Subscribe a client to a channel.
    pub async fn subscribe_client(&self, client_id: &str, channel: &str) -> anyhow::Result<()> {
        let mut clients = self.clients.write().await;
        if let Some(info) = clients.get_mut(client_id) {
            info.subscriptions.insert(channel.to_string());
            tracing::debug!("Client {} subscribed to {}", client_id, channel);
        }
        Ok(())
    }

    /// Unsubscribe a client from a channel.
    pub async fn unsubscribe_client(&self, client_id: &str, channel: &str) -> anyhow::Result<()> {
        let mut clients = self.clients.write().await;
        if let Some(info) = clients.get_mut(client_id) {
            info.subscriptions.remove(channel);
            tracing::debug!("Client {} unsubscribed from {}", client_id, channel);
        }
        Ok(())
    }

    /// Get the number of connected clients.
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Get all connected client info.
    pub async fn get_client_info(&self) -> Vec<ClientInfo> {
        self.clients.read().await.values().cloned().collect()
    }

    /// Broadcast a message to all subscribed clients.
    pub async fn broadcast(&self, msg: WsMessage) {
        let _ = self.broadcaster.send(msg);
    }

    /// Broadcast scenario status update.
    pub async fn scenario_status(&self, scenario_id: &str, status: &str) {
        self.broadcast(WsMessage::scenario_status(
            scenario_id.to_string(),
            status.to_string(),
        )).await;
    }

    /// Broadcast scenario completion.
    pub async fn scenario_completed(&self, scenario_id: &str, result_id: &str, success: bool) {
        self.broadcast(WsMessage::scenario_completed(
            scenario_id.to_string(),
            result_id.to_string(),
            success,
        )).await;
    }

    /// Broadcast gemaal status update.
    pub async fn gemaal_status(&self, code: &str, status: &str, water_level: Option<f64>) {
        self.broadcast(WsMessage::GemaalStatus {
            code: code.to_string(),
            status: status.to_string(),
            water_level,
        }).await;
    }

    /// Broadcast alert.
    pub async fn alert(
        &self,
        id: String,
        severity: AlertSeverity,
        title: String,
        message: String,
    ) {
        let ws_severity = match severity {
            AlertSeverity::Info => WsAlertSeverity::Info,
            AlertSeverity::Warning => WsAlertSeverity::Warning,
            AlertSeverity::Error => WsAlertSeverity::Error,
            AlertSeverity::Critical => WsAlertSeverity::Critical,
        };
        self.broadcast(WsMessage::alert(id, ws_severity, title, message)).await;
    }

    /// Broadcast system status.
    pub async fn system_status(&self, healthy: bool, message: Option<String>) {
        self.broadcast(WsMessage::SystemStatus { healthy, message }).await;
    }
}

impl Default for WebSocketServer {
    fn default() -> Self {
        Self::new()
    }
}
