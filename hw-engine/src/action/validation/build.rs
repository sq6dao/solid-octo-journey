use hw_core::{Color, GameState, Piece, Player, Size, SystemId};

use super::{ActionError, shared};

pub(super) fn validate(
    state: &GameState,
    player: Player,
    system: SystemId,
    ship: Piece,
) -> Result<(), ActionError> {
    shared::require_system(state, system)?;
    shared::require_owned_by(player, ship)?;
    shared::require_action_power(state, player, system, Color::Green)?;
    require_build_piece(state, player, ship)?;
    Ok(())
}

fn require_build_piece(state: &GameState, player: Player, piece: Piece) -> Result<(), ActionError> {
    shared::require_bank_piece(state, piece)?;

    let smallest = smallest_available_piece(state, player, piece.color())
        .ok_or(ActionError::PieceUnavailable { piece })?;
    if smallest.size() == piece.size() {
        Ok(())
    } else {
        Err(ActionError::BuildSizeUnavailable {
            requested: piece,
            smallest,
        })
    }
}

fn smallest_available_piece(state: &GameState, player: Player, color: Color) -> Option<Piece> {
    [Size::Small, Size::Medium, Size::Large]
        .into_iter()
        .find(|size| state.bank().count(color, *size) > 0)
        .map(|size| Piece::owned(color, size, player))
}
