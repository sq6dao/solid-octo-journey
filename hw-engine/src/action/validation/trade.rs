use hw_core::{Color, GameState, Piece, Player, SystemId};

use super::{ActionError, shared};

pub(super) fn validate(
    state: &GameState,
    player: Player,
    system: SystemId,
    from: Piece,
    to: Piece,
) -> Result<(), ActionError> {
    shared::require_system(state, system)?;
    shared::require_owned_by(player, from)?;
    shared::require_owned_by(player, to)?;
    shared::require_ship_present(state, system, from)?;
    shared::require_action_power(state, player, system, Color::Blue)?;

    if from.size() != to.size() {
        return Err(ActionError::SizeMismatch { from, to });
    }

    shared::require_bank_piece(state, to)?;
    Ok(())
}
