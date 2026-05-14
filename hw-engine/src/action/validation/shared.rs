use hw_core::{Color, GameState, Piece, Player, SystemId};

use super::ActionError;

pub(super) fn require_system(state: &GameState, system: SystemId) -> Result<(), ActionError> {
    state
        .system(system)
        .map(|_| ())
        .ok_or(ActionError::UnknownSystem { system })
}

pub(super) fn require_owned_by(player: Player, ship: Piece) -> Result<(), ActionError> {
    match ship.owner() {
        None => Err(ActionError::UnownedShip { ship }),
        Some(owner) if owner != player => Err(ActionError::WrongOwner { player, ship }),
        Some(_) => Ok(()),
    }
}

pub(super) fn require_ship_present(
    state: &GameState,
    system: SystemId,
    ship: Piece,
) -> Result<(), ActionError> {
    let system_ref = state
        .system(system)
        .ok_or(ActionError::UnknownSystem { system })?;

    if system_ref.ships().contains(&ship) {
        Ok(())
    } else {
        Err(ActionError::ShipNotPresent { system, ship })
    }
}

pub(super) fn require_action_power(
    state: &GameState,
    player: Player,
    system: SystemId,
    color: Color,
) -> Result<(), ActionError> {
    let system_ref = state
        .system(system)
        .ok_or(ActionError::UnknownSystem { system })?;

    let has_ship_power = system_ref
        .ships()
        .iter()
        .any(|ship| ship.is_owned_by(player) && ship.color() == color);
    let has_star_power = system_ref.stars().iter().any(|star| star.color() == color);

    if has_ship_power || has_star_power {
        Ok(())
    } else {
        Err(ActionError::MissingActionPower {
            player,
            system,
            color,
        })
    }
}

pub(super) fn require_bank_piece(state: &GameState, piece: Piece) -> Result<(), ActionError> {
    if state.bank().count(piece.color(), piece.size()) == 0 {
        Err(ActionError::PieceUnavailable { piece })
    } else {
        Ok(())
    }
}
