use std::{collections::BTreeMap, fmt, fs, io, path::Path};

use hw_core::{
    Bank, BankError, Color, GameState, GameStateError, Piece, Player, Size, StarSystem,
    StarSystemError, SystemId,
};
use serde::{Deserialize, Serialize};

use crate::{ActionKind, Game, GameOutcome, GameStatus, TurnState};

const SAVE_VERSION: u8 = 1;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SaveExtras {
    pub history: Vec<String>,
    pub commands: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedGame {
    pub game: Game,
    pub history: Vec<String>,
    pub commands: Vec<String>,
}

pub fn to_yaml(game: &Game) -> Result<String, SaveError> {
    serde_norway::to_string(&SaveFile::from_game(game)).map_err(SaveError::Yaml)
}

pub fn to_yaml_with_extras(game: &Game, extras: &SaveExtras) -> Result<String, SaveError> {
    serde_norway::to_string(&SaveFile::from_game_with_extras(game, extras)).map_err(SaveError::Yaml)
}

pub fn from_yaml(input: &str) -> Result<Game, SaveError> {
    let file = serde_norway::from_str::<SaveFile>(input).map_err(SaveError::Yaml)?;
    file.into_game()
}

pub fn from_yaml_with_extras(input: &str) -> Result<SavedGame, SaveError> {
    let file = serde_norway::from_str::<SaveFile>(input).map_err(SaveError::Yaml)?;
    file.into_saved_game()
}

pub fn save_file(game: &Game, path: impl AsRef<Path>) -> Result<(), SaveError> {
    fs::write(path, to_yaml(game)?).map_err(SaveError::Io)
}

pub fn save_file_with_extras(
    game: &Game,
    extras: &SaveExtras,
    path: impl AsRef<Path>,
) -> Result<(), SaveError> {
    fs::write(path, to_yaml_with_extras(game, extras)?).map_err(SaveError::Io)
}

pub fn load_file(path: impl AsRef<Path>) -> Result<Game, SaveError> {
    let input = fs::read_to_string(path).map_err(SaveError::Io)?;
    from_yaml(&input)
}

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Yaml(serde_norway::Error),
    UnsupportedVersion {
        version: u8,
    },
    InvalidPlayers {
        players: Vec<String>,
    },
    InvalidPlayerId {
        value: String,
    },
    InvalidPiece {
        value: String,
    },
    InvalidShip {
        value: String,
    },
    InvalidStatus {
        value: String,
    },
    InvalidActionKind {
        value: String,
    },
    MissingHomeworld {
        player: Player,
    },
    InvalidSystem {
        index: usize,
        error: StarSystemError,
    },
    InvalidState(GameStateError),
    InvalidBank(BankError),
    BankMismatch {
        color: Color,
        size: Size,
        bank: u8,
        board: usize,
        expected_total: u8,
    },
    InvalidTurn {
        message: &'static str,
    },
}

