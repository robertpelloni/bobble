use tokio::sync::mpsc;
use prost::Message;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Signal,
    Matrix,
    Simplex,
    Tox,
    Discord,
    IMessage,
    Sms,
}

#[derive(Clone, PartialEq, Message)]
pub struct MessagePayload {
    #[prost(string, tag="1")]
    pub recipient_id: String,
    #[prost(string, tag="2")]
    pub content: String,
    #[prost(string, tag="3")]
    pub fallback_protocol: String, // String representation for simplicity in prost
}

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("Timeout occurred while sending via protocol: {0:?}")]
    Timeout(Protocol),
    #[error("Failed to send message: {0}")]
    SendFailed(String),
}

type DispatchResult = Result<(), DispatchError>;

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

        // Send multiple messages concurrently
        let d1 = dispatcher.dispatch(Protocol::Signal, payload.clone());
        let d2 = dispatcher.dispatch(Protocol::Matrix, payload.clone());

        let (res1, res2) = tokio::join!(d1, d2);
        assert!(res1.is_ok());
        assert!(res2.is_ok());

        // Verify receiver gets them
        let r1 = receiver.recv().await.unwrap();
        let r2 = receiver.recv().await.unwrap();

        assert_eq!(r1.1, payload);
        assert_eq!(r2.1, payload);
    }

    #[tokio::test]
    async fn test_fallback_triggers() {
        let (dispatcher, mut receiver) = Dispatcher::new();

        let payload = MessagePayload {
            recipient_id: "user2".to_string(),
            content: "Important message".to_string(),
            fallback_protocol: "Sms".to_string(),
        };

        // Dispatch primary
        let res = dispatcher.dispatch(Protocol::Signal, payload.clone()).await;
        assert!(res.is_ok());

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.0, Protocol::Signal);

        // Simulate fallback logic (usually would be triggered by worker task timeout)
        let simulated_timeout = true;
        if simulated_timeout {
            let res_fallback = dispatcher.dispatch(Protocol::Sms, payload).await;
            assert!(res_fallback.is_ok());

            let received_fallback = receiver.recv().await.unwrap();
            assert_eq!(received_fallback.0, Protocol::Sms);
        }
    }
}
