use std::{
    fmt, fs,
    io::{self, BufRead, Write},
    path::Path,
};

use hw_core::Player;
use hw_engine::{Game, HomeworldSetup, save};

use crate::{
    parser::{ParsedCommand, parse_input, parse_setup},
    render::{render_game, render_help, render_turn_summary},
};

const MAX_LOAD_DEPTH: usize = 16;

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

        if run_command(line.trim(), &mut game, &mut output, 0)? == CommandOutcome::Quit {
            break;
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandOutcome {
    Continue,
    Quit,
}

enum LoadSource {
    Save(Game),
    History(String),
}

#[derive(Debug)]
enum LoadSourceError {
    Io(io::Error),
    Save(save::SaveError),
}

impl fmt::Display for LoadSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Save(error) => error.fmt(formatter),
        }
    }
}

fn run_command<W: Write>(
    command: &str,
    game: &mut Game,
    output: &mut W,
    load_depth: usize,
) -> io::Result<CommandOutcome> {
    if command.is_empty() {
        return Ok(CommandOutcome::Continue);
    }

    match parse_input(command, game.turn().current_player()) {
        Ok(parsed) => match parsed.command {
            ParsedCommand::Help => {
                write!(output, "{}", render_help())?;
                render_after_semicolon(parsed.show_after, game, output)?;
                Ok(CommandOutcome::Continue)
            }
            ParsedCommand::Show => {
                write!(output, "{}", render_game(game))?;
                Ok(CommandOutcome::Continue)
            }
            ParsedCommand::Quit => {
                writeln!(output, "Goodbye.")?;
                Ok(CommandOutcome::Quit)
            }
            ParsedCommand::Save(path) => {
                match save::save_file(game, &path) {
                    Ok(()) => {
                        writeln!(output, "Saved to {}.", path.display())?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                    }
                    Err(error) => writeln!(output, "Error: {error}")?,
                }
                Ok(CommandOutcome::Continue)
            }
            ParsedCommand::Load(path) => {
                if load_depth >= MAX_LOAD_DEPTH {
                    writeln!(output, "Error: load nesting limit exceeded")?;
                    return Ok(CommandOutcome::Continue);
                }

                match read_load_source(&path) {
                    Ok(LoadSource::Save(loaded)) => {
                        *game = loaded;
                        writeln!(output, "Loaded from {}.", path.display())?;
                        write!(output, "{}", render_turn_summary(game))?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                        Ok(CommandOutcome::Continue)
                    }
                    Ok(LoadSource::History(history)) => {
                        writeln!(output, "Running commands from {}.", path.display())?;
                        match run_history(&history, game, output, load_depth + 1)? {
                            CommandOutcome::Continue => {
                                writeln!(output, "Finished commands from {}.", path.display())?;
                                render_after_semicolon(parsed.show_after, game, output)?;
                                Ok(CommandOutcome::Continue)
                            }
                            CommandOutcome::Quit => Ok(CommandOutcome::Quit),
                        }
                    }
                    Err(error) => {
                        writeln!(output, "Error: {error}")?;
                        Ok(CommandOutcome::Continue)
                    }
                }
            }
            ParsedCommand::End => {
                match game.end_turn() {
                    Ok(next) => {
                        *game = next;
                        writeln!(output, "Turn ended.")?;
                        write!(output, "{}", render_turn_summary(game))?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                    }
                    Err(error) => writeln!(output, "Error: {}", format_game_error(&error))?,
                }
                Ok(CommandOutcome::Continue)
            }
            ParsedCommand::Action(action) => {
                match game.apply_action(&action) {
                    Ok(next) => {
                        *game = next;
                        writeln!(output, "Action applied.")?;
                        write!(output, "{}", render_turn_summary(game))?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                    }
                    Err(error) => writeln!(output, "Error: {}", format_game_error(&error))?,
                }
                Ok(CommandOutcome::Continue)
            }
        },
        Err(error) => {
            writeln!(output, "Error: {}", error.message())?;
            Ok(CommandOutcome::Continue)
        }
    }
}

fn read_load_source(path: &Path) -> Result<LoadSource, LoadSourceError> {
    let input = fs::read_to_string(path).map_err(LoadSourceError::Io)?;
    match save::from_yaml(&input) {
        Ok(game) => Ok(LoadSource::Save(game)),
        Err(error) if looks_like_yaml_save(&input) => Err(LoadSourceError::Save(error)),
        Err(_) => Ok(LoadSource::History(input)),
    }
}

fn looks_like_yaml_save(input: &str) -> bool {
    input.lines().any(|line| line.trim_start() == "version: 1")
}

