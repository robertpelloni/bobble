use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use crate::discovery::MeshTransportType;

/// Outcome of processing an incoming mesh packet at this node.
#[derive(Debug, PartialEq, Eq)]
pub enum PacketProcessingResult {
    /// The packet is addressed directly to this node and was consumed locally.
    DeliveredLocally(MeshPacket),
    /// The packet is addressed to another node and was queued for forwarding.
    ForwardQueued(MeshPacket),
    /// The packet was dropped (e.g. hop limit exhausted, loop detected, or duplicate).
    Dropped(DropReason),
}

/// Reasons why a mesh packet may be discarded.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DropReason {
    HopLimitExhausted,
    RoutingLoopDetected,
    DuplicatePacket,
    MalformedChecksum,
}

/// Envelope header containing routing metadata for store-and-forward mesh propagation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeshHeader {
    pub packet_id: String,
    pub source_unified_id: String,
    pub destination_unified_id: String,
    pub hop_limit: u8,
    pub transport: MeshTransportType,
    pub visited_nodes: Vec<String>,
}

/// The complete mesh packet containing routing header and payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeshPacket {
    pub header: MeshHeader,
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

impl MeshPacket {
    /// Creates a new mesh packet addressed to a destination.
    pub fn new(
        packet_id: impl Into<String>,
        source_id: impl Into<String>,
        dest_id: impl Into<String>,
        hop_limit: u8,
        transport: MeshTransportType,
        payload: Vec<u8>,
        timestamp: u64,
    ) -> Self {
        let src = source_id.into();
        Self {
            header: MeshHeader {
                packet_id: packet_id.into(),
                source_unified_id: src.clone(),
                destination_unified_id: dest_id.into(),
                hop_limit,
                transport,
                visited_nodes: vec![src],
            },
            payload,
            timestamp,
        }
    }

    /// Prepares the packet for forwarding by decrementing the hop limit
    /// and recording the current node in the visited list.
    pub fn prepare_forward(&self, current_node_id: &str) -> Option<Self> {
        if self.header.hop_limit <= 1 {
            return None;
        }

        let mut forwarded = self.clone();
        forwarded.header.hop_limit -= 1;
        forwarded.header.visited_nodes.push(current_node_id.to_string());
        Some(forwarded)
    }
}

/// Router managing store-and-forward mesh packet queueing, loop detection, and deduplication with TTL cache pruning.
pub struct MeshRouter {
    pub local_node_id: String,
    seen_packets: HashMap<String, u64>, // packet_id -> expiration timestamp
    seen_ttl_seconds: u64,
    forward_queue: VecDeque<MeshPacket>,
    delivered_inbox: Vec<MeshPacket>,
}

impl MeshRouter {
    pub fn new(local_node_id: impl Into<String>) -> Self {
        Self {
            local_node_id: local_node_id.into(),
            seen_packets: HashMap::new(),
            seen_ttl_seconds: 3600, // 1 hour default TTL
            forward_queue: VecDeque::new(),
            delivered_inbox: Vec::new(),
        }
    }

    pub fn with_ttl(local_node_id: impl Into<String>, ttl_seconds: u64) -> Self {
        Self {
            local_node_id: local_node_id.into(),
            seen_packets: HashMap::new(),
            seen_ttl_seconds: ttl_seconds,
            forward_queue: VecDeque::new(),
            delivered_inbox: Vec::new(),
        }
    }

    /// Prunes expired packet records to prevent unbounded memory growth over extended uptimes.
    pub fn prune_expired_seen(&mut self, current_time: u64) -> usize {
        let initial_len = self.seen_packets.len();
        self.seen_packets.retain(|_, &mut exp| exp > current_time);
        initial_len - self.seen_packets.len()
    }

