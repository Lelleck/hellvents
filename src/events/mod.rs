use crate::{client::WsTransceiver, manage::command::StartEvent};
use derive_new::new;
use melee_mania::*;
use radio_spies::RadioSpies;
use sky_eye::*;
use tokio_util::sync::CancellationToken;

mod melee_mania;
mod radio_spies;
mod sky_eye;

pub trait Event {
    fn start(&self);
    fn stop(&self);
}

#[derive(new, Clone)]
pub struct EventContext {
    pub transceiver: WsTransceiver,
    pub token: CancellationToken,
}

pub fn build_event(transceiver: WsTransceiver, start: &StartEvent) -> Box<dyn Event> {
    let ctx = EventContext::new(transceiver, Default::default());

    match start {
        StartEvent::MeleeMania { .. } => Box::new(MeleeMania::new(
            MeleeManiaConfig::from_config(start),
            ctx.transceiver,
        )),
        StartEvent::SkyEye { .. } => Box::new(SkyEye::new(SkyEyeConfig::from_config(start), ctx)),
        StartEvent::RadioSpies {} => Box::new(RadioSpies::new(ctx)),
    }
}
