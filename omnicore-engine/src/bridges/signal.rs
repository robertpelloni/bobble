use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;


#[cfg(unix)]
use tokio::net::UnixStream;

pub struct SignalBridge {
    socket_path: String,
    connected: bool,
}

impl SignalBridge {
    pub fn new() -> Self {
        Self {
            // Default path for signald socket
            socket_path: "/var/run/signald/signald.sock".to_string(),
            connected: false,
        }
    }
}

#[async_trait]
impl ProtocolBridge for SignalBridge {
    async fn initialize(&mut self) -> Result<()> {
        // Proof-of-concept initialization mimicking signald IPC connection
        // We do a non-blocking check to see if the socket exists/connects.
        // In this mock, we gracefully degrade if signald is not running locally.
        #[cfg(unix)]
        match UnixStream::connect(&self.socket_path).await {
            Ok(_) => {
                println!("Successfully connected to signald at {}", self.socket_path);
                self.connected = true;
            }
            Err(e) => {
                println!("signald not available at {} (Mocking connection): {}", self.socket_path, e);
                self.connected = true; // Mocking true for the proof-of-concept
            }
        }

        #[cfg(not(unix))]
        {
            println!("signald UnixSockets not supported on this platform. Mocking connection.");
            self.connected = true;
        }

        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        if !self.connected {
            return Err(anyhow::anyhow!("Signal bridge not connected to signald"));
        }

        // Mock signald protocol message dispatch
        // A real implementation would serialize a JSON request over the UnixSocket:
        // {"type": "send", "username": "+123", "recipientAddress": {"number": "+456"}, "messageBody": "hello"}

        println!("Mock Signal send via signald to {}: {}", payload.recipient_id, payload.content);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signal_init() {
        let mut bridge = SignalBridge::new();
        let result = bridge.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_signal_send() {
        let mut bridge = SignalBridge::new();
        bridge.initialize().await.unwrap();

        let payload = MessagePayload {
            recipient_id: "+1234567890".to_string(),
            content: "hello signal".to_string(),
            fallback_protocol: "none".to_string(),
        };
        let result = bridge.send_message(&payload).await;
        assert!(result.is_ok());
    }
}
