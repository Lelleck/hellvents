use std::{
    error::Error,
    time::{Duration, Instant},
};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::{trace, warn};
use serde::Deserialize;
use tokio::{
    net::TcpStream,
    sync::{
        broadcast::{self, error::TryRecvError},
        mpsc,
    },
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;
use wise_api::{
    messages::{
        ClientWsMessage, ClientWsRequest, CommandRequestKind, CommandResponseKind, ServerWsMessage,
        ServerWsResponse,
    },
    rcon::parsing::PlayerId,
};

pub type RawWsClient = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone, Deserialize)]
pub struct ClientConfig {
    pub address: String,
    pub token: String,
}

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
    pub async fn connect(config: &ClientConfig) -> Result<Self, Box<dyn Error>> {
        let (mut ws_client, _) = connect_async(&config.address).await?;
        ws_client
            .send(Message::Text(config.token.to_string()))
            .await?;

        let Some(Ok(message)) = ws_client.next().await else {
            panic!("Server failed to respond to login token... Is the token valid?")
        };

        let server_message =
            serde_json::from_str::<ServerWsMessage>(message.into_text()?.as_str())?;

        if !matches!(server_message, ServerWsMessage::Authenticated) {
            panic!("Server did not respond with immediate Authentication response");
        }

        Ok(Self::from_client(ws_client))
    }

    /// Create a new transceiver from the given raw client.
    pub fn from_client(raw: RawWsClient) -> Self {
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

    /// Send a [`ClientWsMessage`] to the server and return.
    pub async fn send(&self, message: ClientWsMessage) {
        self.to_server
            .send(message)
            .await
            .expect("Failed to send message");
    }

    pub async fn execute(&mut self, command: CommandRequestKind) {
        self.send(ClientWsMessage::Request {
            id: None,
            value: ClientWsRequest::Execute(command),
        })
        .await;
    }

    /// Send a [`ClientWsRequest`] to the server and return the response from the server or [`None`] after 60 seconds.
    pub async fn request(&mut self, request: ClientWsRequest) -> Option<ServerWsResponse> {
        let id = Uuid::new_v4().to_string();
        let message = ClientWsMessage::Request {
            id: Some(id.clone()),
            value: request,
        };

        self.send(message).await;
        let end = Instant::now().checked_add(Duration::from_secs(60)).unwrap();

        while Instant::now() < end {
            let message = self.receive().await;
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

    /// Clear the incoming channel, should always be done to filter out old unnecessary logs.
    pub async fn clear(&mut self) {
        loop {
            let res = self.to_client.try_recv();
            if matches!(res, Err(TryRecvError::Empty)) {
                return;
            }
        }
    }

    /// Receive the next [`ServerWsMessage`].
    pub async fn receive(&mut self) -> ServerWsMessage {
        self.to_client
            .recv()
            .await
            .expect("Failed to receive message")
    }
}

pub async fn send_message(transceiver: &mut WsTransceiver, id: &PlayerId, message: &str) {
    let command = format!("Message {} {}", id.to_string(), message);

    transceiver
        .send(ClientWsMessage::Request {
            id: None,
            value: ClientWsRequest::Execute(CommandRequestKind::Raw {
                command,
                long_response: false,
            }),
        })
        .await;
}

pub async fn broadcast_message(transceiver: &mut WsTransceiver, message: String) {
    let players = transceiver
        .request(ClientWsRequest::Execute(CommandRequestKind::GetPlayerIds))
        .await
        .unwrap();

    let ServerWsResponse::Execute {
        failure: _,
        response: Some(CommandResponseKind::GetPlayerIds(players)),
    } = players
    else {
        return;
    };

    for player in players {
        let command = format!("Message {} {}", player.id.to_string(), message);
        transceiver
            .send(ClientWsMessage::Request {
                id: None,
                value: ClientWsRequest::Execute(CommandRequestKind::Raw {
                    command,
                    long_response: false,
                }),
            })
            .await;
    }
}

/// Continously receive client messages from the receiver and send them to the server.
async fn to_server_loop(
    mut sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut to_server: mpsc::Receiver<ClientWsMessage>,
) -> Result<(), Box<dyn Error>> {
    while let Some(message) = to_server.recv().await {
        trace!("Sending {:?} to server", message);

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
