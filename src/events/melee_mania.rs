use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use log::{debug, info};
use tokio::{
    sync::Mutex,
    time::{sleep, sleep_until},
};
use tokio_util::sync::CancellationToken;
use wise_api::{
    events::RconEvent,
    messages::{CommandRequestKind, ServerWsMessage},
    rcon::parsing::{
        showlog::{LogKind, LogLine},
        Player, PlayerId,
    },
};

use crate::{
    client::{broadcast_message, send_message, WsTransceiver},
    manage::command::StartEvent,
    messages::melee_mania::*,
};

use super::Event;

#[derive(Debug, Clone)]
pub struct MeleeManiaConfig {
    duration: Duration,
    delay: Duration,
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

impl MeleeManiaConfig {
    pub fn from_config(config: &StartEvent) -> Self {
        #[allow(irrefutable_let_patterns)]
        let StartEvent::MeleeMania { duration, delay }: &StartEvent = config
        else {
            panic!("Tried to build config from invalid start event");
        };

        Self {
            duration: duration.clone(),
            delay: delay.clone(),
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

    fn is_stopped(&self) -> bool {
        self.token.is_cancelled()
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
        broadcast_message(&mut self.transceiver, announce_info).await;
        tokio::select! {
            _ = sleep(self.config.delay) => {},
            _ = self.token.cancelled() => return,
        };
        debug!("Broadcasting start message");
        broadcast_message(&mut self.transceiver, announce_start).await;

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
        broadcast_message(&mut self.transceiver, announce_end).await;
    }

    async fn handle_rcon_event(&mut self, rcon_event: RconEvent) {
        match rcon_event {
            RconEvent::Log(LogLine { timestamp: _, kind }) => self.handle_kill(&kind).await,
            _ => return,
        }
    }

    async fn handle_kill(&mut self, log: &LogKind) {
        if let LogKind::Connect {
            player,
            connect: true,
        } = log
        {
            let message = running_message(&self.end.duration_since(Instant::now()));
            send_message(&mut self.transceiver, &player.id, &message).await;
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
        let message = match self {
            PenaltyKind::Punish => format!(
                "\"Your kill with {} violated the melee only rule. You may only use your melee weapon during this event.\"",
                ctx.weapon
            ),
            PenaltyKind::Kick => format!(
                "\"Your kill with {} violated the melee only rule. Due to previous infractions you have been kicked.\"",
                ctx.weapon
            ),
        };

        let penalty_command = match self {
            PenaltyKind::Punish => format!("Punish {} {}", ctx.killer.name, message),
            PenaltyKind::Kick => format!("Kick {} {}", ctx.killer.name, message),
        };

        let info_text = match self {
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

        transceiver
            .execute(CommandRequestKind::Raw {
                command: penalty_command,
                long_response: false,
            })
            .await;
        send_message(transceiver, &ctx.victim.id, &info_text).await;
    }
}

fn is_weapon_melee(name: &str) -> bool {
    let lower = name.to_lowercase();
    let keywords = vec!["knife", "shovel", "spaten", "spade", "sykes"];

    keywords.iter().any(|word| lower.contains(word))
}
