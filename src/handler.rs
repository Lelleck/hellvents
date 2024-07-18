use clap::Parser;
use log::debug;
use wise_api::{
    events::RconEvent,
    messages::{ClientWsRequest, CommandRequestKind, ServerWsMessage},
    rcon::parsing::{
        showlog::{LogKind, LogLine},
        Player,
    },
};

use crate::{client::WsTransceiver, command::ChatCommand, config::FileConfig, RawWsClient};
use std::error::Error;

const COMMAND_PREFIX: &str = "/";

#[derive(Debug)]
pub struct Handler {
    config: FileConfig,
    transceiver: WsTransceiver,
}

impl Handler {
    pub fn new(config: FileConfig, client: RawWsClient) -> Self {
        let transceiver = WsTransceiver::new(client);

        Self {
            config,
            transceiver,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let message = self.transceiver.recv().await;
            self.server_message(message).await;
        }

        // Handle wise shutting off... Reattempt every few minutes
    }

    async fn server_message(&mut self, message: ServerWsMessage) {
        match message {
            // Handle a chat message.
            ServerWsMessage::Rcon(RconEvent::Log(LogLine {
                kind: LogKind::Chat {
                    sender, content, ..
                },
                ..
            })) => self.chat_log(sender, content).await,
            ServerWsMessage::Authenticated => todo!(),
            _ => {}
        }
    }

    async fn chat_log(&mut self, sender: Player, content: String) {
        if !self
            .config
            .admin
            .allowed_ids
            .contains(&sender.id.to_string())
        {
            return;
        }

        if !content.starts_with(COMMAND_PREFIX) {
            return;
        }

        debug!("Received command from {:?} as \"{}\"", sender, content);

        let mut split = content.split(" ");
        let valid_commands = vec!["/hellvents", "/hv"];
        if !valid_commands.contains(&split.nth(0).unwrap_or("")) {
            return;
        }
        let mut modified_args = vec!["hellvents"];
        modified_args.extend(split);

        let parsed = ChatCommand::try_parse_from(modified_args);

        let command = match parsed {
            Ok(o) => o,
            Err(e) => {
                let rendered = e.render();
                let command = format!("Message {} {}", sender.id.to_string(), rendered.to_string());
                self.transceiver
                    .request(ClientWsRequest::Execute(CommandRequestKind::Raw {
                        command,
                        long_response: false,
                    }))
                    .await;
                return;
            }
        };

        dbg!(command);
    }
}
