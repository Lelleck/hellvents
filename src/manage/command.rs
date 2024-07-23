use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about)]
pub struct HellventCommand {
    #[command(subcommand)]
    pub sub_command: ChatSubcommand,
}

#[derive(Subcommand)]
pub enum ChatSubcommand {
    // Status,
    Start {
        #[command(subcommand)]
        event: StartEvent,
    },
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum StartEvent {
    MeleeMania {
        /// The amount of time the starting of the event should be delayed from the annoucement.
        #[clap(default_value = "2m", value_parser = humantime::parse_duration)]
        delay: Duration,

        /// The time for which the mini game should last.
        #[clap(default_value = "5m", value_parser = humantime::parse_duration)]
        duration: Duration,
    },
    NotifyPlayers,
}
