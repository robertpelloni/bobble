use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyStatus {
    Disabled,
    Connecting,
    Connected,
    CircuitEstablished { circuit_hops: usize },
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorConfig {
    pub socks5_proxy_host: String,
    pub socks5_proxy_port: u16,
    pub control_port: Option<u16>,
    pub enable_onion_routing: bool,
    pub stream_isolation: bool,
}

impl Default for TorConfig {
    fn default() -> Self {
        Self {
            socks5_proxy_host: "127.0.0.1".to_string(),
            socks5_proxy_port: 9050,
            control_port: Some(9051),
            enable_onion_routing: true,
            stream_isolation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnionEndpoint {
    pub onion_address: String,
    pub port: u16,
    pub protocol: String,
}

impl OnionEndpoint {
    pub fn new(onion_address: &str, port: u16, protocol: &str) -> Result<Self> {
        let addr = onion_address.trim().to_lowercase();
        // Tor v3 onion address is 56 characters + ".onion" = 62 characters
        if !addr.ends_with(".onion") {
            return Err(anyhow!("Invalid onion address: must end with .onion"));
        }
        let prefix = addr.trim_end_matches(".onion");
        if prefix.len() != 56 && prefix.len() != 16 {
            return Err(anyhow!(
                "Invalid onion address length: must be v3 (56 chars) or v2 (16 chars)"
            ));
        }

        Ok(Self {
            onion_address: addr,
            port,
            protocol: protocol.to_string(),
        })
    }

    pub fn to_target_url(&self) -> String {
        format!("{}://{}:{}", self.protocol, self.onion_address, self.port)
    }
}

pub struct TorManager {
    config: TorConfig,
    status: ProxyStatus,
}

impl TorManager {
    pub fn new(config: TorConfig) -> Self {
        Self {
            config,
            status: ProxyStatus::Disabled,
        }
    }

    pub fn socks5_proxy_url(&self) -> String {
        format!(
            "socks5h://{}:{}",
            self.config.socks5_proxy_host, self.config.socks5_proxy_port
        )
    }

    pub fn connect_proxy(&mut self) -> Result<()> {
        if !self.config.enable_onion_routing {
            self.status = ProxyStatus::Disabled;
            return Ok(());
        }

        // Mock establishing local Tor daemon SOCKS5 proxy circuit
        self.status = ProxyStatus::CircuitEstablished { circuit_hops: 3 };
        Ok(())
    }

    pub fn status(&self) -> &ProxyStatus {
        &self.status
    }

    pub fn is_connected(&self) -> bool {
        matches!(
            self.status,
            ProxyStatus::Connected | ProxyStatus::CircuitEstablished { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onion_endpoint_validation() {
        // Valid v3 onion address
        let valid_v3 = "vww6ybal4bd7szmgncyruucpgfkqahzddi37ktceo3ah7ngmcopnpyyd.onion";
        let ep = OnionEndpoint::new(valid_v3, 443, "https").expect("v3 address should be valid");
        assert_eq!(
            ep.to_target_url(),
            format!("https://{}:443", valid_v3)
        );

        // Invalid: missing .onion suffix
        let invalid_no_suffix = "vww6ybal4bd7szmgncyruucpgfkqahzddi37ktceo3ah7ngmcopnpyyd.com";
        assert!(OnionEndpoint::new(invalid_no_suffix, 80, "http").is_err());

        // Invalid: bad length
        let invalid_len = "tooshort.onion";
        assert!(OnionEndpoint::new(invalid_len, 80, "http").is_err());
    }

    #[test]
    fn test_tor_manager_socks5_url_and_circuit() {
        let config = TorConfig {
            socks5_proxy_host: "127.0.0.1".to_string(),
            socks5_proxy_port: 9150,
            control_port: Some(9151),
            enable_onion_routing: true,
            stream_isolation: true,
        };

        let mut mgr = TorManager::new(config);
        assert_eq!(mgr.socks5_proxy_url(), "socks5h://127.0.0.1:9150");
        assert!(!mgr.is_connected());

        mgr.connect_proxy().expect("Proxy connection should succeed");
        assert!(mgr.is_connected());
        assert_eq!(
            mgr.status(),
            &ProxyStatus::CircuitEstablished { circuit_hops: 3 }
        );
    }
}