impl fmt::Display for SaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Yaml(error) => write!(formatter, "YAML error: {error}"),
            Self::UnsupportedVersion { version } => {
                write!(formatter, "unsupported save version {version}")
            }
            Self::InvalidPlayers { players } => write!(
                formatter,
                "v1 saves must list exactly p1 and p2, got {players:?}"
            ),
            Self::InvalidPlayerId { value } => write!(formatter, "invalid player id `{value}`"),
            Self::InvalidPiece { value } => write!(formatter, "invalid piece `{value}`"),
            Self::InvalidShip { value } => write!(formatter, "invalid ship `{value}`"),
            Self::InvalidStatus { value } => write!(formatter, "invalid status `{value}`"),
            Self::InvalidActionKind { value } => {
                write!(formatter, "invalid required action `{value}`")
            }
            Self::MissingHomeworld { player } => {
                write!(formatter, "missing homeworld for {}", player_id(*player))
            }
            Self::InvalidSystem { index, error } => {
                write!(formatter, "invalid system {index}: {error:?}")
            }
            Self::InvalidState(error) => write!(formatter, "invalid game state: {error:?}"),
            Self::InvalidBank(error) => write!(formatter, "invalid bank: {error:?}"),
            Self::BankMismatch {
                color,
                size,
                bank,
                board,
                expected_total,
            } => write!(
                formatter,
                "bank mismatch for {} {}: bank {bank} + board {board} != {expected_total}",
                color_name(*color),
                size_name(*size)
            ),
            Self::InvalidTurn { message } => write!(formatter, "invalid turn: {message}"),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Yaml(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SaveFile {
    version: u8,
    players: Vec<String>,
    turn: SaveTurn,
    status: SaveStatus,
    homeworlds: BTreeMap<String, usize>,
    bank: SaveBank,
    systems: Vec<SaveSystem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    history: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    commands: Vec<String>,
}

impl SaveFile {
    fn from_game(game: &Game) -> Self {
        Self::from_game_with_extras(game, &SaveExtras::default())
    }

    fn from_game_with_extras(game: &Game, extras: &SaveExtras) -> Self {
        let state = game.turn().state();
        let homeworlds = Player::ALL
            .into_iter()
            .map(|player| {
                (
                    player_id(player).to_owned(),
                    state.homeworld(player).index(),
                )
            })
            .collect();

        Self {
            version: SAVE_VERSION,
            players: Player::ALL
                .into_iter()
                .map(|player| player_id(player).to_owned())
                .collect(),
            turn: SaveTurn::from_turn(game.turn()),
            status: SaveStatus::from_status(game.status()),
            homeworlds,
            bank: SaveBank::from_bank(state.bank()),
            systems: state
                .systems()
                .iter()
                .map(SaveSystem::from_system)
                .collect(),
            history: extras.history.clone(),
            commands: extras.commands.clone(),
        }
    }

    fn into_game(self) -> Result<Game, SaveError> {
        if self.version != SAVE_VERSION {
            return Err(SaveError::UnsupportedVersion {
                version: self.version,
            });
        }
        validate_players(&self.players)?;

        for player in self.homeworlds.keys() {
            parse_player_id(player)?;
        }

        let systems = self
            .systems
            .iter()
            .enumerate()
            .map(|(index, system)| system.to_system(index))
            .collect::<Result<Vec<_>, _>>()?;
        let bank = self.bank.to_bank()?;
        verify_bank(&systems, &bank)?;

        let homeworlds = [
            homeworld(&self.homeworlds, Player::One)?,
            homeworld(&self.homeworlds, Player::Two)?,
        ];
        let state = GameState::new(systems, homeworlds, bank).map_err(SaveError::InvalidState)?;
        let turn = self.turn.to_turn(state)?;
        let status = self.status.to_status()?;

        Ok(Game::from_parts(turn, status))
    }

    fn into_saved_game(self) -> Result<SavedGame, SaveError> {
        let history = self.history.clone();
        let commands = self.commands.clone();
        let game = self.into_game()?;

        Ok(SavedGame {
            game,
            history,
            commands,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SaveTurn {
    current_player: String,
    remaining_actions: usize,
    required_action: Option<String>,
}

impl SaveTurn {
    fn from_turn(turn: &TurnState) -> Self {
        Self {
            current_player: player_id(turn.current_player()).to_owned(),
            remaining_actions: turn.remaining_actions(),
            required_action: turn
                .required_action()
                .map(action_kind_name)
                .map(str::to_owned),
        }
    }

    fn to_turn(&self, state: GameState) -> Result<TurnState, SaveError> {
        let current_player = parse_player_id(&self.current_player)?;
        let required_action = self
            .required_action
            .as_deref()
            .map(parse_required_action_kind)
            .transpose()?;

        validate_turn_parts(self.remaining_actions, required_action)?;

        Ok(TurnState::from_parts(
            state,
            current_player,
            self.remaining_actions,
            required_action,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
enum SaveStatus {
    Name(String),
    Winner { winner: String },
}

impl SaveStatus {
    fn from_status(status: GameStatus) -> Self {
        match status {
            GameStatus::InProgress => Self::Name("in_progress".to_owned()),
            GameStatus::Finished(GameOutcome::Draw) => Self::Name("draw".to_owned()),
            GameStatus::Finished(GameOutcome::Winner(player)) => Self::Winner {
                winner: player_id(player).to_owned(),
            },
        }
    }

    fn to_status(&self) -> Result<GameStatus, SaveError> {
        match self {
            Self::Name(name) if name == "in_progress" => Ok(GameStatus::InProgress),
            Self::Name(name) if name == "draw" => Ok(GameStatus::Finished(GameOutcome::Draw)),
            Self::Name(value) => Err(SaveError::InvalidStatus {
                value: value.clone(),
            }),
            Self::Winner { winner } => Ok(GameStatus::Finished(GameOutcome::Winner(
                parse_player_id(winner)?,
            ))),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SaveSystem {
    stars: Vec<String>,
    ships: Vec<String>,
}

impl SaveSystem {
    fn from_system(system: &StarSystem) -> Self {
        Self {
            stars: system
                .stars()
                .iter()
                .map(|piece| piece_code(*piece))
                .collect(),
            ships: system.ships().iter().map(|ship| ship_code(*ship)).collect(),
        }
    }

    fn to_system(&self, index: usize) -> Result<StarSystem, SaveError> {
        let stars = self
            .stars
            .iter()
            .map(|piece| parse_piece(piece).map(|(color, size)| Piece::new(color, size)))
            .collect::<Result<Vec<_>, _>>()?;
        let ships = self
            .ships
            .iter()
            .map(|ship| parse_ship(ship))
            .collect::<Result<Vec<_>, _>>()?;

        StarSystem::new(stars, ships).map_err(|error| SaveError::InvalidSystem { index, error })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SaveBank {
    red: SaveCounts,
    yellow: SaveCounts,
    green: SaveCounts,
    blue: SaveCounts,
}

impl SaveBank {
    fn from_bank(bank: &Bank) -> Self {
        Self {
            red: SaveCounts::from_bank(bank, Color::Red),
            yellow: SaveCounts::from_bank(bank, Color::Yellow),
            green: SaveCounts::from_bank(bank, Color::Green),
            blue: SaveCounts::from_bank(bank, Color::Blue),
        }
    }

    fn count(&self, color: Color, size: Size) -> u8 {
        let counts = match color {
            Color::Red => self.red,
            Color::Yellow => self.yellow,
            Color::Green => self.green,
            Color::Blue => self.blue,
        };
        counts.count(size)
    }

    fn to_bank(&self) -> Result<Bank, SaveError> {
        let mut counts = [[0; Size::COUNT]; Color::COUNT];
        for color in Color::ALL {
            for size in Size::ALL {
                counts[color_slot(color)][size_slot(size)] = self.count(color, size);
            }
        }

        Bank::from_counts(counts).map_err(SaveError::InvalidBank)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SaveCounts {
    small: u8,
    medium: u8,
    large: u8,
}

impl SaveCounts {
    fn from_bank(bank: &Bank, color: Color) -> Self {
        Self {
            small: bank.count(color, Size::Small),
            medium: bank.count(color, Size::Medium),
            large: bank.count(color, Size::Large),
        }
    }

    fn count(self, size: Size) -> u8 {
        match size {
            Size::Small => self.small,
            Size::Medium => self.medium,
            Size::Large => self.large,
        }
    }
}

fn validate_players(players: &[String]) -> Result<(), SaveError> {
    let parsed = players
        .iter()
        .map(|player| parse_player_id(player))
        .collect::<Result<Vec<_>, _>>()?;
    if parsed.len() != Player::COUNT
        || !parsed.contains(&Player::One)
        || !parsed.contains(&Player::Two)
    {
        return Err(SaveError::InvalidPlayers {
            players: players.to_vec(),
        });
    }

    Ok(())
}

fn homeworld(homeworlds: &BTreeMap<String, usize>, player: Player) -> Result<SystemId, SaveError> {
    let key = player_id(player);
    homeworlds
        .get(key)
        .copied()
        .map(SystemId::new)
        .ok_or(SaveError::MissingHomeworld { player })
}

fn validate_turn_parts(
    remaining_actions: usize,
    required_action: Option<ActionKind>,
) -> Result<(), SaveError> {
    if remaining_actions > 3 {
        return Err(SaveError::InvalidTurn {
            message: "remaining actions cannot exceed 3",
        });
    }

    if remaining_actions == 0 && required_action.is_some() {
        return Err(SaveError::InvalidTurn {
            message: "required action must be null when no actions remain",
        });
    }

    if remaining_actions > 1 && required_action.is_none() {
        return Err(SaveError::InvalidTurn {
            message: "multi-action turns require a required action",
        });
    }

    Ok(())
}

fn verify_bank(systems: &[StarSystem], bank: &Bank) -> Result<(), SaveError> {
    for color in Color::ALL {
        for size in Size::ALL {
            let board = systems
                .iter()
                .flat_map(|system| system.stars().iter().chain(system.ships()))
                .filter(|piece| piece.color() == color && piece.size() == size)
                .count();
            let bank_count = bank.count(color, size);
            let expected_total = Bank::copies_per_piece();

            if usize::from(bank_count) + board != usize::from(expected_total) {
                return Err(SaveError::BankMismatch {
                    color,
                    size,
                    bank: bank_count,
                    board,
                    expected_total,
                });
            }
        }
    }

    Ok(())
}

fn parse_ship(value: &str) -> Result<Piece, SaveError> {
    let Some((player, piece)) = value.split_once(':') else {
        return Err(SaveError::InvalidShip {
            value: value.to_owned(),
        });
    };
    let player = parse_player_id(player)?;
    let (color, size) = parse_piece(piece).map_err(|_| SaveError::InvalidShip {
        value: value.to_owned(),
    })?;

    Ok(Piece::owned(color, size, player))
}

fn parse_piece(value: &str) -> Result<(Color, Size), SaveError> {
    let normalized = value.to_ascii_lowercase();
    let mut chars = normalized.chars();
    let color = chars
        .next()
        .and_then(parse_color)
        .ok_or_else(|| SaveError::InvalidPiece {
            value: value.to_owned(),
        })?;
    let size = chars
        .next()
        .and_then(parse_size)
        .ok_or_else(|| SaveError::InvalidPiece {
            value: value.to_owned(),
        })?;
    if chars.next().is_some() {
        return Err(SaveError::InvalidPiece {
            value: value.to_owned(),
        });
    }

    Ok((color, size))
}

fn parse_player_id(value: &str) -> Result<Player, SaveError> {
    match value.to_ascii_lowercase().as_str() {
        "p1" => Ok(Player::One),
        "p2" => Ok(Player::Two),
        _ => Err(SaveError::InvalidPlayerId {
            value: value.to_owned(),
        }),
    }
}

fn parse_required_action_kind(value: &str) -> Result<ActionKind, SaveError> {
    match value.to_ascii_lowercase().as_str() {
        "build" => Ok(ActionKind::Build),
        "travel" => Ok(ActionKind::Travel),
        "trade" => Ok(ActionKind::Trade),
        "invade" => Ok(ActionKind::Invade),
        _ => Err(SaveError::InvalidActionKind {
            value: value.to_owned(),
        }),
    }
}

fn parse_color(value: char) -> Option<Color> {
    match value {
        'r' => Some(Color::Red),
        'y' => Some(Color::Yellow),
        'g' => Some(Color::Green),
        'b' => Some(Color::Blue),
        _ => None,
    }
}

fn parse_size(value: char) -> Option<Size> {
    match value {
        's' => Some(Size::Small),
        'm' => Some(Size::Medium),
        'l' => Some(Size::Large),
        _ => None,
    }
}

fn player_id(player: Player) -> &'static str {
    match player {
        Player::One => "p1",
        Player::Two => "p2",
    }
}

fn piece_code(piece: Piece) -> String {
    format!("{}{}", color_code(piece.color()), size_code(piece.size()))
}

fn ship_code(ship: Piece) -> String {
    match ship.owner() {
        Some(player) => format!("{}:{}", player_id(player), piece_code(ship)),
        None => piece_code(ship),
    }
}

fn action_kind_name(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::Build => "build",
        ActionKind::Travel => "travel",
        ActionKind::Trade => "trade",
        ActionKind::Invade => "invade",
        ActionKind::Sacrifice => "sacrifice",
        ActionKind::Catastrophe => "catastrophe",
    }
}

fn color_code(color: Color) -> &'static str {
    match color {
        Color::Red => "r",
        Color::Yellow => "y",
        Color::Green => "g",
        Color::Blue => "b",
    }
}

fn size_code(size: Size) -> &'static str {
    match size {
        Size::Small => "s",
        Size::Medium => "m",
        Size::Large => "l",
    }
}

fn color_name(color: Color) -> &'static str {
    match color {
        Color::Red => "red",
        Color::Yellow => "yellow",
        Color::Green => "green",
        Color::Blue => "blue",
    }
}

fn size_name(size: Size) -> &'static str {
    match size {
        Size::Small => "small",
        Size::Medium => "medium",
        Size::Large => "large",
    }
}

fn color_slot(color: Color) -> usize {
    match color {
        Color::Red => 0,
        Color::Yellow => 1,
        Color::Green => 2,
        Color::Blue => 3,
    }
}

fn size_slot(size: Size) -> usize {
    match size {
        Size::Small => 0,
        Size::Medium => 1,
        Size::Large => 2,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::Action;

    #[test]
    fn default_game_round_trips_through_yaml() {
        let game = Game::default(Player::One);

        let yaml = to_yaml(&game).expect("game serializes");
        let loaded = from_yaml(&yaml).expect("game deserializes");

        assert_eq!(loaded, game);
        assert!(yaml.contains("version: 1"));
        assert!(yaml.contains("players:"));
        assert!(yaml.contains("current_player: p1"));
        assert!(yaml.contains("homeworlds:"));
        assert!(yaml.contains("systems:"));
        assert!(!yaml.contains("history:"));
        assert!(!yaml.contains("commands:"));
    }

    #[test]
    fn metadata_round_trips_through_yaml() {
        let game = Game::default(Player::One);
        let extras = SaveExtras {
            history: vec!["ys bm".to_owned(), "gs".to_owned(), "show".to_owned()],
            commands: vec!["show".to_owned(), "b 0 gs;".to_owned()],
        };

        let yaml = to_yaml_with_extras(&game, &extras).expect("game serializes");
        let loaded = from_yaml_with_extras(&yaml).expect("game loads");

        assert_eq!(loaded.game, game);
        assert_eq!(loaded.history, extras.history);
        assert_eq!(loaded.commands, extras.commands);
        assert!(yaml.contains("history:"));
        assert!(yaml.contains("commands:"));
        assert!(
            yaml.find("systems:").expect("systems field exists")
                < yaml.find("history:").expect("history field exists")
        );
        assert!(
            yaml.find("history:").expect("history field exists")
                < yaml.find("commands:").expect("commands field exists")
        );
    }

    #[test]
    fn game_only_loader_ignores_metadata() {
        let game = Game::default(Player::One);
        let yaml = to_yaml_with_extras(
            &game,
            &SaveExtras {
                history: vec!["show".to_owned()],
                commands: vec!["b 0 gs".to_owned()],
            },
        )
        .expect("game serializes");

        assert_eq!(from_yaml(&yaml).expect("game loads"), game);
    }

    #[test]
    fn empty_metadata_is_omitted_from_yaml() {
        let game = Game::default(Player::One);
        let yaml = to_yaml_with_extras(&game, &SaveExtras::default()).expect("game serializes");

        assert!(!yaml.contains("history:"));
        assert!(!yaml.contains("commands:"));
    }

    #[test]
    fn spent_action_round_trips_through_yaml() {
        let game = Game::default(Player::One)
            .apply_action(&Action::Build {
                player: Player::One,
                system: SystemId::new(0),
                ship: Piece::owned(Color::Green, Size::Small, Player::One),
            })
            .expect("action applies");

        let loaded = from_yaml(&to_yaml(&game).expect("game serializes")).expect("game loads");

        assert_eq!(loaded.turn().remaining_actions(), 0);
        assert_eq!(loaded, game);
    }

    #[test]
    fn sacrifice_turn_round_trips_required_action() {
        let game = Game::default(Player::One)
            .apply_action(&Action::Sacrifice {
                player: Player::One,
                system: SystemId::new(0),
                ship: Piece::owned(Color::Green, Size::Small, Player::One),
            })
            .expect("sacrifice applies");

        let yaml = to_yaml(&game).expect("game serializes");
        let loaded = from_yaml(&yaml).expect("game loads");

        assert!(yaml.contains("required_action: build"));
        assert_eq!(loaded.turn().required_action(), Some(ActionKind::Build));
        assert_eq!(loaded, game);
    }

    #[test]
    fn terminal_statuses_round_trip() {
        let winner = Game::from_parts(
            TurnState::new(
                Game::default(Player::One).turn().state().clone(),
                Player::One,
            ),
            GameStatus::Finished(GameOutcome::Winner(Player::Two)),
        );
        let draw = Game::from_parts(
            TurnState::new(
                Game::default(Player::One).turn().state().clone(),
                Player::One,
            ),
            GameStatus::Finished(GameOutcome::Draw),
        );

        assert_eq!(
            from_yaml(&to_yaml(&winner).expect("winner serializes")).expect("winner loads"),
            winner
        );
        assert_eq!(
            from_yaml(&to_yaml(&draw).expect("draw serializes")).expect("draw loads"),
            draw
        );
    }

    #[test]
    fn pending_catastrophe_state_round_trips() {
        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Blue, Size::Medium),
                    ],
                    vec![
                        Piece::owned(Color::Red, Size::Medium, Player::One),
                        Piece::owned(Color::Red, Size::Large, Player::Two),
                        Piece::owned(Color::Green, Size::Small, Player::One),
                    ],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![
                        Piece::new(Color::Yellow, Size::Small),
                        Piece::new(Color::Blue, Size::Large),
                    ],
                    vec![Piece::owned(Color::Yellow, Size::Medium, Player::Two)],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            bank_for_systems(&[
                (
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Blue, Size::Medium),
                    ],
                    vec![
                        Piece::owned(Color::Red, Size::Medium, Player::One),
                        Piece::owned(Color::Red, Size::Large, Player::Two),
                        Piece::owned(Color::Green, Size::Small, Player::One),
                    ],
                ),
                (
                    vec![
                        Piece::new(Color::Yellow, Size::Small),
                        Piece::new(Color::Blue, Size::Large),
                    ],
                    vec![Piece::owned(Color::Yellow, Size::Medium, Player::Two)],
                ),
            ]),
        )
        .expect("state is valid");
        let game = Game::from_parts(TurnState::new(state, Player::One), GameStatus::InProgress);

        let loaded = from_yaml(&to_yaml(&game).expect("game serializes")).expect("game loads");

        assert_eq!(loaded, game);
    }

    #[test]
    fn load_rejects_unsupported_versions() {
        let yaml = to_yaml(&Game::default(Player::One)).expect("game serializes");
        let yaml = yaml.replacen("version: 1", "version: 2", 1);

        assert!(matches!(
            from_yaml(&yaml),
            Err(SaveError::UnsupportedVersion { version: 2 })
        ));
    }

    #[test]
    fn load_rejects_malformed_pieces() {
        let yaml = default_yaml().replacen("[ys, bm]", "[purple, bm]", 1);

        assert!(matches!(
            from_yaml(&yaml),
            Err(SaveError::InvalidPiece { .. })
        ));
    }

    #[test]
    fn load_rejects_unknown_players() {
        let yaml = to_yaml(&Game::default(Player::One)).expect("game serializes");
        let yaml = yaml.replacen("p1", "p3", 1);

        assert!(matches!(
            from_yaml(&yaml),
            Err(SaveError::InvalidPlayerId { .. })
        ));
    }

    #[test]
    fn load_rejects_bad_homeworld_ids() {
        let yaml = to_yaml(&Game::default(Player::One)).expect("game serializes");
        let yaml = yaml.replacen("p2: 1", "p2: 99", 1);

        assert!(matches!(
            from_yaml(&yaml),
            Err(SaveError::InvalidState(
                GameStateError::HomeworldOutOfRange { .. }
            ))
        ));
    }

    #[test]
    fn load_rejects_invalid_star_ownership() {
        let yaml = default_yaml().replacen("[ys, bm]", "[p1:ys, bm]", 1);

        assert!(matches!(
            from_yaml(&yaml),
            Err(SaveError::InvalidPiece { .. })
        ));
    }

    #[test]
    fn load_rejects_mismatched_bank_counts() {
        let yaml = to_yaml(&Game::default(Player::One)).expect("game serializes");
        let yaml = yaml.replacen("small: 2", "small: 3", 1);

        assert!(matches!(
            from_yaml(&yaml),
            Err(SaveError::BankMismatch { .. })
        ));
    }

    #[test]
    fn load_rejects_invalid_turn_parts() {
        let yaml = to_yaml(&Game::default(Player::One)).expect("game serializes");
        let yaml = yaml.replacen("remaining_actions: 1", "remaining_actions: 4", 1);

        assert!(matches!(
            from_yaml(&yaml),
            Err(SaveError::InvalidTurn { .. })
        ));
    }

    #[test]
    fn file_helpers_save_and_load_yaml() {
        let game = Game::default(Player::One);
        let path = temp_save_path("file_helpers_save_and_load_yaml");

        save_file(&game, &path).expect("game saves");
        let loaded = load_file(&path).expect("game loads");
        let _ = fs::remove_file(path);

        assert_eq!(loaded, game);
    }

    fn bank_for_systems(systems: &[(Vec<Piece>, Vec<Piece>)]) -> Bank {
        let mut counts = [[Bank::copies_per_piece(); Size::COUNT]; Color::COUNT];
        for piece in systems
            .iter()
            .flat_map(|(stars, ships)| stars.iter().chain(ships))
        {
            counts[color_slot(piece.color())][size_slot(piece.size())] -= 1;
        }

        Bank::from_counts(counts).expect("bank counts are valid")
    }

    fn temp_save_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("homeworlds-rs-{name}-{}.yaml", std::process::id()))
    }

    fn default_yaml() -> String {
        "version: 1
players: [p1, p2]
turn:
  current_player: p1
  remaining_actions: 1
  required_action: null
status: in_progress
homeworlds:
  p1: 0
  p2: 1
bank:
  red: { small: 2, medium: 2, large: 3 }
  yellow: { small: 2, medium: 3, large: 3 }
  green: { small: 2, medium: 3, large: 3 }
  blue: { small: 3, medium: 2, large: 2 }
systems:
  - stars: [ys, bm]
    ships: [\"p1:gs\"]
  - stars: [rs, bl]
    ships: [\"p2:rm\"]
"
        .to_owned()
    }
}
