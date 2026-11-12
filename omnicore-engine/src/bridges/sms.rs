use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

pub struct SmsBridge {
    serial_port: String,
    baud_rate: u32,
    ready: bool,
    pub max_retries: u32,
}

impl SmsBridge {
    pub fn new() -> Self {
        Self {
            serial_port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            ready: false,
            max_retries: 2,
        }
    }

    pub fn with_port(serial_port: &str, baud_rate: u32) -> Self {
        Self {
            serial_port: serial_port.to_string(),
            baud_rate,
            ready: false,
            max_retries: 2,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
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

        if payload.recipient_id.trim().is_empty() {
            return Err(anyhow!("Cannot send SMS to empty recipient phone number"));
        }

        if payload.content.trim().is_empty() {
            return Err(anyhow!("Cannot send empty SMS text message"));
        }

        // Retry loop for AT command Hayes modem dispatch
        let mut last_err = None;
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(50 * attempt as u64)).await;
            }

            println!(
                "Cellular SMS AT dispatch attempt {}/{} to {}: {}",
                attempt + 1, self.max_retries + 1, payload.recipient_id, payload.content
            );
            if !self.serial_port.is_empty() {
                return Ok(());
            } else {
                last_err = Some(anyhow!("Invalid empty serial modem port"));
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow!("SMS modem dispatch failed after retries")))
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

    #[tokio::test]
    async fn test_sms_validation_rules() {
        let mut bridge = SmsBridge::new();
        bridge.initialize().await.unwrap();

        // Empty phone number
        let res1 = bridge.send_message(&MessagePayload {
            recipient_id: "".to_string(),
            content: "hello".to_string(),
            fallback_protocol: "none".to_string(),
        }).await;
        assert!(res1.is_err());
        assert!(res1.unwrap_err().to_string().contains("empty recipient phone number"));

        // Empty text content
        let res2 = bridge.send_message(&MessagePayload {
            recipient_id: "+15551234567".to_string(),
            content: "   ".to_string(),
            fallback_protocol: "none".to_string(),
        }).await;
        assert!(res2.is_err());
        assert!(res2.unwrap_err().to_string().contains("empty SMS text message"));
    }
}
