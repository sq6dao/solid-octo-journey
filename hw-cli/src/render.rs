use hw_core::{Color, Piece, Player, Size};
use hw_engine::{Game, GameOutcome, GameStatus};

pub fn render_game(game: &Game) -> String {
    let mut output = String::new();

    push_line(
        &mut output,
        &format!("Status: {}", status_label(game.status())),
    );
    push_line(
        &mut output,
        &format!(
            "Current player: {}",
            player_label(game.turn().current_player())
        ),
    );
    push_line(
        &mut output,
        &format!("Remaining actions: {}", game.turn().remaining_actions()),
    );
    push_line(&mut output, "Systems:");

    let state = game.turn().state();
    for (index, system) in state.systems().iter().enumerate() {
        let system_id = hw_core::SystemId::new(index);
        push_line(
            &mut output,
            &format!("[{index}] {}", system_label(game, system_id)),
        );
        push_line(
            &mut output,
            &format!("  Stars: {}", render_piece_list(system.stars(), false)),
        );
        push_line(
            &mut output,
            &format!("  Ships: {}", render_piece_list(system.ships(), true)),
        );
    }

    push_line(&mut output, "Bank:");
    for color in Color::ALL {
        let counts = Size::ALL
            .into_iter()
            .map(|size| format!("{}={}", size_label(size), state.bank().count(color, size)))
            .collect::<Vec<_>>()
            .join(" ");
        push_line(&mut output, &format!("  {}: {counts}", color_label(color)));
    }

    output
}

pub fn render_turn_summary(game: &Game) -> String {
    match game.status() {
        GameStatus::InProgress => format!(
            "Current player: {}\nRemaining actions: {}\n",
            player_label(game.turn().current_player()),
            game.turn().remaining_actions()
        ),
        GameStatus::Finished(outcome) => {
            format!("Status: {}\n", status_label(GameStatus::Finished(outcome)))
        }
    }
}

pub const fn render_help() -> &'static str {
    "Commands:
  show | s
  help | h
  end | e
  quit | q
  build | b <system> <piece>
  travel | t <from> <piece> existing | x <to>
  travel | t <from> <piece> new | n <star> [<star>]
  trade | tr | x <system> <from-piece> <to-piece>
  sacrifice | sac | s <system> <piece>
  invade | i <system> <your-piece> <target-piece>
  catastrophe | c <system> <color>

Pieces use color then size, for example gs, red large, or yellow small.
"
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

fn status_label(status: GameStatus) -> String {
    match status {
        GameStatus::InProgress => "in progress".to_owned(),
        GameStatus::Finished(GameOutcome::Winner(player)) => {
            format!("finished, winner {}", player_label(player))
        }
        GameStatus::Finished(GameOutcome::Draw) => "finished, draw".to_owned(),
    }
}

fn system_label(game: &Game, system: hw_core::SystemId) -> String {
    let state = game.turn().state();
    let labels = Player::ALL
        .into_iter()
        .filter(|player| state.homeworld(*player) == system)
        .map(|player| format!("homeworld {}", player_label(player)))
        .collect::<Vec<_>>();

    if labels.is_empty() {
        "system".to_owned()
    } else {
        labels.join(", ")
    }
}

fn render_piece_list(pieces: &[Piece], show_owner: bool) -> String {
    if pieces.is_empty() {
        return "none".to_owned();
    }

    pieces
        .iter()
        .map(|piece| render_piece(*piece, show_owner))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_piece(piece: Piece, show_owner: bool) -> String {
    let identity = format!("{}{}", color_short(piece.color()), size_short(piece.size()));
    if show_owner {
        match piece.owner() {
            Some(player) => format!("{} {identity}", player_short(player)),
            None => identity,
        }
    } else {
        identity
    }
}

fn player_label(player: Player) -> &'static str {
    match player {
        Player::One => "Player 1",
        Player::Two => "Player 2",
    }
}

fn player_short(player: Player) -> &'static str {
    match player {
        Player::One => "P1",
        Player::Two => "P2",
    }
}

fn color_label(color: Color) -> &'static str {
    match color {
        Color::Red => "red",
        Color::Yellow => "yellow",
        Color::Green => "green",
        Color::Blue => "blue",
    }
}

fn color_short(color: Color) -> &'static str {
    match color {
        Color::Red => "r",
        Color::Yellow => "y",
        Color::Green => "g",
        Color::Blue => "b",
    }
}

fn size_label(size: Size) -> &'static str {
    match size {
        Size::Small => "small",
        Size::Medium => "medium",
        Size::Large => "large",
    }
}

fn size_short(size: Size) -> &'static str {
    match size {
        Size::Small => "s",
        Size::Medium => "m",
        Size::Large => "l",
    }
}

