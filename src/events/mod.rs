pub mod melee_mania;

pub trait Event {
    fn start(&self);
    fn stop(&self);
    fn is_stopped(&self) -> bool;
}
