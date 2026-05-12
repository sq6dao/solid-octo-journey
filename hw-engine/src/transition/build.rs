use hw_core::{GameState, Piece, Player, SystemId};

use crate::action::ActionError;

use super::{TransitionError, shared};

pub(super) fn apply(
    state: &GameState,
    player: Player,
    system: SystemId,
    ship: Piece,
) -> Result<GameState, TransitionError> {
    let mut bank = state.bank().clone();
    let drawn = bank.draw(ship.color(), ship.size()).map_err(|_| {
        TransitionError::InvalidAction(ActionError::PieceUnavailable { piece: ship })
    })?;
    let (stars, mut ships) = shared::system_parts(state, system)?;
    ships.push(drawn.with_owner(player));

    let mut systems = state.systems().to_vec();
    systems[system.index()] = shared::rebuild_system(stars, ships)?;
    shared::rebuild_state(state, systems, bank)
}
