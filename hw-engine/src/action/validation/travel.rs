use hw_core::{Color, GameState, Piece, Player, Size, StarSystem, SystemId};

use crate::action::TravelTarget;

use super::{ActionError, shared};

pub(super) fn validate(
    state: &GameState,
    player: Player,
    from: SystemId,
    ship: Piece,
    target: &TravelTarget,
) -> Result<(), ActionError> {
    let source_system = state
        .system(from)
        .ok_or(ActionError::UnknownSystem { system: from })?;

    if let TravelTarget::Existing(to) = target {
        shared::require_system(state, *to)?;

        if from == *to {
            return Err(ActionError::SameSystem { system: from });
        }
    }

    shared::require_owned_by(player, ship)?;
    shared::require_ship_present(state, from, ship)?;
    shared::require_action_power(state, player, from, Color::Yellow)?;

    match target {
        TravelTarget::Existing(to) => {
            let target_system = state
                .system(*to)
                .ok_or(ActionError::UnknownSystem { system: *to })?;
            require_distinct_star_sizes(source_system.stars(), target_system.stars())?;
        }
        TravelTarget::New { stars } => {
            StarSystem::new(stars.to_vec(), vec![ship])
                .map(|_| ())
                .map_err(|error| ActionError::InvalidDiscovery { error })?;
            require_discovery_stars_available(state, stars)?;
            require_distinct_star_sizes(source_system.stars(), stars)?;
        }
    }

    Ok(())
}

fn require_distinct_star_sizes(source: &[Piece], target: &[Piece]) -> Result<(), ActionError> {
    if let Some(size) = shared_star_size(source, target) {
        Err(ActionError::StarSizeConflict { size })
    } else {
        Ok(())
    }
}

fn shared_star_size(source: &[Piece], target: &[Piece]) -> Option<Size> {
    source
        .iter()
        .find(|source_star| {
            target
                .iter()
                .any(|target_star| target_star.size() == source_star.size())
        })
        .map(Piece::size)
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
