//! WebSocket IPC Daemon Server & Client Protocol
//!
//! Provides an asynchronous, bi-directional JSON-RPC / WebSocket bridge
//! connecting desktop UI (omnicore-ui) and mobile shell (omnicore-mobile)
//! directly to the headless omnicore-engine daemon.

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};

/// Incoming request payload from client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum DaemonRequest {
    GetStatus,
    SendMessage {
        recipient_id: String,
        content: String,
        primary_protocol: String,
        fallback_protocol: Option<String>,
    },
    SubscribeEvents {
        channel_id: Option<String>,
    },
    SyncFederation {
        channel_id: String,
        since_clock: HashMap<String, u64>,
    },
}

/// Outgoing response payload from engine to client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", content = "data")]
pub enum DaemonResponse {
    Ok {
        message: String,
        details: Option<serde_json::Value>,
    },
    Error {
        code: u32,
        reason: String,
    },
    EventNotification {
        event_type: String,
        payload: serde_json::Value,
    },
}

/// Active client session tracking.
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub session_id: String,
    pub client_type: String, // "desktop_ui", "mobile_shell", or "cli"
    pub subscribed_channel: Option<String>,
    pub connected_at_epoch: u64,
}

/// Server manager maintaining daemon endpoint configuration and active sessions.
#[derive(Debug, Clone)]
pub struct WebSocketServer {
    pub bind_address: String,
    pub port: u16,
    pub is_running: bool,
    sessions: Arc<RwLock<HashMap<String, ClientSession>>>,
    // Live socket writers per session (network listener only). In-process
    // sessions have no sender; broadcast_event still counts them logically.
    senders: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<String>>>>,
}

impl WebSocketServer {
    pub fn new(bind_address: &str, port: u16) -> Self {
        Self {
            bind_address: bind_address.to_string(),
            port,
            is_running: false,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn url(&self) -> String {
        format!("ws://{}:{}", self.bind_address, self.port)
    }

    pub async fn start(&mut self) -> Result<()> {
        self.is_running = true;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.is_running = false;
        let mut sessions = self.sessions.write().await;
        sessions.clear();
        drop(sessions);
        self.senders.write().await.clear();
        Ok(())
    }

    pub async fn register_session(&self, session_id: &str, client_type: &str, epoch: u64) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(
            session_id.to_string(),
            ClientSession {
                session_id: session_id.to_string(),
                client_type: client_type.to_string(),
                subscribed_channel: None,
                connected_at_epoch: epoch,
            },
        );
    }

    pub async fn unregister_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        drop(sessions);
        self.senders.write().await.remove(session_id);
    }

    /// Attaches a live socket writer to a session so broadcast_event pushes
    /// event frames over the network, not just counts them.
    pub async fn attach_sender(&self, session_id: &str, tx: mpsc::UnboundedSender<String>) {
        self.senders.write().await.insert(session_id.to_string(), tx);
    }

    /// Subscribes a registered session to a channel (or clears it with None).
    /// Returns false when the session is not registered. This is the typed
    /// equivalent of the SubscribeEvents JSON-RPC request.
    pub async fn subscribe_session(&self, session_id: &str, channel_id: Option<String>) -> bool {
        let mut sessions = self.sessions.write().await;
        match sessions.get_mut(session_id) {
            Some(sess) => {
                sess.subscribed_channel = channel_id;
                true
            }
            None => false,
        }
    }

