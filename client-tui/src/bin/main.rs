use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

use client_tui::app::ChatApp;

/// Default server port
const DEFAULT_SERVER_PORT: u16 = 6969;

/// Chat Café Command Line Arguments
#[derive(Parser)]
#[command(name = "Chat Café", about = "A cozy terminal TUI chat client")]
struct Args {
    /// Server address
    #[arg(short, long)]
    address: Option<String>,

    /// Server port
    #[arg(short, long)]
    port: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerConfig {
    address: String,
    port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    server: ServerConfig,
}

fn get_config_path() -> anyhow::Result<PathBuf> {
    let base_dir = std::env::var("XDG_CONFIG")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var("XDG_CONFIG_HOME").ok().filter(|v| !v.is_empty()))
        .or_else(|| {
            std::env::var("HOME").ok().filter(|v| !v.is_empty()).map(|home| {
                format!("{}/.config", home)
            })
        })
        .context("Could not find configuration directory (XDG_CONFIG, XDG_CONFIG_HOME, and HOME are all unset or empty)")?;
    
    Ok(PathBuf::from(base_dir).join("chat-cafe.yml"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    log4rs::init_file("client-tui/log4rs.yml", Default::default())
        .context("Unable to initialize logger")?;

    // Parse arguments
    let args = Args::parse();

    let config_path = get_config_path()?;

    let (address, port) = if args.address.is_some() || args.port.is_some() {
        // Parse existing config if it exists, to merge omitted values
        let existing_config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).ok();
            content.and_then(|c| serde_yaml::from_str::<Config>(&c).ok())
        } else {
            None
        };

        let address = match args.address {
            Some(addr) => addr,
            None => {
                if let Some(ref cfg) = existing_config {
                    cfg.server.address.clone()
                } else {
                    anyhow::bail!("No server address provided. Please specify --address, or ensure it is configured in {:?}", config_path);
                }
            }
        };

        let port = match args.port {
            Some(p) => p,
            None => {
                if let Some(ref cfg) = existing_config {
                    cfg.server.port
                } else {
                    DEFAULT_SERVER_PORT
                }
            }
        };

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }
        let config = Config {
            server: ServerConfig {
                address: address.clone(),
                port,
            },
        };
        let yaml = serde_yaml::to_string(&config)
            .context("Failed to serialize config to YAML")?;
        std::fs::write(&config_path, yaml)
            .with_context(|| format!("Failed to write config file: {:?}", config_path))?;
        log::info!("Saved server configuration to {:?}", config_path);

        (address, port)
    } else {
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
            let config: Config = serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;
            log::info!("Loaded server configuration from {:?}", config_path);
            (config.server.address, config.server.port)
        } else {
            anyhow::bail!("No configuration file found at {:?}, and no server address arguments were provided.", config_path);
        }
    };

    // Connect to server
    let stream = loop {
        let host_port = format!("{}:{}", address, port);
        match tokio::net::lookup_host(&host_port).await {
            Ok(mut addrs) => {
                let mut connected = None;
                for addr in addrs.by_ref() {
                    match TcpStream::connect(addr).await {
                        Ok(stream) => {
                            connected = Some(stream);
                            break;
                        }
                        Err(e) => {
                            log::debug!("Failed to connect to address {:?}: {}", addr, e);
                        }
                    }
                }
                if let Some(stream) = connected {
                    break stream;
                }
                println!(
                    "Unable to connect to any resolved address for {}. Retrying in 2 seconds...",
                    host_port
                );
            }
            Err(e) => {
                println!(
                    "Unable to resolve host {}: {}. Retrying in 2 seconds...",
                    host_port, e
                );
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    };

    // Run chat application
    ChatApp::new(stream)
        .context("Unable to create chat application")?
        .run()
        .await
}
