use clap::{Parser, Subcommand};

use crate::events::{melee_mania::MeleeManiaConfig, sky_eye::SkyEyeConfig, EventKind};

#[derive(Parser)]
#[command(version, about)]
pub struct HellventCommand {
    #[command(subcommand)]
    pub sub_command: ChatSubcommand,
}

#[derive(Subcommand)]
pub enum ChatSubcommand {
    /// Start an event.
    Start {
        #[command(subcommand)]
        event: StartEvent,
    },

    /// End the current event.
    Stop { kind: EventKind },

    /// Stop all ongoing events.
    StopAll,

    /// See the current status of all or one ongoing event.
    Status { kind: Option<EventKind> },

    /// Immediately stop the hellvents application.
    #[clap(aliases = ["eexit"])]
    EmergencyExit,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum StartEvent {
    #[clap(aliases = ["mm"])]
    MeleeMania(MeleeManiaConfig),

    #[clap(aliases = ["se"])]
    SkyEye(SkyEyeConfig),

    #[clap(aliases = ["rs"])]
    RadioSpies,
}
