use hw_core::{Color, GameState, Piece, Player, StarSystem, SystemId};

use crate::action::MoveTarget;

use super::{ActionError, shared};

pub(super) fn validate(
    state: &GameState,
    player: Player,
    from: SystemId,
    ship: Piece,
    target: &MoveTarget,
) -> Result<(), ActionError> {
    shared::require_system(state, from)?;

    if let MoveTarget::Existing(to) = target {
        shared::require_system(state, *to)?;

        if from == *to {
            return Err(ActionError::SameSystem { system: from });
        }
    }

    shared::require_owned_by(player, ship)?;
    shared::require_ship_present(state, from, ship)?;
    shared::require_action_power(state, player, from, Color::Yellow)?;

    if let MoveTarget::New { stars } = target {
        StarSystem::new(stars.to_vec(), vec![ship])
            .map(|_| ())
            .map_err(|error| ActionError::InvalidDiscovery { error })?;
    }

    Ok(())
}
