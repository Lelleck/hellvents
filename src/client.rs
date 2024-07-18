use std::{
    error::Error,
    time::{Duration, Instant},
};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::warn;
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc},
};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;
use wise_api::messages::{ClientWsMessage, ClientWsRequest, ServerWsMessage, ServerWsResponse};

use crate::RawWsClient;

#[derive(Debug)]
pub struct WsTransceiver {
    /// Messages being sent to the server.
    to_server: mpsc::Sender<ClientWsMessage>,

    /// Messages being received from the server.
    to_client: broadcast::Receiver<ServerWsMessage>,
}

impl Clone for WsTransceiver {
    fn clone(&self) -> Self {
        Self {
            to_server: self.to_server.clone(),
            to_client: self.to_client.resubscribe(),
        }
    }
}

impl WsTransceiver {
    /// Create a new connection based on the
    pub fn new(raw: RawWsClient) -> Self {
        let (to_server_tx, to_server_rx) = mpsc::channel(10);
        let (to_client_tx, to_client_rx) = broadcast::channel(10);
        let (sink, stream) = raw.split();

        _ = tokio::spawn(async move {
            _ = to_server_loop(sink, to_server_rx).await;
        });

        _ = tokio::spawn(async move {
            _ = to_client_loop(stream, to_client_tx).await;
        });

        Self {
            to_server: to_server_tx,
            to_client: to_client_rx,
        }
    }

    pub async fn send(&self, message: ClientWsMessage) {
        self.to_server
            .send(message)
            .await
            .expect("Failed to send message");
    }

    pub async fn request(&mut self, request: ClientWsRequest) -> Option<ServerWsResponse> {
        let id = Uuid::new_v4().to_string();
        let message = ClientWsMessage::Request {
            id: id.clone(),
            value: request,
        };

        self.send(message).await;
        let end = Instant::now().checked_add(Duration::from_secs(60)).unwrap();

        while Instant::now() < end {
            let message = self.recv().await;
            let ServerWsMessage::Response {
                id: response_id,
                value,
            } = message
            else {
                continue;
            };

            if response_id != id {
                continue;
            }

            return Some(value);
        }

        None
    }

    pub async fn recv(&mut self) -> ServerWsMessage {
        self.to_client
            .recv()
            .await
            .expect("Failed to receive message")
    }
}

/// Continously receive client messages from the receiver and send them to the server.
async fn to_server_loop(
    mut sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut to_server: mpsc::Receiver<ClientWsMessage>,
) -> Result<(), Box<dyn Error>> {
    while let Some(message) = to_server.recv().await {
        let json = match serde_json::to_string(&message) {
            Ok(json) => json,
            Err(err) => {
                warn!("Failed to serialize client message: {}", err);
                continue;
            }
        };

        sink.send(Message::text(json)).await?;
    }

    Ok(())
}

/// Continously receive server messages from the stream and send them to the client.
async fn to_client_loop(
    mut stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    to_client: broadcast::Sender<ServerWsMessage>,
) -> Result<(), Box<dyn Error>> {
    while let Some(Ok(ws_message)) = stream.next().await {
        let text = match ws_message {
            Message::Text(json) => json,
            _ => {
                warn!("Server sent non text message");
                continue;
            }
        };

        let message = match serde_json::from_str::<ServerWsMessage>(&text) {
            Ok(o) => o,
            Err(e) => {
                warn!("Failed to parse server message: {}", e);
                continue;
            }
        };

        to_client.send(message)?;
    }

    Ok(())
}
