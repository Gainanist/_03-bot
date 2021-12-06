use crate::components::BygonePart;

#[derive(Clone, Copy, Debug)]
pub enum PlayerCommand {
    StartGame,
    Strike(BygonePart),
}