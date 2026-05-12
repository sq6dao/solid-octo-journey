#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Color {
    Red,
    Yellow,
    Green,
    Blue,
}

impl Color {
    pub const COUNT: usize = 4;
    pub const ALL: [Self; Self::COUNT] = [Self::Red, Self::Yellow, Self::Green, Self::Blue];

    const fn index(self) -> usize {
        match self {
            Self::Red => 0,
            Self::Yellow => 1,
            Self::Green => 2,
            Self::Blue => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    pub const COUNT: usize = 3;
    pub const ALL: [Self; Self::COUNT] = [Self::Small, Self::Medium, Self::Large];

    const fn index(self) -> usize {
        match self {
            Self::Small => 0,
            Self::Medium => 1,
            Self::Large => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Player {
    One,
    Two,
}

impl Player {
    pub const COUNT: usize = 2;
    pub const ALL: [Self; Self::COUNT] = [Self::One, Self::Two];

    const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }
}

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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Piece {
    color: Color,
    size: Size,
    owner: Option<Player>,
}

impl Piece {
    pub const fn new(color: Color, size: Size) -> Self {
        Self {
            color,
            size,
            owner: None,
        }
    }

    pub const fn owned(color: Color, size: Size, owner: Player) -> Self {
        Self {
            color,
            size,
            owner: Some(owner),
        }
    }

    pub const fn color(&self) -> Color {
        self.color
    }

    pub const fn size(&self) -> Size {
        self.size
    }

    pub const fn owner(&self) -> Option<Player> {
        self.owner
    }

    pub const fn is_owned(&self) -> bool {
        self.owner.is_some()
    }

    pub fn is_owned_by(&self, player: Player) -> bool {
        self.owner == Some(player)
    }

    pub const fn with_owner(self, owner: Player) -> Self {
        Self {
            owner: Some(owner),
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BankError {
    PieceUnavailable,
    OwnedPiece,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bank {
    counts: [[u8; Size::COUNT]; Color::COUNT],
}

impl Bank {
    pub fn new() -> Self {
        Self {
            counts: [[Self::copies_per_piece(); Size::COUNT]; Color::COUNT],
        }
    }

    pub const fn copies_per_piece() -> u8 {
        Player::COUNT as u8 + 1
    }

    pub const fn count(&self, color: Color, size: Size) -> u8 {
        self.counts[color.index()][size.index()]
    }

    pub fn draw(&mut self, color: Color, size: Size) -> Result<Piece, BankError> {
        let count = &mut self.counts[color.index()][size.index()];
        if *count == 0 {
            return Err(BankError::PieceUnavailable);
        }

        *count -= 1;
        Ok(Piece::new(color, size))
    }

    pub fn return_piece(&mut self, piece: Piece) -> Result<(), BankError> {
        if piece.is_owned() {
            return Err(BankError::OwnedPiece);
        }

        self.counts[piece.color().index()][piece.size().index()] += 1;
        Ok(())
    }
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StarSystemError {
    TooManyStars,
    OwnedStar,
    NoShips,
    UnownedShip,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StarSystem {
    stars: Vec<Piece>,
    ships: Vec<Piece>,
}

impl StarSystem {
    pub fn new(stars: Vec<Piece>, ships: Vec<Piece>) -> Result<Self, StarSystemError> {
        if stars.len() > 2 {
            return Err(StarSystemError::TooManyStars);
        }

        if stars.iter().any(Piece::is_owned) {
            return Err(StarSystemError::OwnedStar);
        }

        if ships.is_empty() {
            return Err(StarSystemError::NoShips);
        }

        if ships.iter().any(|ship| !ship.is_owned()) {
            return Err(StarSystemError::UnownedShip);
        }

        Ok(Self { stars, ships })
    }

    pub fn stars(&self) -> &[Piece] {
        &self.stars
    }

    pub fn ships(&self) -> &[Piece] {
        &self.ships
    }

    pub fn has_presence(&self, player: Player) -> bool {
        self.ships.iter().any(|ship| ship.is_owned_by(player))
    }

    pub fn owners_present(&self) -> Vec<Player> {
        Player::ALL
            .into_iter()
            .filter(|player| self.has_presence(*player))
            .collect()
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

    #[test]
    fn colors_are_the_four_homeworlds_colors() {
        assert_eq!(
            Color::ALL,
            [Color::Red, Color::Yellow, Color::Green, Color::Blue]
        );
    }

    #[test]
    fn sizes_are_the_three_piece_sizes() {
        assert_eq!(Size::ALL, [Size::Small, Size::Medium, Size::Large]);
    }

    #[test]
    fn players_are_the_two_supported_seats() {
        assert_eq!(Player::ALL, [Player::One, Player::Two]);
    }

    #[test]
    fn system_id_preserves_its_index() {
        let id = SystemId::new(3);

        assert_eq!(id.index(), 3);
    }

    #[test]
    fn piece_can_be_constructed_without_an_owner() {
        let piece = Piece::new(Color::Blue, Size::Large);

        assert_eq!(piece.color(), Color::Blue);
        assert_eq!(piece.size(), Size::Large);
        assert_eq!(piece.owner(), None);
        assert!(!piece.is_owned());
    }

    #[test]
    fn piece_can_be_constructed_with_a_player_owner() {
        let piece = Piece::owned(Color::Green, Size::Small, Player::Two);

        assert_eq!(piece.color(), Color::Green);
        assert_eq!(piece.size(), Size::Small);
        assert_eq!(piece.owner(), Some(Player::Two));
        assert!(piece.is_owned());
        assert!(piece.is_owned_by(Player::Two));
        assert!(!piece.is_owned_by(Player::One));
    }

    #[test]
    fn piece_owner_can_be_changed_without_changing_identity() {
        let piece = Piece::new(Color::Red, Size::Medium).with_owner(Player::One);

        assert_eq!(piece.color(), Color::Red);
        assert_eq!(piece.size(), Size::Medium);
        assert_eq!(piece.owner(), Some(Player::One));
    }

    #[test]
    fn bank_starts_with_player_count_plus_one_copies_of_every_piece() {
        let bank = Bank::new();

        assert_eq!(Bank::copies_per_piece(), 3);

        for color in Color::ALL {
            for size in Size::ALL {
                assert_eq!(bank.count(color, size), Bank::copies_per_piece());
            }
        }
    }

    #[test]
    fn drawing_a_piece_returns_it_unowned_and_reduces_its_count() {
        let mut bank = Bank::new();

        let piece = bank.draw(Color::Yellow, Size::Small).expect("piece exists");

        assert_eq!(piece, Piece::new(Color::Yellow, Size::Small));
        assert_eq!(
            bank.count(Color::Yellow, Size::Small),
            Bank::copies_per_piece() - 1
        );
    }

    #[test]
    fn drawing_more_than_available_returns_an_error() {
        let mut bank = Bank::new();

        for _ in 0..Bank::copies_per_piece() {
            bank.draw(Color::Red, Size::Large).expect("piece exists");
        }

        assert_eq!(
            bank.draw(Color::Red, Size::Large),
            Err(BankError::PieceUnavailable)
        );
        assert_eq!(bank.count(Color::Red, Size::Large), 0);
    }

    #[test]
    fn returning_an_unowned_piece_increases_its_count() {
        let mut bank = Bank::new();
        let piece = bank.draw(Color::Green, Size::Medium).expect("piece exists");

        bank.return_piece(piece).expect("unowned piece can return");

        assert_eq!(
            bank.count(Color::Green, Size::Medium),
            Bank::copies_per_piece()
        );
    }

    #[test]
    fn returning_an_owned_piece_is_rejected_without_changing_counts() {
        let mut bank = Bank::new();
        let piece = Piece::owned(Color::Blue, Size::Large, Player::One);

        assert_eq!(bank.return_piece(piece), Err(BankError::OwnedPiece));
        assert_eq!(
            bank.count(Color::Blue, Size::Large),
            Bank::copies_per_piece()
        );
    }

    #[test]
    fn star_system_can_be_constructed_with_one_star_and_one_ship() {
        let star = Piece::new(Color::Yellow, Size::Medium);
        let ship = Piece::owned(Color::Blue, Size::Small, Player::One);

        let system = StarSystem::new(vec![star], vec![ship]).expect("system is valid");

        assert_eq!(system.stars(), &[star]);
        assert_eq!(system.ships(), &[ship]);
    }

    #[test]
    fn star_system_can_be_constructed_with_two_stars() {
        let stars = vec![
            Piece::new(Color::Yellow, Size::Medium),
            Piece::new(Color::Blue, Size::Large),
        ];
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];

        let system = StarSystem::new(stars.clone(), ships).expect("system is valid");

        assert_eq!(system.stars(), stars.as_slice());
    }

    #[test]
    fn star_system_can_be_constructed_with_zero_stars() {
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];
        let system = StarSystem::new(vec![], ships).expect("system is valid");

        assert!(system.stars().is_empty());
    }

    #[test]
    fn star_system_rejects_more_than_two_stars() {
        let stars = vec![
            Piece::new(Color::Yellow, Size::Small),
            Piece::new(Color::Blue, Size::Medium),
            Piece::new(Color::Red, Size::Large),
        ];
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];

        assert_eq!(
            StarSystem::new(stars, ships),
            Err(StarSystemError::TooManyStars)
        );
    }

    #[test]
    fn star_system_rejects_owned_stars() {
        let stars = vec![Piece::owned(Color::Yellow, Size::Small, Player::One)];
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];

        assert_eq!(
            StarSystem::new(stars, ships),
            Err(StarSystemError::OwnedStar)
        );
    }

    #[test]
    fn star_system_rejects_zero_ships() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];

        assert_eq!(
            StarSystem::new(stars, vec![]),
            Err(StarSystemError::NoShips)
        );
    }

    #[test]
    fn star_system_rejects_unowned_ships() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];
        let ships = vec![Piece::new(Color::Green, Size::Small)];

        assert_eq!(
            StarSystem::new(stars, ships),
            Err(StarSystemError::UnownedShip)
        );
    }

    #[test]
    fn star_system_detects_player_presence() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];
        let system = StarSystem::new(stars, ships).expect("system is valid");

        assert!(system.has_presence(Player::One));
        assert!(!system.has_presence(Player::Two));
    }

    #[test]
    fn star_system_reports_both_players_when_both_have_ships() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];
        let ships = vec![
            Piece::owned(Color::Green, Size::Small, Player::Two),
            Piece::owned(Color::Blue, Size::Large, Player::One),
        ];
        let system = StarSystem::new(stars, ships).expect("system is valid");

        assert_eq!(system.owners_present(), vec![Player::One, Player::Two]);
    }

    #[test]
    fn star_system_reports_each_player_once() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];
        let ships = vec![
            Piece::owned(Color::Green, Size::Small, Player::One),
            Piece::owned(Color::Blue, Size::Large, Player::One),
        ];
        let system = StarSystem::new(stars, ships).expect("system is valid");

        assert_eq!(system.owners_present(), vec![Player::One]);
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
