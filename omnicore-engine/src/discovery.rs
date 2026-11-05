use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerAnnouncement {
    pub client_id: String,
    pub unified_id: String,
    pub port: u16,
    pub supported_protocols: Vec<String>,
    pub timestamp: u64,
}

impl PeerAnnouncement {
    pub fn new(client_id: &str, unified_id: &str, port: u16, supported_protocols: Vec<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            client_id: client_id.to_string(),
            unified_id: unified_id.to_string(),
            port,
            supported_protocols,
            timestamp,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| anyhow!("Failed to serialize peer announcement: {}", e))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| anyhow!("Failed to deserialize peer announcement: {}", e))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPeer {
    pub client_id: String,
    pub unified_id: String,
    pub socket_addr: SocketAddr,
    pub supported_protocols: Vec<String>,
    pub last_seen: u64,
}

pub struct PeerDiscovery {
    pub broadcast_port: u16,
    pub client_id: String,
    pub unified_id: String,
}

impl PeerDiscovery {
    pub fn new(broadcast_port: u16, client_id: &str, unified_id: &str) -> Self {
        Self {
            broadcast_port,
            client_id: client_id.to_string(),
            unified_id: unified_id.to_string(),
        }
    }

    pub fn create_beacon(&self, service_port: u16, protocols: Vec<String>) -> PeerAnnouncement {
        PeerAnnouncement::new(&self.client_id, &self.unified_id, service_port, protocols)
    }

    pub fn parse_peer(&self, bytes: &[u8], remote_addr: SocketAddr) -> Result<DiscoveredPeer> {
        let announcement = PeerAnnouncement::from_bytes(bytes)?;
        let mut target_addr = remote_addr;
        target_addr.set_port(announcement.port);

        Ok(DiscoveredPeer {
            client_id: announcement.client_id,
            unified_id: announcement.unified_id,
            socket_addr: target_addr,
            supported_protocols: announcement.supported_protocols,
            last_seen: announcement.timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_announcement_serialization() {
        let announcement = PeerAnnouncement::new(
            "client_abc",
            "alice_unified_1",
            4242,
            vec!["Signal".to_string(), "Matrix".to_string()],
        );

        let bytes = announcement.to_bytes().expect("Serialization should succeed");
        let decoded = PeerAnnouncement::from_bytes(&bytes).expect("Deserialization should succeed");

        assert_eq!(announcement.client_id, decoded.client_id);
        assert_eq!(announcement.unified_id, decoded.unified_id);
        assert_eq!(announcement.port, decoded.port);
        assert_eq!(announcement.supported_protocols, decoded.supported_protocols);
    }

    #[test]
    fn test_peer_discovery_parsing() {
        let discovery = PeerDiscovery::new(51234, "my_device", "me_unified");
        let beacon = discovery.create_beacon(8080, vec!["Simplex".to_string()]);
        let bytes = beacon.to_bytes().unwrap();

        let remote_addr: SocketAddr = "192.168.1.150:49152".parse().unwrap();
        let peer = discovery.parse_peer(&bytes, remote_addr).unwrap();

        assert_eq!(peer.client_id, "my_device");
        assert_eq!(peer.unified_id, "me_unified");
        assert_eq!(peer.socket_addr, "192.168.1.150:8080".parse().unwrap());
        assert_eq!(peer.supported_protocols, vec!["Simplex"]);
    }
}
