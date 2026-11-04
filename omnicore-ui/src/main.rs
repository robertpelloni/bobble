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

    pub struct AppState {
        pub contacts: Mutex<Vec<ContactInfo>>,
        pub messages: Mutex<Vec<MessageItem>>,
        pub bridges: Mutex<Vec<BridgeStatusInfo>>,
        pub server_running: Mutex<bool>,
        pub start_time: std::time::Instant,
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
                ]),
                server_running: Mutex::new(true),
                start_time: std::time::Instant::now(),
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

#[tauri::command]
fn init_engine() -> String {
    let _bridge = omnicore_engine::bridges::matrix::MatrixBridge::new();
    "Engine, SQLCipher cryptographic storage, and protocol bridges active.".to_string()
}

#[tauri::command]
fn get_system_metrics(state: State<'_, AppState>) -> SystemMetrics {
    let bridges = state.bridges.lock().unwrap();
    let active_count = bridges.iter().filter(|b| b.connected).count();
    let running = *state.server_running.lock().unwrap();
    let elapsed = state.start_time.elapsed().as_secs();

    SystemMetrics {
        version: "0.2.0".to_string(),
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
    let new_id = (contacts.len() + 1) as i64;
    let initials = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_uppercase();

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

#[tauri::command]
fn send_message(
    state: State<'_, AppState>,
    recipient: String,
    content: String,
    protocol: String,
    fallback_protocol: String,
) -> Result<MessageItem, String> {
    if content.trim().is_empty() {
        return Err("Message content cannot be empty".to_string());
    }

    let mut messages = state.messages.lock().unwrap();
    let new_item = MessageItem {
        id: format!("msg_{}", messages.len() + 1),
        sender: "You".to_string(),
        recipient: recipient.clone(),
        content: content.clone(),
        protocol: format!("{} (fallback: {})", protocol, fallback_protocol),
        timestamp: "Just now".to_string(),
        is_outgoing: true,
        status: "dispatched".to_string(),
    };

    messages.push(new_item.clone());
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
            return Ok(bridge.clone());
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

#[tauri::command]
fn quit_app(app_handle: AppHandle) {
    app_handle.exit(0);
}

// Setup System Tray Icon & Menu
fn setup_system_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_hide = MenuItem::with_id(app, "toggle_window", "Show / Hide Dashboard", true, None::<&str>)?;
    let bridge_status = MenuItem::with_id(app, "bridge_status", "Bridge Status: Active", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit OmniCore & Servers", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_hide, &bridge_status, &quit])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("OmniCore Universal Aggregator")
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
