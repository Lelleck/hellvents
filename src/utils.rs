use futures_util::future::join_all;
use wise_api::rcon::parsing::Player;

use crate::client::{WsTransceiver, WsTransceiverExt};

pub async fn get_players_with_team(transceiver: &mut WsTransceiver) -> Vec<(Player, String)> {
    // Get all players
    // For each player request the player info
    let players = transceiver.get_players().await.unwrap();

    let player_infos = join_all(players.iter().map(|player| {
        let mut clone = transceiver.clone();
        async move {
            clone.get_playerinfo(player.name.clone()).await
        }
    }))
    .await;

    players
        .into_iter()
        .zip(player_infos.into_iter())
        .filter_map(|(player, player_info)| {
            if let Some(player_info) = player_info {
                Some((player, player_info.team))
            } else {
                None
            }
        })
        .collect()
}
