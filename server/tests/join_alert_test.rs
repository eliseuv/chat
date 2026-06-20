use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use futures::{SinkExt, StreamExt};
use chrono::Utc;

use server::{
    client::Client as ServerClient,
    server::Server as ChatServer,
    remote::codec::ClientCodec,
    remote::packet::{ClientMessage, ClientRemotePacket, ServerCommand, ServerMessage},
};

#[tokio::test]
async fn test_join_alert() {
    // 1. Bind to a random local port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    // 2. Start the server core
    let config = server::config::ServerConfig::default();
    let db = server::db::Database::new(":memory:").unwrap();
    let (server, cmd_tx, bcast_tx) = ChatServer::new(config.channel_capacity, db);
    tokio::spawn(async move {
        server.run().await;
    });

    // 3. Start listener accept loop in the background
    let cmd_tx_clone = cmd_tx.clone();
    let bcast_tx_clone = bcast_tx.clone();
    let config_loop = config.clone();
    tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            let client = ServerClient::new(addr, stream, cmd_tx_clone.clone(), &bcast_tx_clone, config_loop.clone());
            tokio::spawn(async move {
                let _ = client.run().await;
            });
        }
    });

    // 4. Connect Client A (Alice)
    let stream_a = TcpStream::connect(local_addr).await.unwrap();
    let mut client_a = Framed::new(stream_a, ClientCodec::default());

    // Alice immediately receives ActiveUsers [] upon connecting
    let packet = client_a.next().await.unwrap().unwrap();
    match packet.message {
        ServerMessage::Command(ServerCommand::ActiveUsers { usernames }) => {
            assert!(usernames.is_empty());
        }
        other => panic!("Expected initial ActiveUsers, got {:?}", other),
    }

    // Send Login for Alice
    let login_a = ClientRemotePacket {
        timestamp: Utc::now().timestamp_millis(),
        message: ClientMessage::Login("alice".to_string()),
    };
    client_a.send(login_a).await.unwrap();

    // Alice should receive Welcome packet
    let packet = client_a.next().await.unwrap().unwrap();
    match packet.message {
        ServerMessage::Command(ServerCommand::Welcome(_)) => {}
        other => panic!("Expected Welcome, got {:?}", other),
    }

    // Alice should receive ActiveUsers ["alice"]
    let packet = client_a.next().await.unwrap().unwrap();
    match packet.message {
        ServerMessage::Command(ServerCommand::ActiveUsers { usernames }) => {
            assert_eq!(usernames, vec!["alice".to_string()]);
        }
        other => panic!("Expected ActiveUsers with alice, got {:?}", other),
    }

    // 5. Connect Client B (Bob)
    let stream_b = TcpStream::connect(local_addr).await.unwrap();
    let mut client_b = Framed::new(stream_b, ClientCodec::default());

    // On Bob connect, a broadcast of ActiveUsers ["alice"] is sent.
    // So both Alice and Bob receive it.
    
    // Alice receives ActiveUsers ["alice"]
    let packet = client_a.next().await.unwrap().unwrap();
    match packet.message {
        ServerMessage::Command(ServerCommand::ActiveUsers { usernames }) => {
            assert_eq!(usernames, vec!["alice".to_string()]);
        }
        other => panic!("Expected ActiveUsers broadcast on Alice, got {:?}", other),
    }

    // Bob receives ActiveUsers ["alice"]
    let packet = client_b.next().await.unwrap().unwrap();
    match packet.message {
        ServerMessage::Command(ServerCommand::ActiveUsers { usernames }) => {
            assert_eq!(usernames, vec!["alice".to_string()]);
        }
        other => panic!("Expected initial ActiveUsers on Bob, got {:?}", other),
    }

    // Send Login for Bob
    let login_b = ClientRemotePacket {
        timestamp: Utc::now().timestamp_millis(),
        message: ClientMessage::Login("bob".to_string()),
    };
    client_b.send(login_b).await.unwrap();

    // Bob should receive Welcome packet
    let packet = client_b.next().await.unwrap().unwrap();
    match packet.message {
        ServerMessage::Command(ServerCommand::Welcome(_)) => {}
        other => panic!("Expected Welcome, got {:?}", other),
    }

    // Bob should receive ActiveUsers with both alice and bob
    let packet = client_b.next().await.unwrap().unwrap();
    match packet.message {
        ServerMessage::Command(ServerCommand::ActiveUsers { usernames }) => {
            assert!(usernames.contains(&"alice".to_string()));
            assert!(usernames.contains(&"bob".to_string()));
        }
        other => panic!("Expected ActiveUsers on Bob after login, got {:?}", other),
    }

    // Alice should receive Joined("bob")
    let packet = client_a.next().await.unwrap().unwrap();
    match packet.message {
        ServerMessage::Command(ServerCommand::Joined(username)) => {
            assert_eq!(username, "bob");
        }
        other => panic!("Expected Joined alert on Alice, got {:?}", other),
    }

    // Alice should receive updated ActiveUsers with both alice and bob
    let packet = client_a.next().await.unwrap().unwrap();
    match packet.message {
        ServerMessage::Command(ServerCommand::ActiveUsers { usernames }) => {
            assert!(usernames.contains(&"alice".to_string()));
            assert!(usernames.contains(&"bob".to_string()));
        }
        other => panic!("Expected updated ActiveUsers on Alice, got {:?}", other),
    }
}
