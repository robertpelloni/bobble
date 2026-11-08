use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshTransportType {
    UdpBroadcast,
    BleMesh,
    WifiDirect,
}

impl Default for MeshTransportType {
    fn default() -> Self {
        Self::UdpBroadcast
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerAnnouncement {
    pub client_id: String,
    pub unified_id: String,
    pub port: u16,
    pub supported_protocols: Vec<String>,
    pub timestamp: u64,
    pub transport: MeshTransportType,
    pub hop_limit: u8,
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
            transport: MeshTransportType::UdpBroadcast,
            hop_limit: 3,
        }
    }

    pub fn with_transport_and_hop(
        client_id: &str,
        unified_id: &str,
        port: u16,
        supported_protocols: Vec<String>,
        transport: MeshTransportType,
        hop_limit: u8,
    ) -> Self {
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
            transport,
            hop_limit,
        }
    }

    /// Decrements the hop limit for multi-hop mesh propagation. Returns None if hop limit exhausted.
    pub fn decrement_hop(&self) -> Option<Self> {
        if self.hop_limit <= 1 {
            None
        } else {
            let mut clone = self.clone();
            clone.hop_limit -= 1;
            Some(clone)
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
    peer_cache: HashMap<String, DiscoveredPeer>,
}

impl PeerDiscovery {
    pub fn new(broadcast_port: u16, client_id: &str, unified_id: &str) -> Self {
        Self {
            broadcast_port,
            client_id: client_id.to_string(),
            unified_id: unified_id.to_string(),
            peer_cache: HashMap::new(),
        }
    }

    pub fn cache_peer(&mut self, peer: DiscoveredPeer) {
        self.peer_cache.insert(peer.unified_id.clone(), peer);
    }

    pub fn get_cached_peers(&self) -> Vec<DiscoveredPeer> {
        self.peer_cache.values().cloned().collect()
    }

    pub fn cached_peer_count(&self) -> usize {
        self.peer_cache.len()
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

    #[test]
    fn test_mesh_announcement_hop_decrement() {
        let announcement = PeerAnnouncement::with_transport_and_hop(
            "client_hop",
            "hop_unified",
            9000,
            vec!["Tox".to_string()],
            MeshTransportType::BleMesh,
            3,
        );

        let hop2 = announcement.decrement_hop().expect("Should decrement to 2");
        assert_eq!(hop2.hop_limit, 2);
        assert_eq!(hop2.transport, MeshTransportType::BleMesh);

        let hop1 = hop2.decrement_hop().expect("Should decrement to 1");
        assert_eq!(hop1.hop_limit, 1);

        assert!(hop1.decrement_hop().is_none());
    }

    #[test]
    fn test_peer_discovery_cache_operations() {
        let mut discovery = PeerDiscovery::new(51234, "my_device", "me_unified");
        assert_eq!(discovery.cached_peer_count(), 0);

        let peer1 = DiscoveredPeer {
            client_id: "dev_1".to_string(),
            unified_id: "alice_contact".to_string(),
            socket_addr: "192.168.1.10:8000".parse().unwrap(),
            supported_protocols: vec!["Signal".to_string()],
            last_seen: 1700000000,
        };

        discovery.cache_peer(peer1);
        assert_eq!(discovery.cached_peer_count(), 1);

        let cached = discovery.get_cached_peers();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].unified_id, "alice_contact");
    }
}
