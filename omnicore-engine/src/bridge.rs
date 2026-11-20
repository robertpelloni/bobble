use async_trait::async_trait;
use anyhow::Result;
use crate::dispatcher::MessagePayload;

/// A protocol transport.
///
/// Contract (v1.20.2 honesty rule): `send_message` may only return
/// `Ok(())` if the message was actually handed to a live transport.
/// Simulated transports must be explicit — in this codebase they are
/// constructed via each bridge's `new_mock()` constructor and are for
/// tests/simulations only. Production constructors (`new`,
/// `with_config`, …) must fail with an error rather than report a
/// delivery that never happened.
#[async_trait]
pub trait ProtocolBridge: Send + Sync {
    async fn initialize(&mut self) -> Result<()>;
    async fn send_message(&self, payload: &MessagePayload) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
}
