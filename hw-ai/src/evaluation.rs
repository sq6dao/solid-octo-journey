use hw_core::{Color, GameState, Player, Size, StarSystem, SystemId};
use hw_engine::{Game, GameOutcome, GameStatus};

const WIN_SCORE: i32 = 1_000_000;
const LOSS_SCORE: i32 = -1_000_000;
const DRAW_SCORE: i32 = -500_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Evaluation {
    pub total: i32,
    pub homeworld_safety: i32,
    pub material: i32,
    pub color_access: i32,
    pub pressure: i32,
    pub bank: i32,
}

pub fn evaluate_position(game: &Game, player: Player) -> Evaluation {
    match game.status() {
        GameStatus::Finished(GameOutcome::Winner(winner)) if winner == player => Evaluation {
            total: WIN_SCORE,
            ..Evaluation::default()
        },
        GameStatus::Finished(GameOutcome::Winner(_)) => Evaluation {
            total: LOSS_SCORE,
            ..Evaluation::default()
        },
        GameStatus::Finished(GameOutcome::Draw) => Evaluation {
            total: DRAW_SCORE,
            ..Evaluation::default()
        },
        GameStatus::InProgress => evaluate_non_terminal(game, player),
    }
}

fn evaluate_non_terminal(game: &Game, player: Player) -> Evaluation {
    let state = game.turn().state();
    let mut evaluation = Evaluation {
        total: 0,
        homeworld_safety: homeworld_safety_score(state, player),
        material: material_score(state, player),
        color_access: color_access_score(state, player),
        pressure: pressure_score(state, player),
        bank: bank_score(state, player),
    };
    evaluation.total = evaluation.homeworld_safety
        + evaluation.material
        + evaluation.color_access
        + evaluation.pressure
        + evaluation.bank;
    evaluation
}

fn homeworld_safety_score(state: &GameState, player: Player) -> i32 {
    let Some(homeworld) = state.system(state.homeworld(player)) else {
        return -500;
    };

    let mut score = if homeworld.has_presence(player) {
        100
    } else {
        -400
    };

    for ship in homeworld
        .ships()
        .iter()
        .filter(|ship| ship.is_owned_by(player))
    {
        score += 20 + 10 * size_value(ship.size());
    }

    for ship in homeworld
        .ships()
        .iter()
        .filter(|ship| ship.is_owned_by(other_player(player)))
    {
        score -= 60 + 20 * size_value(ship.size());
    }

    for color in Color::ALL {
        match color_count(homeworld, color) {
            0..=2 => {}
            3 => score -= 60,
            _ => score -= 300,
        }
    }

    score
}

fn material_score(state: &GameState, player: Player) -> i32 {
    state
        .systems()
        .iter()
        .flat_map(StarSystem::ships)
        .map(|ship| {
            if ship.is_owned_by(player) {
                ship_material_value(ship.size())
            } else if ship.is_owned_by(other_player(player)) {
                -ship_material_value(ship.size())
            } else {
                0
            }
        })
        .sum()
}

fn color_access_score(state: &GameState, player: Player) -> i32 {
    player_color_access_score(state, player)
        - player_color_access_score(state, other_player(player))
}

fn player_color_access_score(state: &GameState, player: Player) -> i32 {
    state
        .systems()
        .iter()
        .filter(|system| system.has_presence(player))
        .map(|system| system_action_colors(system, player).len() as i32 * 12)
        .sum()
}

fn pressure_score(state: &GameState, player: Player) -> i32 {
    invasion_threat_score(state, player, other_player(player))
        - invasion_threat_score(state, other_player(player), player)
}

fn invasion_threat_score(state: &GameState, attacker: Player, defender: Player) -> i32 {
    let mut score = 0;

    for (system_index, system) in state.systems().iter().enumerate() {
        if !has_action_power(system, attacker, Color::Red) {
            continue;
        }

        let attacker_ships = system
            .ships()
            .iter()
            .filter(|ship| ship.is_owned_by(attacker))
            .collect::<Vec<_>>();

        for target in system
            .ships()
            .iter()
            .filter(|ship| ship.is_owned_by(defender))
        {
            if !attacker_ships
                .iter()
                .any(|attacker_ship| attacker_ship.size() >= target.size())
            {
                continue;
            }

            score += 25 + 10 * size_value(target.size());
            if SystemId::new(system_index) == state.homeworld(defender) {
                score += 40;
            }
        }
    }

    score
}

fn bank_score(state: &GameState, player: Player) -> i32 {
    player_bank_score(state, player) - player_bank_score(state, other_player(player))
}

