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
        require_discovery_stars_available(state, stars)?;
    }

    Ok(())
}

fn require_discovery_stars_available(
    state: &GameState,
    stars: &[Piece],
) -> Result<(), ActionError> {
    for star in stars {
        let requested_count = stars
            .iter()
            .filter(|other| other.color() == star.color() && other.size() == star.size())
            .count();
        if usize::from(state.bank().count(star.color(), star.size())) < requested_count {
            return Err(ActionError::PieceUnavailable { piece: *star });
        }
    }

    Ok(())
}
