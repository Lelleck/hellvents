/*
CONCEPT
Allow players to communicate via text across team borders.

KILLS
On kill both killer and victim are informed of each other.
A short lived "aetherial" channel is opened between the two.
The victim can send a message to the killer.

CHAT
Messages sent in ALL chat are transmitted to the other side in 30s intervals.
Messages sent in UNIT chat are transmitted to the corresponding unit in 10s intervals.
*/

use std::{
    collections::HashMap,
    fmt::Display,
    str::FromStr,
    time::{Duration, Instant},
};

use derive_new::new;
use log::{debug, info};
use tokio::{
    task::JoinHandle,
    time::{self},
};
use tokio_util::sync::CancellationToken;
use wise_api::{
    events::RconEvent,
    messages::ServerWsMessage,
    rcon::parsing::showlog::{LogKind, LogLine},
};

use crate::{
    client::{WsTransceiver, WsTransceiverExt},
    utils::get_players_with_team,
};

use super::EventHandle;

pub struct RadioSpiesHandle {
    token: CancellationToken,
    join_handle: JoinHandle<()>,
}

impl EventHandle for RadioSpiesHandle {
    fn stop(&self) {
        self.token.cancel();
    }

    fn abort(&self) {
        self.join_handle.abort();
    }

    fn short_info(&self) -> String {
        "".to_string()
    }

    fn long_info(&self) -> String {
        "".to_string()
    }
}

#[derive(Clone)]
pub struct RadioSpies {
    transceiver: WsTransceiver,
    token: CancellationToken,
    messages: HashMap<String, Vec<CachedMessage>>,
}

/*
impl Event for RadioSpies {
    fn start(&self) {
        let clone = self.clone();
        _ = tokio::spawn(clone.run());
    }

    fn stop(&self) {
        self.ctx.token.cancel();
    }
}
*/

impl RadioSpies {
    pub fn new(transceiver: WsTransceiver) -> Box<dyn EventHandle> {
        let token = CancellationToken::new();
        let event = Self {
            transceiver,
            token: token.clone(),
            messages: HashMap::new(),
        };

        let join_handle = tokio::spawn(event.run());
        Box::new(RadioSpiesHandle { token, join_handle })
    }

    async fn run(mut self) {
        info!("Enforcing Radio Spies");
        let mut interval = time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        interval.reset();

        loop {
            tokio::select! {
                _ = self.token.cancelled() => {
                    info!("Cancellation detected -> Stopping");
                    return;
                }

                _ = interval.tick() => {
                    self.flush_cached_messages().await;
                }

                message = self.transceiver.receive() => {
                    let ServerWsMessage::Rcon(event) = message else {
                        continue;
                    };

                    self.handle_rcon_event(event).await;
                }
            }
        }
    }

    async fn flush_cached_messages(&mut self) {
        let mut team_messages = HashMap::new();
        for (team, messages) in &self.messages {
            team_messages.insert(team, build_collected_message(messages));
        }

        let player_teams = get_players_with_team(&mut self.transceiver).await;
        for (player, team) in player_teams {
            let Some(opposite_team) = opposite_team(&team) else {
                continue;
            };

            let Some(message) = team_messages.get(&opposite_team.to_string()) else {
                continue;
            };

            self.transceiver.message_player(&player.id, &message).await;
        }

        self.messages.clear();
        // self.messages.insert("Allies".to_string(), Vec::new());
        // self.messages.insert("Axis".to_string(), Vec::new());
        debug!("Flushed cached messages");
    }

    async fn handle_rcon_event(&mut self, event: RconEvent) {
        let RconEvent::Log(LogLine {
            timestamp: _,
            kind:
                LogKind::Chat {
                    sender,
                    team,
                    reach: _,
                    content,
                },
        }) = event
        else {
            return;
        };

        let cached = CachedMessage::new(Instant::now(), sender.name, content);
        self.messages
            .entry(team)
            .or_insert_with(Vec::new)
            .push(cached.clone());
        debug!("Cached message {:?}", cached);
    }
}

#[derive(new, Clone, Debug)]
struct CachedMessage {
    time: Instant,
    sender: String,
    content: String,
}

impl Display for CachedMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let now = self.time.elapsed();
        let secs_only = Duration::from_secs(now.as_secs());

        let string = format!(
            "[{} ago] {}: {}",
            humantime::format_duration(secs_only),
            self.sender,
            self.content
        );
        f.write_str(&string)
    }
}

fn build_collected_message(messages: &Vec<CachedMessage>) -> String {
    /*
    if messages.is_empty() {
        return "Intelligence has failed to gather any messages. It is upon you to tempt them to reveal vital information!".to_string();
    }
    */

    let base =
        String::from_str("Intelligence has gathered the following messages\n\n---\n\n").unwrap();

    messages
        .iter()
        .rev()
        .fold(base, |base, m| format!("{}\n{}", base, m))
}

fn opposite_team(team_in: &str) -> Option<&'static str> {
    match team_in {
        "Allies" => Some("Axis"),
        "Axis" => Some("Allies"),
        _ => None,
    }
}
