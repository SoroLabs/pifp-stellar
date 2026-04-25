//! WebSocket server for real-time event broadcasting.

use crate::events::PifpEvent;
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{ broadcast, RwLock };
use tokio_tungstenite::tungstenite::{ Message, Utf8Bytes };
use tokio_tungstenite::WebSocketStream;
use tracing::{ error, info };

type Tx = broadcast::Sender<String>;
type PeerMap = Arc<
    RwLock<
        HashMap<std::net::SocketAddr, SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>>
    >
>;

#[derive(Clone)]
pub struct WsState {
    pub tx: Tx,
}

impl WsState {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { tx }
    }

    pub fn broadcast_event(&self, event: &PifpEvent) {
        if let Ok(json) = serde_json::to_string(event) {
            let _ = self.tx.send(json);
        }
    }
}

pub async fn run(addr: String, state: WsState) {
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind WebSocket address");
    info!("WebSocket server listening on ws://{}", addr);

    let peers = PeerMap::default();

    while let Ok((stream, addr)) = listener.accept().await {
        let ws_stream = tokio_tungstenite
            ::accept_async(stream).await
            .expect("Error during WebSocket handshake");
        let (write, mut read) = ws_stream.split();
        peers.write().await.insert(addr, write);

        let tx = state.tx.clone();
        let peers_clone = peers.clone();

        tokio::spawn(async move {
            let mut rx = tx.subscribe();
            let mut write = peers_clone.write().await.remove(&addr).unwrap();

            loop {
                tokio::select! {
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Close(_))) => break,
                            Some(Err(e)) => {
                                error!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                    result = rx.recv() => {
                        match result {
                            Ok(msg) => {
                                if write.send(Message::Text(Utf8Bytes::from(msg))).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            }

            peers_clone.write().await.remove(&addr);
        });
    }
}
