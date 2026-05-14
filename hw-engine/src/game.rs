use hw_core::{
    Bank, Color, GameState, GameStateError, Piece, Player, Size, StarSystem, StarSystemError,
    SystemId,
};

use crate::{Action, TurnError, TurnState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Game {
    turn: TurnState,
    status: GameStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameStatus {
    InProgress,
    Finished(GameOutcome),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameOutcome {
    Winner(Player),
    Draw,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameError {
    InvalidHomeworld {
        player: Player,
        error: StarSystemError,
    },
    InvalidState(GameStateError),
    PieceUnavailable {
        piece: Piece,
    },
    WrongHomeworldShipOwner {
        player: Player,
        ship: Piece,
    },
    Terminal {
        outcome: GameOutcome,
    },
    Turn(TurnError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HomeworldSetup {
    stars: Vec<Piece>,
    ship: Piece,
}

impl HomeworldSetup {
    pub fn new(stars: Vec<Piece>, ship: Piece) -> Self {
        Self { stars, ship }
    }
}

impl Game {
    pub fn new(
        homeworlds: [HomeworldSetup; Player::COUNT],
        starting_player: Player,
    ) -> Result<Self, GameError> {
        let [player_one, player_two] = homeworlds;
        let mut bank = Bank::new();
        let systems = vec![
            build_homeworld(&mut bank, Player::One, player_one)?,
            build_homeworld(&mut bank, Player::Two, player_two)?,
        ];
        let state = GameState::new(systems, [SystemId::new(0), SystemId::new(1)], bank)?;

        Ok(Self {
            turn: TurnState::new(state, starting_player),
            status: GameStatus::InProgress,
        })
    }

    pub fn default(starting_player: Player) -> Self {
        let homeworlds = [
            HomeworldSetup::new(
                vec![Piece::new(Color::Yellow, Size::Small)],
                Piece::owned(Color::Green, Size::Small, Player::One),
            ),
            HomeworldSetup::new(
                vec![Piece::new(Color::Blue, Size::Large)],
                Piece::owned(Color::Red, Size::Medium, Player::Two),
            ),
        ];

        match Self::new(homeworlds, starting_player) {
            Ok(game) => game,
            Err(error) => panic!("default game setup is invalid: {error:?}"),
        }
    }

    pub const fn turn(&self) -> &TurnState {
        &self.turn
    }

    pub const fn status(&self) -> GameStatus {
        self.status
    }

    pub fn apply_action(&self, action: &Action) -> Result<Self, GameError> {
        self.ensure_in_progress()?;

        Ok(Self {
            turn: self.turn.apply_action(action)?,
            status: self.status,
        })
    }

    pub fn end_turn(&self) -> Result<Self, GameError> {
        self.ensure_in_progress()?;

        Ok(Self {
            turn: self.turn.end_turn()?,
            status: self.status,
        })
    }

    fn ensure_in_progress(&self) -> Result<(), GameError> {
        match self.status {
            GameStatus::InProgress => Ok(()),
            GameStatus::Finished(outcome) => Err(GameError::Terminal { outcome }),
        }
    }
}

fn build_homeworld(
    bank: &mut Bank,
    player: Player,
    setup: HomeworldSetup,
) -> Result<StarSystem, GameError> {
    if setup.ship.owner() != Some(player) {
        return Err(GameError::WrongHomeworldShipOwner {
            player,
            ship: setup.ship,
        });
    }

    let system = StarSystem::new(setup.stars.clone(), vec![setup.ship])
        .map_err(|error| GameError::InvalidHomeworld { player, error })?;

    for piece in setup.stars.iter().chain(std::iter::once(&setup.ship)) {
        draw_setup_piece(bank, *piece)?;
    }

    Ok(system)
}

fn draw_setup_piece(bank: &mut Bank, piece: Piece) -> Result<(), GameError> {
    bank.draw(piece.color(), piece.size())
        .map(|_| ())
        .map_err(|_| GameError::PieceUnavailable { piece })
}

impl From<GameStateError> for GameError {
    fn from(error: GameStateError) -> Self {
        Self::InvalidState(error)
    }
}

impl From<TurnError> for GameError {
    fn from(error: TurnError) -> Self {
        Self::Turn(error)
    }
}

#[cfg(test)]
mod tests {
    use hw_core::{Color, Size, SystemId};

    use super::*;

    #[test]
    fn explicit_setup_creates_initial_game_state() {
        let one_ship = Piece::owned(Color::Green, Size::Small, Player::One);
        let two_ship = Piece::owned(Color::Red, Size::Medium, Player::Two);
        let game = Game::new(
            [
                HomeworldSetup::new(vec![Piece::new(Color::Yellow, Size::Small)], one_ship),
                HomeworldSetup::new(vec![Piece::new(Color::Blue, Size::Large)], two_ship),
            ],
            Player::Two,
        )
        .expect("game initializes");

        let state = game.turn().state();

        assert_eq!(game.status(), GameStatus::InProgress);
        assert_eq!(game.turn().current_player(), Player::Two);
        assert_eq!(state.homeworld(Player::One), SystemId::new(0));
        assert_eq!(state.homeworld(Player::Two), SystemId::new(1));
        assert_eq!(
            state.system(SystemId::new(0)).expect("system exists").stars(),
            &[Piece::new(Color::Yellow, Size::Small)]
        );
        assert_eq!(
            state.system(SystemId::new(0)).expect("system exists").ships(),
            &[one_ship]
        );
        assert_eq!(
            state.system(SystemId::new(1)).expect("system exists").stars(),
            &[Piece::new(Color::Blue, Size::Large)]
        );
        assert_eq!(
            state.system(SystemId::new(1)).expect("system exists").ships(),
            &[two_ship]
        );
        assert_eq!(
            state.bank().count(Color::Yellow, Size::Small),
            Bank::copies_per_piece() - 1
        );
        assert_eq!(
            state.bank().count(Color::Green, Size::Small),
            Bank::copies_per_piece() - 1
        );
        assert_eq!(
            state.bank().count(Color::Blue, Size::Large),
            Bank::copies_per_piece() - 1
        );
        assert_eq!(
            state.bank().count(Color::Red, Size::Medium),
            Bank::copies_per_piece() - 1
        );
    }

    #[test]
    fn setup_rejects_a_ship_owned_by_the_wrong_player() {
        let ship = Piece::owned(Color::Green, Size::Small, Player::Two);

        assert_eq!(
            Game::new(
                [
                    HomeworldSetup::new(vec![Piece::new(Color::Yellow, Size::Small)], ship),
                    HomeworldSetup::new(
                        vec![Piece::new(Color::Blue, Size::Large)],
                        Piece::owned(Color::Red, Size::Medium, Player::Two),
                    ),
                ],
                Player::One,
            ),
            Err(GameError::WrongHomeworldShipOwner {
                player: Player::One,
                ship,
            })
        );
    }

    #[test]
    fn setup_rejects_invalid_homeworld_systems() {
        assert_eq!(
            Game::new(
                [
                    HomeworldSetup::new(
                        vec![Piece::owned(Color::Yellow, Size::Small, Player::One)],
                        Piece::owned(Color::Green, Size::Small, Player::One),
                    ),
                    HomeworldSetup::new(
                        vec![Piece::new(Color::Blue, Size::Large)],
                        Piece::owned(Color::Red, Size::Medium, Player::Two),
                    ),
                ],
                Player::One,
            ),
            Err(GameError::InvalidHomeworld {
                player: Player::One,
                error: StarSystemError::OwnedStar,
            })
        );
    }

    #[test]
    fn setup_rejects_unavailable_bank_pieces() {
        let repeated = Piece::new(Color::Yellow, Size::Small);

        assert_eq!(
            Game::new(
                [
                    HomeworldSetup::new(
                        vec![repeated, repeated],
                        Piece::owned(Color::Yellow, Size::Small, Player::One),
                    ),
                    HomeworldSetup::new(
                        vec![Piece::new(Color::Blue, Size::Large)],
                        Piece::owned(Color::Yellow, Size::Small, Player::Two),
                    ),
                ],
                Player::One,
            ),
            Err(GameError::PieceUnavailable {
                piece: Piece::owned(Color::Yellow, Size::Small, Player::Two),
            })
        );
    }

    #[test]
    fn default_game_is_deterministic_and_valid() {
        let game = Game::default(Player::One);

        assert_eq!(game.status(), GameStatus::InProgress);
        assert_eq!(game.turn().current_player(), Player::One);
        assert_eq!(game.turn().state().systems().len(), 2);
        assert_eq!(game.turn().state().homeworld(Player::One), SystemId::new(0));
        assert_eq!(game.turn().state().homeworld(Player::Two), SystemId::new(1));
    }
}
