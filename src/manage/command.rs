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
    #[clap(aliases = ["s"])]
    Start {
        #[command(subcommand)]
        event: StartEvent,
    },
    #[clap(aliases = ["e"])]
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum StartEvent {
    #[clap(aliases = ["mm"])]
    MeleeMania {
        /// The amount of time the starting of the event should be delayed from the annoucement.
        #[clap(default_value = "2m", value_parser = humantime::parse_duration)]
        delay: Duration,

        /// The time for which the mini game should last.
        #[clap(default_value = "5m", value_parser = humantime::parse_duration)]
        duration: Duration,
    },

    #[clap(skip)]
    // #[clap(aliases = ["se"])]
    SkyEye {
        /// How many players are supposed to be given admin cam access per team.
        #[clap(default_value = "5")]
        cam_count: usize,
    },

    #[clap(aliases = ["rs"])]
    RadioSpies {},
}
