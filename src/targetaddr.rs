use crate::portreachable::is_port_reachable_with_timeout;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TargetAddr {
    pub host: String,
    pub port: u16,
}

impl TargetAddr {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }

    pub fn to_host_port(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn is_reachable(&self, timeout: Duration) -> bool {
        is_port_reachable_with_timeout(self.to_host_port(), timeout)
    }
}