fn run_history<W: Write>(
    history: &str,
    game: &mut Game,
    output: &mut W,
    load_depth: usize,
) -> io::Result<CommandOutcome> {
    for command in history.lines() {
        if run_command(command.trim(), game, output, load_depth)? == CommandOutcome::Quit {
            return Ok(CommandOutcome::Quit);
        }
    }

    Ok(CommandOutcome::Continue)
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
    use std::{fs, io::Cursor, path::PathBuf};

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

    #[test]
    fn save_command_writes_yaml() {
        let path = temp_save_path("save_command_writes_yaml");
        let script = format!(
            "ys bm
gs
bl rl
rm
v {};
q
",
            path.display()
        );

        let output = run_script(&script);
        let yaml = fs::read_to_string(&path).expect("save file exists");
        let _ = fs::remove_file(path);

        assert!(output.contains("Saved to "));
        assert!(output.contains("Status: in progress"));
        assert!(yaml.contains("version: 1"));
        assert!(yaml.contains("systems:"));
    }

    #[test]
    fn load_command_replaces_the_current_game() {
        let path = temp_save_path("load_command_replaces_the_current_game");
        fs::write(
            &path,
            save::to_yaml(&Game::default(Player::Two)).expect("game serializes"),
        )
        .expect("save fixture writes");
        let script = format!(
            "ys bm
gs
bl rl
rm
l {}
q
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(output.contains("Loaded from "));
        assert!(output.contains("Current player: Player 2"));
    }

    #[test]
    fn load_with_semicolon_prints_the_loaded_state() {
        let path = temp_save_path("load_with_semicolon_prints_the_loaded_state");
        fs::write(
            &path,
            save::to_yaml(&Game::default(Player::Two)).expect("game serializes"),
        )
        .expect("save fixture writes");
        let script = format!(
            "ys bm
gs
bl rl
rm
l {};
q
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(output.contains("Loaded from "));
        assert!(output.contains("Status: in progress"));
        assert!(output.contains("Current player: Player 2"));
    }

    #[test]
    fn load_command_replays_command_history_file() {
        let path = temp_history_path("load_command_replays_command_history_file");
        fs::write(
            &path,
            "b 0 gs
e
show
",
        )
        .expect("history fixture writes");
        let script = format!(
            "ys bm
gs
bl rl
rm
l {}
q
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(output.contains("Running commands from "));
        assert!(output.contains("Action applied."));
        assert!(output.contains("Turn ended."));
        assert!(output.contains("Current player: Player 2"));
        assert!(output.contains("Finished commands from "));
    }

    #[test]
    fn history_load_supports_semicolon_state_printing() {
        let path = temp_history_path("history_load_supports_semicolon_state_printing");
        fs::write(
            &path, "b 0 gs;
",
        )
        .expect("history fixture writes");
        let script = format!(
            "ys bm
gs
bl rl
rm
l {}
q
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(output.contains("Action applied."));
        assert!(output.contains("Status: in progress"));
        assert!(output.contains("Ships: P1 gs, P1 gs"));
    }

    #[test]
    fn history_load_quit_exits_the_session() {
        let path = temp_history_path("history_load_quit_exits_the_session");
        fs::write(
            &path, "q
",
        )
        .expect("history fixture writes");
        let script = format!(
            "ys bm
gs
bl rl
rm
l {}
show
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(output.contains("Running commands from "));
        assert!(output.contains("Goodbye."));
        assert!(!output.contains("Status: in progress"));
    }

    #[test]
    fn failed_load_keeps_the_current_game() {
        let path = temp_save_path("failed_load_keeps_the_current_game");
        let script = format!(
            "ys bm
gs
bl rl
rm
l {}
show
q
",
            path.display()
        );

        let output = run_script(&script);

        assert!(output.contains("Error: I/O error:"));
        assert!(output.contains("Current player: Player 1"));
        assert!(output.contains("Remaining actions: 1"));
    }

    #[test]
    fn malformed_yaml_save_is_not_replayed_as_history() {
        let path = temp_save_path("malformed_yaml_save_is_not_replayed_as_history");
        fs::write(
            &path,
            "version: 1
players:
  - p1
",
        )
        .expect("save fixture writes");
        let script = format!(
            "ys bm
gs
bl rl
rm
l {}
show
q
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(output.contains("Error:"));
        assert!(!output.contains("Running commands from "));
        assert!(output.contains("Current player: Player 1"));
        assert!(output.contains("Remaining actions: 1"));
    }

    fn run_script(input: &str) -> String {
        let mut output = Vec::new();
        run(Cursor::new(input), &mut output).expect("script runs");
        String::from_utf8(output).expect("output is utf8")
    }

    fn temp_save_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("homeworlds-rs-{name}-{}.yaml", std::process::id()))
    }

    fn temp_history_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("homeworlds-rs-{name}-{}.txt", std::process::id()))
    }
}
