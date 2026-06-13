use std::net::{IpAddr, SocketAddr};

use clap::Parser;
use tokio::net::TcpListener;

use server::{client::Client, config::ServerConfig, db::Database, server::Server};

/// Server Command Line Arguments
#[derive(Parser)]
struct Args {
    /// Server address
    #[arg(short, long)]
    address: Option<IpAddr>,

    /// Server port
    #[arg(short, long)]
    port: Option<u16>,

    /// Config file path
    #[arg(short, long, default_value = "config.yaml")]
    config: std::path::PathBuf,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    let config = if args.config.exists() {
        let content = std::fs::read_to_string(&args.config).expect("Failed to read config file");
        serde_yaml::from_str(&content).expect("Failed to parse config file")
    } else {
        log::info!("Config file not found, using default configuration");
        ServerConfig::default()
    };

    let socket = SocketAddr::new(
        args.address.unwrap_or(config.address),
        args.port.unwrap_or(config.port),
    );
    log::info!("Listening on {}", socket);
    let listener = TcpListener::bind(socket)
        .await
        .expect("Failed to bind to socket");

    // Initialize database
    let db = Database::new(&config.database_path).expect("Failed to initialize database");

    // Spawn server core
    let (server, cmd_tx, bcast_tx) = Server::new(config.channel_capacity, db);
    tokio::spawn(async move {
        server.run().await;
    });

    // Listen for incoming connections
    loop {
        match listener.accept().await {
            Err(e) => log::error!("Failed to accept connection: {}", e),

            Ok((stream, addr)) => {
                log::info!("New connection from {}", addr);
                let client = Client::new(addr, stream, cmd_tx.clone(), &bcast_tx, config.clone());

                // Spawn a new worker task to handle the client connection asynchronously
                tokio::spawn(async move {
                    if let Err(e) = client.run().await {
                        log::error!("Client error: {}", e);
                    }
                });
            }
        }
    }
}
