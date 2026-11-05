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
}
