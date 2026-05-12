use hw_core::{Color, GameState, Piece, Player, SystemId};

use super::{ActionError, shared};

pub(super) fn validate(
    state: &GameState,
    player: Player,
    system: SystemId,
    target: Piece,
) -> Result<(), ActionError> {
    shared::require_system(state, system)?;

    match target.owner() {
        None => return Err(ActionError::UnownedShip { ship: target }),
        Some(owner) if owner == player => {
            return Err(ActionError::CannotInvadeOwnShip {
                player,
                ship: target,
            });
        }
        Some(_) => {}
    }

    shared::require_ship_present(state, system, target)?;
    shared::require_action_power(state, player, system, Color::Red)?;
    Ok(())
}
