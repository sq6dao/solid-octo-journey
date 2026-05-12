mod build;
mod shared;

use hw_core::{GameState, GameStateError, StarSystemError};

use crate::action::{Action, ActionError, ActionKind, validate_action};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionError {
    InvalidAction(ActionError),
    InvalidState(GameStateError),
    InvalidSystem(StarSystemError),
    UnsupportedAction(ActionKind),
}

pub fn apply_action(state: &GameState, action: &Action) -> Result<GameState, TransitionError> {
    validate_action(state, action).map_err(TransitionError::InvalidAction)?;

    match action {
        Action::Build {
            player,
            system,
            ship,
        } => build::apply(state, *player, *system, *ship),
        _ => Err(TransitionError::UnsupportedAction(action.kind())),
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
