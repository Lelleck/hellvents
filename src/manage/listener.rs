use std::{collections::HashMap, error::Error, process};

use clap::Parser;
use log::{debug, info};
use wise_api::{
    events::RconEvent,
    messages::ServerWsMessage,
    rcon::parsing::{
        showlog::{LogKind, LogLine},
        Player,
    },
};

use super::command::{HellventCommand, StartEvent};
use crate::{
    client::{WsTransceiver, WsTransceiverExt},
    config::FileConfig,
    events::{build_event, EventHandle, EventKind},
    manage::command::ChatSubcommand,
};

const COMMAND_PREFIX: &str = "/";

pub struct CommandListener {
    config: FileConfig,
    transceiver: WsTransceiver,
    events: HashMap<EventKind, Box<dyn EventHandle>>,
}

pub enum Reply {
    Ok(String),
    Err(String),
    Clap(String),
    Silent,
}

impl Reply {
    fn to_string(self) -> Option<String> {
        match self {
            Reply::Ok(s) => Some(format!("HELLVENTS | OK\n\n{}", s)),
            Reply::Err(s) => Some(format!("HELLVENTS | ERROR\n\n{}", s)),
            Reply::Clap(s) => Some(s),
            Reply::Silent => None,
        }
    }
}

impl CommandListener {
    pub fn new(config: FileConfig, transceiver: WsTransceiver) -> Self {
        Self {
            config,
            transceiver,
            events: HashMap::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        info!("Listening for commands");
        loop {
            let message = self.transceiver.receive().await;
            self.handle_server_message(message).await;
        }
    }

    /// Handle a generic server message.
    async fn handle_server_message(&mut self, message: ServerWsMessage) {
        let ServerWsMessage::Rcon(RconEvent::Log(LogLine {
            kind: LogKind::Chat {
                sender, content, ..
            },
            ..
        })) = message
        else {
            return;
        };

        match self.handle_chat(&sender, &content).await.to_string() {
            None => {}
            Some(s) => self.transceiver.message_player(&sender.id, &s).await,
        }
    }

    /// Handle a chat message and check whether it is a command to be executed.
    async fn handle_chat(&mut self, sender: &Player, content: &str) -> Reply {
        if !self
            .config
            .admin
            .allowed_ids
            .contains(&sender.id.to_string())
        {
            return Reply::Silent;
        }

        if !content.starts_with(COMMAND_PREFIX) {
            return Reply::Silent;
        }

        let parsed = parse_command(content);
        let command = match parsed {
            Ok(Some(cmd)) => cmd,
            Ok(None) => return Reply::Silent, // the command was no command at all
            Err(e) => {
                return Reply::Clap(e.render().to_string());
            }
        };

        debug!("Received command from {:?} as \"{}\"", sender, content);

        match command.sub_command {
            ChatSubcommand::Start { event } => self.start_event(event),
            ChatSubcommand::Stop { kind } => self.stop_event(kind),
            ChatSubcommand::StopAll => Reply::Err("Not implemented".to_string()),
            ChatSubcommand::Status { kind: _ } => Reply::Err("Not implemented".to_string()),
            ChatSubcommand::EmergencyExit => process::exit(1),
        }
    }

    /// Start an event from the given [`StartEvent`] command and abort any old events.
    fn start_event(&mut self, event: StartEvent) -> Reply {
        let (kind, handle) = build_event(self.transceiver.clone(), &event);
        let Some(old_handle) = self.events.insert(kind.clone(), handle) else {
            return Reply::Ok(format!("Started event {:?}.", event));
        };

        info!("Aborting old event {:?}", kind);
        old_handle.abort();
        Reply::Ok(format!(
            "Aborted previous event of the same type and started event {:?}.",
            event
        ))
    }

    /// Stop the given event if present.
    fn stop_event(&mut self, kind: EventKind) -> Reply {
        let Some(event) = self.events.remove(&kind) else {
            return Reply::Err(format!("No event of type {:?} is currently running", kind));
        };

        event.stop();
        Reply::Ok(format!("Stopped event {:?}", kind))
    }
}

fn parse_command(content: &str) -> Result<Option<HellventCommand>, clap::Error> {
    let mut split = content.split(" ");
    let valid_commands = vec!["/hellvents", "/hv"];
    if !valid_commands.contains(&split.nth(0).unwrap_or("")) {
        return Ok(None);
    }
    let mut modified_args = vec!["hellvents"];
    modified_args.extend(split);

    HellventCommand::try_parse_from(modified_args).map(|o| Some(o))
}
