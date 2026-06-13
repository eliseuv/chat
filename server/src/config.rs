use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_ping_interval")]
    pub ping_interval_secs: u64,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_channel_capacity")]
    pub channel_capacity: usize,
    #[serde(default = "default_max_frame_length")]
    pub max_frame_length: usize,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_address")]
    pub address: std::net::IpAddr,
    #[serde(default = "default_database_path")]
    pub database_path: String,
}

fn default_ping_interval() -> u64 {
    10
}

fn default_timeout() -> u64 {
    30
}

fn default_channel_capacity() -> usize {
    32
}

fn default_max_frame_length() -> usize {
    1024 * 1024
}

fn default_port() -> u16 {
    6969
}

fn default_address() -> std::net::IpAddr {
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))
}

fn default_database_path() -> String {
    let base = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.local/share", home)
    });
    format!("{}/chat/chat.db", base)
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ping_interval_secs: default_ping_interval(),
            timeout_secs: default_timeout(),
            channel_capacity: default_channel_capacity(),
            max_frame_length: default_max_frame_length(),
            port: default_port(),
            address: default_address(),
            database_path: default_database_path(),
        }
    }
}