fn player_bank_score(state: &GameState, player: Player) -> i32 {
    let colors = global_action_colors(state, player);
    let mut score = 0;

    if colors.contains(&Color::Green) {
        score += total_bank_pieces(state);
    }

    if colors.contains(&Color::Yellow) {
        score += total_bank_pieces(state);
    }

    if colors.contains(&Color::Blue) {
        for ship in state
            .systems()
            .iter()
            .flat_map(StarSystem::ships)
            .filter(|ship| ship.is_owned_by(player))
        {
            for color in Color::ALL {
                if color != ship.color() {
                    score += i32::from(state.bank().count(color, ship.size()));
                }
            }
        }
    }

    score
}

fn total_bank_pieces(state: &GameState) -> i32 {
    Color::ALL
        .into_iter()
        .flat_map(|color| {
            Size::ALL
                .into_iter()
                .map(move |size| i32::from(state.bank().count(color, size)))
        })
        .sum()
}

fn global_action_colors(state: &GameState, player: Player) -> Vec<Color> {
    let mut colors = Vec::new();

    for system in state
        .systems()
        .iter()
        .filter(|system| system.has_presence(player))
    {
        for color in system_action_colors(system, player) {
            push_unique_color(&mut colors, color);
        }
    }

    colors
}

fn system_action_colors(system: &StarSystem, player: Player) -> Vec<Color> {
    let mut colors = Vec::new();

    for star in system.stars() {
        push_unique_color(&mut colors, star.color());
    }

    for ship in system
        .ships()
        .iter()
        .filter(|ship| ship.is_owned_by(player))
    {
        push_unique_color(&mut colors, ship.color());
    }

    colors
}

fn has_action_power(system: &StarSystem, player: Player, color: Color) -> bool {
    system.stars().iter().any(|star| star.color() == color)
        || system
            .ships()
            .iter()
            .any(|ship| ship.is_owned_by(player) && ship.color() == color)
}

fn push_unique_color(colors: &mut Vec<Color>, color: Color) {
    if !colors.contains(&color) {
        colors.push(color);
    }
}

fn color_count(system: &StarSystem, color: Color) -> usize {
    system
        .stars()
        .iter()
        .chain(system.ships())
        .filter(|piece| piece.color() == color)
        .count()
}

const fn size_value(size: Size) -> i32 {
    match size {
        Size::Small => 1,
        Size::Medium => 2,
        Size::Large => 3,
    }
}

const fn ship_material_value(size: Size) -> i32 {
    10 + 5 * size_value(size)
}

