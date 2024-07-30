use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use clap::Args;
use log::{debug, info};
use tokio::{
    sync::Mutex,
    time::{sleep, sleep_until},
};
use tokio_util::sync::CancellationToken;
use wise_api::{
    events::RconEvent,
    messages::ServerWsMessage,
    rcon::parsing::{
        showlog::{LogKind, LogLine},
        Player, PlayerId,
    },
};

use crate::{
    client::{WsTransceiver, WsTransceiverExt},
    messages::melee_mania::*,
};

use super::Event;

#[derive(Debug, Clone, Args, PartialEq, Eq)]
pub struct MeleeManiaConfig {
    /// The amount of time the starting of the event should be delayed from the annoucement.
    #[clap(default_value = "2m", value_parser = humantime::parse_duration)]
    delay: Duration,

    /// The time for which the mini game should last.
    #[clap(default_value = "5m", value_parser = humantime::parse_duration)]
    duration: Duration,
}

const FIVE_MINUTES: u64 = 60 * 5;
const TWO_MINUTES: u64 = 60 * 2 + 10;
impl Default for MeleeManiaConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(FIVE_MINUTES),
            delay: Duration::from_secs(TWO_MINUTES),
        }
    }
}

#[derive(Clone)]
pub struct MeleeMania {
    infractions: Arc<Mutex<HashMap<PlayerId, i32>>>,
    end: Instant,
    config: Arc<MeleeManiaConfig>,
    token: CancellationToken,
    transceiver: WsTransceiver,
}

struct PenaltyContext {
    killer: Player,
    victim: Player,
    weapon: String,
}

impl PenaltyContext {
    fn new(killer: Player, victim: Player, weapon: String) -> Self {
        Self {
            killer,
            victim,
            weapon,
        }
    }
}

impl Event for MeleeMania {
    fn start(&self) {
        let clone = self.clone();
        _ = tokio::spawn(clone.run());
    }

    fn stop(&self) {
        self.token.cancel();
    }
}

impl MeleeMania {
    pub fn new(config: MeleeManiaConfig, transceiver: WsTransceiver) -> Self {
        Self {
            infractions: Default::default(),
            end: Instant::now()
                .checked_add(config.duration + config.delay)
                .unwrap(),
            config: Arc::new(config),
            token: Default::default(),
            transceiver,
        }
    }

    async fn run(mut self) {
        info!("Starting Melee Mania with config {:?}", self.config);

        let announce_info = info_message(&self.config.delay, &self.config.duration);
        let announce_start = start_message(&self.config.duration);
        let announce_end = end_message();

        debug!("Broadcasting info message");
        self.transceiver.broadcast_message(&announce_info).await;
        tokio::select! {
            _ = sleep(self.config.delay) => {},
            _ = self.token.cancelled() => return,
        };
        debug!("Broadcasting start message");
        self.transceiver.broadcast_message(&announce_start).await;

        self.transceiver.clear().await;
        info!(
            "Enforcing Melee Mania for {}",
            humantime::format_duration(self.config.duration)
        );

        loop {
            tokio::select! {
                _ = self.token.cancelled() => {
                    info!("Cancellation detected -> Stopping");
                    break;
                }
                _ = sleep_until(self.end.into()) => {
                    info!("Reached end of Melee Mania -> Stopping");
                    break;
                }
                message = self.transceiver.receive() => {
                    let ServerWsMessage::Rcon(rcon_event) = message else {
                        continue;
                    };

                    self.handle_rcon_event(rcon_event).await;
                }
            }

            debug_assert!(
                Arc::strong_count(&self.infractions) != 1,
                "Orphaned thread detected"
            );
        }

        self.token.cancel();
        debug!("Broadcasting end message");
        self.transceiver.broadcast_message(&announce_end).await;
    }

    async fn handle_rcon_event(&mut self, rcon_event: RconEvent) {
        match rcon_event {
            RconEvent::Log(LogLine { timestamp: _, kind }) => self.handle_log(&kind).await,
            _ => return,
        }
    }

    async fn handle_log(&mut self, log: &LogKind) {
        if let LogKind::Connect {
            player,
            connect: true,
        } = log
        {
            let message = running_message(&self.end.duration_since(Instant::now()));
            self.transceiver.message_player(&player.id, &message).await;
            return;
        }

        let LogKind::Kill {
            killer,
            killer_faction: _,
            victim,
            victim_faction: _,
            is_teamkill: _,
            weapon,
        } = log
        else {
            return;
        };

        if is_weapon_melee(&weapon) {
            debug!("Not punishing {:?} for the use of {}", &killer.id, weapon);
            return;
        }

        let ctx = PenaltyContext::new(killer.clone(), victim.clone(), weapon.clone());
        self.calculate_penalty(&killer.id)
            .await
            .execute(&ctx, &mut self.transceiver)
            .await;
    }

    async fn calculate_penalty(&mut self, id: &PlayerId) -> PenaltyKind {
        let mut guard = self.infractions.lock().await;
        if !guard.contains_key(id) {
            guard.insert(id.clone(), 0);
        }

        let count = guard.get_mut(id).unwrap();
        *count += 1;

        match count {
            1 | 2 => PenaltyKind::Punish,
            _ => PenaltyKind::Kick,
        }
    }
}

#[derive(Debug)]
enum PenaltyKind {
    Punish,
    Kick,
}

impl PenaltyKind {
    pub async fn execute(&self, ctx: &PenaltyContext, transceiver: &mut WsTransceiver) {
        let killer_text = match self {
            PenaltyKind::Punish => format!(
                "\"Your kill with {} violated the melee only rule. You may only use your melee weapon during this event.\"",
                ctx.weapon
            ),
            PenaltyKind::Kick => format!(
                "\"Your kill with {} violated the melee only rule. Due to previous infractions you have been kicked.\"",
                ctx.weapon
            ),
        };

        let victim_text = match self {
            PenaltyKind::Punish => format!(
                "Your killer {} has been redeployed for killing you with {}.",
                ctx.killer.name, ctx.weapon
            ),
            PenaltyKind::Kick => format!(
                "Your killer {} has been kicked for killing you with {}.",
                ctx.killer.name, ctx.weapon
            ),
        };

        debug!(
            "Enforcing penalty {:?} for {:?} for the use of {}",
            self, &ctx.killer, &ctx.weapon
        );

        match self {
            PenaltyKind::Punish => transceiver.punish_player(&ctx.killer.name, &killer_text),
            PenaltyKind::Kick => transceiver.kick_player(&ctx.killer.name, &killer_text),
        }
        .await;

        transceiver
            .message_player(&ctx.victim.id, &victim_text)
            .await;
    }
}

fn is_weapon_melee(name: &str) -> bool {
    let lower = name.to_lowercase();
    let keywords = vec!["knife", "shovel", "spaten", "spade", "sykes"];

    keywords.iter().any(|word| lower.contains(word))
}
