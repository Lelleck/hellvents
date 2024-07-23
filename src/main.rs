pub mod client;
pub mod config;
pub mod events;
pub mod manage;

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
    run_manager(config, transceiver).await
}

async fn run_manager(config: FileConfig, transceiver: WsTransceiver) -> Result<(), Box<dyn Error>> {
    let mut handler = CommandListener::new(config, transceiver);
    handler.run().await?;
    Ok(())
}
