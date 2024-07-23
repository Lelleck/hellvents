use std::error::Error;

use clap::Parser;
use log::{debug, info};
use wise_api::{
    events::RconEvent,
    messages::{ClientWsRequest, CommandRequestKind, ServerWsMessage},
    rcon::parsing::{
        showlog::{LogKind, LogLine},
        Player,
    },
};

use super::command::{HellventCommand, StartEvent};
use crate::{
    client::WsTransceiver,
    config::FileConfig,
    events::{
        melee_mania::{MeleeMania, MeleeManiaConfig},
        notify_players::NotifyPlayers,
        Event,
    },
    manage::command::ChatSubcommand,
};

const COMMAND_PREFIX: &str = "/";

pub struct CommandListener {
    config: FileConfig,
    transceiver: WsTransceiver,
    event: Option<Box<dyn Event>>,
}

impl CommandListener {
    pub fn new(config: FileConfig, transceiver: WsTransceiver) -> Self {
        Self {
            config,
            transceiver,
            event: None,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        info!("Listening for commands");
        loop {
            let message = self.transceiver.receive().await;
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

        let parsed = parse_command(content);
        let command = match parsed {
            Ok(Some(cmd)) => cmd,
            Ok(None) => return,
            Err(e) => {
                self.send_error_reply(e, &sender).await;
                return;
            }
        };

        match command.sub_command {
            ChatSubcommand::Start { event } => self.start_event(event),
            ChatSubcommand::End => self.stop_event(),
        }
    }

    async fn send_error_reply(&mut self, error: clap::Error, sender: &Player) {
        let rendered = error.render();
        let command = format!("Message {} {}", sender.id.to_string(), rendered.to_string());
        self.transceiver
            .request(ClientWsRequest::Execute(CommandRequestKind::Raw {
                command,
                long_response: false,
            }))
            .await;
        return;
    }

    fn start_event(&mut self, event: StartEvent) {
        if let Some(current_event) = self.event.take() {
            info!("Stopping current before starting new");
            current_event.stop();
        }

        let event: Box<dyn Event> = match &event {
            StartEvent::MeleeMania {
                duration: _,
                delay: _,
            } => Box::new(MeleeMania::new(
                MeleeManiaConfig::from_config(&event),
                self.transceiver.clone(),
            )),
            StartEvent::NotifyPlayers => Box::new(NotifyPlayers::new(self.transceiver.clone())),
        };

        event.start();
        self.event = Some(event);
    }

    fn stop_event(&mut self) {
        let Some(event) = self.event.take() else {
            return;
        };

        event.stop();
    }
}

fn parse_command(content: String) -> Result<Option<HellventCommand>, clap::Error> {
    let mut split = content.split(" ");
    let valid_commands = vec!["/hellvents", "/hv"];
    if !valid_commands.contains(&split.nth(0).unwrap_or("")) {
        return Ok(None);
    }
    let mut modified_args = vec!["hellvents"];
    modified_args.extend(split);

    HellventCommand::try_parse_from(modified_args).map(|o| Some(o))
}
