use prost::Message;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    Signal,
    Matrix,
    Simplex,
    Tox,
    Discord,
    IMessage,
    Sms,
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Message)]
pub struct MessagePayload {
    #[prost(string, tag = "1")]
    pub recipient_id: String,
    #[prost(string, tag = "2")]
    pub content: String,
    #[prost(string, tag = "3")]
    pub fallback_protocol: String,
}

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("Timeout occurred while sending via protocol: {0:?}")]
    Timeout(Protocol),
    #[error("Failed to send message: {0}")]
    SendFailed(String),
}

pub type DispatchResult = Result<(), DispatchError>;

pub struct Dispatcher {
    sender: mpsc::Sender<(Protocol, MessagePayload)>,
}

impl Dispatcher {
    pub fn new() -> (Self, mpsc::Receiver<(Protocol, MessagePayload)>) {
        let (sender, receiver) = mpsc::channel(100);
        (Self { sender }, receiver)
    }

    pub async fn dispatch(&self, protocol: Protocol, payload: MessagePayload) -> DispatchResult {
        self.sender
            .send((protocol, payload))
            .await
            .map_err(|e| DispatchError::SendFailed(e.to_string()))
    }

    /// Dispatches a message to a primary protocol with automated fallback if sending times out or fails.
    pub async fn dispatch_with_fallback(
        &self,
        primary: Protocol,
        fallback: Protocol,
        payload: MessagePayload,
        timeout_duration: Duration,
    ) -> DispatchResult {
        // Attempt dispatch on primary channel with timeout constraint
        let primary_attempt = timeout(timeout_duration, self.dispatch(primary, payload.clone())).await;

        match primary_attempt {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) | Err(_) => {
                // Primary delivery failed or timed out: Trigger fallback protocol automatically
                self.dispatch(fallback, payload).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_message_queue_processing() {
        let (dispatcher, mut receiver) = Dispatcher::new();

        let payload = MessagePayload {
            recipient_id: "user1".to_string(),
            content: "Hello!".to_string(),
            fallback_protocol: "Sms".to_string(),
        };

        let d1 = dispatcher.dispatch(Protocol::Signal, payload.clone());
        let d2 = dispatcher.dispatch(Protocol::Matrix, payload.clone());

        let (res1, res2) = tokio::join!(d1, d2);
        assert!(res1.is_ok());
        assert!(res2.is_ok());

        let r1 = receiver.recv().await.unwrap();
        let r2 = receiver.recv().await.unwrap();

        assert_eq!(r1.1, payload);
        assert_eq!(r2.1, payload);
    }

    #[tokio::test]
    async fn test_dispatch_with_fallback_success() {
        let (dispatcher, mut receiver) = Dispatcher::new();

        let payload = MessagePayload {
            recipient_id: "user2".to_string(),
            content: "Important message".to_string(),
            fallback_protocol: "Sms".to_string(),
        };

        // When primary channel has capacity and succeeds within duration
        let res = dispatcher
            .dispatch_with_fallback(Protocol::Signal, Protocol::Sms, payload.clone(), Duration::from_millis(500))
            .await;
        assert!(res.is_ok());

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.0, Protocol::Signal);
        assert_eq!(received.1, payload);
    }
}
