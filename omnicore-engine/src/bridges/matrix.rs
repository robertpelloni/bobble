use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

use matrix_sdk::{Client};

/// Matrix bridge.
///
/// HONESTY NOTE (v1.20.2): `initialize` only *constructs* a matrix-sdk
/// client. It does not log in, sync, or join any room, and
/// `send_message` never calls the client — so a production bridge
/// returns an explicit not-implemented error instead of the previous
/// behaviour (print "send attempt 1/3" and report success). The
/// simulated send exists only behind `new_mock()`, for tests.
pub struct MatrixBridge {
    client: Option<Client>,
    max_retries: u32,
    homeserver_url: String,
    mock: bool,
}

impl MatrixBridge {
    pub fn new() -> Self {
        Self::build("https://matrix.org".to_string(), 3, false)
    }

    pub fn with_config(homeserver_url: &str, max_retries: u32) -> Self {
        Self::build(homeserver_url.to_string(), max_retries, false)
    }

    /// Explicit mock transport: sends "succeed" by printing, without any
    /// network delivery. For tests/simulations only.
    pub fn new_mock() -> Self {
        Self::build("https://matrix.org".to_string(), 3, true)
    }

    fn build(homeserver_url: String, max_retries: u32, mock: bool) -> Self {
        Self {
            client: None,
            max_retries,
            homeserver_url,
            mock,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.mock
    }

    /// True only when an SDK client object has been constructed. This
    /// does NOT mean the bridge is logged in or can deliver messages.
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }
}

#[async_trait]
impl ProtocolBridge for MatrixBridge {
    async fn initialize(&mut self) -> Result<()> {
        // Constructs an unauthenticated matrix-sdk client targeting the
        // configured homeserver. No login or sync is performed.
        let client = Client::builder()
            .homeserver_url(&self.homeserver_url)
            .build()
            .await?;

        self.client = Some(client);
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        let _client = self.client.as_ref().ok_or_else(|| anyhow::anyhow!("Matrix client not initialized"))?;

        if payload.recipient_id.trim().is_empty() {
            return Err(anyhow::anyhow!("Matrix recipient/room identifier cannot be empty"));
        }
        if payload.content.trim().is_empty() {
            return Err(anyhow::anyhow!("Matrix message content cannot be empty"));
        }

        if self.mock {
            println!(
                "Mock Matrix send to {}: {}",
                payload.recipient_id, payload.content
            );
            return Ok(());
        }

        // The client is not logged in and no room send was ever wired.
        // Fail honestly instead of printing a fake attempt and returning
        // success. (max_retries applied to a send that never happens
        // would be theatre; it is kept for the future live transport.)
        let _ = self.max_retries;
        Err(anyhow::anyhow!(
            "Matrix live delivery not implemented: bridge is not logged in and no room send is wired \
             (mock transport: MatrixBridge::new_mock())"
        ))
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.client = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_matrix_init() {
        let mut bridge = MatrixBridge::new();
        let result = bridge.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_matrix_send_not_implemented_in_production() {
        let mut bridge = MatrixBridge::new();
        bridge.initialize().await.unwrap();

        let payload = MessagePayload {
            recipient_id: "room123".to_string(),
            content: "hello matrix".to_string(),
            fallback_protocol: "none".to_string(),
        };
        let result = bridge.send_message(&payload).await;
        assert!(result.is_err(), "production Matrix send must not fake delivery");
        assert!(result.unwrap_err().to_string().contains("not implemented"));

        // Empty content validation still fires.
        let empty_payload = MessagePayload {
            recipient_id: "room123".to_string(),
            content: "".to_string(),
            fallback_protocol: "none".to_string(),
        };
        assert!(bridge.send_message(&empty_payload).await.is_err());
    }

    #[tokio::test]
    async fn test_matrix_mock_send() {
        let mut bridge = MatrixBridge::new_mock();
        assert!(bridge.is_mock());
        bridge.initialize().await.unwrap();

        let payload = MessagePayload {
            recipient_id: "room123".to_string(),
            content: "hello matrix".to_string(),
            fallback_protocol: "none".to_string(),
        };
        assert!(bridge.send_message(&payload).await.is_ok());
    }
}
