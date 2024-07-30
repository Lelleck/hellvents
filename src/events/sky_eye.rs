use std::{collections::HashMap, sync::Arc};

use clap::Args;
use rand::{seq::IteratorRandom, thread_rng};
use tokio::{sync::Mutex, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use wise_api::rcon::parsing::{Player, PlayerId};

use crate::{client::WsTransceiver, utils::get_players_with_team};

use super::EventHandle;

#[derive(Debug, Clone, Args, PartialEq, Eq)]
pub struct SkyEyeConfig {
    /// How many players are supposed to be given admin cam access per team.
    #[clap(default_value = "5")]
    cam_count: usize,
}

pub struct SkyEyeHandle {
    token: CancellationToken,
    join_handle: JoinHandle<()>,
}

impl EventHandle for SkyEyeHandle {
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
pub struct SkyEye {
    transceiver: WsTransceiver,
    _token: CancellationToken,
    config: Arc<SkyEyeConfig>,
    #[allow(dead_code)]
    camers: Arc<Mutex<Vec<PlayerId>>>,
}

impl SkyEye {
    pub fn new(config: SkyEyeConfig, transceiver: WsTransceiver) -> Box<dyn EventHandle> {
        let token = CancellationToken::new();
        let event = Self {
            transceiver,
            _token: token.clone(),
            config: Arc::new(config),
            camers: Arc::default(),
        };

        let join_handle = tokio::spawn(event.run());
        Box::new(SkyEyeHandle { token, join_handle })
    }

    pub async fn run(mut self) {
        let player_teams = get_players_with_team(&mut self.transceiver).await;
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
