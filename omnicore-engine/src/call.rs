use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models::ProtocolKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    AudioOnly,
    AudioVideo,
    ScreenShare,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallState {
    Initiating,
    Ringing,
    Connected,
    Ended { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdpOfferAnswer {
    pub sdp: String,
    pub sdp_type: String, // "offer" or "answer"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSession {
    pub call_id: String,
    pub protocol: ProtocolKind,
    pub caller_id: String,
    pub callee_id: String,
    pub media_type: MediaType,
    pub state: CallState,
    pub offer: Option<SdpOfferAnswer>,
    pub answer: Option<SdpOfferAnswer>,
    pub candidates: Vec<IceCandidate>,
    pub started_at: u64,
}

impl CallSession {
    pub fn new(
        call_id: &str,
        protocol: ProtocolKind,
        caller_id: &str,
        callee_id: &str,
        media_type: MediaType,
    ) -> Self {
        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            call_id: call_id.to_string(),
            protocol,
            caller_id: caller_id.to_string(),
            callee_id: callee_id.to_string(),
            media_type,
            state: CallState::Initiating,
            offer: None,
            answer: None,
            candidates: Vec::new(),
            started_at,
        }
    }

    pub fn set_offer(&mut self, sdp: &str) -> Result<()> {
        if self.state != CallState::Initiating && self.state != CallState::Ringing {
            return Err(anyhow!("Cannot set offer when call is in state {:?}", self.state));
        }
        self.offer = Some(SdpOfferAnswer {
            sdp: sdp.to_string(),
            sdp_type: "offer".to_string(),
        });
        self.state = CallState::Ringing;
        Ok(())
    }

    pub fn set_answer(&mut self, sdp: &str) -> Result<()> {
        if self.state != CallState::Ringing {
            return Err(anyhow!("Cannot set answer when call is in state {:?}", self.state));
        }
        self.answer = Some(SdpOfferAnswer {
            sdp: sdp.to_string(),
            sdp_type: "answer".to_string(),
        });
        self.state = CallState::Connected;
        Ok(())
    }

    pub fn add_ice_candidate(&mut self, candidate: IceCandidate) {
        self.candidates.push(candidate);
    }

    pub fn end_call(&mut self, reason: &str) {
        self.state = CallState::Ended {
            reason: reason.to_string(),
        };
    }
}

/// A media packet payload for real-time audio/video RTP streaming across bridged gateways.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaPacket {
    pub sequence: u32,
    pub timestamp: u64,
    pub ssrc: u32,
    pub payload: Vec<u8>,
    pub media_type: MediaType,
}

/// Routing descriptor mapping source and destination protocol endpoints for bridged calls.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallGatewayRoute {
    pub route_id: String,
    pub source_protocol: ProtocolKind,
    pub source_session_id: String,
    pub target_protocol: ProtocolKind,
    pub target_session_id: String,
    pub is_active: bool,
}

/// Aggregates and bridges media sessions between diverse voice transports (e.g. WebRTC, Matrix VoIP, Discord voice).
pub struct CrossProtocolCallGateway {
    active_routes: Vec<CallGatewayRoute>,
    relayed_packet_count: u64,
}

impl CrossProtocolCallGateway {
    pub fn new() -> Self {
        Self {
            active_routes: Vec::new(),
            relayed_packet_count: 0,
        }
    }

    /// Establishes a bidirectional media bridge between two distinct call sessions.
    pub fn bridge_sessions(
        &mut self,
        source_protocol: ProtocolKind,
        source_session_id: &str,
        target_protocol: ProtocolKind,
        target_session_id: &str,
    ) -> Result<String> {
        let route_id = format!("{}:{}-{}:{}",
            source_protocol.as_str(), source_session_id,
            target_protocol.as_str(), target_session_id
        );

        if self.active_routes.iter().any(|r| r.route_id == route_id && r.is_active) {
            return Err(anyhow!("Active bridge route already exists for {}", route_id));
        }

        let route = CallGatewayRoute {
            route_id: route_id.clone(),
            source_protocol,
            source_session_id: source_session_id.to_string(),
            target_protocol,
            target_session_id: target_session_id.to_string(),
            is_active: true,
        };

        self.active_routes.push(route);
        Ok(route_id)
    }

    /// Transcodes or forwards an RTP/media packet across the established gateway route.
    pub fn transcode_and_forward(
        &mut self,
        route_id: &str,
        packet: MediaPacket,
    ) -> Result<MediaPacket> {
        let _route = self.active_routes
            .iter()
            .find(|r| r.route_id == route_id && r.is_active)
            .ok_or_else(|| anyhow!("No active route found for id: {}", route_id))?;

        self.relayed_packet_count += 1;

        // In a production transcoding pipeline, sample-rate or codec re-encoding
        // (Opus 48kHz for Discord, WebRTC RTP packetization) occurs here.
        // We preserve packet integrity while retargeting the sequence & timestamp.
        let mut forwarded = packet;
        forwarded.sequence += 1;
        Ok(forwarded)
    }

