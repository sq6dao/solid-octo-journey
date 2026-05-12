use hw_core::{GameState, Piece, Player, SystemId};

use super::{ActionError, shared};

pub(super) fn validate(
    state: &GameState,
    player: Player,
    system: SystemId,
    ship: Piece,
) -> Result<(), ActionError> {
    shared::require_system(state, system)?;
    shared::require_owned_by(player, ship)?;
    shared::require_ship_present(state, system, ship)?;
    Ok(())
}