#[cfg(test)]
mod tests {
    use hw_core::{Color, Piece, Player, Size};
    use hw_engine::{Action, HomeworldSetup};

    use super::*;

    #[test]
    fn renders_current_turn_systems_and_bank() {
        let game = Game::new(
            [
                HomeworldSetup::new(
                    vec![
                        Piece::new(Color::Yellow, Size::Small),
                        Piece::new(Color::Blue, Size::Medium),
                    ],
                    Piece::owned(Color::Green, Size::Small, Player::One),
                ),
                HomeworldSetup::new(
                    vec![
                        Piece::new(Color::Blue, Size::Large),
                        Piece::new(Color::Red, Size::Large),
                    ],
                    Piece::owned(Color::Red, Size::Medium, Player::Two),
                ),
            ],
            Player::One,
        )
        .expect("game starts");

        let rendered = render_game(&game);

        assert!(rendered.contains("Status: in progress"));
        assert!(rendered.contains("Current player: Player 1"));
        assert!(rendered.contains("Remaining actions: 1"));
        assert!(rendered.contains("[0] homeworld Player 1"));
        assert!(rendered.contains("Stars: ys, bm"));
        assert!(rendered.contains("Ships: P1 gs"));
        assert!(rendered.contains("[1] homeworld Player 2"));
        assert!(rendered.contains("Stars: bl, rl"));
        assert!(rendered.contains("Ships: P2 rm"));
        assert!(rendered.contains("red: small=3 medium=2 large=2"));
        assert!(rendered.contains("green: small=2 medium=3 large=3"));
    }

    #[test]
    fn renders_turn_summary_after_an_action() {
        let game = Game::default(Player::One);
        let action = Action::Build {
            system: hw_core::SystemId::new(0),
            player: Player::One,
            ship: Piece::owned(Color::Green, Size::Small, Player::One),
        };
        let game = game.apply_action(&action).expect("action applies");

        assert_eq!(
            render_turn_summary(&game),
            "Current player: Player 1\nRemaining actions: 0\n"
        );
    }

    #[test]
    fn renders_terminal_status_in_turn_summary() {
        let game = Game::new(
            [
                HomeworldSetup::new(
                    vec![
                        Piece::new(Color::Yellow, Size::Small),
                        Piece::new(Color::Blue, Size::Large),
                    ],
                    Piece::owned(Color::Green, Size::Small, Player::One),
                ),
                HomeworldSetup::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Red, Size::Medium),
                    ],
                    Piece::owned(Color::Green, Size::Small, Player::Two),
                ),
            ],
            Player::One,
        )
        .expect("game starts");
        let game = game
            .apply_action(&Action::Build {
                system: hw_core::SystemId::new(0),
                player: Player::One,
                ship: Piece::owned(Color::Green, Size::Small, Player::One),
            })
            .expect("player one builds");
        let game = game.end_turn().expect("player one ends turn");
        let game = game
            .apply_action(&Action::Build {
                system: hw_core::SystemId::new(1),
                player: Player::Two,
                ship: Piece::owned(Color::Red, Size::Small, Player::Two),
            })
            .expect("player two builds");
        let game = game.end_turn().expect("player two ends turn");
        let game = game
            .apply_action(&Action::Build {
                system: hw_core::SystemId::new(0),
                player: Player::One,
                ship: Piece::owned(Color::Blue, Size::Small, Player::One),
            })
            .expect("player one builds again");
        let game = game.end_turn().expect("player one ends turn again");
        let game = game
            .apply_action(&Action::Build {
                system: hw_core::SystemId::new(1),
                player: Player::Two,
                ship: Piece::owned(Color::Red, Size::Small, Player::Two),
            })
            .expect("player two builds again");
        let game = game.end_turn().expect("player two ends turn again");
        let game = game
            .apply_action(&Action::Catastrophe {
                system: hw_core::SystemId::new(1),
                color: Color::Red,
            })
            .expect("catastrophe applies");
        let game = game
            .apply_action(&Action::Build {
                system: hw_core::SystemId::new(0),
                player: Player::One,
                ship: Piece::owned(Color::Yellow, Size::Small, Player::One),
            })
            .expect("player one spends action after catastrophe");
        let game = game.end_turn().expect("terminal turn ends");

        assert_eq!(
            render_turn_summary(&game),
            "Status: finished, winner Player 1\n"
        );
    }

    #[test]
    fn renders_help_for_supported_commands() {
        let help = render_help();

        assert!(help.contains("show | s"));
        assert!(help.contains("build | b"));
        assert!(help.contains("trade | tr | x"));
        assert!(help.contains("catastrophe | c"));
    }
}