    pub async fn subscribed_channel(&self, session_id: &str) -> Option<Option<String>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| s.subscribed_channel.clone())
    }

    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Broadcasts an event to registered client sessions, optionally filtering by subscribed channel.
    pub async fn broadcast_event(
        &self,
        event_type: &str,
        payload: serde_json::Value,
        target_channel: Option<&str>,
    ) -> usize {
        let sessions = self.sessions.read().await;
        let mut delivered_count = 0;

        let notification = DaemonResponse::EventNotification {
            event_type: event_type.to_string(),
            payload,
        };
        let frame = serde_json::to_string(&notification).unwrap_or_default();
        let senders = self.senders.read().await;

        for (session_id, session) in sessions.iter() {
            let should_deliver = match (target_channel, &session.subscribed_channel) {
                (None, _) => true, // Global broadcast
                (Some(target), Some(sub)) => target == sub,
                (Some(_), None) => false,
            };

            if should_deliver {
                delivered_count += 1;
                // Live-push to network clients; in-process sessions have no
                // sender and are counted logically (see struct docs).
                if let Some(tx) = senders.get(session_id) {
                    let _ = tx.send(frame.clone());
                }
            }
        }

        delivered_count
    }

    /// Handles one WebSocket connection: registers a session, replies to
    /// JSON-RPC requests, and forwards broadcast events until disconnect.
    pub async fn handle_websocket_connection(server: Arc<WebSocketServer>, stream: TcpStream, peer: SocketAddr) -> Result<()> {
        let ws = tokio_tungstenite::accept_async(stream).await.map_err(|e| anyhow!("WebSocket handshake failed: {}", e))?;
        let (mut sink, mut stream) = ws.split();
        let epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let session_id = format!("ws-{}-{}", peer, epoch);
        server.register_session(&session_id, "websocket", epoch).await;
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        server.attach_sender(&session_id, tx).await;

        let writer = tokio::spawn(async move {
            while let Some(frame) = rx.recv().await {
                if sink.send(tokio_tungstenite::tungstenite::Message::Text(frame)).await.is_err() {
                    break;
                }
            }
        });

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                    let response = match server.handle_raw_request(&session_id, &text).await {
                        Ok(r) => r,
                        Err(e) => serde_json::to_string(&DaemonResponse::Error { code: 400, reason: e.to_string() }).unwrap_or_default(),
                    };
                    let senders = server.senders.read().await;
                    if let Some(tx) = senders.get(&session_id) {
                        let _ = tx.send(response);
                    }
                }
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        server.unregister_session(&session_id).await;
        writer.abort();
        Ok(())
    }

    /// Accepts connections forever on an already-bound listener. Bind with
    /// TcpListener::bind((server.bind_address, server.port)) or port 0 in tests.
    pub async fn run_tcp_listener(server: Arc<WebSocketServer>, listener: TcpListener) -> Result<()> {
        loop {
            let (stream, peer) = listener.accept().await?;
            let srv = Arc::clone(&server);
            tokio::spawn(async move {
                let _ = WebSocketServer::handle_websocket_connection(srv, stream, peer).await;
            });
        }
    }

    /// Processes an incoming JSON string request from a connected client.
    pub async fn handle_raw_request(&self, session_id: &str, raw_json: &str) -> Result<String> {
        let req: DaemonRequest = serde_json::from_str(raw_json)
            .map_err(|e| anyhow!("Invalid JSON RPC request payload: {}", e))?;

        let response = match req {
            DaemonRequest::GetStatus => DaemonResponse::Ok {
                message: "OmniCore engine daemon active".to_string(),
                details: Some(serde_json::json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "running": self.is_running,
                    "bind": self.url(),
                })),
            },
            DaemonRequest::SendMessage {
                recipient_id,
                content,
                primary_protocol,
                fallback_protocol,
            } => DaemonResponse::Ok {
                message: "Message queued for multi-path dispatch".to_string(),
                details: Some(serde_json::json!({
                    "recipient": recipient_id,
                    "content_length": content.len(),
                    "primary": primary_protocol,
                    "fallback": fallback_protocol,
                    "status": "Queued",
                })),
            },
            DaemonRequest::SubscribeEvents { channel_id } => {
                let mut sessions = self.sessions.write().await;
                if let Some(sess) = sessions.get_mut(session_id) {
                    sess.subscribed_channel = channel_id.clone();
                }
                DaemonResponse::Ok {
                    message: format!("Subscribed to channel: {:?}", channel_id),
                    details: None,
                }
            }
            DaemonRequest::SyncFederation {
                channel_id,
                since_clock,
            } => DaemonResponse::Ok {
                message: format!("Federation delta sync initiated for {}", channel_id),
                details: Some(serde_json::json!({
                    "channel_id": channel_id,
                    "clock_keys": since_clock.len(),
                })),
            },
        };

        serde_json::to_string(&response)
            .map_err(|e| anyhow!("Failed to serialize response: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_lifecycle_and_url() {
        let mut server = WebSocketServer::new("127.0.0.1", 8765);
        assert_eq!(server.url(), "ws://127.0.0.1:8765");
        assert!(!server.is_running);

        server.start().await.unwrap();
        assert!(server.is_running);

        server.register_session("sess_1", "mobile_shell", 1700000000).await;
        assert_eq!(server.session_count().await, 1);

        server.stop().await.unwrap();
        assert!(!server.is_running);
        assert_eq!(server.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_request_dispatch_handling() {
        let mut server = WebSocketServer::new("127.0.0.1", 8765);
        server.start().await.unwrap();
        server.register_session("sess_ui", "desktop_ui", 1700000000).await;

        // Test GetStatus
        let req_status = serde_json::to_string(&DaemonRequest::GetStatus).unwrap();
        let res_status = server.handle_raw_request("sess_ui", &req_status).await.unwrap();
        assert!(res_status.contains("OmniCore engine daemon active"));

        // Test SendMessage
        let req_send = serde_json::to_string(&DaemonRequest::SendMessage {
            recipient_id: "alice_recip".to_string(),
            content: "Hello from UI client".to_string(),
            primary_protocol: "Signal".to_string(),
            fallback_protocol: Some("Matrix".to_string()),
        })
        .unwrap();
        let res_send = server.handle_raw_request("sess_ui", &req_send).await.unwrap();
        assert!(res_send.contains("Message queued for multi-path dispatch"));

        // Test SubscribeEvents
        let req_sub = serde_json::to_string(&DaemonRequest::SubscribeEvents {
            channel_id: Some("general_chan".to_string()),
        })
        .unwrap();
        let res_sub = server.handle_raw_request("sess_ui", &req_sub).await.unwrap();
        assert!(res_sub.contains("Subscribed to channel"));
    }

    #[tokio::test]
    async fn test_subscribe_session_typed_api() {
        let server = WebSocketServer::new("127.0.0.1", 8765);
        // Unknown session cannot subscribe.
        assert!(!server.subscribe_session("ghost", Some("general".to_string())).await);
        server.register_session("sess_1", "desktop_ui", 1700000000).await;
        assert!(server.subscribe_session("sess_1", Some("general".to_string())).await);
        assert_eq!(server.subscribed_channel("sess_1").await, Some(Some("general".to_string())));
        // Channel-targeted broadcast reaches only the subscriber.
        assert_eq!(server.broadcast_event("ping", serde_json::json!({}), Some("general")).await, 1);
        assert_eq!(server.broadcast_event("ping", serde_json::json!({}), Some("other")).await, 0);
        // Clearing the subscription stops channel-targeted delivery.
        assert!(server.subscribe_session("sess_1", None).await);
        assert_eq!(server.broadcast_event("ping", serde_json::json!({}), Some("general")).await, 0);
    }

    #[tokio::test]
    async fn test_websocket_listener_roundtrip_and_event_push() {
        use futures_util::{SinkExt as _, StreamExt as _};
        let server = Arc::new(WebSocketServer::new("127.0.0.1", 0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let srv = Arc::clone(&server);
        tokio::spawn(async move { let _ = WebSocketServer::run_tcp_listener(srv, listener).await; });

        let (mut client, _) = tokio_tungstenite::connect_async(format!("ws://{}", addr)).await.unwrap();
        // GetStatus RPC over the real socket.
        let req = serde_json::to_string(&DaemonRequest::GetStatus).unwrap();
        client.send(tokio_tungstenite::tungstenite::Message::Text(req)).await.unwrap();
        let reply = client.next().await.unwrap().unwrap().into_text().unwrap();
        assert!(reply.contains("OmniCore engine daemon active"), "reply: {}", reply);
        assert_eq!(server.session_count().await, 1);

        // Subscribe via RPC, then a broadcast must arrive as a pushed frame.
        let sub = serde_json::to_string(&DaemonRequest::SubscribeEvents { channel_id: Some("ops".to_string()) }).unwrap();
        client.send(tokio_tungstenite::tungstenite::Message::Text(sub)).await.unwrap();
        let _sub_reply = client.next().await.unwrap().unwrap();
        assert_eq!(server.broadcast_event("message_dispatched", serde_json::json!({"id": 1}), Some("ops")).await, 1);
        let pushed = tokio::time::timeout(std::time::Duration::from_secs(5), client.next()).await.unwrap().unwrap().unwrap().into_text().unwrap();
        assert!(pushed.contains("message_dispatched"), "pushed: {}", pushed);

        client.close(None).await.unwrap();
    }

    #[tokio::test]
    async fn test_broadcast_event_channel_filtering() {
        let server = WebSocketServer::new("127.0.0.1", 8765);

        // Register session 1 (subscribed to "dev_channel")
        server.register_session("sess_desktop", "desktop_ui", 1700000000).await;
        let sub_dev = serde_json::to_string(&DaemonRequest::SubscribeEvents {
            channel_id: Some("dev_channel".to_string()),
        })
        .unwrap();
        server.handle_raw_request("sess_desktop", &sub_dev).await.unwrap();

        // Register session 2 (subscribed to "random_channel")
        server.register_session("sess_mobile", "mobile_shell", 1700000000).await;
        let sub_rand = serde_json::to_string(&DaemonRequest::SubscribeEvents {
            channel_id: Some("random_channel".to_string()),
        })
        .unwrap();
        server.handle_raw_request("sess_mobile", &sub_rand).await.unwrap();

        // Target "dev_channel": only sess_desktop matches
        let count_dev = server
            .broadcast_event(
                "new_message",
                serde_json::json!({"text": "Hello devs"}),
                Some("dev_channel"),
            )
            .await;
        assert_eq!(count_dev, 1);

        // Global broadcast (target None): both sessions match
        let count_global = server
            .broadcast_event(
                "system_alert",
                serde_json::json!({"alert": "Server restart"}),
                None,
            )
            .await;
        assert_eq!(count_global, 2);
    }
}
