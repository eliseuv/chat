use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use clap::Parser;
use tokio::net::{TcpListener, TcpStream};

use server::{client::Client, server::Server};

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
async fn main() {
    env_logger::init();
    let args = Args::parse();

    let socket = SocketAddr::new(args.address, args.port);
    log::info!("Listening on {}", socket);
    let listener = TcpListener::bind(socket)
        .await
        .expect("Failed to bind to socket");

    // Spwan server core
    let (server, cmd_tx, bcast_tx) = Server::new();
    tokio::spawn(async move {
        server.run().await;
    });

    // Spawn mock clients
    let mock_socket = socket;
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        for i in 1..=5 {
            log::info!("[MockSpawner] Spawning mock client {}", i);
            if let Err(e) = TcpStream::connect(mock_socket).await {
                log::error!("[MockSpawner] Client {} failed to connect: {}", i, e);
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    // Listen for incoming connections
    loop {
        match listener.accept().await {
            Err(e) => log::error!("Failed to accept connection: {}", e),

            Ok((stream, addr)) => {
                log::info!("New connection from {}", addr);
                let client = Client::new(addr, stream, cmd_tx.clone(), &bcast_tx);

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
