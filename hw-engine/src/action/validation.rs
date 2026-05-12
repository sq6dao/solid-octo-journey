use hw_core::{Color, GameState, Piece, Player, Size, StarSystem, StarSystemError, SystemId};

use super::{Action, MoveTarget};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionError {
    UnknownSystem {
        system: SystemId,
    },
    UnownedShip {
        ship: Piece,
    },
    WrongOwner {
        player: Player,
        ship: Piece,
    },
    CannotInvadeOwnShip {
        player: Player,
        ship: Piece,
    },
    ShipNotPresent {
        system: SystemId,
        ship: Piece,
    },
    SameSystem {
        system: SystemId,
    },
    PieceUnavailable {
        piece: Piece,
    },
    BuildSizeUnavailable {
        requested: Piece,
        smallest: Piece,
    },
    SizeMismatch {
        from: Piece,
        to: Piece,
    },
    MissingActionPower {
        player: Player,
        system: SystemId,
        color: Color,
    },
    InvalidDiscovery {
        error: StarSystemError,
    },
    NoCatastrophe {
        system: SystemId,
        color: Color,
        count: usize,
    },
}

pub fn validate_action(state: &GameState, action: &Action) -> Result<(), ActionError> {
    match action {
        Action::Build {
            player,
            system,
            ship,
        } => validate_build(state, *player, *system, *ship),
        Action::Move {
            player,
            from,
            ship,
            target,
        } => validate_move(state, *player, *from, *ship, target),
        Action::Trade {
            player,
            system,
            from,
            to,
        } => validate_trade(state, *player, *system, *from, *to),
        Action::Sacrifice {
            player,
            system,
            ship,
        } => validate_sacrifice(state, *player, *system, *ship),
        Action::Invade {
            player,
            system,
            target,
        } => validate_invade(state, *player, *system, *target),
        Action::Catastrophe { system, color } => validate_catastrophe(state, *system, *color),
    }
}

fn validate_build(
    state: &GameState,
    player: Player,
    system: SystemId,
    ship: Piece,
) -> Result<(), ActionError> {
    require_system(state, system)?;
    require_owned_by(player, ship)?;
    require_action_power(state, player, system, Color::Green)?;
    require_build_piece(state, player, ship)?;
    Ok(())
}

fn validate_move(
    state: &GameState,
    player: Player,
    from: SystemId,
    ship: Piece,
    target: &MoveTarget,
) -> Result<(), ActionError> {
    require_system(state, from)?;

    if let MoveTarget::Existing(to) = target {
        require_system(state, *to)?;

        if from == *to {
            return Err(ActionError::SameSystem { system: from });
        }
    }

    require_owned_by(player, ship)?;
    require_ship_present(state, from, ship)?;
    require_action_power(state, player, from, Color::Yellow)?;

    if let MoveTarget::New { stars } = target {
        StarSystem::new(stars.to_vec(), vec![ship])
            .map(|_| ())
            .map_err(|error| ActionError::InvalidDiscovery { error })?;
    }

    Ok(())
}

fn validate_trade(
    state: &GameState,
    player: Player,
    system: SystemId,
    from: Piece,
    to: Piece,
) -> Result<(), ActionError> {
    require_system(state, system)?;
    require_owned_by(player, from)?;
    require_owned_by(player, to)?;
    require_ship_present(state, system, from)?;
    require_action_power(state, player, system, Color::Blue)?;

    if from.size() != to.size() {
        return Err(ActionError::SizeMismatch { from, to });
    }

    require_bank_piece(state, to)?;
    Ok(())
}

fn validate_sacrifice(
    state: &GameState,
    player: Player,
    system: SystemId,
    ship: Piece,
) -> Result<(), ActionError> {
    require_system(state, system)?;
    require_owned_by(player, ship)?;
    require_ship_present(state, system, ship)?;
    Ok(())
}

fn validate_invade(
    state: &GameState,
    player: Player,
    system: SystemId,
    target: Piece,
) -> Result<(), ActionError> {
    require_system(state, system)?;

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

    require_ship_present(state, system, target)?;
    require_action_power(state, player, system, Color::Red)?;
    Ok(())
}

fn validate_catastrophe(
    state: &GameState,
    system: SystemId,
    color: Color,
) -> Result<(), ActionError> {
    let system_ref = state
        .system(system)
        .ok_or(ActionError::UnknownSystem { system })?;
    let count = system_ref
        .stars()
        .iter()
        .chain(system_ref.ships())
        .filter(|piece| piece.color() == color)
        .count();

    if count >= 4 {
        Ok(())
    } else {
        Err(ActionError::NoCatastrophe {
            system,
            color,
            count,
        })
    }
}

fn require_system(state: &GameState, system: SystemId) -> Result<(), ActionError> {
    state
        .system(system)
        .map(|_| ())
        .ok_or(ActionError::UnknownSystem { system })
}

fn require_owned_by(player: Player, ship: Piece) -> Result<(), ActionError> {
    match ship.owner() {
        None => Err(ActionError::UnownedShip { ship }),
        Some(owner) if owner != player => Err(ActionError::WrongOwner { player, ship }),
        Some(_) => Ok(()),
    }
}

fn require_ship_present(
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

fn require_action_power(
    state: &GameState,
    player: Player,
    system: SystemId,
    color: Color,
) -> Result<(), ActionError> {
    let system_ref = state
        .system(system)
        .ok_or(ActionError::UnknownSystem { system })?;

    if system_ref
        .ships()
        .iter()
        .any(|ship| ship.is_owned_by(player) && ship.color() == color)
    {
        Ok(())
    } else {
        Err(ActionError::MissingActionPower {
            player,
            system,
            color,
        })
    }
}

fn require_build_piece(state: &GameState, player: Player, piece: Piece) -> Result<(), ActionError> {
    require_bank_piece(state, piece)?;

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

fn require_bank_piece(state: &GameState, piece: Piece) -> Result<(), ActionError> {
    if state.bank().count(piece.color(), piece.size()) == 0 {
        Err(ActionError::PieceUnavailable { piece })
    } else {
        Ok(())
    }
}
