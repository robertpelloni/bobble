// Domain models for OmniCore universal messaging engine
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolKind {
    Signal,
    Matrix,
    Discord,
    Simplex,
    Tox,
    IMessage,
    Sms,
}

impl ProtocolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Signal => "Signal",
            Self::Matrix => "Matrix",
            Self::Discord => "Discord",
            Self::Simplex => "Simplex",
            Self::Tox => "Tox",
            Self::IMessage => "iMessage",
            Self::Sms => "SMS",
        }
    }

    pub fn from_str_name(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "signal" => Some(Self::Signal),
            "matrix" => Some(Self::Matrix),
            "discord" => Some(Self::Discord),
            "simplex" => Some(Self::Simplex),
            "tox" => Some(Self::Tox),
            "imessage" => Some(Self::IMessage),
            "sms" => Some(Self::Sms),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryState {
    Queued,
    Sent,
    Delivered,
    Read,
    Failed(String),
    FallbackTriggered {
        from: ProtocolKind,
        to: ProtocolKind,
        reason: String,
    },
}

impl DeliveryState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Sent => "Sent",
            Self::Delivered => "Delivered",
            Self::Read => "Read",
            Self::Failed(_) => "Failed",
            Self::FallbackTriggered { .. } => "FallbackTriggered",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedContact {
    pub id: i64,
    pub name: String,
    pub unified_id: String,
    pub primary_protocol: ProtocolKind,
    pub fallback_protocols: Vec<ProtocolKind>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMapping {
    pub id: i64,
    pub contact_id: i64,
    pub protocol: ProtocolKind,
    pub platform_account_id: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: i64,
    pub contact_id: i64,
    pub protocol: ProtocolKind,
    pub sender: String,
    pub recipient: String,
    pub content: String,
    pub state: DeliveryState,
    pub timestamp: i64,
    pub is_outgoing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingStrategy {
    pub primary: ProtocolKind,
    pub fallbacks: Vec<ProtocolKind>,
    pub timeout_millis: u64,
    pub retry_count: u32,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self {
            primary: ProtocolKind::Signal,
            fallbacks: vec![ProtocolKind::Matrix, ProtocolKind::Sms],
            timeout_millis: 5000,
            retry_count: 2,
        }
    }
}