    /// Terminates an active gateway route bridge.
    pub fn terminate_bridge(&mut self, route_id: &str) -> bool {
        if let Some(pos) = self.active_routes.iter().position(|r| r.route_id == route_id) {
            self.active_routes.remove(pos);
            true
        } else {
            false
        }
    }

    /// Returns the number of currently active call gateway routes.
    pub fn active_gateway_count(&self) -> usize {
        self.active_routes.iter().filter(|r| r.is_active).count()
    }

    /// Returns the total number of relayed media packets across the gateway.
    pub fn relayed_packet_count(&self) -> u64 {
        self.relayed_packet_count
    }
}

impl Default for CrossProtocolCallGateway {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration settings for the RTP Jitter Buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JitterBufferConfig {
    pub max_capacity: usize,
    pub playout_delay_packets: usize,
}

impl Default for JitterBufferConfig {
    fn default() -> Self {
        Self {
            max_capacity: 64,
            playout_delay_packets: 3,
        }
    }
}

/// Manages packet reordering, duplicate suppression, and loss concealment for media streams.
pub struct RtpJitterBuffer {
    config: JitterBufferConfig,
    packets: Vec<MediaPacket>,
    last_popped_sequence: Option<u32>,
    lost_packets_count: u64,
}

impl RtpJitterBuffer {
    pub fn new(config: JitterBufferConfig) -> Self {
        Self {
            config,
            packets: Vec::new(),
            last_popped_sequence: None,
            lost_packets_count: 0,
        }
    }

    /// Ingests an incoming media packet, sorting it by sequence number and discarding duplicates.
    pub fn push_packet(&mut self, packet: MediaPacket) -> bool {
        // Discard already played-out packets
        if let Some(last_seq) = self.last_popped_sequence {
            if packet.sequence <= last_seq {
                return false;
            }
        }

        // Check for duplicates in buffer
        if self.packets.iter().any(|p| p.sequence == packet.sequence) {
            return false;
        }

        self.packets.push(packet);
        // Sort in ascending order of sequence number
        self.packets.sort_by_key(|p| p.sequence);

        // Enforce max capacity
        if self.packets.len() > self.config.max_capacity {
            self.packets.remove(0);
        }
        true
    }

    /// Pops the next ready packet from the playout queue once delay threshold is met.
    pub fn pop_packet(&mut self) -> Option<MediaPacket> {
        if self.packets.len() < self.config.playout_delay_packets && self.last_popped_sequence.is_none() {
            return None;
        }

        if self.packets.is_empty() {
            return None;
        }

        let next_packet = self.packets.remove(0);

        // Detect missing sequence numbers (loss detection)
        if let Some(last_seq) = self.last_popped_sequence {
            if next_packet.sequence > last_seq + 1 {
                let missing = (next_packet.sequence - (last_seq + 1)) as u64;
                self.lost_packets_count += missing;
            }
        }

        self.last_popped_sequence = Some(next_packet.sequence);
        Some(next_packet)
    }

    /// Generates a synthetic packet loss concealment (PLC) frame to bridge missing audio gaps.
    pub fn generate_plc_frame(&self, expected_sequence: u32, ssrc: u32) -> MediaPacket {
        MediaPacket {
            sequence: expected_sequence,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ssrc,
            payload: vec![0u8; 160], // Standard 20ms silence/comfort noise frame
            media_type: MediaType::AudioOnly,
        }
    }

    pub fn buffered_count(&self) -> usize {
        self.packets.len()
    }

    pub fn lost_packet_count(&self) -> u64 {
        self.lost_packets_count
    }
}

/// Supported audio codec profiles for voice call media streaming and transcoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodecProfile {
    OpusWideband, // 48kHz stereo/mono, 6kbps - 510kbps
    G711uLaw,     // 8kHz standard telephony (North America)
    G711aLaw,     // 8kHz standard telephony (Europe / International)
    PcmLinear16,  // Uncompressed raw audio samples
}

/// Controls dynamic bit-rate and frame duration adaptations based on real-time network conditions.
pub struct BitrateController {
    pub codec: AudioCodecProfile,
    pub current_bitrate_bps: u32,
    pub min_bitrate_bps: u32,
    pub max_bitrate_bps: u32,
    pub frame_duration_ms: u32,
}

