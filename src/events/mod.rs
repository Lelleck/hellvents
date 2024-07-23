pub mod melee_mania;
pub mod notify_players;

pub trait Event {
    fn start(&self);
    fn stop(&self);
    fn is_stopped(&self) -> bool;
}
