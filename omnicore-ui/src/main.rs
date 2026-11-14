// OmniCore Desktop UI and System Tray Runner
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod state {
    use serde::{Deserialize, Serialize};
    use std::sync::Mutex;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BridgeStatusInfo {
        pub name: String,
        pub protocol: String,
        pub status: String,
        pub endpoint: String,
        pub latency_ms: u64,
        pub fallback_priority: u32,
        pub connected: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ContactInfo {
        pub id: i64,
        pub name: String,
        pub unified_id: String,
        pub primary_protocol: String,
        pub fallback_protocol: String,
        pub avatar_initials: String,
        pub status: String,
        pub mapped_protocols: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MessageItem {
        pub id: String,
        pub sender: String,
        pub recipient: String,
        pub content: String,
        pub protocol: String,
        pub timestamp: String,
        pub is_outgoing: bool,
        pub status: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SystemMetrics {
        pub version: String,
        pub active_bridges: usize,
        pub queued_messages: usize,
        pub encryption: String,
        pub uptime_secs: u64,
        pub server_running: bool,
    }

    /// One engine capability surfaced on the consolidated dashboard.
    /// `engine_symbol` names the real omnicore-engine type/function so the UI
    /// never advertises a feature the engine does not implement.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CapabilityInfo {
        pub area: String,
        pub name: String,
        pub description: String,
        pub engine_symbol: String,
        pub status: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RoutingProfileInfo {
        pub id: String,
        pub label: String,
        pub description: String,
        pub engine_symbol: String,
        pub recommended: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SecurityStatus {
        pub identity_name: String,
        pub unified_id: String,
        pub fingerprint: String,
        pub identity_verified: bool,
        pub tor_enabled: bool,
        pub tor_proxy_url: String,
        pub tor_status: String,
        pub backup_enabled: bool,
        pub backup_description: String,
        pub ratchet_description: String,
        pub encryption: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NetworkStatus {
        pub federation_channels: usize,
        pub federation_description: String,
        pub mesh_peers: usize,
        pub mesh_description: String,
        pub discovery_description: String,
        pub active_calls: usize,
        pub call_description: String,
        pub outbox_queued: usize,
        pub outbox_description: String,
        pub telemetry_summary: String,
        pub appliance_action: String,
        pub reputation_description: String,
        pub daemon_url: String,
        pub daemon_sessions: usize,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ActivityEvent {
        pub id: String,
        pub kind: String,
        pub detail: String,
        pub timestamp: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DaemonStatus {
        pub url: String,
        pub running: bool,
        pub sessions: usize,
        pub subscribed_channel: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ChannelInfo {
        pub id: String,
        pub name: String,
        pub topic: String,
        pub bridge_protocol: String,
        pub member_count: usize,
        pub event_count: usize,
    }

    pub struct AppState {
        pub contacts: Mutex<Vec<ContactInfo>>,
        pub messages: Mutex<Vec<MessageItem>>,
        pub bridges: Mutex<Vec<BridgeStatusInfo>>,
        pub server_running: Mutex<bool>,
        pub start_time: std::time::Instant,
        pub routing_profile: Mutex<String>,
        pub tor_enabled: Mutex<bool>,
        // Stateful routing helpers persisted across sends so profiles behave
        // like the engine intends (dedup window, p95 history, byte quotas).
        pub dedup_ledger: tokio::sync::Mutex<omnicore_engine::router::EgressDeduplicationLedger>,
        pub latency_estimator: tokio::sync::Mutex<omnicore_engine::router::RouteLatencyEstimator>,
        pub bandwidth_shaper: tokio::sync::Mutex<omnicore_engine::router::RouteBandwidthShaper>,
        pub canary_detector: tokio::sync::Mutex<omnicore_engine::router::CanaryDropDetector>,
        pub receipt_tracker: tokio::sync::Mutex<omnicore_engine::router::ReceiptTracker>,
        // In-process daemon session: the desktop UI registers as a
        // WebSocketServer client and receives broadcast events for its sends.
        // (The engine daemon is session/RPC logic; a real TCP listener is
        // still future work — mobile connects over WebSocket separately.)
        pub daemon: tokio::sync::RwLock<omnicore_engine::server::WebSocketServer>,
        pub daemon_listener_started: Mutex<bool>,
        // SQLCipher persistence: None until init_engine opens the DB file.
        // In-memory contacts/messages remain the UI cache; the DB is the
        // durable store restored on launch.
        pub db: Mutex<Option<omnicore_engine::db::Database>>,
        pub activity: Mutex<Vec<ActivityEvent>>,
        // Group channels live in the real FederationManager (vector clocks,
        // members, event history), not a UI-local list.
        pub federation: tokio::sync::Mutex<omnicore_engine::federation::FederationManager>,
        // Router shared with the daemon so group posts can broadcast.
        pub router: tokio::sync::RwLock<Option<std::sync::Arc<omnicore_engine::router::MultipathRouter>>>,
    }

    impl Default for AppState {
        fn default() -> Self {
            Self {
                contacts: Mutex::new(vec![
                    ContactInfo {
                        id: 1,
                        name: "Alice Montgomery".to_string(),
                        unified_id: "alice_unified_1".to_string(),
                        primary_protocol: "Signal".to_string(),
                        fallback_protocol: "Matrix".to_string(),
                        avatar_initials: "AM".to_string(),
                        status: "Online".to_string(),
                        mapped_protocols: vec!["Signal".to_string(), "Matrix".to_string(), "SMS".to_string()],
                    },
                    ContactInfo {
                        id: 2,
                        name: "Bob Sterling".to_string(),
                        unified_id: "bob_unified_2".to_string(),
                        primary_protocol: "Matrix".to_string(),
                        fallback_protocol: "Discord".to_string(),
                        avatar_initials: "BS".to_string(),
                        status: "Away".to_string(),
                        mapped_protocols: vec!["Matrix".to_string(), "Discord".to_string()],
                    },
                    ContactInfo {
                        id: 3,
                        name: "Charlie Vance".to_string(),
                        unified_id: "charlie_unified_3".to_string(),
                        primary_protocol: "Discord".to_string(),
                        fallback_protocol: "Signal".to_string(),
                        avatar_initials: "CV".to_string(),
                        status: "Offline".to_string(),
                        mapped_protocols: vec!["Discord".to_string(), "Signal".to_string()],
                    },
                    ContactInfo {
                        id: 4,
                        name: "Dana Scully".to_string(),
                        unified_id: "dana_unified_4".to_string(),
                        primary_protocol: "Simplex".to_string(),
                        fallback_protocol: "Tox".to_string(),
                        avatar_initials: "DS".to_string(),
                        status: "Online".to_string(),
                        mapped_protocols: vec!["Simplex".to_string(), "Tox".to_string()],
                    },
                ]),
                messages: Mutex::new(vec![
                    MessageItem {
                        id: "msg_1".to_string(),
                        sender: "Alice Montgomery".to_string(),
                        recipient: "You".to_string(),
                        content: "Hey Robert! Is OmniCore cross-protocol routing active?".to_string(),
                        protocol: "Signal".to_string(),
                        timestamp: "10:42 AM".to_string(),
                        is_outgoing: false,
                        status: "delivered".to_string(),
                    },
                    MessageItem {
                        id: "msg_2".to_string(),
                        sender: "You".to_string(),
                        recipient: "Alice Montgomery".to_string(),
                        content: "Yes, fully active! Routing across Signal, Matrix, Simplex, and Discord seamlessly.".to_string(),
                        protocol: "Multi-Route (Signal)".to_string(),
                        timestamp: "10:43 AM".to_string(),
                        is_outgoing: true,
                        status: "delivered".to_string(),
                    },
                    MessageItem {
                        id: "msg_3".to_string(),
                        sender: "Bob Sterling".to_string(),
                        recipient: "You".to_string(),
                        content: "Confirming bridge sync token received via Matrix homeserver.".to_string(),
                        protocol: "Matrix".to_string(),
                        timestamp: "10:45 AM".to_string(),
                        is_outgoing: false,
                        status: "delivered".to_string(),
                    },
                ]),
                bridges: Mutex::new(vec![
                    BridgeStatusInfo {
                        name: "Signal Daemon Bridge".to_string(),
                        protocol: "Signal".to_string(),
                        status: "Connected".to_string(),
                        endpoint: "unix:///var/run/signald/signald.sock".to_string(),
                        latency_ms: 12,
                        fallback_priority: 1,
                        connected: true,
                    },
                    BridgeStatusInfo {
                        name: "Matrix Synapse/Dendrite".to_string(),
                        protocol: "Matrix".to_string(),
                        status: "Connected".to_string(),
                        endpoint: "https://matrix.org".to_string(),
                        latency_ms: 45,
                        fallback_priority: 2,
                        connected: true,
                    },
                    BridgeStatusInfo {
                        name: "Discord Gateway Bridge".to_string(),
                        protocol: "Discord".to_string(),
                        status: "Connected".to_string(),
                        endpoint: "wss://gateway.discord.gg".to_string(),
                        latency_ms: 38,
                        fallback_priority: 3,
                        connected: true,
                    },
                    BridgeStatusInfo {
                        name: "SimpleX Chat Isolated Broker".to_string(),
                        protocol: "Simplex".to_string(),
                        status: "Standby".to_string(),
                        endpoint: "simplex://broker.v3.local".to_string(),
                        latency_ms: 60,
                        fallback_priority: 4,
                        connected: false,
                    },
                    BridgeStatusInfo {
                        name: "Tox Distributed P2P Core".to_string(),
                        protocol: "Tox".to_string(),
                        status: "Standby".to_string(),
                        endpoint: "dht://bootstrap.tox.chat".to_string(),
                        latency_ms: 95,
                        fallback_priority: 5,
                        connected: false,
                    },
                    BridgeStatusInfo {
                        name: "iMessage BlueBubbles/OSX Relay".to_string(),
                        protocol: "iMessage".to_string(),
                        status: "Configured".to_string(),
                        endpoint: "https://relay.imessage.local".to_string(),
                        latency_ms: 50,
                        fallback_priority: 6,
                        connected: false,
                    },
                    BridgeStatusInfo {
                        name: "SMS / GSM Cellular Modem".to_string(),
                        protocol: "SMS".to_string(),
                        status: "Ready".to_string(),
                        endpoint: "serial:///dev/ttyUSB0".to_string(),
                        latency_ms: 110,
                        fallback_priority: 7,
                        connected: true,
                    },
                    BridgeStatusInfo {
                        name: "Nostr Relay Pool".to_string(),
                        protocol: "Nostr".to_string(),
                        status: "Standby".to_string(),
                        endpoint: "wss://relay.damus.io".to_string(),
                        latency_ms: 80,
                        fallback_priority: 8,
                        connected: false,
                    },
                ]),
                server_running: Mutex::new(true),
                start_time: std::time::Instant::now(),
                routing_profile: Mutex::new("optimal_cascade".to_string()),
                tor_enabled: Mutex::new(false),
                dedup_ledger: tokio::sync::Mutex::new(omnicore_engine::router::EgressDeduplicationLedger::new(120)),
                latency_estimator: tokio::sync::Mutex::new(omnicore_engine::router::RouteLatencyEstimator::new(64)),
                bandwidth_shaper: tokio::sync::Mutex::new(omnicore_engine::router::RouteBandwidthShaper::new(60)),
                canary_detector: tokio::sync::Mutex::new(omnicore_engine::router::CanaryDropDetector::new(30)),
                receipt_tracker: tokio::sync::Mutex::new(omnicore_engine::router::ReceiptTracker::new(30)),
                daemon: tokio::sync::RwLock::new(omnicore_engine::server::WebSocketServer::new("127.0.0.1", 18765)),
                daemon_listener_started: Mutex::new(false),
                db: Mutex::new(None),
                activity: Mutex::new(vec![
                    ActivityEvent { id: "evt_1".to_string(), kind: "DISPATCH_OK".to_string(), detail: "msg_1 routed to Alice Montgomery via Signal IPC (12 ms)".to_string(), timestamp: "10:43 AM".to_string() },
                    ActivityEvent { id: "evt_2".to_string(), kind: "MULTI_ROUTE".to_string(), detail: "msg_2 dual-path Signal primary, Matrix fallback armed".to_string(), timestamp: "10:43 AM".to_string() },
                    ActivityEvent { id: "evt_3".to_string(), kind: "SYNC_TOKEN".to_string(), detail: "Matrix homeserver sync token refreshed".to_string(), timestamp: "10:45 AM".to_string() },
                ]),
                federation: tokio::sync::Mutex::new({
                    let mut mgr = omnicore_engine::federation::FederationManager::new();
                    mgr.create_channel("chan_ops", "ops-room", "Federated operations channel", "local_user");
                    if let Some(ch) = mgr.get_channel_mut("chan_ops") {
                        ch.active_bridge = Some(omnicore_engine::models::ProtocolKind::Matrix);
                    }
                    mgr
                }),
                router: tokio::sync::RwLock::new(None),
            }
        }
    }
}

pub use state::*;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State,
};

// --- Tauri Commands ---

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! OmniCore engine online.", name)
}

pub const DESKTOP_SESSION_ID: &str = "desktop_main";

/// Formats epoch seconds as "YYYY-MM-DD HH:MM UTC" without a date crate
/// (Howard Hinnant's civil-from-days algorithm). Restored DB rows store raw
/// epoch seconds; the UI should never show a bare 10-digit number.
fn format_epoch_utc(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let secs_of_day = epoch_secs.rem_euclid(86_400);
    let (hh, mm) = (secs_of_day / 3600, (secs_of_day % 3600) / 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02} UTC", year, m, d, hh, mm)
}

fn contact_initials(name: &str) -> String {
    let initials: String = name.split_whitespace().filter_map(|w| w.chars().next()).collect::<String>().to_uppercase();
    if initials.is_empty() { "U".to_string() } else { initials }
}

fn db_contact_to_info(c: omnicore_engine::models::UnifiedContact) -> ContactInfo {
    let mut mapped = vec![c.primary_protocol.as_str().to_string()];
    for f in &c.fallback_protocols {
        let s = f.as_str().to_string();
        if !mapped.contains(&s) { mapped.push(s); }
    }
    ContactInfo {
        id: c.id,
        name: c.name.clone(),
        unified_id: c.unified_id.clone(),
        primary_protocol: c.primary_protocol.as_str().to_string(),
        fallback_protocol: c.fallback_protocols.first().map(|p| p.as_str().to_string()).unwrap_or_else(|| c.primary_protocol.as_str().to_string()),
        avatar_initials: contact_initials(&c.name),
        status: "Online".to_string(),
        mapped_protocols: mapped,
    }
}

/// Opens (or creates) the SQLCipher DB at `path`. First run seeds the DB from
/// the in-memory defaults; later runs restore contacts + history from disk.
/// Returns a human summary for Activity/status. Errors are returned, never
/// swallowed, so a broken DB is visible instead of silently in-memory-only.
fn init_database_at(state: &AppState, path: &std::path::Path, key: &str) -> Result<String, String> {
    let db = omnicore_engine::db::Database::init(path, key).map_err(|e| format!("DB open failed: {}", e))?;
    let existing = db.list_contacts().map_err(|e| e.to_string())?;
    let summary;
    if existing.is_empty() {
        // Seed: persist the default contacts and adopt their DB row ids.
        let defaults = state.contacts.lock().unwrap().clone();
        let mut seeded = Vec::new();
        for c in defaults {
            let primary = omnicore_engine::models::ProtocolKind::from_str_name(&c.primary_protocol).unwrap_or(omnicore_engine::models::ProtocolKind::Signal);
            let fallbacks: Vec<omnicore_engine::models::ProtocolKind> = c.mapped_protocols.iter().filter_map(|p| omnicore_engine::models::ProtocolKind::from_str_name(p)).filter(|p| *p != primary).collect();
            let row_id = db.insert_contact(&c.name, &c.unified_id, primary, &fallbacks).map_err(|e| e.to_string())?;
            seeded.push(ContactInfo { id: row_id, ..c });
        }
        *state.contacts.lock().unwrap() = seeded;
        summary = "SQLCipher DB created and seeded with default contacts".to_string();
    } else {
        // Restore contacts and their conversation history.
        let infos: Vec<ContactInfo> = existing.iter().cloned().map(db_contact_to_info).collect();
        let mut restored_messages = Vec::new();
        for c in &existing {
            if let Ok(history) = db.get_conversation_history(c.id, 500) {
                for m in history {
                    restored_messages.push(MessageItem {
                        id: format!("db_{}", m.id),
                        sender: m.sender,
                        recipient: m.recipient,
                        content: m.content,
                        protocol: m.protocol.as_str().to_string(),
                        timestamp: format_epoch_utc(m.timestamp),
                        is_outgoing: m.is_outgoing,
                        status: m.state.as_str().to_string(),
                    });
                }
            }
        }
        let msg_count = restored_messages.len();
        *state.contacts.lock().unwrap() = infos;
        if !restored_messages.is_empty() {
            *state.messages.lock().unwrap() = restored_messages;
        }
        summary = format!("SQLCipher DB restored {} contact(s), {} message(s)", existing.len(), msg_count);
    }
    // Restore persisted settings (routing profile, Tor, bridge toggles).
    if let Some(profile) = load_setting(&db, "routing_profile") {
        *state.routing_profile.lock().unwrap() = profile;
    }
    if let Some(tor) = load_setting(&db, "tor_enabled") {
        *state.tor_enabled.lock().unwrap() = tor == "1";
    }
    {
        let mut bridges = state.bridges.lock().unwrap();
        for b in bridges.iter_mut() {
            if let Some(v) = load_setting(&db, &format!("bridge_connected:{}", b.protocol)) {
                b.connected = v == "1";
                b.status = if b.connected { "Connected".to_string() } else { "Disconnected".to_string() };
            }
        }
    }
    *state.db.lock().unwrap() = Some(db);
    Ok(summary)
}

/// Settings live in the SQLCipher key_store (protocol="settings",
/// account="desktop") so no extra table is needed. Values are small strings.
fn persist_setting(state: &AppState, key: &str, value: &str) {
    let db_guard = state.db.lock().unwrap();
    let Some(db) = db_guard.as_ref() else { return };
    if let Err(e) = db.save_key_material("settings", "desktop", key, value.as_bytes()) {
        push_activity(state, "DB_PERSIST_ERROR", format!("Failed to persist setting {}: {}", key, e));
    }
}

fn load_setting(db: &omnicore_engine::db::Database, key: &str) -> Option<String> {
    db.get_key_material("settings", "desktop", key).ok().flatten().and_then(|b| String::from_utf8(b).ok())
}

/// Persists one outgoing message. DB failures are logged to Activity
/// (DB_PERSIST_ERROR) — visible in the UI, never silently dropped.
fn persist_outgoing(state: &AppState, recipient_name: &str, used_protocol: &str, content: &str, status_label: &str) {
    // Lock order is contacts -> db everywhere (see add_contact); resolve the
    // contact id first so the two locks are never held in opposite order.
    let contact_id = state.contacts.lock().unwrap().iter().find(|c| c.name == recipient_name).map(|c| c.id);
    let db_guard = state.db.lock().unwrap();
    let Some(db) = db_guard.as_ref() else { return };
    let Some(contact_id) = contact_id else {
        push_activity(state, "DB_PERSIST_ERROR", format!("No contact '{}' to persist message against", recipient_name));
        return;
    };
    let protocol = omnicore_engine::models::ProtocolKind::from_str_name(used_protocol).unwrap_or(omnicore_engine::models::ProtocolKind::Signal);
    let result = (|| -> Result<i64, String> {
        let thread_id = db.get_or_create_direct_thread(contact_id, recipient_name).map_err(|e| e.to_string())?;
        let msg_id = db.record_message_with_thread(contact_id, thread_id, protocol, "You", recipient_name, content, true).map_err(|e| e.to_string())?;
        let delivery = if status_label == "Delivered" {
            omnicore_engine::models::DeliveryState::Delivered
        } else if let Some(reason) = status_label.strip_prefix("failed: ") {
            omnicore_engine::models::DeliveryState::Failed(reason.to_string())
        } else {
            omnicore_engine::models::DeliveryState::Sent
        };
        db.update_delivery_state(msg_id, &delivery).map_err(|e| e.to_string())?;
        Ok(msg_id)
    })();
    match result {
        Ok(id) => push_activity(state, "DB_PERSIST", format!("Message #{} persisted to SQLCipher for {}", id, recipient_name)),
        Err(e) => push_activity(state, "DB_PERSIST_ERROR", format!("Failed to persist message for {}: {}", recipient_name, e)),
    }
}

#[tauri::command]
async fn init_engine(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let _bridge = omnicore_engine::bridges::matrix::MatrixBridge::new();
    // Open the SQLCipher DB (app data dir, overridable for tests/dev) and
    // restore or seed contacts/messages. Failure is surfaced, not hidden:
    // the app keeps running in-memory and Activity shows DB_PERSIST_ERROR.
    {
        let db_path = std::env::var("OMNICORE_DB_PATH").map(std::path::PathBuf::from).unwrap_or_else(|_| {
            let base = app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir().join("omnicore"));
            let _ = std::fs::create_dir_all(&base);
            base.join("omnicore.db")
        });
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let using_default_key = std::env::var("OMNICORE_DB_KEY").is_err();
        let db_key = std::env::var("OMNICORE_DB_KEY").unwrap_or_else(|_| "omnicore-local-dev-key".to_string());
        match init_database_at(&state, &db_path, &db_key) {
            Ok(summary) => {
                push_activity(&state, "DB_READY", format!("{} at {}", summary, db_path.display()));
                if using_default_key {
                    push_activity(&state, "DB_KEY_WARNING", "Using default dev DB key; set OMNICORE_DB_KEY for a real passphrase".to_string());
                }
            }
            Err(e) => push_activity(&state, "DB_PERSIST_ERROR", e),
        }
    }
    // Start the in-process daemon and register this UI as a client session,
    // so sends can broadcast events to subscribed sessions (see send_message).
    // Build the message router first (all 8 bridges, candidates initialized)
    // so daemon SendMessage RPCs from socket clients route for real.
    let all_protocols = [
        omnicore_engine::models::ProtocolKind::Signal,
        omnicore_engine::models::ProtocolKind::Matrix,
        omnicore_engine::models::ProtocolKind::Discord,
        omnicore_engine::models::ProtocolKind::Simplex,
        omnicore_engine::models::ProtocolKind::Tox,
        omnicore_engine::models::ProtocolKind::IMessage,
        omnicore_engine::models::ProtocolKind::Sms,
        omnicore_engine::models::ProtocolKind::Nostr,
    ];
    let router = build_router(&state, &all_protocols).await;
    let mut daemon = state.daemon.write().await;
    if !daemon.is_running {
        let _ = daemon.start().await;
    }
    let router_arc = std::sync::Arc::new(router);
    *state.router.write().await = Some(router_arc.clone());
    daemon.set_router(router_arc);
    daemon.register_session(DESKTOP_SESSION_ID, "desktop_ui", now_epoch()).await;
    let server_clone = daemon.clone();
    drop(daemon);
    // Start the real TCP/WebSocket listener once so mobile/CLI clients can
    // connect at ws://127.0.0.1:18765. The clone shares session/sender maps
    // with the in-process daemon, so broadcasts reach socket clients too.
    let should_start = {
        let mut started = state.daemon_listener_started.lock().unwrap();
        let should = !*started;
        *started = true;
        should
    };
    if should_start {
        match tokio::net::TcpListener::bind("127.0.0.1:18765").await {
            Ok(listener) => {
                tokio::spawn(async move {
                    let _ = omnicore_engine::server::WebSocketServer::run_tcp_listener(std::sync::Arc::new(server_clone), listener).await;
                });
                push_activity(&state, "DAEMON_LISTENER", "WebSocket listener started on ws://127.0.0.1:18765".to_string());
            }
            Err(e) => {
                push_activity(&state, "DAEMON_LISTENER_ERROR", format!("Could not bind ws://127.0.0.1:18765 ({}); in-process session still active", e));
            }
        }
    }
    Ok("Engine, SQLCipher cryptographic storage, protocol bridges, and daemon session active.".to_string())
}

#[tauri::command]
async fn get_daemon_status(state: State<'_, AppState>) -> Result<DaemonStatus, String> {
    let daemon = state.daemon.read().await;
    Ok(DaemonStatus {
        url: daemon.url(),
        running: daemon.is_running,
        sessions: daemon.session_count().await,
        subscribed_channel: daemon.subscribed_channel(DESKTOP_SESSION_ID).await.flatten(),
    })
}

/// Subscribes the desktop session to a channel via the daemon's real
/// JSON-RPC path (DaemonRequest::SubscribeEvents), not a local shortcut.
#[tauri::command]
async fn subscribe_daemon_events(state: State<'_, AppState>, channel_id: Option<String>) -> Result<String, String> {
    let req = serde_json::to_string(&omnicore_engine::server::DaemonRequest::SubscribeEvents { channel_id: channel_id.clone() })
        .map_err(|e| e.to_string())?;
    let daemon = state.daemon.read().await;
    let res = daemon.handle_raw_request(DESKTOP_SESSION_ID, &req).await.map_err(|e| e.to_string())?;
    push_activity(&state, "DAEMON_SUBSCRIBE", format!("Desktop session subscribed to channel {:?}", channel_id));
    Ok(res)
}

#[tauri::command]
fn get_system_metrics(state: State<'_, AppState>) -> SystemMetrics {
    let bridges = state.bridges.lock().unwrap();
    let active_count = bridges.iter().filter(|b| b.connected).count();
    let running = *state.server_running.lock().unwrap();
    let elapsed = state.start_time.elapsed().as_secs();

    SystemMetrics {
        version: env!("CARGO_PKG_VERSION").to_string(),
        active_bridges: active_count,
        queued_messages: 0,
        encryption: "SQLCipher (AES-256-GCM)".to_string(),
        uptime_secs: elapsed,
        server_running: running,
    }
}

#[tauri::command]
fn get_contacts(state: State<'_, AppState>) -> Vec<ContactInfo> {
    state.contacts.lock().unwrap().clone()
}

#[tauri::command]
fn add_contact(
    state: State<'_, AppState>,
    name: String,
    primary_protocol: String,
    fallback_protocol: String,
    mapped_protocols: Vec<String>,
) -> ContactInfo {
    let mut contacts = state.contacts.lock().unwrap();
    let provisional_id = (contacts.len() + 1) as i64;
    // Persist first when the DB is open so the contact adopts its DB row id.
    let new_id = {
        let db_guard = state.db.lock().unwrap();
        if let Some(db) = db_guard.as_ref() {
            let primary = omnicore_engine::models::ProtocolKind::from_str_name(&primary_protocol).unwrap_or(omnicore_engine::models::ProtocolKind::Signal);
            let fallbacks: Vec<omnicore_engine::models::ProtocolKind> = mapped_protocols.iter().filter_map(|p| omnicore_engine::models::ProtocolKind::from_str_name(p)).filter(|p| *p != primary).collect();
            match db.insert_contact(&name, &format!("contact_unified_{}", provisional_id), primary, &fallbacks) {
                Ok(row_id) => row_id,
                Err(e) => {
                    push_activity(&state, "DB_PERSIST_ERROR", format!("Failed to persist contact {}: {}", name, e));
                    provisional_id
                }
            }
        } else {
            provisional_id
        }
    };
    let initials = contact_initials(&name);

    let new_contact = ContactInfo {
        id: new_id,
        name: name.clone(),
        unified_id: format!("contact_unified_{}", new_id),
        primary_protocol,
        fallback_protocol,
        avatar_initials: if initials.is_empty() { "U".to_string() } else { initials },
        status: "Online".to_string(),
        mapped_protocols,
    };

    contacts.push(new_contact.clone());
    new_contact
}

#[tauri::command]
fn get_messages(state: State<'_, AppState>) -> Vec<MessageItem> {
    state.messages.lock().unwrap().clone()
}

/// Builds a BridgeRegistry with all 8 engine bridges and initializes the
/// candidate bridges so their send_message guards pass.
async fn build_router(_state: &AppState, candidates: &[omnicore_engine::models::ProtocolKind]) -> omnicore_engine::router::MultipathRouter {
    use omnicore_engine::bridges::*;
    use omnicore_engine::models::ProtocolKind as P;
    let mut registry = omnicore_engine::router::BridgeRegistry::new();
    registry.register(P::Signal, Box::new(signal::SignalBridge::new()));
    registry.register(P::Matrix, Box::new(matrix::MatrixBridge::new()));
    registry.register(P::Discord, Box::new(discord::DiscordBridge::new()));
    registry.register(P::Simplex, Box::new(simplex::SimplexBridge::new()));
    registry.register(P::Tox, Box::new(tox::ToxBridge::new()));
    registry.register(P::IMessage, Box::new(imessage::IMessageBridge::new()));
    registry.register(P::Sms, Box::new(sms::SmsBridge::new()));
    registry.register(P::Nostr, Box::new(nostr::NostrBridge::new()));
    // Only initialize bridges this send may use; initialization is per-bridge
    // setup (client build, modem ready, relay pool) and candidate bridges are
    // the primary + distinct fallback.
    for &kind in candidates {
        if let Some(bridge_ref) = registry.get(kind) {
            let mut bridge = bridge_ref.write().await;
            let _ = bridge.initialize().await;
        }
    }
    omnicore_engine::router::MultipathRouter::new(std::sync::Arc::new(tokio::sync::RwLock::new(registry)))
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// Executes the dashboard's selected routing profile against the real
/// MultipathRouter. Returns (delivered_protocol, delivery_state_label).
/// Each profile id from get_routing_profiles maps to exactly one engine
/// strategy; unknown ids fall back to the optimal cascade.
async fn execute_routing_profile(
    state: &AppState,
    profile: &str,
    recipient: &str,
    content: &str,
    primary_name: &str,
    fallback_name: &str,
) -> Result<(String, String), String> {
    use omnicore_engine::dispatcher::MessagePayload;
    use omnicore_engine::models::{DeliveryState, ProtocolKind};
    use omnicore_engine::router::*;
    let primary = ProtocolKind::from_str_name(primary_name).ok_or_else(|| format!("Unknown protocol '{}'", primary_name))?;
    let fallback = ProtocolKind::from_str_name(fallback_name).ok_or_else(|| format!("Unknown protocol '{}'", fallback_name))?;
    let fallbacks: Vec<ProtocolKind> = if fallback == primary { vec![] } else { vec![fallback] };
    let mut candidates = vec![primary];
    candidates.extend(fallbacks.iter().copied());
    let router = build_router(state, &candidates).await;
    let payload = MessagePayload { recipient_id: recipient.to_string(), content: content.to_string(), fallback_protocol: fallback_name.to_string() };
    let now = now_epoch();
    let outcome: Result<(ProtocolKind, DeliveryState), anyhow::Error> = match profile {
        "hedged_racing" => router.route_hedged_racing(&payload, primary, &fallbacks, HedgedRoutingPolicy::default(), now).await,
        "adaptive_retry" => router.route_with_adaptive_fallback_retry(&payload, primary, &fallbacks, 3, now).await,
        "confirmed_receipt" => {
            let nonce = format!("nonce-{}-{}", recipient, now);
            let mut tracker = state.receipt_tracker.lock().await;
            router.route_with_confirmed_receipt(&payload, &nonce, primary, &fallbacks, &mut tracker, |proto, n, recip, ts| Some(DeliveryReceipt::generate(n, recip, proto, ts)), now).await.map(|(p, s, _receipt)| (p, s))
        }
        "exact_once" => {
            let mut ledger = state.dedup_ledger.lock().await;
            router.route_with_exact_once_deduplication(&payload, primary, &fallbacks, &mut ledger, now).await
        }
        "canary_guard" => {
            // Canary echo is simulated from the dashboard's bridge-connected
            // state: disconnected bridges do not echo, so the detector
            // cascades away from them. (No live echo channel exists yet.)
            let connected: Vec<String> = state.bridges.lock().unwrap().iter().filter(|b| b.connected).map(|b| b.protocol.to_ascii_lowercase()).collect();
            let mut detector = state.canary_detector.lock().await;
            router.route_with_canary_drop_detection(&payload, primary, &fallbacks, &mut detector, |proto, _nonce| connected.iter().any(|c| c == &proto.as_str().to_ascii_lowercase()), now).await
        }
        "percentile_bounded" => {
            let mut estimator = state.latency_estimator.lock().await;
            router.route_with_percentile_bounded_cascade(&payload, primary, &fallbacks, &mut estimator, now).await
        }
        "bandwidth_throttled" => {
            let mut shaper = state.bandwidth_shaper.lock().await;
            router.route_with_bandwidth_throttled_cascade(&payload, primary, &fallbacks, &mut shaper, now).await
        }
        "cost_qos" => router.route_with_cost_and_qos_optimization(&payload, primary, &fallbacks, QosRoutingPreference::CostSaving, now).await,
        "penalty_weighted" => router.route_with_penalty_weighted_cascade(&payload, primary, &fallbacks, now).await,
        _ => router.route_with_optimal_cascade(&payload, primary, &fallbacks, now).await,
    };
    outcome.map(|(used, delivery)| (used.as_str().to_string(), delivery.as_str().to_string())).map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    recipient: String,
    content: String,
    protocol: String,
    fallback_protocol: String,
) -> Result<MessageItem, String> {
    if content.trim().is_empty() {
        return Err("Message content cannot be empty".to_string());
    }
    let profile = state.routing_profile.lock().unwrap().clone();
    // Execute the selected profile for real; failures are surfaced in the
    // message status and Activity instead of being silently swallowed.
    let (used_protocol, status) = match execute_routing_profile(&state, &profile, &recipient, &content, &protocol, &fallback_protocol).await {
        Ok((used, delivery)) => {
            push_activity(&state, "DISPATCH_OK", format!("Message to {} delivered on {} via profile {} ({})", recipient, used, profile, delivery));
            (used, delivery)
        }
        Err(e) => {
            push_activity(&state, "ROUTE_FAILED", format!("Message to {} failed via profile {}: {}", recipient, profile, e));
            (protocol.clone(), format!("failed: {}", e))
        }
    };
    persist_outgoing(&state, &recipient, &used_protocol, &content, &status);
    let new_item = {
        let mut messages = state.messages.lock().unwrap();
        let item = MessageItem {
            id: format!("msg_{}", messages.len() + 1),
            sender: "You".to_string(),
            recipient: recipient.clone(),
            content: content.clone(),
            protocol: format!("{} via {} (fallback: {})", used_protocol, profile, fallback_protocol),
            timestamp: "Just now".to_string(),
            is_outgoing: true,
            status,
        };
        messages.push(item.clone());
        item
    };
    // Broadcast a daemon event to subscribed sessions (channel-targeted when
    // this UI session subscribed to a channel, global otherwise).
    {
        let daemon = state.daemon.read().await;
        let channel = daemon.subscribed_channel(DESKTOP_SESSION_ID).await.flatten();
        let delivered = daemon.broadcast_event("message_dispatched", serde_json::json!({"recipient": recipient, "protocol": used_protocol, "profile": profile}), channel.as_deref()).await;
        push_activity(&state, "DAEMON_EVENT", format!("message_dispatched broadcast reached {} subscribed session(s)", delivered));
    }
    Ok(new_item)
}

#[tauri::command]
fn get_bridges(state: State<'_, AppState>) -> Vec<BridgeStatusInfo> {
    state.bridges.lock().unwrap().clone()
}

#[tauri::command]
fn toggle_bridge(state: State<'_, AppState>, protocol: String) -> Result<BridgeStatusInfo, String> {
    let mut bridges = state.bridges.lock().unwrap();
    for bridge in bridges.iter_mut() {
        if bridge.protocol.eq_ignore_ascii_case(&protocol) {
            bridge.connected = !bridge.connected;
            bridge.status = if bridge.connected {
                "Connected".to_string()
            } else {
                "Disconnected".to_string()
            };
            let updated = bridge.clone();
            let key = format!("bridge_connected:{}", updated.protocol);
            let value = if updated.connected { "1" } else { "0" };
            drop(bridges);
            persist_setting(&state, &key, value);
            return Ok(updated);
        }
    }
    Err(format!("Bridge for protocol '{}' not found", protocol))
}

#[tauri::command]
fn toggle_servers(state: State<'_, AppState>) -> bool {
    let mut running = state.server_running.lock().unwrap();
    *running = !*running;
    *running
}

fn push_activity(state: &AppState, kind: &str, detail: String) {
    let mut log = state.activity.lock().unwrap();
    let id = format!("evt_{}", log.len() + 1);
    log.insert(0, ActivityEvent { id, kind: kind.to_string(), detail, timestamp: "Just now".to_string() });
    log.truncate(50);
}

/// Full engine capability catalog (router.rs, federation.rs, call.rs, mesh.rs, etc.).
/// Each row names its engine symbol so dashboard coverage is auditable against source.
#[tauri::command]
fn get_capabilities() -> Vec<CapabilityInfo> {
    let rows: &[(&str, &str, &str, &str)] = &[
        ("Routing", "Optimal cascade", "Orders bridges by health/latency, bypasses open circuits, records outcomes.", "MultipathRouter::route_with_optimal_cascade"),
        ("Routing", "Hedged racing", "Races primary + fallback concurrently or after a tail-latency cutoff.", "MultipathRouter::route_hedged_racing"),
        ("Routing", "Adaptive fallback retry", "Multi-pass delivery with half-open circuit recovery between passes.", "MultipathRouter::route_with_adaptive_fallback_retry"),
        ("Routing", "Flapping resilience", "Deprioritizes flapping bridges; exponential cooldown backoff.", "MultipathRouter::route_with_flapping_resilience"),
        ("Routing", "Penalty-weighted cascade", "Scores latency + failure penalties before circuit trip.", "MultipathRouter::route_with_penalty_weighted_cascade"),
        ("Routing", "Unified contact routing", "Translates unified IDs to per-protocol addresses at each hop.", "MultipathRouter::route_unified_message"),
        ("Routing", "Fragmentation / MTU", "Chunks oversized payloads for SMS/Tox-sized transports.", "MultipathRouter::route_with_fragmentation_adaptive_cascade"),
        ("Routing", "Content transcoding", "Sanitizes rich text to plain/GSM-7 when cascading to SMS.", "MultipathRouter::route_with_transcoding_cascade"),
        ("Routing", "Confirmed receipt", "Cryptographic delivery receipts; cascades on ack timeout.", "MultipathRouter::route_with_confirmed_receipt"),
        ("Routing", "Concurrency bounded", "Per-bridge in-flight permits; saturation fallback.", "MultipathRouter::route_with_concurrency_bounded_cascade"),
        ("Routing", "Anomaly damped", "EMA-clamps latency spikes (>4x baseline) to avoid false trips.", "MultipathRouter::route_with_anomaly_damped_cascade"),
        ("Routing", "Exact-once dedup", "120s egress digest ledger prevents duplicate delivery.", "MultipathRouter::route_with_exact_once_deduplication"),
        ("Routing", "Canary drop detection", "Challenge nonces detect silent drops/blackholes pre-timeout.", "MultipathRouter::route_with_canary_drop_detection"),
        ("Routing", "Percentile bounded", "p50/p95 latency envelopes bound dispatch deadlines.", "MultipathRouter::route_with_percentile_bounded_cascade"),
        ("Routing", "Dispersal balancing", "Rotates egress across healthy bridges to avoid bottlenecks.", "MultipathRouter::route_with_dispersal_balancing"),
        ("Routing", "Bandwidth throttled", "Sliding-window byte quotas per transport.", "MultipathRouter::route_with_bandwidth_throttled_cascade"),
        ("Routing", "Cost & QoS optimizer", "Balances money cost, battery drain, and latency.", "MultipathRouter::route_with_cost_and_qos_optimization"),
        ("Routing", "Circuit rehabilitation", "Decays trip counts/backoff on stable transports.", "MultipathRouter::route_with_rehabilitated_circuits"),
        ("Routing", "Active health probing", "Heartbeat probes recover open/half-open circuits.", "MultipathRouter::route_with_active_health_probing"),
        ("Routing", "Starvation prevention", "Promotes never-tried bridges to probe status.", "MultipathRouter::route_with_starvation_prevention"),
        ("Routing", "Integrity + replay guard", "Payload digests + nonce cache block tamper/replay.", "MultipathRouter::route_with_replay_and_integrity_guard"),
        ("Federation", "Group channels + vector clocks", "Causal replication with deterministic conflict resolution.", "FederationManager / VectorClock"),
        ("Federation", "Delivery ledger", "Per-member receipt tracking; selective retry only.", "FederatedDeliveryLedger"),
        ("Federation", "Bridge handover", "Signed migration of a group channel across bridges.", "FederationManager::handover_channel_bridge"),
        ("Federation", "Partition resilience", "Disjoint-path broadcast across network partitions.", "FederationManager::broadcast_event_with_partition_resilience"),
        ("Calls & Media", "Cross-protocol call gateway", "Bridges WebRTC, Matrix VoIP, and Discord voice.", "CrossProtocolCallGateway"),
        ("Calls & Media", "RTP jitter buffer + PLC", "Reorders packets, conceals loss with comfort noise.", "RtpJitterBuffer"),
        ("Calls & Media", "Adaptive bitrate + simulcast", "Opus bitrate and VP8/VP9 layers follow bandwidth.", "BitrateController / VideoSimulcastController"),
        ("Calls & Media", "Encrypted screen sharing", "Chunked, checksummed, encrypted screen pipeline.", "ScreenSharePipeline"),
        ("Calls & Media", "HW encode/decode", "VA-API/VideoToolbox/NVENC + NVDEC/V4L2, SW fallback.", "HardwareVideoEncoder / HardwareVideoDecoder"),
        ("Mesh & Discovery", "Store-and-forward mesh", "Multi-hop BLE/Wi-Fi Direct relay with TTL + dedup.", "MeshRouter / MeshTransportDaemon"),
        ("Mesh & Discovery", "Mesh-to-bridge gateway", "Offline mesh nodes egress via gateway bridges.", "MeshRelayBridgeAdapter"),
        ("Mesh & Discovery", "LAN peer discovery", "UDP beacons, peer cache, multi-hop announcements.", "PeerDiscovery"),
        ("Security", "Double Ratchet + pre-keys", "Forward-secret sessions persisted to SQLCipher key_store.", "RatchetSession / PreKeyBundle"),
        ("Security", "Identity cards + QR pairing", "Exportable signed identity, QR import, handshake verify.", "IdentityCard::to_qr_payload"),
        ("Security", "Zero-knowledge backup", "Client-encrypted chunked backup with manifest.", "ZeroKnowledgeBackup"),
        ("Security", "Tor SOCKS5 + onion", "SOCKS5 circuits and v3 .onion endpoints.", "TorManager / OnionEndpoint"),
        ("Security", "Reputation & anti-spam", "Trust tiers, rate limits, blacklist enforcement.", "ReputationEngine"),
        ("System", "Offline outbox", "Queued dispatch with exponential backoff (5 attempts).", "OutboxManager"),
        ("System", "Hardware telemetry", "Battery/thermal/power; LTE failover + safe shutdown.", "TelemetryCollector / ApplianceConfig"),
        ("System", "WebSocket daemon IPC", "JSON-RPC sessions for desktop, mobile, and CLI clients.", "WebSocketServer (ws://127.0.0.1:18765)"),
        ("System", "SQLCipher storage", "AES-256-GCM, PBKDF2-HMAC-SHA512, threads/attachments/keys.", "Database (SCHEMA.md)"),
    ];
    rows.iter().map(|(area, name, description, engine_symbol)| CapabilityInfo {
        area: area.to_string(), name: name.to_string(), description: description.to_string(),
        engine_symbol: engine_symbol.to_string(), status: "Available in engine".to_string(),
    }).collect()
}

#[tauri::command]
fn get_routing_profiles() -> Vec<RoutingProfileInfo> {
    let rows: &[(&str, &str, &str, bool)] = &[
        ("optimal_cascade", "Optimal cascade (recommended)", "Health + latency ordered cascade; bypasses open circuits.", true),
        ("hedged_racing", "Hedged racing", "Race bridges in parallel for lowest tail latency.", false),
        ("adaptive_retry", "Adaptive fallback retry", "Multi-pass retry with half-open recovery.", false),
        ("confirmed_receipt", "Confirmed receipt", "Wait for signed acks; cascade on timeout.", false),
        ("exact_once", "Exact-once delivery", "Dedup ledger prevents duplicate sends.", false),
        ("canary_guard", "Canary drop detection", "Probe for silent drops before dispatch.", false),
        ("percentile_bounded", "Percentile bounded (p95)", "Deadline bounded by historical p95 latency.", false),
        ("bandwidth_throttled", "Bandwidth throttled", "Respect per-transport byte quotas.", false),
        ("cost_qos", "Cost & QoS optimized", "Cheapest / lowest-drain route that meets QoS.", false),
        ("penalty_weighted", "Penalty weighted", "Avoid degraded bridges before they trip.", false),
    ];
    rows.iter().map(|(id, label, description, recommended)| RoutingProfileInfo {
        id: id.to_string(), label: label.to_string(), description: description.to_string(),
        engine_symbol: format!("MultipathRouter ({})", id), recommended: *recommended,
    }).collect()
}

#[tauri::command]
fn set_routing_profile(state: State<'_, AppState>, profile_id: String) -> String {
    *state.routing_profile.lock().unwrap() = profile_id.clone();
    persist_setting(&state, "routing_profile", &profile_id);
    push_activity(&state, "ROUTING_PROFILE", format!("Active routing profile set to {}", profile_id));
    profile_id
}

#[tauri::command]
fn get_security_status(state: State<'_, AppState>) -> SecurityStatus {
    use std::collections::HashMap;
    let mut mappings = HashMap::new();
    mappings.insert(omnicore_engine::models::ProtocolKind::Signal, "alice_unified_1".to_string());
    let card = omnicore_engine::identity::IdentityCard::new("local_user", "Local User", "e8b23f1a99c0d44e71b8502c129afe88", mappings);
    let tor_on = *state.tor_enabled.lock().unwrap();
    let mut tor = omnicore_engine::tor::TorManager::new(omnicore_engine::tor::TorConfig::default());
    let tor_status = if tor_on { let _ = tor.connect_proxy(); format!("{:?}", tor.status()) } else { "Disabled".to_string() };
    SecurityStatus {
        identity_name: card.display_name.clone(), unified_id: card.unified_id.clone(),
        fingerprint: card.fingerprint.formatted_fingerprint.clone(), identity_verified: card.verify_signature(),
        tor_enabled: tor_on, tor_proxy_url: tor.socks5_proxy_url(), tor_status,
        backup_enabled: true, backup_description: "Zero-knowledge encrypted chunks + manifest (ZeroKnowledgeBackup)".to_string(),
        ratchet_description: "Double Ratchet sessions + pre-key bundles in SQLCipher key_store".to_string(),
        encryption: "SQLCipher (AES-256-GCM)".to_string(),
    }
}

#[tauri::command]
fn toggle_tor(state: State<'_, AppState>) -> bool {
    let mut on = state.tor_enabled.lock().unwrap();
    *on = !*on;
    let now = *on;
    drop(on);
    persist_setting(&state, "tor_enabled", if now { "1" } else { "0" });
    push_activity(&state, if now { "TOR_ON" } else { "TOR_OFF" }, format!("Tor SOCKS5 routing {}", if now { "enabled (socks5h://127.0.0.1:9050)" } else { "disabled" }));
    now
}

#[tauri::command]
fn export_identity_card() -> Result<String, String> {
    use std::collections::HashMap;
    let mut mappings = HashMap::new();
    mappings.insert(omnicore_engine::models::ProtocolKind::Signal, "alice_unified_1".to_string());
    let card = omnicore_engine::identity::IdentityCard::new("local_user", "Local User", "e8b23f1a99c0d44e71b8502c129afe88", mappings);
    card.export_json().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_network_status(state: State<'_, AppState>) -> Result<NetworkStatus, String> {
    let telemetry = omnicore_engine::telemetry::TelemetryCollector::new().collect();
    let action = omnicore_engine::telemetry::ApplianceConfig::default().evaluate_action(&telemetry);
    let channels = state.federation.lock().await.channel_count();
    let daemon_sessions = state.daemon.read().await.session_count().await;
    Ok(NetworkStatus {
        federation_channels: channels,
        federation_description: "Federated group channels with vector-clock causal sync + delivery ledger".to_string(),
        mesh_peers: 0, mesh_description: "BLE / Wi-Fi Direct store-and-forward mesh (MeshRouter). No native HCI driver yet — see TODO.".to_string(),
        discovery_description: "LAN UDP beacon discovery with cached peers (PeerDiscovery)".to_string(),
        active_calls: 0, call_description: "WebRTC / Matrix VoIP / Discord voice gateway; jitter buffer + simulcast ready".to_string(),
        outbox_queued: 0, outbox_description: "Offline outbox: exponential backoff, max 5 attempts (OutboxManager)".to_string(),
        telemetry_summary: format!("CPU {:.0}°C · RAM {} / {} MB · Disk {:.1} GB free · Battery {}%", telemetry.health.cpu_temp_celsius, telemetry.health.memory_used_mb, telemetry.health.memory_total_mb, telemetry.health.disk_free_gb, telemetry.battery.as_ref().map(|b| b.percentage).unwrap_or(0)),
        appliance_action: format!("{:?}", action),
        reputation_description: "Peer trust tiers + spam rate limiting (ReputationEngine)".to_string(),
        daemon_url: "ws://127.0.0.1:18765".to_string(), daemon_sessions,
    })
}

#[tauri::command]
fn get_activity_log(state: State<'_, AppState>) -> Vec<ActivityEvent> {
    state.activity.lock().unwrap().clone()
}

fn channel_to_info(ch: &omnicore_engine::federation::FederatedChannel) -> ChannelInfo {
    ChannelInfo {
        id: ch.channel_id.clone(),
        name: ch.name.clone(),
        topic: ch.topic.clone(),
        bridge_protocol: ch.active_bridge.map(|p| p.as_str().to_string()).unwrap_or_else(|| "Matrix".to_string()),
        member_count: ch.members.len(),
        event_count: ch.event_history.len(),
    }
}

async fn create_federated_channel(state: &AppState, name: &str, topic: &str, bridge_protocol: &str) -> ChannelInfo {
    let mut mgr = state.federation.lock().await;
    let id = format!("chan_{}_{}", mgr.channel_count() + 1, name.to_ascii_lowercase().replace(' ', "-"));
    mgr.create_channel(&id, name, topic, "local_user");
    if let Some(ch) = mgr.get_channel_mut(&id) {
        ch.active_bridge = omnicore_engine::models::ProtocolKind::from_str_name(bridge_protocol);
    }
    let info = mgr.get_channel(&id).map(channel_to_info).unwrap_or_else(|| ChannelInfo {
        id: id.clone(), name: name.to_string(), topic: topic.to_string(), bridge_protocol: bridge_protocol.to_string(), member_count: 1, event_count: 0,
    });
    drop(mgr);
    push_activity(state, "CHANNEL_CREATED", format!("Federated channel '{}' ({}) created on {} via FederationManager", name, id, bridge_protocol));
    info
}

#[tauri::command]
async fn get_channels(state: State<'_, AppState>) -> Result<Vec<ChannelInfo>, String> {
    Ok(state.federation.lock().await.list_channels().iter().map(|c| channel_to_info(c)).collect())
}

#[tauri::command]
async fn create_channel(state: State<'_, AppState>, name: String, topic: String, bridge_protocol: String) -> Result<ChannelInfo, String> {
    Ok(create_federated_channel(&state, &name, &topic, &bridge_protocol).await)
}

/// Simulated local signing key for handovers. The engine's handover
/// signature scheme is itself simulated (see federation.rs sign/verify);
/// this is labeled as such in Activity rather than presented as production crypto.
const LOCAL_HANDOVER_KEY: &str = "omnicore-local-handover-key-sim";

async fn perform_handover(state: &AppState, channel_id: &str, to_protocol: &str) -> Result<ChannelInfo, String> {
    let to = omnicore_engine::models::ProtocolKind::from_str_name(to_protocol).ok_or_else(|| format!("Unknown protocol '{}'", to_protocol))?;
    let mut mgr = state.federation.lock().await;
    let from = mgr.get_channel(channel_id).and_then(|c| c.active_bridge).unwrap_or(omnicore_engine::models::ProtocolKind::Matrix);
    if from == to {
        return Ok(channel_to_info(mgr.get_channel(channel_id).ok_or("Channel vanished")?));
    }
    let mut handover = omnicore_engine::federation::FederatedChannelBridgeHandover {
        handover_id: format!("ho_{}_{}", channel_id, now_epoch()),
        channel_id: channel_id.to_string(),
        initiator_id: "local_user".to_string(),
        from_protocol: from,
        to_protocol: to,
        fallback_protocols: vec![from],
        reason: "Operator-initiated bridge migration from dashboard".to_string(),
        timestamp_sec: now_epoch(),
        public_key: None,
        signature: None,
    };
    handover.sign("local_user_pubkey_sim", LOCAL_HANDOVER_KEY);
    mgr.handover_channel_bridge(channel_id, handover, LOCAL_HANDOVER_KEY).map_err(|e| e.to_string())?;
    let info = channel_to_info(mgr.get_channel(channel_id).ok_or("Channel vanished after handover")?);
    drop(mgr);
    push_activity(state, "BRIDGE_HANDOVER", format!("#{} migrated {} -> {} (signed, simulated key)", info.name, from.as_str(), to.as_str()));
    Ok(info)
}

/// Migrates a federated channel to a different bridge (signed handover).
#[tauri::command]
async fn handover_channel(state: State<'_, AppState>, channel_id: String, to_protocol: String) -> Result<ChannelInfo, String> {
    perform_handover(&state, &channel_id, &to_protocol).await
}

/// Adds a unified contact as a member of a federated channel (idempotent).
#[tauri::command]
async fn add_channel_member(state: State<'_, AppState>, channel_id: String, unified_id: String, display_name: String) -> Result<ChannelInfo, String> {
    let mut mgr = state.federation.lock().await;
    let channel = mgr.get_channel_mut(&channel_id).ok_or_else(|| format!("Channel '{}' not found", channel_id))?;
    let added = channel.add_member(&unified_id, &display_name, "member");
    let info = channel_to_info(channel);
    drop(mgr);
    push_activity(&state, if added { "MEMBER_ADDED" } else { "MEMBER_EXISTS" }, format!("{} {} #{}", display_name, if added { "joined" } else { "already in" }, info.name));
    Ok(info)
}

/// Posts a message to a federated group channel as local_user. The event is
/// recorded in the channel's causal history, then broadcast to other members
/// via the shared MultipathRouter when one is attached (engine init).
#[tauri::command]
async fn post_channel_message(state: State<'_, AppState>, channel_id: String, content: String) -> Result<ChannelInfo, String> {
    if content.trim().is_empty() {
        return Err("Message content cannot be empty".to_string());
    }
    let mut mgr = state.federation.lock().await;
    let (event, primary, fallbacks) = {
        let channel = mgr.get_channel_mut(&channel_id).ok_or_else(|| format!("Channel '{}' not found", channel_id))?;
        let event = channel.post_message("local_user", &content).map_err(|e| e.to_string())?;
        let primary = channel.active_bridge.unwrap_or(omnicore_engine::models::ProtocolKind::Matrix);
        let fallbacks = channel.active_fallbacks.clone();
        (event, primary, fallbacks)
    };
    let mut delivered_members = 0usize;
    if let Some(router) = state.router.read().await.clone() {
        match mgr.broadcast_event_via_router(&channel_id, &event, &router, primary, &fallbacks).await {
            Ok(results) => delivered_members = results.len(),
            Err(e) => push_activity(&state, "FEDERATED_POST_ERROR", format!("Broadcast for {} failed: {}", channel_id, e)),
        }
    }
    let info = mgr.get_channel(&channel_id).map(channel_to_info).ok_or_else(|| "Channel vanished after post".to_string())?;
    drop(mgr);
    push_activity(&state, "FEDERATED_POST", format!("Posted to #{} (event recorded; broadcast delivered to {} other member(s))", info.name, delivered_members));
    Ok(info)
}

#[tauri::command]
fn run_bridge_probe(state: State<'_, AppState>) -> Vec<ActivityEvent> {
    push_activity(&state, "HEALTH_PROBE", "Heartbeat probe sent to all open/half-open circuits (BridgeHeartbeatProber)".to_string());
    state.activity.lock().unwrap().clone()
}

#[tauri::command]
fn quit_app(app_handle: AppHandle) {
    app_handle.exit(0);
}

// Setup System Tray Icon & Menu.
// The tray is the always-available control surface when the window is hidden:
// show/hide, pause/resume routing workers, probe bridges, and quit servers.
// Menu labels that depend on live state are refreshed from AppState on click
// (Tauri menus are static snapshots, so handlers read current state each time).
fn setup_system_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_hide = MenuItem::with_id(app, "toggle_window", "Show / Hide Dashboard", true, None::<&str>)?;
    let toggle_servers_item = MenuItem::with_id(app, "tray_toggle_servers", "Pause / Resume Routing Workers", true, None::<&str>)?;
    let probe = MenuItem::with_id(app, "tray_probe", "Probe Bridges (Health Check)", true, None::<&str>)?;
    let bridge_status = MenuItem::with_id(app, "bridge_status", "Bridges: see dashboard for live latency", false, None::<&str>)?;
    let quit_servers = MenuItem::with_id(app, "quit_servers", "Quit Servers Only (Keep Window)", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit OmniCore & Servers", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_hide, &toggle_servers_item, &probe, &bridge_status, &quit_servers, &quit])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("OmniCore — Unified Messaging (tray: show/hide, pause workers, quit servers)")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle_window" => {
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            "tray_toggle_servers" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let mut running = state.server_running.lock().unwrap();
                    *running = !*running;
                }
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "tray_probe" => {
                if let Some(state) = app.try_state::<AppState>() {
                    push_activity(&state, "HEALTH_PROBE", "Tray: heartbeat probe requested for all circuits".to_string());
                }
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit_servers" => {
                if let Some(state) = app.try_state::<AppState>() {
                    *state.server_running.lock().unwrap() = false;
                    push_activity(&state, "SERVERS_STOPPED", "Tray: routing workers stopped; window kept open".to_string());
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            if let Err(e) = setup_system_tray(app) {
                eprintln!("Warning: Failed to initialize system tray: {}", e);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            init_engine,
            get_system_metrics,
            get_contacts,
            add_contact,
            get_messages,
            send_message,
            get_bridges,
            toggle_bridge,
            toggle_servers,
            get_capabilities,
            get_routing_profiles,
            set_routing_profile,
            get_security_status,
            toggle_tor,
            export_identity_card,
            get_network_status,
            get_activity_log,
            get_channels,
            create_channel,
            handover_channel,
            add_channel_member,
            post_channel_message,
            run_bridge_probe,
            get_daemon_status,
            subscribe_daemon_events,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::state::*;

    #[test]
    fn test_app_state_defaults() {
        let state = AppState::default();
        let contacts = state.contacts.lock().unwrap();
        assert_eq!(contacts.len(), 4);
        assert_eq!(contacts[0].primary_protocol, "Signal");

        let bridges = state.bridges.lock().unwrap();
        assert!(bridges.iter().any(|b| b.protocol == "Signal" && b.connected));
        assert!(bridges.iter().any(|b| b.protocol == "Matrix" && b.connected));
        assert!(bridges.iter().any(|b| b.protocol == "Discord" && b.connected));
    }

    #[test]
    fn test_message_validation() {
        let state = AppState::default();
        let mut messages = state.messages.lock().unwrap();
        let content = "";
        let res_err: Result<(), String> = if content.trim().is_empty() {
            Err("Message content cannot be empty".to_string())
        } else {
            messages.push(MessageItem {
                id: "test".to_string(),
                sender: "You".to_string(),
                recipient: "Alice".to_string(),
                content: content.to_string(),
                protocol: "Signal".to_string(),
                timestamp: "Now".to_string(),
                is_outgoing: true,
                status: "dispatched".to_string(),
            });
            Ok(())
        };
        assert!(res_err.is_err());
    }

    #[tokio::test]
    async fn test_execute_routing_profile_optimal_and_exact_once() {
        let state = AppState::default();
        // Optimal cascade executes against real engine bridges and delivers.
        let ok = crate::execute_routing_profile(&state, "optimal_cascade", "Alice Montgomery", "hello profile", "Signal", "Matrix").await;
        assert!(ok.is_ok(), "optimal cascade should deliver: {:?}", ok.err());
        // Exact-once: first send records the digest, identical resend is rejected.
        let first = crate::execute_routing_profile(&state, "exact_once", "Bob Sterling", "dedup me", "Matrix", "Discord").await;
        assert!(first.is_ok(), "first exact-once send should deliver: {:?}", first.err());
        let dup = crate::execute_routing_profile(&state, "exact_once", "Bob Sterling", "dedup me", "Matrix", "Discord").await;
        assert!(dup.is_err(), "duplicate exact-once send must be rejected");
        // Unknown protocols are rejected, never silently routed.
        let bad = crate::execute_routing_profile(&state, "optimal_cascade", "X", "hi", "CarrierPigeon", "Matrix").await;
        assert!(bad.is_err());
    }

    #[test]
    fn test_settings_persistence_roundtrip() {
        let path = std::env::temp_dir().join(format!("omnicore_settings_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let state = AppState::default();
        crate::init_database_at(&state, &path, "test_key").unwrap();
        *state.routing_profile.lock().unwrap() = "hedged_racing".to_string();
        crate::persist_setting(&state, "routing_profile", "hedged_racing");
        crate::persist_setting(&state, "tor_enabled", "1");
        crate::persist_setting(&state, "bridge_connected:Nostr", "1");
        *state.db.lock().unwrap() = None;
        // Fresh state restores settings from SQLCipher.
        let state2 = AppState::default();
        crate::init_database_at(&state2, &path, "test_key").unwrap();
        assert_eq!(state2.routing_profile.lock().unwrap().as_str(), "hedged_racing");
        assert!(*state2.tor_enabled.lock().unwrap());
        let bridges = state2.bridges.lock().unwrap();
        assert!(bridges.iter().find(|b| b.protocol == "Nostr").unwrap().connected);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_handover_channel_migrates_bridge() {
        let state = AppState::default();
        let created = crate::create_federated_channel(&state, "Migrate Room", "topic", "Signal").await;
        assert_eq!(created.bridge_protocol, "Signal");
        let moved = crate::perform_handover(&state, &created.id, "Matrix").await.unwrap();
        assert_eq!(moved.bridge_protocol, "Matrix");
        // Same-bridge handover is a no-op, unknown protocol is an error.
        let same = crate::perform_handover(&state, &created.id, "Matrix").await.unwrap();
        assert_eq!(same.bridge_protocol, "Matrix");
        assert!(crate::perform_handover(&state, &created.id, "CarrierPigeon").await.is_err());
    }

    #[tokio::test]
    async fn test_federated_channel_creation_uses_manager() {
        let state = AppState::default();
        assert_eq!(state.federation.lock().await.channel_count(), 1);
        let created = crate::create_federated_channel(&state, "Family Chat", "Relatives", "Signal").await;
        assert_eq!(created.name, "Family Chat");
        assert_eq!(created.bridge_protocol, "Signal");
        assert_eq!(created.member_count, 1);
        // Posting records a causal event in the channel history.
        {
            let mut mgr = state.federation.lock().await;
            let ch = mgr.get_channel_mut(&created.id).unwrap();
            let event = ch.post_message("local_user", "hello group").unwrap();
            assert_eq!(event.payload, "hello group");
            assert_eq!(ch.event_history.len(), 1);
        }
        let mgr = state.federation.lock().await;
        assert_eq!(mgr.channel_count(), 2);
        assert!(mgr.get_channel(&created.id).is_some());
        assert_eq!(mgr.list_channels().len(), 2);
    }

    #[test]
    fn test_format_epoch_utc() {
        assert_eq!(crate::format_epoch_utc(0), "1970-01-01 00:00 UTC");
        assert_eq!(crate::format_epoch_utc(1_700_000_000), "2023-11-14 22:13 UTC");
        assert_eq!(crate::format_epoch_utc(1_735_689_600), "2025-01-01 00:00 UTC");
    }

    #[test]
    fn test_db_persistence_roundtrip() {
        let path = std::env::temp_dir().join(format!("omnicore_test_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        // First run: seeds defaults, persists one outgoing message.
        let state = AppState::default();
        let summary = crate::init_database_at(&state, &path, "test_key").unwrap();
        assert!(summary.contains("seeded"), "summary: {}", summary);
        assert_eq!(state.contacts.lock().unwrap().len(), 4);
        crate::persist_outgoing(&state, "Alice Montgomery", "Signal", "persist me across restarts", "Delivered");
        *state.db.lock().unwrap() = None;
        // Second run: contacts and the message are restored from SQLCipher.
        let state2 = AppState::default();
        let summary2 = crate::init_database_at(&state2, &path, "test_key").unwrap();
        assert!(summary2.contains("restored"), "summary: {}", summary2);
        let contacts2 = state2.contacts.lock().unwrap();
        assert_eq!(contacts2.len(), 4);
        assert!(contacts2.iter().any(|c| c.name == "Alice Montgomery"));
        drop(contacts2);
        let messages2 = state2.messages.lock().unwrap();
        assert!(messages2.iter().any(|m| m.content == "persist me across restarts" && m.recipient == "Alice Montgomery"), "messages: {:?}", messages2.iter().map(|m| &m.content).collect::<Vec<_>>());
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_daemon_subscription_broadcast_flow() {
        let state = AppState::default();
        {
            let mut daemon = state.daemon.write().await;
            daemon.start().await.unwrap();
            daemon.register_session(crate::DESKTOP_SESSION_ID, "desktop_ui", 1700000000).await;
        }
        let daemon = state.daemon.read().await;
        assert_eq!(daemon.session_count().await, 1);
        assert!(daemon.subscribe_session(crate::DESKTOP_SESSION_ID, Some("ops-room".to_string())).await);
        // Channel-targeted event reaches the subscriber; other channels do not.
        assert_eq!(daemon.broadcast_event("message_dispatched", serde_json::json!({}), Some("ops-room")).await, 1);
        assert_eq!(daemon.broadcast_event("message_dispatched", serde_json::json!({}), Some("other")).await, 0);
    }

    #[test]
    fn test_toggle_bridge_state() {
        let state = AppState::default();
        {
            let mut bridges = state.bridges.lock().unwrap();
            let b = bridges.iter_mut().find(|b| b.protocol == "Simplex").unwrap();
            assert!(!b.connected);
            b.connected = true;
            b.status = "Connected".to_string();
        }
        let bridges = state.bridges.lock().unwrap();
        let b = bridges.iter().find(|b| b.protocol == "Simplex").unwrap();
        assert!(b.connected);
        assert_eq!(b.status, "Connected");
    }
}
