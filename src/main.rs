pub mod client;
pub mod command;
pub mod config;
pub mod handler;

use config::{parse_config, FileConfig};
use futures_util::{SinkExt, StreamExt};
use handler::Handler;
use log::info;
use std::error::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use wise_api::messages::ServerWsMessage;

pub type RawWsClient = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install default crypto provider");

    let config = parse_config()?;
    info!("Intialized file config");
    let client = connect_to_wise(&config).await?;
    info!("Succesfully connected to wise");

    run_client(config, client).await
}

async fn connect_to_wise(config: &FileConfig) -> Result<RawWsClient, Box<dyn Error>> {
    let (mut ws_client, _) = connect_async(&config.wise.address).await?;
    ws_client
        .send(Message::Text(config.wise.token.to_string()))
        .await?;

    let Some(Ok(message)) = ws_client.next().await else {
        panic!("Server failed to respond to login token... Is the token valid?")
    };

    let server_message = serde_json::from_str::<ServerWsMessage>(message.into_text()?.as_str())?;

    if !matches!(server_message, ServerWsMessage::Authenticated) {
        panic!("Server did not respond with immediate Authentication response");
    }

    Ok(ws_client)
}

async fn run_client(config: FileConfig, client: RawWsClient) -> Result<(), Box<dyn Error>> {
    let mut handler = Handler::new(config, client);
    handler.run().await?;
    Ok(())
}
