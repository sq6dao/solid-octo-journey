use std::io::{self, BufRead, Write};

use hw_core::Player;
use hw_engine::{Game, HomeworldSetup};

use crate::{
    parser::{ParsedCommand, parse_input, parse_setup},
    render::{render_game, render_help, render_turn_summary},
};

pub fn run_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run(stdin.lock(), stdout.lock())
}

pub fn run<R, W>(mut input: R, mut output: W) -> io::Result<()>
where
    R: BufRead,
    W: Write,
{
    writeln!(output, "Homeworlds hot seat")?;
    writeln!(output, "Enter pieces as gs, red medium, or yellow small.")?;

    let Some(mut game) = prompt_game(&mut input, &mut output)? else {
        return Ok(());
    };

    writeln!(output, "Game started.")?;
    write!(output, "{}", render_turn_summary(&game))?;

    let mut line = String::new();
    loop {
        write!(output, "{}> ", prompt_label(game.turn().current_player()))?;
        output.flush()?;

        if read_line(&mut input, &mut line)? == ReadLine::Eof {
            break;
        }

        let command = line.trim();
        if command.is_empty() {
            continue;
        }

        match parse_input(command, game.turn().current_player()) {
            Ok(parsed) => match parsed.command {
                ParsedCommand::Help => {
                    write!(output, "{}", render_help())?;
                    render_after_semicolon(parsed.show_after, &game, &mut output)?;
                }
                ParsedCommand::Show => write!(output, "{}", render_game(&game))?,
                ParsedCommand::Quit => {
                    writeln!(output, "Goodbye.")?;
                    break;
                }
                ParsedCommand::End => match game.end_turn() {
                    Ok(next) => {
                        game = next;
                        writeln!(output, "Turn ended.")?;
                        write!(output, "{}", render_turn_summary(&game))?;
                        render_after_semicolon(parsed.show_after, &game, &mut output)?;
                    }
                    Err(error) => writeln!(output, "Error: {}", format_game_error(&error))?,
                },
                ParsedCommand::Action(action) => match game.apply_action(&action) {
                    Ok(next) => {
                        game = next;
                        writeln!(output, "Action applied.")?;
                        write!(output, "{}", render_turn_summary(&game))?;
                        render_after_semicolon(parsed.show_after, &game, &mut output)?;
                    }
                    Err(error) => writeln!(output, "Error: {}", format_game_error(&error))?,
                },
            },
            Err(error) => writeln!(output, "Error: {}", error.message())?,
        }
    }

    Ok(())
}

fn render_after_semicolon<W: Write>(
    show_after: bool,
    game: &Game,
    output: &mut W,
) -> io::Result<()> {
    if show_after {
        write!(output, "{}", render_game(game))?;
    }
    Ok(())
}

fn prompt_game<R, W>(input: &mut R, output: &mut W) -> io::Result<Option<Game>>
where
    R: BufRead,
    W: Write,
{
    loop {
        let Some(player_one) = prompt_setup(input, output, Player::One)? else {
            return Ok(None);
        };
        let Some(player_two) = prompt_setup(input, output, Player::Two)? else {
            return Ok(None);
        };

        match Game::new([player_one, player_two], Player::One) {
            Ok(game) => return Ok(Some(game)),
            Err(error) => {
                writeln!(
                    output,
                    "Error: invalid homeworld setup: {}",
                    format_game_error(&error)
                )?;
            }
        }
    }
}

fn prompt_setup<R, W>(
    input: &mut R,
    output: &mut W,
    player: Player,
) -> io::Result<Option<HomeworldSetup>>
where
    R: BufRead,
    W: Write,
{
    let mut stars = String::new();
    let mut ship = String::new();

    loop {
        writeln!(output, "{} setup", player_label(player))?;
        write!(output, "{} stars> ", player_label(player))?;
        output.flush()?;
        if read_line(input, &mut stars)? == ReadLine::Eof {
            return Ok(None);
        }

        write!(output, "{} ship> ", player_label(player))?;
        output.flush()?;
        if read_line(input, &mut ship)? == ReadLine::Eof {
            return Ok(None);
        }

        match parse_setup(stars.trim(), ship.trim(), player) {
            Ok(setup) => return Ok(Some(setup)),
            Err(error) => writeln!(output, "Error: {}", error.message())?,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReadLine {
    Eof,
    Read,
}

fn read_line<R: BufRead>(input: &mut R, line: &mut String) -> io::Result<ReadLine> {
    line.clear();
    if input.read_line(line)? == 0 {
        Ok(ReadLine::Eof)
    } else {
        Ok(ReadLine::Read)
    }
}

fn format_game_error(error: &hw_engine::GameError) -> String {
    format!("{error:?}")
}

fn prompt_label(player: Player) -> &'static str {
    match player {
        Player::One => "P1",
        Player::Two => "P2",
    }
}

fn player_label(player: Player) -> &'static str {
    match player {
        Player::One => "Player 1",
        Player::Two => "Player 2",
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn show_prints_the_current_game_state() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
show
q
",
        );

        assert!(output.contains("Homeworlds hot seat"));
        assert!(output.contains("Player 1 stars> "));
        assert!(output.contains("Game started."));
        assert!(output.contains("[0] homeworld Player 1"));
        assert!(output.contains("Stars: ys, bm"));
        assert!(output.contains("Ships: P1 gs"));
        assert!(output.contains("[1] homeworld Player 2"));
        assert!(output.contains("Stars: bl, rl"));
        assert!(output.contains("Ships: P2 rm"));
        assert!(output.contains("Goodbye."));
    }

    #[test]
    fn invalid_commands_do_not_advance_the_turn() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
nonsense
show
q
",
        );

        assert!(output.contains("Error: unknown command"));
        assert!(output.contains("Current player: Player 1"));
        assert!(output.contains("Remaining actions: 1"));
    }

    #[test]
    fn short_action_notation_drives_hot_seat_turns() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
b 0 gs
e
s
q
",
        );

        assert!(output.contains("Action applied."));
        assert!(output.contains("Turn ended."));
        assert!(output.contains("Current player: Player 2"));
        assert!(output.contains("Remaining actions: 1"));
    }

    #[test]
    fn scripted_game_can_reach_a_winner() {
        let output = run_script(
            "gs gm
gl
ys bl
gs
b 0 gs
e
c 0 g
b 1 rs
e
q
",
        );

        assert!(output.contains("Status: finished, winner Player 2"));
    }

    #[test]
    fn semicolon_prints_state_after_a_successful_command() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
b 0 gs;
q
",
        );

        assert!(output.contains("Action applied."));
        assert!(output.contains("Status: in progress"));
        assert!(output.contains("Ships: P1 gs, P1 gs"));
    }

    #[test]
    fn show_with_semicolon_only_prints_state_once() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
show;
q
",
        );

        assert_eq!(output.matches("Status: in progress").count(), 1);
    }

    #[test]
    fn semicolon_does_not_print_state_after_an_error() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
bad;
q
",
        );

        assert!(output.contains("Error: unknown command"));
        assert_eq!(output.matches("Status: in progress").count(), 0);
    }

    fn run_script(input: &str) -> String {
        let mut output = Vec::new();
        run(Cursor::new(input), &mut output).expect("script runs");
        String::from_utf8(output).expect("output is utf8")
    }
}
