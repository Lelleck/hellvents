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
use tokio::time::{self};
use wise_api::{
    events::RconEvent,
    messages::ServerWsMessage,
    rcon::parsing::showlog::{LogKind, LogLine},
};

use crate::{client::WsTransceiverExt, utils::get_players_with_team};

use super::{Event, EventContext};

#[derive(Clone)]
pub struct RadioSpies {
    ctx: EventContext,
    messages: HashMap<String, Vec<CachedMessage>>,
}

impl Event for RadioSpies {
    fn start(&self) {
        let clone = self.clone();
        _ = tokio::spawn(clone.run());
    }

    fn stop(&self) {
        todo!()
    }
}

impl RadioSpies {
    pub fn new(ctx: EventContext) -> Self {
        Self {
            ctx,
            messages: HashMap::new(),
        }
    }

    async fn run(mut self) {
        info!("Enforcing Radio Spies");
        let mut interval = time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        interval.reset();

        loop {
            tokio::select! {
                _ = self.ctx.token.cancelled() => {
                    info!("Cancellation detected -> Stopping");
                    return;
                }

                _ = interval.tick() => {
                    self.flush_cached_messages().await;
                }

                message = self.ctx.transceiver.receive() => {
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

        let player_teams = get_players_with_team(&mut self.ctx.transceiver).await;
        for (player, team) in player_teams {
            let Some(opposite_team) = opposite_team(&team) else {
                continue;
            };

            let Some(message) = team_messages.get(&opposite_team.to_string()) else {
                continue;
            };

            self.ctx
                .transceiver
                .message_player(&player.id, &message)
                .await;
        }

        self.messages.clear();
        self.messages.insert("Allies".to_string(), Vec::new());
        self.messages.insert("Axis".to_string(), Vec::new());
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
    if messages.is_empty() {
        return "Intelligence has failed to gather any messages. It is upon you to tempt them to reveal vital information!".to_string();
    }

    let mut base =
        String::from_str("Intelligence has gathered the following messages\n---\n\n").unwrap();

    for message in messages.iter().rev() {
        base.push_str(&format!("{}\n", message));
    }

    base
}

fn opposite_team(team_in: &str) -> Option<&'static str> {
    match team_in {
        "Allies" => Some("Axis"),
        "Axis" => Some("Allies"),
        _ => None,
    }
}
