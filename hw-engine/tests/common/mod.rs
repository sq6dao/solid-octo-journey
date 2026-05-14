#![allow(dead_code)]

use hw_core::{Bank, Color, GameState, Piece, Player, Size, StarSystem, SystemId};
use hw_engine::{Action, Game, HomeworldSetup, TravelTarget};

pub const P1_HOME: SystemId = SystemId::new(0);
pub const P2_HOME: SystemId = SystemId::new(1);

pub fn piece(color: Color, size: Size) -> Piece {
    Piece::new(color, size)
}

pub fn ship(player: Player, color: Color, size: Size) -> Piece {
    Piece::owned(color, size, player)
}

pub fn setup(stars: Vec<Piece>, ship: Piece) -> HomeworldSetup {
    HomeworldSetup::new(stars, ship)
}

pub fn system(stars: Vec<Piece>, ships: Vec<Piece>) -> StarSystem {
    StarSystem::new(stars, ships).expect("system is valid")
}

pub fn state(systems: Vec<StarSystem>, homeworlds: [SystemId; Player::COUNT]) -> GameState {
    GameState::new(systems, homeworlds, Bank::new()).expect("state is valid")
}

pub fn simple_game(starting_player: Player) -> Game {
    Game::new(
        [
            setup(
                vec![
                    piece(Color::Red, Size::Medium),
                    piece(Color::Red, Size::Medium),
                ],
                ship(Player::One, Color::Green, Size::Small),
            ),
            setup(
                vec![
                    piece(Color::Blue, Size::Large),
                    piece(Color::Blue, Size::Large),
                ],
                ship(Player::Two, Color::Green, Size::Small),
            ),
        ],
        starting_player,
    )
    .expect("game initializes")
}

pub fn build(player: Player, system: SystemId, color: Color, size: Size) -> Action {
    Action::Build {
        player,
        system,
        ship: ship(player, color, size),
    }
}

pub fn travel_existing(player: Player, from: SystemId, ship: Piece, to: SystemId) -> Action {
    Action::Travel {
        player,
        from,
        ship,
        target: TravelTarget::Existing(to),
    }
}

pub fn travel_new(player: Player, from: SystemId, ship: Piece, stars: Vec<Piece>) -> Action {
    Action::Travel {
        player,
        from,
        ship,
        target: TravelTarget::New { stars },
    }
}

pub fn trade(player: Player, system: SystemId, from: Piece, to: Piece) -> Action {
    Action::Trade {
        player,
        system,
        from,
        to,
    }
}

pub fn invade(player: Player, system: SystemId, target: Piece) -> Action {
    Action::Invade {
        player,
        system,
        target,
    }
}

pub fn sacrifice(player: Player, system: SystemId, ship: Piece) -> Action {
    Action::Sacrifice {
        player,
        system,
        ship,
    }
}

pub fn catastrophe(system: SystemId, color: Color) -> Action {
    Action::Catastrophe { system, color }
}

pub fn count_ship(state: &GameState, system_id: SystemId, ship: Piece) -> usize {
    state
        .system(system_id)
        .expect("system exists")
        .ships()
        .iter()
        .filter(|candidate| **candidate == ship)
        .count()
}

pub fn apply_and_end(game: &Game, action: &Action) -> Game {
    game.apply_action(action)
        .expect("action applies")
        .end_turn()
        .expect("turn ends")
}
