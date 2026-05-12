use hw_core::{Bank, GameState, Piece, Player, StarSystem, SystemId};

use crate::action::ActionError;

use super::TransitionError;

pub(super) fn system_parts(
    state: &GameState,
    system: SystemId,
) -> Result<(Vec<Piece>, Vec<Piece>), TransitionError> {
    let system_ref =
        state
            .system(system)
            .ok_or(TransitionError::InvalidAction(ActionError::UnknownSystem {
                system,
            }))?;

    Ok((system_ref.stars().to_vec(), system_ref.ships().to_vec()))
}

pub(super) fn rebuild_system(
    stars: Vec<Piece>,
    ships: Vec<Piece>,
) -> Result<StarSystem, TransitionError> {
    StarSystem::new(stars, ships).map_err(TransitionError::from)
}

pub(super) fn rebuild_state(
    state: &GameState,
    systems: Vec<StarSystem>,
    bank: Bank,
) -> Result<GameState, TransitionError> {
    GameState::new(systems, homeworlds(state), bank).map_err(TransitionError::from)
}

pub(super) fn homeworlds(state: &GameState) -> [SystemId; Player::COUNT] {
    [state.homeworld(Player::One), state.homeworld(Player::Two)]
}