impl Default for BitrateController {
    fn default() -> Self {
        Self {
            codec: AudioCodecProfile::OpusWideband,
            current_bitrate_bps: 48000,
            min_bitrate_bps: 12000,
            max_bitrate_bps: 128000,
            frame_duration_ms: 20,
        }
    }
}

impl BitrateController {
    pub fn new(codec: AudioCodecProfile) -> Self {
        let (min, max, default_br) = match codec {
            AudioCodecProfile::OpusWideband => (12000, 128000, 48000),
            AudioCodecProfile::G711uLaw | AudioCodecProfile::G711aLaw => (64000, 64000, 64000),
            AudioCodecProfile::PcmLinear16 => (128000, 256000, 128000),
        };

        Self {
            codec,
            current_bitrate_bps: default_br,
            min_bitrate_bps: min,
            max_bitrate_bps: max,
            frame_duration_ms: 20,
        }
    }

    /// Dynamically adjusts bit-rate and frame duration given packet loss fraction (0.0 to 1.0) and round-trip latency.
    pub fn update_network_conditions(&mut self, loss_fraction: f32, rtt_ms: u64) -> u32 {
        if self.min_bitrate_bps == self.max_bitrate_bps {
            return self.current_bitrate_bps;
        }

        if loss_fraction > 0.10 || rtt_ms > 300 {
            // High loss or high latency: step down bit-rate aggressively (-25%) and bundle larger frame sizes
            let reduced = (self.current_bitrate_bps as f32 * 0.75) as u32;
            self.current_bitrate_bps = reduced.max(self.min_bitrate_bps);
            self.frame_duration_ms = 40; // Increase packet duration to reduce IP packet header overhead
        } else if loss_fraction > 0.03 || rtt_ms > 150 {
            // Moderate loss: moderate reduction (-10%)
            let reduced = (self.current_bitrate_bps as f32 * 0.90) as u32;
            self.current_bitrate_bps = reduced.max(self.min_bitrate_bps);
            self.frame_duration_ms = 20;
        } else {
            // Clean network (loss <= 3% and low latency): incrementally step up (+10%) towards max quality
            let increased = (self.current_bitrate_bps as f32 * 1.10) as u32;
            self.current_bitrate_bps = increased.min(self.max_bitrate_bps);
            self.frame_duration_ms = 20;
        }

        self.current_bitrate_bps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_lifecycle() {
        let mut call = CallSession::new(
            "call_test_123",
            ProtocolKind::Matrix,
            "@alice:matrix.org",
            "@bob:matrix.org",
            MediaType::AudioVideo,
        );
        assert_eq!(call.state, CallState::Initiating);

        // Set SDP Offer
        call.set_offer("v=0\r\no=alice ...").expect("Setting offer should succeed");
        assert_eq!(call.state, CallState::Ringing);

        // Add ICE candidate
        call.add_ice_candidate(IceCandidate {
            candidate: "candidate:1 1 UDP 2130706431 192.168.1.50 50000 typ host".to_string(),
            sdp_mid: Some("0".to_string()),
            sdp_m_line_index: Some(0),
        });
        assert_eq!(call.candidates.len(), 1);

        // Set SDP Answer
        call.set_answer("v=0\r\no=bob ...").expect("Setting answer should succeed");
        assert_eq!(call.state, CallState::Connected);

        // End Call
        call.end_call("User hung up");
        assert_eq!(
            call.state,
            CallState::Ended {
                reason: "User hung up".to_string()
            }
        );
    }

    #[test]
    fn test_cross_protocol_call_gateway_lifecycle() {
        let mut gateway = CrossProtocolCallGateway::new();
        assert_eq!(gateway.active_gateway_count(), 0);

        // Bridge Matrix VoIP session with Discord voice channel
        let route_id = gateway
            .bridge_sessions(
                ProtocolKind::Matrix,
                "matrix_call_sess_01",
                ProtocolKind::Discord,
                "discord_voice_channel_99",
            )
            .expect("Bridge should succeed");

        assert_eq!(gateway.active_gateway_count(), 1);

        // Media packet forwarding
        let packet = MediaPacket {
            sequence: 100,
            timestamp: 1600000000,
            ssrc: 0xDEADBEEF,
            payload: vec![0x12, 0x34, 0x56, 0x78],
            media_type: MediaType::AudioOnly,
        };

        let forwarded = gateway
            .transcode_and_forward(&route_id, packet)
            .expect("Transcoding and forwarding should succeed");

        assert_eq!(forwarded.sequence, 101);
        assert_eq!(gateway.relayed_packet_count(), 1);

        // Terminate bridge
        let terminated = gateway.terminate_bridge(&route_id);
        assert!(terminated);
        assert_eq!(gateway.active_gateway_count(), 0);

        // Forwarding over terminated route should fail
        let fail_result = gateway.transcode_and_forward(
            &route_id,
            MediaPacket {
                sequence: 102,
                timestamp: 1600000020,
                ssrc: 0xDEADBEEF,
                payload: vec![],
                media_type: MediaType::AudioOnly,
            },
        );
        assert!(fail_result.is_err());
    }

    #[test]
    fn test_rtp_jitter_buffer_reordering_and_deduplication() {
        let config = JitterBufferConfig {
            max_capacity: 10,
            playout_delay_packets: 3,
        };
        let mut jb = RtpJitterBuffer::new(config);

        // Push out of order: seq 3, seq 1, seq 2
        jb.push_packet(MediaPacket {
            sequence: 3,
            timestamp: 1000,
            ssrc: 1,
            payload: vec![3],
            media_type: MediaType::AudioOnly,
        });

        jb.push_packet(MediaPacket {
            sequence: 1,
            timestamp: 1000,
            ssrc: 1,
            payload: vec![1],
            media_type: MediaType::AudioOnly,
        });

        // Duplicate seq 1 should be rejected
        let dup_added = jb.push_packet(MediaPacket {
            sequence: 1,
            timestamp: 1000,
            ssrc: 1,
            payload: vec![1],
            media_type: MediaType::AudioOnly,
        });
        assert!(!dup_added);

        // Playout threshold not met yet (2 packets vs threshold 3)
        assert!(jb.pop_packet().is_none());

        jb.push_packet(MediaPacket {
            sequence: 2,
            timestamp: 1000,
            ssrc: 1,
            payload: vec![2],
            media_type: MediaType::AudioOnly,
        });

        // Threshold of 3 packets now met; packets must pop in sorted order: 1, 2, 3
        let p1 = jb.pop_packet().expect("Should pop seq 1");
        assert_eq!(p1.sequence, 1);

        let p2 = jb.pop_packet().expect("Should pop seq 2");
        assert_eq!(p2.sequence, 2);

        let p3 = jb.pop_packet().expect("Should pop seq 3");
        assert_eq!(p3.sequence, 3);

        assert!(jb.pop_packet().is_none());
    }

    #[test]
    fn test_rtp_jitter_buffer_loss_detection_and_plc() {
        let config = JitterBufferConfig {
            max_capacity: 10,
            playout_delay_packets: 1,
        };
        let mut jb = RtpJitterBuffer::new(config);

        jb.push_packet(MediaPacket {
            sequence: 10,
            timestamp: 2000,
            ssrc: 42,
            payload: vec![10],
            media_type: MediaType::AudioOnly,
        });

        let p1 = jb.pop_packet().unwrap();
        assert_eq!(p1.sequence, 10);

        // Packet 11 was lost, packet 12 arrives
        jb.push_packet(MediaPacket {
            sequence: 12,
            timestamp: 2020,
            ssrc: 42,
            payload: vec![12],
            media_type: MediaType::AudioOnly,
        });

        let p2 = jb.pop_packet().unwrap();
        assert_eq!(p2.sequence, 12);
        // Loss detection counter should record 1 lost packet
        assert_eq!(jb.lost_packet_count(), 1);

        // Generate synthetic PLC concealment frame for missing seq 11
        let plc = jb.generate_plc_frame(11, 42);
        assert_eq!(plc.sequence, 11);
        assert_eq!(plc.ssrc, 42);
        assert_eq!(plc.payload.len(), 160);
    }

    #[test]
    fn test_bitrate_controller_adaptation_rules() {
        let mut controller = BitrateController::new(AudioCodecProfile::OpusWideband);
        assert_eq!(controller.current_bitrate_bps, 48000);
        assert_eq!(controller.frame_duration_ms, 20);

        // Under high packet loss (15%), bit-rate decreases by 25% and frame duration increases to 40ms
        let br1 = controller.update_network_conditions(0.15, 50);
        assert_eq!(br1, 36000);
        assert_eq!(controller.frame_duration_ms, 40);

        // Under clean conditions (loss 0%, RTT 30ms), bit-rate ramps up by 10%
        let br2 = controller.update_network_conditions(0.0, 30);
        assert_eq!(br2, 39600);
        assert_eq!(controller.frame_duration_ms, 20);

        // Constant bit-rate codecs (G.711) do not change
        let mut g711 = BitrateController::new(AudioCodecProfile::G711uLaw);
        assert_eq!(g711.update_network_conditions(0.50, 600), 64000);
    }
}
