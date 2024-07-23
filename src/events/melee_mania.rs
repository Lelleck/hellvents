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

struct PenaltyContext(Player, String);

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

        let announce_info = format!(
            "HELLVENTS | INFO\n
The mini game MELEE MANIA will start in {}. \
For a period of {}, only melee weapons will be allowed.\n
You will receive a message when the mini game has started and ended.\n
Invalid kills result in penalities!",
            humantime::format_duration(self.config.delay),
            humantime::format_duration(self.config.duration)
        );

        let announce_start = format!(
            "HELLVENTS | START\n
The mini game MELEE MANIA has started. \
For a period of {}, only melee weapons will be allowed.\n
You will receive a message when the mini game has ended.\n
Invalid kills result in penalities!",
            humantime::format_duration(self.config.duration)
        );

        let announce_end = format!(
            "HELLVENTS | END\n
The mini game MELEE MANIA has ended.\n
Thanks for participating."
        );

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
            let message = format!(
                "HELLVENTS | RUNNING\n\nThe mini game MELEE MANIA is currently running.\n
For a period of {}, only melee weapons will be allowed.\n
You will receive a message when the mini game has ended.\n
Invalid kills result in penalities!",
                humantime::format_duration(self.end.duration_since(Instant::now()))
            );
            send_message(&mut self.transceiver, &player.id, &message).await;
            return;
        }

        let LogKind::Kill {
            killer,
            killer_faction: _,
            victim: _,
            victim_faction: _,
            is_teamkill: _,
            weapon,
        } = log
        else {
            return;
        };

        if is_weapon_melee(&weapon) {
            return;
        }

        let ctx = PenaltyContext(killer.clone(), weapon.clone());
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
                ctx.1
            ),
            PenaltyKind::Kick => format!(
                "\"Your kill with {} violated the melee only rule. Due to previous infractions you have been kicked.\"",
                ctx.1
            ),
        };

        let command = match self {
            PenaltyKind::Punish => format!("Punish {} {}", ctx.0.name, message),
            PenaltyKind::Kick => format!("Kick {} {}", ctx.0.name, message),
        };

        debug!(
            "Enforcing penalty {:?} for {:?} for the use of {}",
            self, &ctx.0, &ctx.1
        );
        transceiver
            .execute(CommandRequestKind::Raw {
                command,
                long_response: false,
            })
            .await;
    }
}

fn is_weapon_melee(name: &str) -> bool {
    let lower = name.to_lowercase();
    let keywords = vec!["knife", "shovel", "spaten", "spade"]; // TODO:

    keywords.iter().any(|word| lower.contains(word))
}
