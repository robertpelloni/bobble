use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;


#[cfg(unix)]
use tokio::net::UnixStream;

/// Signal bridge.
///
/// HONESTY NOTE (v1.20.2): there is no live Signal transport in this
/// codebase. The signald JSON dispatch protocol was never wired, so a
/// production bridge (`new` / `with_socket_path`) fails loudly:
/// `initialize` errors when signald is unreachable, and `send_message`
/// always errors instead of reporting a delivery that never happened.
/// The simulated transport exists only behind the explicit
/// `new_mock()` constructor, for tests and simulations.
pub struct SignalBridge {
    socket_path: String,
    connected: bool,
    mock: bool,
}

impl SignalBridge {
    pub fn new() -> Self {
        Self::with_socket_path("/var/run/signald/signald.sock")
    }

    pub fn with_socket_path(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
            connected: false,
            mock: false,
        }
    }

    /// Explicit mock transport: initialization always "connects" and
    /// sends always "succeed" by printing. For tests/simulations only —
    /// never use in production wiring.
    pub fn new_mock() -> Self {
        Self {
            socket_path: "/var/run/signald/signald.sock".to_string(),
            connected: false,
            mock: true,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.mock
    }
}

#[async_trait]
impl ProtocolBridge for SignalBridge {
    // Both cfg branches above return; the trailing Ok(()) only satisfies
    // the compiler's flow analysis.
    #[allow(unreachable_code)]
    async fn initialize(&mut self) -> Result<()> {
        if self.mock {
            println!("Mock Signal bridge initialized (simulated signald connection)");
            self.connected = true;
            return Ok(());
        }

        #[cfg(unix)]
        match UnixStream::connect(&self.socket_path).await {
            Ok(_) => {
                println!("Successfully connected to signald at {}", self.socket_path);
                self.connected = true;
                return Ok(());
            }
            Err(e) => {
                self.connected = false;
                return Err(anyhow::anyhow!(
                    "signald not available at {}: {} (Signal bridge has no live transport; \
                     mock transport is available via SignalBridge::new_mock() for tests)",
                    self.socket_path, e
                ));
            }
        }

        #[cfg(not(unix))]
        {
            self.connected = false;
            return Err(anyhow::anyhow!(
                "signald Unix sockets are not supported on this platform and the Signal \
                 bridge has no other live transport (mock transport: SignalBridge::new_mock())"
            ));
        }

        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        if !self.connected {
            return Err(anyhow::anyhow!("Signal bridge not connected to signald"));
        }

        if self.mock {
            println!("Mock Signal send via signald to {}: {}", payload.recipient_id, payload.content);
            return Ok(());
        }

        // Connected to a signald socket, but the JSON send protocol was
        // never implemented. Report that honestly instead of faking a
        // delivery. A real implementation would serialize a JSON request
        // over the socket:
        // {"type": "send", "username": "+123", "recipientAddress": {"number": "+456"}, "messageBody": "hello"}
        Err(anyhow::anyhow!(
            "Signal live delivery not implemented: signald send protocol is not wired in this bridge"
        ))
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
    async fn test_signal_init_fails_without_signald() {
        // Point at a socket path that does not exist: initialization must
        // fail honestly instead of mocking a connection.
        let mut bridge = SignalBridge::with_socket_path("/nonexistent/signald.sock");
        let result = bridge.initialize().await;
        assert!(result.is_err(), "init without signald must error, got: {:?}", result);
    }

    #[tokio::test]
    async fn test_signal_mock_init_and_send() {
        let mut bridge = SignalBridge::new_mock();
        assert!(bridge.is_mock());
        bridge.initialize().await.unwrap();

        let payload = MessagePayload {
            recipient_id: "+1234567890".to_string(),
            content: "hello signal".to_string(),
            fallback_protocol: "none".to_string(),
        };
        let result = bridge.send_message(&payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_signal_send_requires_connection() {
        let bridge = SignalBridge::new();
        let payload = MessagePayload {
            recipient_id: "+1234567890".to_string(),
            content: "hello signal".to_string(),
            fallback_protocol: "none".to_string(),
        };
        assert!(bridge.send_message(&payload).await.is_err());
    }
}
