use clap::{Parser, Subcommand};

use crate::events::melee_mania::MeleeManiaConfig;

#[derive(Parser)]
#[command(version, about)]
pub struct HellventCommand {
    #[command(subcommand)]
    pub sub_command: ChatSubcommand,
}

#[derive(Subcommand)]
pub enum ChatSubcommand {
    /// Start an event.
    #[clap(aliases = ["s"])]
    Start {
        #[command(subcommand)]
        event: StartEvent,
    },

    /// End the current event.
    #[clap(aliases = ["e"])]
    End,

    /// Immediately stop the hellvents application.
    #[clap(aliases = ["eexit"])]
    EmergencyExit,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum StartEvent {
    #[clap(aliases = ["mm"])]
    MeleeMania(MeleeManiaConfig),

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