    /// Ingests an incoming mesh packet from a local transport.
    pub fn process_incoming_packet(&mut self, packet: MeshPacket) -> PacketProcessingResult {
        let now = packet.timestamp;
        // 1. Check for duplicates in seen cache
        if let Some(&expires_at) = self.seen_packets.get(&packet.header.packet_id) {
            if expires_at > now {
                return PacketProcessingResult::Dropped(DropReason::DuplicatePacket);
            }
        }
        self.seen_packets.insert(packet.header.packet_id.clone(), now + self.seen_ttl_seconds);

        // 2. Check if addressed to us
        if packet.header.destination_unified_id == self.local_node_id {
            self.delivered_inbox.push(packet.clone());
            return PacketProcessingResult::DeliveredLocally(packet);
        }

        // 3. Loop detection: if our node is already in visited list, drop to prevent cycles
        if packet.header.visited_nodes.contains(&self.local_node_id) {
            return PacketProcessingResult::Dropped(DropReason::RoutingLoopDetected);
        }

        // 4. Check hop limit
        if packet.header.hop_limit <= 1 {
            return PacketProcessingResult::Dropped(DropReason::HopLimitExhausted);
        }

        // 5. Decrement hop and queue for store-and-forward relay
        if let Some(forward_pkt) = packet.prepare_forward(&self.local_node_id) {
            self.forward_queue.push_back(forward_pkt.clone());
            PacketProcessingResult::ForwardQueued(forward_pkt)
        } else {
            PacketProcessingResult::Dropped(DropReason::HopLimitExhausted)
        }
    }

    /// Returns the number of packets currently queued for relay forwarding.
    pub fn queued_forward_count(&self) -> usize {
        self.forward_queue.len()
    }

    /// Pops the next packet queued for forwarding across local mesh transports.
    pub fn next_forward_packet(&mut self) -> Option<MeshPacket> {
        self.forward_queue.pop_front()
    }

    /// Returns references to all packets successfully delivered to the local node.
    pub fn delivered_packets(&self) -> &[MeshPacket] {
        &self.delivered_inbox
    }

    pub fn seen_packet_count(&self) -> usize {
        self.seen_packets.len()
    }
}

/// Represents an attached mesh transport peer channel (e.g. BLE GATT characteristic or Wi-Fi Direct socket).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPeerChannel {
    pub peer_id: String,
    pub transport: MeshTransportType,
    pub is_connected: bool,
}

/// Daemon coordinating mesh transports and dispatching packets across physical interfaces.
pub struct MeshTransportDaemon {
    router: MeshRouter,
    channels: Vec<MeshPeerChannel>,
}

impl MeshTransportDaemon {
    pub fn new(local_node_id: impl Into<String>) -> Self {
        Self {
            router: MeshRouter::new(local_node_id),
            channels: Vec::new(),
        }
    }

    pub fn attach_channel(&mut self, peer_id: impl Into<String>, transport: MeshTransportType) {
        self.channels.push(MeshPeerChannel {
            peer_id: peer_id.into(),
            transport,
            is_connected: true,
        });
    }

    pub fn connected_channel_count(&self) -> usize {
        self.channels.iter().filter(|c| c.is_connected).count()
    }

    pub fn router_mut(&mut self) -> &mut MeshRouter {
        &mut self.router
    }

    pub fn router(&self) -> &MeshRouter {
        &self.router
    }

