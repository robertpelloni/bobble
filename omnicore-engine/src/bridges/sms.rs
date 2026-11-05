use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

pub struct SmsBridge {
    serial_port: String,
    baud_rate: u32,
    ready: bool,
}

impl SmsBridge {
    pub fn new() -> Self {
        Self {
            serial_port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            ready: false,
        }
    }

    pub fn with_port(serial_port: &str, baud_rate: u32) -> Self {
        Self {
            serial_port: serial_port.to_string(),
            baud_rate,
            ready: false,
        }
    }
}

#[async_trait]
impl ProtocolBridge for SmsBridge {
    async fn initialize(&mut self) -> Result<()> {
        println!(
            "Initializing GSM modem at {} ({} baud) for cellular SMS fallback",
            self.serial_port, self.baud_rate
        );
        self.ready = true;
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        if !self.ready {
            return Err(anyhow!("SMS modem bridge not ready"));
        }
        println!(
            "Mock Cellular SMS dispatch to {}: {}",
            payload.recipient_id, payload.content
        );
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.ready = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sms_init_and_send() {
        let mut bridge = SmsBridge::new();
        assert!(bridge.initialize().await.is_ok());

        let payload = MessagePayload {
            recipient_id: "+15550199".to_string(),
            content: "Emergency SMS Fallback test".to_string(),
            fallback_protocol: "none".to_string(),
        };

        assert!(bridge.send_message(&payload).await.is_ok());
        assert!(bridge.disconnect().await.is_ok());
        assert!(bridge.send_message(&payload).await.is_err());
    }
}
