use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
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

/// Router managing store-and-forward mesh packet queueing, loop detection, and deduplication.
pub struct MeshRouter {
    pub local_node_id: String,
    seen_packets: HashSet<String>,
    forward_queue: VecDeque<MeshPacket>,
    delivered_inbox: Vec<MeshPacket>,
}

impl MeshRouter {
    pub fn new(local_node_id: impl Into<String>) -> Self {
        Self {
            local_node_id: local_node_id.into(),
            seen_packets: HashSet::new(),
            forward_queue: VecDeque::new(),
            delivered_inbox: Vec::new(),
        }
    }

    /// Ingests an incoming mesh packet from a local transport.
    pub fn process_incoming_packet(&mut self, packet: MeshPacket) -> PacketProcessingResult {
        // 1. Check for duplicates
        if self.seen_packets.contains(&packet.header.packet_id) {
            return PacketProcessingResult::Dropped(DropReason::DuplicatePacket);
        }
        self.seen_packets.insert(packet.header.packet_id.clone());

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
}