const fn other_player(player: Player) -> Player {
    match player {
        Player::One => Player::Two,
        Player::Two => Player::One,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hw_core::{Bank, Color, GameState, Piece, Player, Size, StarSystem, SystemId};
    use hw_engine::{Game, GameOutcome, GameStatus, TurnState};

    #[test]
    fn terminal_statuses_have_decisive_scores() {
        let turn = Game::default(Player::One).turn().clone();
        let win = Game::from_parts(
            turn.clone(),
            GameStatus::Finished(GameOutcome::Winner(Player::One)),
        );
        let loss = Game::from_parts(
            turn.clone(),
            GameStatus::Finished(GameOutcome::Winner(Player::Two)),
        );
        let draw = Game::from_parts(turn, GameStatus::Finished(GameOutcome::Draw));
        let non_terminal = evaluate_position(&Game::default(Player::One), Player::One);

        assert_eq!(
            evaluate_position(&win, Player::One),
            Evaluation {
                total: WIN_SCORE,
                ..Evaluation::default()
            }
        );
        assert_eq!(
            evaluate_position(&loss, Player::One),
            Evaluation {
                total: LOSS_SCORE,
                ..Evaluation::default()
            }
        );
        assert_eq!(
            evaluate_position(&draw, Player::One),
            Evaluation {
                total: DRAW_SCORE,
                ..Evaluation::default()
            }
        );
        assert!(evaluate_position(&win, Player::One).total > non_terminal.total);
        assert!(evaluate_position(&loss, Player::One).total < non_terminal.total);
        assert!(evaluate_position(&draw, Player::One).total < non_terminal.total);
    }

    #[test]
    fn homeworld_safety_rewards_presence_and_defenders() {
        assert_eq!(
            evaluate_position(&defended_homeworld_game(), Player::One).homeworld_safety,
            170
        );
        assert_eq!(
            evaluate_position(&empty_homeworld_game(), Player::One).homeworld_safety,
            -400
        );
        assert_eq!(
            evaluate_position(&overpopulated_homeworld_game(), Player::One).homeworld_safety,
            -80
        );
    }

    #[test]
    fn material_scores_ship_count_and_size_against_the_opponent() {
        assert_eq!(
            evaluate_position(&material_game(), Player::One).material,
            25
        );
        assert_eq!(
            evaluate_position(&material_game(), Player::Two).material,
            -25
        );
    }

    #[test]
    fn color_access_scores_available_action_colors() {
        assert_eq!(
            evaluate_position(&color_access_game(), Player::One).color_access,
            12
        );
        assert_eq!(
            evaluate_position(&color_access_game(), Player::Two).color_access,
            -12
        );
    }

    #[test]
    fn pressure_scores_direct_invasion_threats() {
        assert_eq!(
            evaluate_position(&invasion_pressure_game(), Player::One).pressure,
            85
        );
        assert_eq!(
            evaluate_position(&invasion_pressure_game(), Player::Two).pressure,
            -85
        );
    }

    #[test]
    fn bank_component_tracks_useful_available_pieces() {
        assert_eq!(
            evaluate_position(&bank_access_game(Bank::new()), Player::One).bank,
            36
        );

        let empty_bank =
            Bank::from_counts([[0; Size::COUNT]; Color::COUNT]).expect("counts are valid");
        assert_eq!(
            evaluate_position(&bank_access_game(empty_bank), Player::One).bank,
            0
        );
    }

    #[test]
    fn evaluation_is_deterministic() {
        let game = defended_homeworld_game();

        assert_eq!(
            evaluate_position(&game, Player::One),
            evaluate_position(&game, Player::One)
        );
    }

    fn defended_homeworld_game() -> Game {
        game_from_systems(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Small)],
                    vec![
                        Piece::owned(Color::Green, Size::Small, Player::One),
                        Piece::owned(Color::Blue, Size::Medium, Player::One),
                    ],
                )
                .expect("system is valid"),
                opponent_homeworld(),
            ],
            Bank::new(),
        )
    }

    fn empty_homeworld_game() -> Game {
        game_from_systems(
            vec![
                StarSystem::new(vec![Piece::new(Color::Yellow, Size::Small)], vec![])
                    .expect("system is valid"),
                opponent_homeworld(),
            ],
            Bank::new(),
        )
    }

    fn overpopulated_homeworld_game() -> Game {
        game_from_systems(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Red, Size::Small)],
                    vec![
                        Piece::owned(Color::Red, Size::Small, Player::One),
                        Piece::owned(Color::Red, Size::Medium, Player::One),
                        Piece::owned(Color::Red, Size::Large, Player::One),
                    ],
                )
                .expect("system is valid"),
                opponent_homeworld(),
            ],
            Bank::new(),
        )
    }

    fn material_game() -> Game {
        game_from_systems(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Small)],
                    vec![
                        Piece::owned(Color::Green, Size::Small, Player::One),
                        Piece::owned(Color::Blue, Size::Large, Player::One),
                    ],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Blue, Size::Medium)],
                    vec![Piece::owned(Color::Red, Size::Small, Player::Two)],
                )
                .expect("system is valid"),
            ],
            Bank::new(),
        )
    }

    fn color_access_game() -> Game {
        game_from_systems(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Red, Size::Small)],
                    vec![Piece::owned(Color::Blue, Size::Medium, Player::One)],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Green, Size::Large)],
                    vec![Piece::owned(Color::Green, Size::Small, Player::Two)],
                )
                .expect("system is valid"),
            ],
            Bank::new(),
        )
    }

    fn invasion_pressure_game() -> Game {
        game_from_systems(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Small)],
                    vec![Piece::owned(Color::Green, Size::Small, Player::One)],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Red, Size::Large)],
                    vec![
                        Piece::owned(Color::Blue, Size::Large, Player::One),
                        Piece::owned(Color::Green, Size::Medium, Player::Two),
                    ],
                )
                .expect("system is valid"),
            ],
            Bank::new(),
        )
    }

    fn bank_access_game(bank: Bank) -> Game {
        game_from_systems(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Green, Size::Small)],
                    vec![Piece::owned(Color::Red, Size::Small, Player::One)],
                )
                .expect("system is valid"),
                StarSystem::new(vec![Piece::new(Color::Blue, Size::Medium)], vec![])
                    .expect("system is valid"),
            ],
            bank,
        )
    }

    fn opponent_homeworld() -> StarSystem {
        StarSystem::new(
            vec![Piece::new(Color::Blue, Size::Medium)],
            vec![Piece::owned(Color::Red, Size::Small, Player::Two)],
        )
        .expect("system is valid")
    }

    fn game_from_systems(systems: Vec<StarSystem>, bank: Bank) -> Game {
        let state = GameState::new(systems, [SystemId::new(0), SystemId::new(1)], bank)
            .expect("state is valid");

        Game::from_parts(TurnState::new(state, Player::One), GameStatus::InProgress)
    }
}
