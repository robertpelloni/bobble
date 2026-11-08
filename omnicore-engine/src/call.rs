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
}
