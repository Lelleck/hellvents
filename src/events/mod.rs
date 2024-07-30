use crate::{client::WsTransceiver, manage::command::StartEvent};
use clap::ValueEnum;
use derive_new::new;
use melee_mania::*;
use radio_spies::RadioSpies;
use sky_eye::*;
use tokio::{sync::watch, task::JoinHandle};
use tokio_util::sync::CancellationToken;

pub mod melee_mania;
pub mod radio_spies;
pub mod sky_eye;

#[derive(Debug, ValueEnum, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum EventKind {
    MeleeMania,
    SkyEye,
    RadioSpies,
}

pub trait EventHandle: Send {
    /// Stop the underlying event.
    fn stop(&self);

    /// Forcefully abort the underlying event.
    fn abort(&self);

    /// Generate a short information about the event.
    fn short_info(&self) -> String;

    /// Generate a long information about the event.
    fn long_info(&self) -> String;
}

#[derive(new)]
pub struct GenericEventHandle {
    token: CancellationToken,
    join_handle: JoinHandle<()>,
    info_watch: watch::Receiver<(String, String)>,
}

impl EventHandle for GenericEventHandle {
    fn stop(&self) {
        self.token.cancel();
    }

    fn abort(&self) {
        self.join_handle.abort();
    }

    fn short_info(&self) -> String {
        self.info_watch.borrow().0.clone()
    }

    fn long_info(&self) -> String {
        self.info_watch.borrow().1.clone()
    }
}

pub fn build_event(
    transceiver: WsTransceiver,
    start: &StartEvent,
) -> (EventKind, Box<dyn EventHandle>) {
    match start {
        StartEvent::MeleeMania(config) => (
            EventKind::MeleeMania,
            MeleeMania::new(config.clone(), transceiver),
        ),
        StartEvent::SkyEye(config) => (EventKind::SkyEye, SkyEye::new(config.clone(), transceiver)),
        StartEvent::RadioSpies {} => (EventKind::RadioSpies, RadioSpies::new(transceiver)),
    }
}
