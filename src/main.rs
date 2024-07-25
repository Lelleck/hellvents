pub mod client;
pub mod config;
pub mod events;
pub mod manage;
pub mod messages;
pub mod utils;

use client::WsTransceiver;
pub use config::{parse_config, FileConfig};
use log::debug;
use manage::listener::CommandListener;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Off)
        .filter_module("hellvents", log::LevelFilter::Trace)
        .init();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install default crypto provider");

    let config = parse_config()?;
    debug!("Initialized file config");
    let transceiver = WsTransceiver::connect(&config.wise).await?;
    debug!("Succesfully connected to wise");

    let mut handler = CommandListener::new(config, transceiver);
    handler.run().await
}