    /// Broadcasts the next queued forward packet across all matching connected transport channels.
    pub fn dispatch_next_forward(&mut self) -> Option<(MeshPacket, usize)> {
        if let Some(packet) = self.router.next_forward_packet() {
            let matching_channels = self.channels
                .iter()
                .filter(|c| c.is_connected && c.transport == packet.header.transport)
                .count();
            Some((packet, matching_channels))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_packet_delivery_to_destination() {
        let mut router = MeshRouter::new("bob_device");
        let packet = MeshPacket::new(
            "pkt_001",
            "alice_device",
            "bob_device",
            4,
            MeshTransportType::BleMesh,
            b"hello bob".to_vec(),
            1700000000,
        );

        let result = router.process_incoming_packet(packet.clone());
        assert_eq!(result, PacketProcessingResult::DeliveredLocally(packet));
        assert_eq!(router.delivered_packets().len(), 1);
        assert_eq!(router.queued_forward_count(), 0);
    }

    #[test]
    fn test_mesh_packet_forwarding_decrement_and_loop_prevention() {
        let mut relay_router = MeshRouter::new("relay_node_1");
        let packet = MeshPacket::new(
            "pkt_002",
            "alice_device",
            "charlie_device",
            3,
            MeshTransportType::WifiDirect,
            b"relay payload".to_vec(),
            1700000000,
        );

        let result = relay_router.process_incoming_packet(packet);
        match result {
            PacketProcessingResult::ForwardQueued(fwd) => {
                assert_eq!(fwd.header.hop_limit, 2);
                assert_eq!(fwd.header.visited_nodes, vec!["alice_device", "relay_node_1"]);
            }
            _ => panic!("Expected ForwardQueued result"),
        }
        assert_eq!(relay_router.queued_forward_count(), 1);

        // Deduplication: re-sending the same packet ID must be dropped
        let dup_packet = MeshPacket::new(
            "pkt_002",
            "alice_device",
            "charlie_device",
            3,
            MeshTransportType::WifiDirect,
            b"relay payload".to_vec(),
            1700000000,
        );
        let dup_result = relay_router.process_incoming_packet(dup_packet);
        assert_eq!(dup_result, PacketProcessingResult::Dropped(DropReason::DuplicatePacket));
    }

    #[test]
    fn test_mesh_packet_hop_limit_exhaustion() {
        let mut router = MeshRouter::new("relay_node_2");
        let packet = MeshPacket::new(
            "pkt_003",
            "alice_device",
            "dave_device",
            1, // Hop limit 1 cannot be forwarded
            MeshTransportType::UdpBroadcast,
            b"short lived".to_vec(),
            1700000000,
        );

        let result = router.process_incoming_packet(packet);
        assert_eq!(result, PacketProcessingResult::Dropped(DropReason::HopLimitExhausted));
        assert_eq!(router.queued_forward_count(), 0);
    }

    #[test]
    fn test_mesh_packet_cycle_loop_detection() {
        let mut router = MeshRouter::new("cycle_node");
        let mut packet = MeshPacket::new(
            "pkt_004",
            "alice_device",
            "target_node",
            5,
            MeshTransportType::BleMesh,
            b"loop test".to_vec(),
            1700000000,
        );
        // Simulate cycle where packet already passed through cycle_node
        packet.header.visited_nodes.push("cycle_node".to_string());

        let result = router.process_incoming_packet(packet);
        assert_eq!(result, PacketProcessingResult::Dropped(DropReason::RoutingLoopDetected));
    }

    #[test]
    fn test_mesh_seen_ttl_eviction() {
        let mut router = MeshRouter::with_ttl("node_ttl", 60); // 60s TTL
        let packet1 = MeshPacket::new(
            "pkt_ttl_01",
            "alice",
            "bob",
            3,
            MeshTransportType::BleMesh,
            vec![],
            1000,
        );

        router.process_incoming_packet(packet1);
        assert_eq!(router.seen_packet_count(), 1);

        // At t=1050 (within 60s), packet is not expired
        let pruned = router.prune_expired_seen(1050);
        assert_eq!(pruned, 0);
        assert_eq!(router.seen_packet_count(), 1);

        // At t=1061 (past 60s TTL), packet is pruned
        let pruned = router.prune_expired_seen(1061);
        assert_eq!(pruned, 1);
        assert_eq!(router.seen_packet_count(), 0);
    }

    #[test]
    fn test_mesh_transport_daemon_dispatch() {
        let mut daemon = MeshTransportDaemon::new("relay_daemon_1");
        daemon.attach_channel("peer_ble_01", MeshTransportType::BleMesh);
        daemon.attach_channel("peer_wifi_01", MeshTransportType::WifiDirect);

        assert_eq!(daemon.connected_channel_count(), 2);

        let packet = MeshPacket::new(
            "pkt_fwd_01",
            "sender_node",
            "recipient_node",
            4,
            MeshTransportType::BleMesh,
            b"ble mesh payload".to_vec(),
            2000,
        );

        let res = daemon.router_mut().process_incoming_packet(packet);
        assert!(matches!(res, PacketProcessingResult::ForwardQueued(_)));

        // Dispatch forward packet to matching BLE channels
        let (fwd_packet, matching_channels) = daemon.dispatch_next_forward().expect("Should dispatch queued packet");
        assert_eq!(fwd_packet.header.packet_id, "pkt_fwd_01");
        assert_eq!(fwd_packet.header.hop_limit, 3);
        assert_eq!(matching_channels, 1); // Only 1 BLE channel matches
    }
}
