use std::{collections::HashMap, sync::Arc};

use rand::{seq::IteratorRandom, thread_rng};
use tokio::sync::Mutex;
use wise_api::rcon::parsing::{Player, PlayerId};

use crate::{manage::command::StartEvent, utils::get_players_with_team};

use super::{Event, EventContext};

pub struct SkyEyeConfig {
    cam_count: usize,
}

impl SkyEyeConfig {
    pub fn from_config(config: &StartEvent) -> Self {
        let StartEvent::SkyEye { cam_count } = config else {
            panic!("Tried to build config from invalid start event");
        };

        Self {
            cam_count: *cam_count,
        }
    }
}

#[derive(Clone)]
pub struct SkyEye {
    config: Arc<SkyEyeConfig>,
    ctx: EventContext,
    #[allow(dead_code)]
    camers: Arc<Mutex<Vec<PlayerId>>>,
}

impl Event for SkyEye {
    fn start(&self) {
        let clone = self.clone();
        _ = tokio::spawn(clone.run());
    }

    fn stop(&self) {
        todo!()
    }
}

impl SkyEye {
    pub fn new(config: SkyEyeConfig, ctx: EventContext) -> Self {
        Self {
            config: Arc::new(config),
            ctx,
            camers: Arc::default(),
        }
    }

    pub async fn run(mut self) {
        let player_teams = get_players_with_team(&mut self.ctx.transceiver).await;
        let _selected = select_random_players(self.config.cam_count, player_teams);

        loop {}
    }
}

fn select_random_players(
    amount: usize,
    players_with_teams: Vec<(Player, String)>,
) -> Vec<(Player, String)> {
    let mut teams = HashMap::new();

    for (player, team) in players_with_teams {
        if team == "None" {
            continue;
        }

        teams.entry(team).or_insert_with(Vec::new).push(player);
    }

    let mut selected_players = Vec::new();
    let mut rng = thread_rng();

    for (team, players) in teams {
        let selected = players.iter().choose_multiple(&mut rng, amount);

        for player in selected {
            selected_players.push((player.clone(), team.clone()));
        }
    }

    selected_players
}
