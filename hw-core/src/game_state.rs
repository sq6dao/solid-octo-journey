use crate::{Bank, Player, StarSystem};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SystemId(usize);

impl SystemId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameStateError {
    HomeworldOutOfRange {
        player: Player,
        homeworld: SystemId,
    },
    DuplicateHomeworld {
        players: [Player; 2],
        homeworld: SystemId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameState {
    systems: Vec<StarSystem>,
    homeworlds: [SystemId; Player::COUNT],
    bank: Bank,
}

impl GameState {
    pub fn new(
        systems: Vec<StarSystem>,
        homeworlds: [SystemId; Player::COUNT],
        bank: Bank,
    ) -> Result<Self, GameStateError> {
        for player in Player::ALL {
            let homeworld = homeworlds[player.index()];
            if homeworld.index() >= systems.len() {
                return Err(GameStateError::HomeworldOutOfRange { player, homeworld });
            }
        }

        if homeworlds[0] == homeworlds[1] {
            return Err(GameStateError::DuplicateHomeworld {
                players: [Player::One, Player::Two],
                homeworld: homeworlds[0],
            });
        }

        Ok(Self {
            systems,
            homeworlds,
            bank,
        })
    }

    pub fn systems(&self) -> &[StarSystem] {
        &self.systems
    }

    pub fn system(&self, id: SystemId) -> Option<&StarSystem> {
        self.systems.get(id.index())
    }

    pub fn homeworld(&self, player: Player) -> SystemId {
        self.homeworlds[player.index()]
    }

    pub fn bank(&self) -> &Bank {
        &self.bank
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Piece, Size};

    #[test]
    fn system_id_preserves_its_index() {
        let id = SystemId::new(3);

        assert_eq!(id.index(), 3);
    }

    #[test]
    fn game_state_new_stores_systems_homeworlds_and_bank() {
        let systems = vec![
            valid_system(Player::One, Color::Yellow, Color::Green),
            valid_system(Player::Two, Color::Blue, Color::Red),
        ];
        let mut bank = Bank::new();
        bank.draw(Color::Red, Size::Small).expect("piece exists");

        let state = GameState::new(systems.clone(), [SystemId::new(0), SystemId::new(1)], bank)
            .expect("state is valid");

        assert_eq!(state.systems(), systems.as_slice());
        assert_eq!(state.homeworld(Player::One), SystemId::new(0));
        assert_eq!(state.homeworld(Player::Two), SystemId::new(1));
        assert_eq!(
            state.bank().count(Color::Red, Size::Small),
            Bank::copies_per_piece() - 1
        );
    }

    #[test]
    fn game_state_system_returns_a_system_by_id() {
        let systems = vec![
            valid_system(Player::One, Color::Yellow, Color::Green),
            valid_system(Player::Two, Color::Blue, Color::Red),
        ];
        let state = GameState::new(
            systems.clone(),
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");

        assert_eq!(state.system(SystemId::new(1)), Some(&systems[1]));
    }

    #[test]
    fn game_state_system_returns_none_for_external_ids() {
        let state = valid_game_state();

        assert_eq!(state.system(SystemId::new(2)), None);
    }

    #[test]
    fn game_state_rejects_player_one_homeworld_outside_systems() {
        let systems = vec![valid_system(Player::One, Color::Yellow, Color::Green)];

        assert_eq!(
            GameState::new(systems, [SystemId::new(1), SystemId::new(0)], Bank::new(),),
            Err(GameStateError::HomeworldOutOfRange {
                player: Player::One,
                homeworld: SystemId::new(1),
            })
        );
    }

    #[test]
    fn game_state_rejects_player_two_homeworld_outside_systems() {
        let systems = vec![valid_system(Player::One, Color::Yellow, Color::Green)];

        assert_eq!(
            GameState::new(systems, [SystemId::new(0), SystemId::new(1)], Bank::new(),),
            Err(GameStateError::HomeworldOutOfRange {
                player: Player::Two,
                homeworld: SystemId::new(1),
            })
        );
    }

    #[test]
    fn game_state_rejects_duplicate_homeworlds() {
        let systems = vec![
            valid_system(Player::One, Color::Yellow, Color::Green),
            valid_system(Player::Two, Color::Blue, Color::Red),
        ];

        assert_eq!(
            GameState::new(systems, [SystemId::new(0), SystemId::new(0)], Bank::new(),),
            Err(GameStateError::DuplicateHomeworld {
                players: [Player::One, Player::Two],
                homeworld: SystemId::new(0),
            })
        );
    }

    #[test]
    fn game_state_allows_a_homeworld_without_player_presence() {
        let systems = vec![
            valid_system(Player::Two, Color::Yellow, Color::Green),
            valid_system(Player::One, Color::Blue, Color::Red),
        ];

        let state = GameState::new(systems, [SystemId::new(0), SystemId::new(1)], Bank::new())
            .expect("state is valid");

        assert!(
            !state
                .system(state.homeworld(Player::One))
                .expect("homeworld exists")
                .has_presence(Player::One)
        );
    }

    #[test]
    fn game_state_allows_a_homeworld_with_zero_stars() {
        let systems = vec![
            StarSystem::new(
                vec![],
                vec![Piece::owned(Color::Green, Size::Small, Player::One)],
            )
            .expect("system is valid"),
            valid_system(Player::Two, Color::Blue, Color::Red),
        ];

        let state = GameState::new(systems, [SystemId::new(0), SystemId::new(1)], Bank::new())
            .expect("state is valid");

        assert!(
            state
                .system(state.homeworld(Player::One))
                .expect("homeworld exists")
                .stars()
                .is_empty()
        );
    }

    #[test]
    fn game_state_allows_a_homeworld_with_zero_ships() {
        let systems = vec![
            StarSystem::new(vec![Piece::new(Color::Yellow, Size::Small)], vec![])
                .expect("system is valid"),
            valid_system(Player::Two, Color::Blue, Color::Red),
        ];

        let state = GameState::new(systems, [SystemId::new(0), SystemId::new(1)], Bank::new())
            .expect("state is valid");

        assert!(
            state
                .system(state.homeworld(Player::One))
                .expect("homeworld exists")
                .ships()
                .is_empty()
        );
    }

    #[test]
    fn game_state_accessors_do_not_consume_the_state() {
        let state = valid_game_state();

        assert_eq!(state.systems().len(), 2);
        assert_eq!(
            state.bank().count(Color::Yellow, Size::Small),
            Bank::copies_per_piece()
        );
        assert_eq!(state.systems().len(), 2);
    }

    fn valid_system(player: Player, star_color: Color, ship_color: Color) -> StarSystem {
        StarSystem::new(
            vec![Piece::new(star_color, Size::Small)],
            vec![Piece::owned(ship_color, Size::Small, player)],
        )
        .expect("system is valid")
    }

    fn valid_game_state() -> GameState {
        GameState::new(
            vec![
                valid_system(Player::One, Color::Yellow, Color::Green),
                valid_system(Player::Two, Color::Blue, Color::Red),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid")
    }
}
