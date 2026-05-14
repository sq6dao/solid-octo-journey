use hw_core::{Color, Piece, Player, SystemId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Build {
        player: Player,
        system: SystemId,
        ship: Piece,
    },
    Travel {
        player: Player,
        from: SystemId,
        ship: Piece,
        target: TravelTarget,
    },
    Trade {
        player: Player,
        system: SystemId,
        from: Piece,
        to: Piece,
    },
    Sacrifice {
        player: Player,
        system: SystemId,
        ship: Piece,
    },
    Invade {
        player: Player,
        system: SystemId,
        target: Piece,
    },
    Catastrophe {
        system: SystemId,
        color: Color,
    },
}

impl Action {
    pub const fn kind(&self) -> ActionKind {
        match self {
            Self::Build { .. } => ActionKind::Build,
            Self::Travel { .. } => ActionKind::Travel,
            Self::Trade { .. } => ActionKind::Trade,
            Self::Sacrifice { .. } => ActionKind::Sacrifice,
            Self::Invade { .. } => ActionKind::Invade,
            Self::Catastrophe { .. } => ActionKind::Catastrophe,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TravelTarget {
    Existing(SystemId),
    New { stars: Vec<Piece> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionKind {
    Build,
    Travel,
    Trade,
    Sacrifice,
    Invade,
    Catastrophe,
}
