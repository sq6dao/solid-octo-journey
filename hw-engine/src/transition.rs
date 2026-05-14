mod build;
mod catastrophe;
mod invade;
mod sacrifice;
mod shared;
mod trade;
mod travel;

use hw_core::{GameState, GameStateError, StarSystemError};

use crate::action::{Action, ActionError, validate_action};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionError {
    InvalidAction(ActionError),
    InvalidState(GameStateError),
    InvalidSystem(StarSystemError),
}

pub fn apply_action(state: &GameState, action: &Action) -> Result<GameState, TransitionError> {
    validate_action(state, action).map_err(TransitionError::InvalidAction)?;

    match action {
        Action::Build {
            player,
            system,
            ship,
        } => build::apply(state, *player, *system, *ship),
        Action::Travel {
            player,
            from,
            ship,
            target,
        } => travel::apply(state, *player, *from, *ship, target),
        Action::Trade {
            player,
            system,
            from,
            to,
        } => trade::apply(state, *player, *system, *from, *to),
        Action::Invade {
            player,
            system,
            target,
        } => invade::apply(state, *player, *system, *target),
        Action::Sacrifice {
            player,
            system,
            ship,
        } => sacrifice::apply(state, *player, *system, *ship),
        Action::Catastrophe { system, color } => catastrophe::apply(state, *system, *color),
    }
}

impl From<GameStateError> for TransitionError {
    fn from(error: GameStateError) -> Self {
        Self::InvalidState(error)
    }
}

impl From<StarSystemError> for TransitionError {
    fn from(error: StarSystemError) -> Self {
        Self::InvalidSystem(error)
    }
}

#[cfg(test)]
mod tests;
