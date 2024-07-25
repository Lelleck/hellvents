use crate::{client::WsTransceiver, manage::command::StartEvent};
use melee_mania::*;

pub mod melee_mania;

pub trait Event {
    fn start(&self);
    fn stop(&self);
    fn is_stopped(&self) -> bool;
}

pub fn build_event(transceiver: WsTransceiver, start: &StartEvent) -> Box<dyn Event> {
    match start {
        StartEvent::MeleeMania {
            duration: _,
            delay: _,
        } => Box::new(MeleeMania::new(
            MeleeManiaConfig::from_config(start),
            transceiver,
        )),
    }
}
