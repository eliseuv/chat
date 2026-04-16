use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::Context;
use clap::Parser;
use tokio::net::TcpStream;

use client_tui::app::ChatApp;

/// Default server address
const DEFAULT_SERVER_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
/// Default server port
const DEFAULT_SERVER_PORT: u16 = 6969;

/// Server Command Line Arguments
#[derive(Parser)]
struct Args {
    /// Server address
    #[arg(short, long, default_value_t = DEFAULT_SERVER_ADDRESS)]
    address: IpAddr,

    /// Server port
    #[arg(short, long, default_value_t = DEFAULT_SERVER_PORT)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    log4rs::init_file("client-tui/log4rs.yml", Default::default())
        .context("Unable to initialize logger")?;

    // Parse arguments
    let args = Args::parse();

    // Server socket address
    let socket = SocketAddr::new(args.address, args.port);

    // Connect to server
    let stream = loop {
        match TcpStream::connect(socket).await {
            Ok(stream) => break stream,
            Err(e) => {
                println!("Unable to connect to server: {}. Retrying in 2 seconds...", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    };

    // Run chat application
    ChatApp::new(stream).run().await
}
