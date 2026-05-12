mod build;
mod catastrophe;
mod invade;
// `move` is a Rust keyword; use a raw identifier so the file can still
// match the action name.
mod r#move;
mod sacrifice;
mod shared;
mod trade;

use hw_core::{Color, GameState, Piece, Player, Size, StarSystemError, SystemId};

use super::Action;

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
    StarSizeConflict {
        size: Size,
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
        } => build::validate(state, *player, *system, *ship),
        Action::Move {
            player,
            from,
            ship,
            target,
        } => r#move::validate(state, *player, *from, *ship, target),
        Action::Trade {
            player,
            system,
            from,
            to,
        } => trade::validate(state, *player, *system, *from, *to),
        Action::Sacrifice {
            player,
            system,
            ship,
        } => sacrifice::validate(state, *player, *system, *ship),
        Action::Invade {
            player,
            system,
            target,
        } => invade::validate(state, *player, *system, *target),
        Action::Catastrophe { system, color } => catastrophe::validate(state, *system, *color),
    }
}
