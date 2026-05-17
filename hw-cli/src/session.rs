use std::{
    fmt, fs,
    io::{self, BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
};

use hw_ai::{AiDecision, FirstLegalStrategy, PriorityStrategy, SearchStrategy, Strategy};
use hw_core::{Color, Piece, Player, Size};
use hw_engine::{
    Action, Game, GameStatus, HomeworldSetup, TravelTarget, has_possible_catastrophe, save,
};
use rustyline::{
    Context, Editor, Helper,
    completion::{Completer, Pair},
    error::ReadlineError,
    highlight::Highlighter,
    hint::Hinter,
    history::DefaultHistory,
    validate::Validator,
};

use crate::{
    parser::{AiCommand, AiStrategy, ParsedCommand, parse_input, parse_setup},
    render::{render_game, render_help, render_turn_summary},
};

const MAX_LOAD_DEPTH: usize = 16;
const COMMAND_NAMES: &[&str] = &[
    "help",
    "show",
    "end",
    "quit",
    "ai",
    "save",
    "save-history",
    "load",
    "build",
    "travel",
    "trade",
    "sacrifice",
    "invade",
    "catastrophe",
];
const COMMAND_ALIASES: &[(&str, &str)] = &[
    ("h", "help"),
    ("e", "end"),
    ("q", "quit"),
    ("v", "save"),
    ("sh", "save-history"),
    ("l", "load"),
    ("b", "build"),
    ("t", "travel"),
    ("tr", "trade"),
    ("x", "trade"),
    ("sac", "sacrifice"),
    ("i", "invade"),
    ("c", "catastrophe"),
];
const TRAVEL_TARGET_WORDS: &[&str] = &["existing", "new", "x", "n"];
const COLOR_WORDS: &[&str] = &["red", "yellow", "green", "blue", "r", "y", "g", "b"];
const AI_TARGET_WORDS: &[&str] = &["show", "p1", "p2"];
const AI_MODE_WORDS: &[&str] = &["first", "priority", "search", "off"];
const MAX_AI_DECISIONS: usize = 512;

struct PromptedGame {
    game: Game,
    history: Option<LoadedHistory>,
    show_after: bool,
}

struct LoadedHistory {
    path: PathBuf,
    commands: String,
    show_after: bool,
}

type TypedHistory = Vec<String>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AiControl {
    player_one: Option<AiStrategy>,
    player_two: Option<AiStrategy>,
}

impl AiControl {
    fn strategy(&self, player: Player) -> Option<AiStrategy> {
        match player {
            Player::One => self.player_one,
            Player::Two => self.player_two,
        }
    }

    fn set_strategy(&mut self, player: Player, strategy: AiStrategy) {
        match player {
            Player::One => self.player_one = Some(strategy),
            Player::Two => self.player_two = Some(strategy),
        }
    }

    fn disable(&mut self, player: Player) {
        match player {
            Player::One => self.player_one = None,
            Player::Two => self.player_two = None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct CompletionSnapshot {
    system_ids: Vec<String>,
    pieces: Vec<String>,
}

impl CompletionSnapshot {
    fn from_game(game: &Game) -> Self {
        let state = game.turn().state();
        let system_ids = (0..state.systems().len())
            .map(|index| index.to_string())
            .collect();
        let mut pieces = Vec::new();

        for system in state.systems() {
            for piece in system.stars().iter().chain(system.ships()) {
                push_piece_completion(&mut pieces, *piece);
            }
        }

        Self { system_ids, pieces }
    }
}

enum PromptLine {
    Eof,
    Interrupted,
    Read(String),
}

trait LineInput {
    fn read_prompted_line<W: Write>(
        &mut self,
        prompt: &str,
        output: &mut W,
    ) -> io::Result<PromptLine>;

    fn add_history_entry(&mut self, line: &str) -> io::Result<()>;

    fn set_completion_snapshot(&mut self, _snapshot: CompletionSnapshot) {}
}

struct ScriptedLineInput<R> {
    input: R,
}

impl<R> ScriptedLineInput<R> {
    fn new(input: R) -> Self {
        Self { input }
    }
}

impl<R: BufRead> LineInput for ScriptedLineInput<R> {
    fn read_prompted_line<W: Write>(
        &mut self,
        prompt: &str,
        output: &mut W,
    ) -> io::Result<PromptLine> {
        write!(output, "{prompt}")?;
        output.flush()?;

        let mut line = String::new();
        if self.input.read_line(&mut line)? == 0 {
            Ok(PromptLine::Eof)
        } else {
            Ok(PromptLine::Read(line))
        }
    }

    fn add_history_entry(&mut self, _line: &str) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
struct CliHelper {
    snapshot: CompletionSnapshot,
}

impl CliHelper {
    fn set_completion_snapshot(&mut self, snapshot: CompletionSnapshot) {
        self.snapshot = snapshot;
    }
}

impl Completer for CliHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let Some((start, replacements)) = completion_candidates(line, pos, &self.snapshot) else {
            return Ok((0, Vec::new()));
        };

        Ok((
            start,
            replacements
                .into_iter()
                .map(|replacement| Pair {
                    display: replacement.clone(),
                    replacement,
                })
                .collect(),
        ))
    }
}

impl Helper for CliHelper {}
impl Highlighter for CliHelper {}
impl Hinter for CliHelper {
    type Hint = String;
}
impl Validator for CliHelper {}

fn completion_candidates(
    line: &str,
    pos: usize,
    snapshot: &CompletionSnapshot,
) -> Option<(usize, Vec<String>)> {
    if line.contains(';') {
        return None;
    }

    if let Some((start, replacement)) = command_completion(line, pos) {
        return Some((start, vec![replacement.to_owned()]));
    }

    argument_completion(line, pos, snapshot)
}

fn command_completion(line: &str, pos: usize) -> Option<(usize, &'static str)> {
    if line.contains(';') {
        return None;
    }

    let token_start = line
        .char_indices()
        .find_map(|(index, ch)| (!ch.is_whitespace()).then_some(index))?;
    let token_end = line[token_start..]
        .char_indices()
        .find_map(|(offset, ch)| ch.is_whitespace().then_some(token_start + offset))
        .unwrap_or(line.len());

    if pos != token_end {
        return None;
    }

    let token = &line[token_start..token_end];
    command_word_completion(token).map(|replacement| (token_start, replacement))
}

fn command_word_completion(token: &str) -> Option<&'static str> {
    let token = token.to_ascii_lowercase();

    if let Some((_, target)) = COMMAND_ALIASES
        .iter()
        .find(|(alias, _)| *alias == token.as_str())
    {
        return Some(*target);
    }

    if COMMAND_NAMES.contains(&token.as_str()) {
        return None;
    }

    let mut matched = None;
    for target in COMMAND_NAMES
        .iter()
        .copied()
        .filter(|target| target.starts_with(&token))
        .chain(
            COMMAND_ALIASES
                .iter()
                .filter(|(alias, _)| alias.starts_with(&token))
                .map(|(_, target)| *target),
        )
    {
        match matched {
            None => matched = Some(target),
            Some(existing) if existing == target => {}
            Some(_) => return None,
        }
    }

    matched
}

fn argument_completion(
    line: &str,
    pos: usize,
    snapshot: &CompletionSnapshot,
) -> Option<(usize, Vec<String>)> {
    if pos != line.len() || !line.is_char_boundary(pos) {
        return None;
    }

    let ends_with_space = line.chars().last().is_some_and(char::is_whitespace);
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() || (!ends_with_space && tokens.len() == 1) {
        return None;
    }

    let has_args = tokens.len() > 1 || ends_with_space;
    let command = canonical_command(tokens[0], has_args)?;
    let current = if ends_with_space {
        ""
    } else {
        tokens.last().copied()?
    };
    let start = if ends_with_space {
        pos
    } else {
        pos - current.len()
    };
    let arg_index = if ends_with_space {
        tokens.len() - 1
    } else {
        tokens.len() - 2
    };

    let candidates = argument_candidates(command, arg_index, &tokens, current, snapshot)?;
    (!candidates.is_empty()).then_some((start, candidates))
}

fn canonical_command(token: &str, has_args: bool) -> Option<&'static str> {
    let token = token.to_ascii_lowercase();

    if token == "s" && has_args {
        return Some("sacrifice");
    }

    if let Some(command) = COMMAND_NAMES
        .iter()
        .copied()
        .find(|command| *command == token.as_str())
    {
        return Some(command);
    }

    if let Some((_, target)) = COMMAND_ALIASES
        .iter()
        .find(|(alias, _)| *alias == token.as_str())
    {
        return Some(*target);
    }

    command_word_completion(&token)
}

fn argument_candidates(
    command: &str,
    arg_index: usize,
    tokens: &[&str],
    current: &str,
    snapshot: &CompletionSnapshot,
) -> Option<Vec<String>> {
    match command {
        "save" | "save-history" | "load" if arg_index == 0 => Some(path_candidates(current)),
        "build" | "sacrifice" => match arg_index {
            0 => Some(word_candidates(current, &snapshot.system_ids)),
            1 => Some(word_candidates(current, &snapshot.pieces)),
            _ => None,
        },
        "trade" => match arg_index {
            0 => Some(word_candidates(current, &snapshot.system_ids)),
            1 | 2 => Some(word_candidates(current, &snapshot.pieces)),
            _ => None,
        },
        "travel" => match arg_index {
            0 => Some(word_candidates(current, &snapshot.system_ids)),
            1 => Some(word_candidates(current, &snapshot.pieces)),
            2 => Some(static_word_candidates(current, TRAVEL_TARGET_WORDS)),
            3 => match tokens.get(3).map(|token| token.to_ascii_lowercase()) {
                Some(target) if matches!(target.as_str(), "existing" | "x") => {
                    Some(word_candidates(current, &snapshot.system_ids))
                }
                Some(target) if matches!(target.as_str(), "new" | "n") => {
                    Some(word_candidates(current, &snapshot.pieces))
                }
                _ => None,
            },
            4 => match tokens.get(3).map(|token| token.to_ascii_lowercase()) {
                Some(target) if matches!(target.as_str(), "new" | "n") => {
                    Some(word_candidates(current, &snapshot.pieces))
                }
                _ => None,
            },
            _ => None,
        },
        "invade" => match arg_index {
            0 => Some(word_candidates(current, &snapshot.system_ids)),
            1 => Some(word_candidates(current, &snapshot.pieces)),
            _ => None,
        },
        "catastrophe" => match arg_index {
            0 => Some(word_candidates(current, &snapshot.system_ids)),
            1 => Some(static_word_candidates(current, COLOR_WORDS)),
            _ => None,
        },
        "ai" => match arg_index {
            0 => Some(static_word_candidates(current, AI_TARGET_WORDS)),
            1 if matches!(tokens.get(1), Some(&"p1" | &"p2")) => {
                Some(static_word_candidates(current, AI_MODE_WORDS))
            }
            _ => None,
        },
        _ => None,
    }
}

fn word_candidates(current: &str, candidates: &[String]) -> Vec<String> {
    let current = current.to_ascii_lowercase();
    candidates
        .iter()
        .filter(|candidate| {
            let candidate_lower = candidate.to_ascii_lowercase();
            candidate_lower.starts_with(&current) && candidate_lower != current
        })
        .cloned()
        .collect()
}

fn static_word_candidates(current: &str, candidates: &[&str]) -> Vec<String> {
    let current = current.to_ascii_lowercase();
    candidates
        .iter()
        .copied()
        .filter(|candidate| candidate.starts_with(&current) && *candidate != current)
        .map(str::to_owned)
        .collect()
}

fn path_candidates(current: &str) -> Vec<String> {
    let path = Path::new(current);
    let (directory, prefix) = if current.ends_with(std::path::MAIN_SEPARATOR) {
        (path, "")
    } else {
        (
            path.parent().unwrap_or_else(|| Path::new("")),
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(""),
        )
    };
    let read_directory = if directory.as_os_str().is_empty() {
        Path::new(".")
    } else {
        directory
    };

    let Ok(entries) = fs::read_dir(read_directory) else {
        return Vec::new();
    };
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if !name.starts_with(prefix) {
                return None;
            }

            let path = if directory.as_os_str().is_empty() {
                PathBuf::from(name)
            } else {
                directory.join(name)
            };
            let mut replacement = path.display().to_string();
            if replacement == current || replacement.chars().any(char::is_whitespace) {
                return None;
            }
            if entry.file_type().ok()?.is_dir() {
                replacement.push(std::path::MAIN_SEPARATOR);
            }
            Some(replacement)
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
}

fn push_piece_completion(pieces: &mut Vec<String>, piece: Piece) {
    let token = compact_piece(piece);
    if !pieces.contains(&token) {
        pieces.push(token);
    }
}

fn compact_piece(piece: Piece) -> String {
    format!("{}{}", color_short(piece.color()), size_short(piece.size()))
}

fn color_short(color: Color) -> char {
    match color {
        Color::Red => 'r',
        Color::Yellow => 'y',
        Color::Green => 'g',
        Color::Blue => 'b',
    }
}

fn size_short(size: Size) -> char {
    match size {
        Size::Small => 's',
        Size::Medium => 'm',
        Size::Large => 'l',
    }
}

struct RustylineInput {
    editor: Editor<CliHelper, DefaultHistory>,
}

impl RustylineInput {
    fn new() -> io::Result<Self> {
        let mut editor =
            Editor::<CliHelper, DefaultHistory>::new().map_err(readline_error_to_io)?;
        editor.set_helper(Some(CliHelper::default()));
        Ok(Self { editor })
    }
}

impl LineInput for RustylineInput {
    fn read_prompted_line<W: Write>(
        &mut self,
        prompt: &str,
        output: &mut W,
    ) -> io::Result<PromptLine> {
        output.flush()?;

        match self.editor.readline(prompt) {
            Ok(line) => Ok(PromptLine::Read(line)),
            Err(ReadlineError::Eof) => Ok(PromptLine::Eof),
            Err(ReadlineError::Interrupted) => Ok(PromptLine::Interrupted),
            Err(error) => Err(readline_error_to_io(error)),
        }
    }

    fn add_history_entry(&mut self, line: &str) -> io::Result<()> {
        self.editor
            .add_history_entry(line.to_owned())
            .map(|_| ())
            .map_err(readline_error_to_io)
    }

    fn set_completion_snapshot(&mut self, snapshot: CompletionSnapshot) {
        if let Some(helper) = self.editor.helper_mut() {
            helper.set_completion_snapshot(snapshot);
        }
    }
}

fn readline_error_to_io(error: ReadlineError) -> io::Error {
    match error {
        ReadlineError::Io(error) => error,
        error => io::Error::other(error.to_string()),
    }
}

pub fn run_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if !stdin.is_terminal() {
        return run(stdin.lock(), stdout.lock());
    }

    let mut input = RustylineInput::new()?;
    run_with_line_input(&mut input, stdout.lock())
}

pub fn run<R, W>(input: R, output: W) -> io::Result<()>
where
    R: BufRead,
    W: Write,
{
    let mut input = ScriptedLineInput::new(input);
    run_with_line_input(&mut input, output)
}

fn run_with_line_input<I, W>(input: &mut I, mut output: W) -> io::Result<()>
where
    I: LineInput,
    W: Write,
{
    writeln!(output, "Homeworlds hot seat")?;
    writeln!(
        output,
        "Enter pieces as gs, red medium, or yellow small, or load <path>."
    )?;

    let mut typed_history = TypedHistory::new();
    let Some(prompted) = prompt_game(input, &mut output, &mut typed_history)? else {
        return Ok(());
    };
    let mut game = prompted.game;
    let mut ai_control = AiControl::default();

    writeln!(output, "Game started.")?;
    let render_full_after_setup = prompted.history.is_none() && prompted.show_after;
    if render_full_after_setup {
        write!(output, "{}", render_game(&game))?;
    } else {
        write!(output, "{}", render_turn_summary(&game))?;
    }
    if let Some(history) = prompted.history {
        writeln!(output, "Running commands from {}.", history.path.display())?;
        if run_loaded_history(
            history,
            &mut game,
            &mut ai_control,
            &mut output,
            1,
            &typed_history,
        )? == CommandOutcome::Quit
        {
            return Ok(());
        }
    } else if !render_full_after_setup {
        render_after_semicolon(prompted.show_after, &game, &mut output)?;
    }

    loop {
        let prompt = format!("{}> ", prompt_label(game.turn().current_player()));
        input.set_completion_snapshot(CompletionSnapshot::from_game(&game));
        let line = match read_prompted_line(input, &prompt, &mut output)? {
            PromptLine::Read(line) => line,
            PromptLine::Eof => break,
            PromptLine::Interrupted => {
                write_quit_message(&mut output)?;
                break;
            }
        };

        let command = line.trim();
        record_user_history(input, &mut typed_history, command)?;

        if run_command(
            command,
            &mut game,
            &mut ai_control,
            &mut output,
            0,
            &typed_history,
        )? == CommandOutcome::Quit
        {
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
    Save(save::SavedGame),
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
    ai_control: &mut AiControl,
    output: &mut W,
    load_depth: usize,
    typed_history: &[String],
) -> io::Result<CommandOutcome> {
    if command.is_empty() {
        return Ok(CommandOutcome::Continue);
    }

    let show_after_error = command_requests_state_after_error(command);

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
            ParsedCommand::Ai(command) => {
                run_ai_command(command, ai_control, output)?;
                render_after_semicolon(parsed.show_after, game, output)?;
                run_ai_turns(game, ai_control, output)?;
                Ok(CommandOutcome::Continue)
            }
            ParsedCommand::Save(path) => {
                match save::save_file(game, &path) {
                    Ok(()) => {
                        writeln!(output, "Saved to {}.", path.display())?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                    }
                    Err(error) => {
                        writeln!(output, "Error: {error}")?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                    }
                }
                Ok(CommandOutcome::Continue)
            }
            ParsedCommand::SaveHistory(path) => {
                let extras = save::SaveExtras {
                    history: typed_history.to_vec(),
                    commands: Vec::new(),
                };
                match save::save_file_with_extras(game, &extras, &path) {
                    Ok(()) => {
                        writeln!(output, "Saved history to {}.", path.display())?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                    }
                    Err(error) => {
                        writeln!(output, "Error: {error}")?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                    }
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
                        *game = loaded.game;
                        writeln!(output, "Loaded from {}.", path.display())?;
                        write!(output, "{}", render_turn_summary(game))?;
                        if loaded.commands.is_empty() {
                            render_after_semicolon(parsed.show_after, game, output)?;
                            run_ai_turns(game, ai_control, output)?;
                            Ok(CommandOutcome::Continue)
                        } else {
                            writeln!(output, "Running commands from {}.", path.display())?;
                            run_loaded_history(
                                LoadedHistory {
                                    path,
                                    commands: loaded.commands.join("\n"),
                                    show_after: parsed.show_after,
                                },
                                game,
                                ai_control,
                                output,
                                load_depth + 1,
                                typed_history,
                            )
                        }
                    }
                    Ok(LoadSource::History(history)) => {
                        writeln!(output, "Running commands from {}.", path.display())?;
                        run_loaded_history(
                            LoadedHistory {
                                path,
                                commands: history,
                                show_after: parsed.show_after,
                            },
                            game,
                            ai_control,
                            output,
                            load_depth + 1,
                            typed_history,
                        )
                    }
                    Err(error) => {
                        writeln!(output, "Error: {error}")?;
                        render_after_semicolon(parsed.show_after, game, output)?;
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
                        run_ai_turns(game, ai_control, output)?;
                    }
                    Err(error) => {
                        writeln!(output, "Error: {}", format_game_error(&error))?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                    }
                }
                Ok(CommandOutcome::Continue)
            }
            ParsedCommand::Action(action) => {
                match game.apply_action(&action) {
                    Ok(next) => {
                        *game = next;
                        writeln!(output, "Action applied.")?;
                        auto_end_turn_if_ready(game, output)?;
                        write!(output, "{}", render_turn_summary(game))?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                        run_ai_turns(game, ai_control, output)?;
                    }
                    Err(error) => {
                        writeln!(output, "Error: {}", format_game_error(&error))?;
                        render_after_semicolon(parsed.show_after, game, output)?;
                    }
                }
                Ok(CommandOutcome::Continue)
            }
        },
        Err(error) => {
            writeln!(output, "Error: {}", error.message())?;
            render_after_semicolon(show_after_error, game, output)?;
            Ok(CommandOutcome::Continue)
        }
    }
}

fn command_requests_state_after_error(command: &str) -> bool {
    let trimmed = command.trim();
    matches!(trimmed.find(';'), Some(index) if index == trimmed.len() - 1)
}

fn run_ai_command<W: Write>(
    command: AiCommand,
    ai_control: &mut AiControl,
    output: &mut W,
) -> io::Result<()> {
    match command {
        AiCommand::Show => writeln!(output, "{}", render_ai_control(ai_control)),
        AiCommand::Set { player, strategy } => {
            ai_control.set_strategy(player, strategy);
            writeln!(
                output,
                "AI {} set to {}.",
                player_label(player),
                ai_strategy_label(strategy)
            )
        }
        AiCommand::Off { player } => {
            ai_control.disable(player);
            writeln!(output, "AI {} disabled.", player_label(player))
        }
    }
}

fn render_ai_control(ai_control: &AiControl) -> String {
    format!(
        "AI players: {} {}, {} {}",
        player_label(Player::One),
        ai_strategy_or_human(ai_control.strategy(Player::One)),
        player_label(Player::Two),
        ai_strategy_or_human(ai_control.strategy(Player::Two))
    )
}

fn ai_strategy_or_human(strategy: Option<AiStrategy>) -> &'static str {
    strategy.map_or("human", ai_strategy_label)
}

fn ai_strategy_label(strategy: AiStrategy) -> &'static str {
    match strategy {
        AiStrategy::First => "first",
        AiStrategy::Priority => "priority",
        AiStrategy::Search => "search",
    }
}

fn run_ai_turns<W: Write>(
    game: &mut Game,
    ai_control: &AiControl,
    output: &mut W,
) -> io::Result<()> {
    for _ in 0..MAX_AI_DECISIONS {
        if game.status() != GameStatus::InProgress {
            return Ok(());
        }

        let player = game.turn().current_player();
        let Some(strategy) = ai_control.strategy(player) else {
            return Ok(());
        };
        let Some(decision) = choose_ai_decision(strategy, game) else {
            writeln!(output, "AI has no legal decision.")?;
            return Ok(());
        };

        writeln!(
            output,
            "{}> {}",
            prompt_label(player),
            ai_decision_command(&decision)
        )?;
        apply_ai_decision(game, decision, output)?;
    }

    writeln!(output, "AI stopped after {MAX_AI_DECISIONS} decisions.")
}

fn choose_ai_decision(strategy: AiStrategy, game: &Game) -> Option<AiDecision> {
    match strategy {
        AiStrategy::First => FirstLegalStrategy.choose(game),
        AiStrategy::Priority => PriorityStrategy.choose(game),
        AiStrategy::Search => SearchStrategy::default().choose(game),
    }
}

fn apply_ai_decision<W: Write>(
    game: &mut Game,
    decision: AiDecision,
    output: &mut W,
) -> io::Result<()> {
    match decision {
        AiDecision::Action(action) => match game.apply_action(&action) {
            Ok(next) => {
                *game = next;
                writeln!(output, "Action applied.")?;
                auto_end_turn_if_ready(game, output)?;
                write!(output, "{}", render_turn_summary(game))
            }
            Err(error) => {
                writeln!(output, "Error: {}", format_game_error(&error))
            }
        },
        AiDecision::EndTurn => match game.end_turn() {
            Ok(next) => {
                *game = next;
                writeln!(output, "Turn ended.")?;
                write!(output, "{}", render_turn_summary(game))
            }
            Err(error) => {
                writeln!(output, "Error: {}", format_game_error(&error))
            }
        },
    }
}

fn ai_decision_command(decision: &AiDecision) -> String {
    match decision {
        AiDecision::EndTurn => "e".to_owned(),
        AiDecision::Action(action) => ai_action_command(action),
    }
}

fn ai_action_command(action: &Action) -> String {
    match action {
        Action::Build { system, ship, .. } => {
            format!("b {} {}", system.index(), compact_piece(*ship))
        }
        Action::Travel {
            from, ship, target, ..
        } => match target {
            TravelTarget::Existing(to) => {
                format!(
                    "t {} {} x {}",
                    from.index(),
                    compact_piece(*ship),
                    to.index()
                )
            }
            TravelTarget::New { stars } => format!(
                "t {} {} n {}",
                from.index(),
                compact_piece(*ship),
                stars
                    .iter()
                    .map(|star| compact_piece(*star))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        },
        Action::Trade {
            system, from, to, ..
        } => format!(
            "x {} {} {}",
            system.index(),
            compact_piece(*from),
            compact_piece(*to)
        ),
        Action::Sacrifice { system, ship, .. } => {
            format!("s {} {}", system.index(), compact_piece(*ship))
        }
        Action::Invade { system, target, .. } => {
            format!("i {} {}", system.index(), compact_piece(*target))
        }
        Action::Catastrophe { system, color } => {
            format!("c {} {}", system.index(), color_short(*color))
        }
    }
}

fn auto_end_turn_if_ready<W: Write>(game: &mut Game, output: &mut W) -> io::Result<()> {
    if game.status() != GameStatus::InProgress
        || game.turn().remaining_actions() != 0
        || has_possible_catastrophe(game.turn().state())
    {
        return Ok(());
    }

    match game.end_turn() {
        Ok(next) => {
            *game = next;
            writeln!(output, "Turn ended.")?;
        }
        Err(error) => writeln!(output, "Error: {}", format_game_error(&error))?,
    }

    Ok(())
}

fn record_user_history<I: LineInput>(
    input: &mut I,
    history: &mut TypedHistory,
    line: &str,
) -> io::Result<()> {
    let trimmed = line.trim();
    if should_record_user_history(trimmed) {
        history.push(trimmed.to_owned());
        input.add_history_entry(trimmed)?;
    }
    Ok(())
}

fn should_record_user_history(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && !is_save_history_command(trimmed)
}

fn is_save_history_command(command: &str) -> bool {
    command
        .split_whitespace()
        .next()
        .is_some_and(|token| matches!(token.to_ascii_lowercase().as_str(), "save-history" | "sh"))
}

fn read_load_source(path: &Path) -> Result<LoadSource, LoadSourceError> {
    let input = fs::read_to_string(path).map_err(LoadSourceError::Io)?;
    match save::from_yaml_with_extras(&input) {
        Ok(saved) => Ok(LoadSource::Save(saved)),
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
    ai_control: &mut AiControl,
    output: &mut W,
    load_depth: usize,
    typed_history: &[String],
) -> io::Result<CommandOutcome> {
    for command in history_commands(history) {
        writeln!(
            output,
            "{}> {command}",
            prompt_label(game.turn().current_player())
        )?;
        if run_command(command, game, ai_control, output, load_depth, typed_history)?
            == CommandOutcome::Quit
        {
            return Ok(CommandOutcome::Quit);
        }
    }

    Ok(CommandOutcome::Continue)
}

fn run_loaded_history<W: Write>(
    history: LoadedHistory,
    game: &mut Game,
    ai_control: &mut AiControl,
    output: &mut W,
    load_depth: usize,
    typed_history: &[String],
) -> io::Result<CommandOutcome> {
    match run_history(
        &history.commands,
        game,
        ai_control,
        output,
        load_depth,
        typed_history,
    )? {
        CommandOutcome::Continue => {
            writeln!(output, "Finished commands from {}.", history.path.display())?;
            render_after_semicolon(history.show_after, game, output)?;
            run_ai_turns(game, ai_control, output)?;
            Ok(CommandOutcome::Continue)
        }
        CommandOutcome::Quit => Ok(CommandOutcome::Quit),
    }
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

fn read_prompted_line<I, W>(input: &mut I, prompt: &str, output: &mut W) -> io::Result<PromptLine>
where
    I: LineInput,
    W: Write,
{
    input.read_prompted_line(prompt, output)
}

fn write_quit_message<W: Write>(output: &mut W) -> io::Result<()> {
    writeln!(output, "Goodbye.")
}

fn prompt_game<I, W>(
    input: &mut I,
    output: &mut W,
    typed_history: &mut TypedHistory,
) -> io::Result<Option<PromptedGame>>
where
    I: LineInput,
    W: Write,
{
    loop {
        let (player_one, player_one_show_after) =
            match prompt_setup(input, output, Player::One, typed_history)? {
                SetupPrompt::Setup { setup, show_after } => (setup, show_after),
                SetupPrompt::Loaded(prompted) => return Ok(Some(prompted)),
                SetupPrompt::Eof => return Ok(None),
            };
        let (player_two, player_two_show_after) =
            match prompt_setup(input, output, Player::Two, typed_history)? {
                SetupPrompt::Setup { setup, show_after } => (setup, show_after),
                SetupPrompt::Loaded(prompted) => return Ok(Some(prompted)),
                SetupPrompt::Eof => return Ok(None),
            };

        match Game::new([player_one, player_two], Player::One) {
            Ok(game) => {
                return Ok(Some(PromptedGame {
                    game,
                    history: None,
                    show_after: player_one_show_after || player_two_show_after,
                }));
            }
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

enum SetupPrompt {
    Setup {
        setup: HomeworldSetup,
        show_after: bool,
    },
    Loaded(PromptedGame),
    Eof,
}

fn prompt_setup<I, W>(
    input: &mut I,
    output: &mut W,
    player: Player,
    typed_history: &mut TypedHistory,
) -> io::Result<SetupPrompt>
where
    I: LineInput,
    W: Write,
{
    loop {
        writeln!(output, "{} setup", player_label(player))?;
        let prompt = format!("{} stars> ", player_label(player));
        let stars = match read_prompted_line(input, &prompt, output)? {
            PromptLine::Read(line) => line,
            PromptLine::Eof => return Ok(SetupPrompt::Eof),
            PromptLine::Interrupted => {
                write_quit_message(output)?;
                return Ok(SetupPrompt::Eof);
            }
        };
        let stars_line = stars.trim();
        record_user_history(input, typed_history, stars_line)?;
        let stars_show_after = command_requests_state_after_error(stars_line);

        if let Some(parsed) = parse_setup_load(stars_line) {
            match parsed {
                Ok((path, show_after)) => {
                    if let Some(prompted) = load_game_from_setup(path, show_after, output)? {
                        return Ok(SetupPrompt::Loaded(prompted));
                    }
                }
                Err(error) => writeln!(output, "Error: {}", error.message())?,
            }
            continue;
        }

        let prompt = format!("{} ship> ", player_label(player));
        let ship = match read_prompted_line(input, &prompt, output)? {
            PromptLine::Read(line) => line,
            PromptLine::Eof => return Ok(SetupPrompt::Eof),
            PromptLine::Interrupted => {
                write_quit_message(output)?;
                return Ok(SetupPrompt::Eof);
            }
        };
        let ship_line = ship.trim();
        record_user_history(input, typed_history, ship_line)?;
        let show_after = stars_show_after || command_requests_state_after_error(ship_line);

        match parse_setup(stars_line, ship_line, player) {
            Ok(setup) => return Ok(SetupPrompt::Setup { setup, show_after }),
            Err(error) => writeln!(output, "Error: {}", error.message())?,
        }
    }
}

fn parse_setup_load(line: &str) -> Option<Result<(PathBuf, bool), crate::parser::ParseError>> {
    let first = line.split_whitespace().next()?;
    if !matches!(first.to_ascii_lowercase().as_str(), "load" | "l") {
        return None;
    }

    Some(
        parse_input(line, Player::One).map(|parsed| match parsed.command {
            ParsedCommand::Load(path) => (path, parsed.show_after),
            _ => unreachable!("load command prefix parsed as a non-load command"),
        }),
    )
}

fn load_game_from_setup<W: Write>(
    path: PathBuf,
    show_after: bool,
    output: &mut W,
) -> io::Result<Option<PromptedGame>> {
    match read_load_source(&path) {
        Ok(LoadSource::Save(saved)) => {
            writeln!(output, "Loaded from {}.", path.display())?;
            let history = if saved.commands.is_empty() {
                None
            } else {
                Some(LoadedHistory {
                    path,
                    commands: saved.commands.join("\n"),
                    show_after,
                })
            };
            let show_after = if history.is_some() { false } else { show_after };
            Ok(Some(PromptedGame {
                game: saved.game,
                history,
                show_after,
            }))
        }
        Ok(LoadSource::History(history)) => match game_from_history_setup(&history) {
            Ok((game, commands)) => Ok(Some(PromptedGame {
                game,
                history: Some(LoadedHistory {
                    path,
                    commands,
                    show_after,
                }),
                show_after: false,
            })),
            Err(error) => {
                writeln!(output, "Error: {error}")?;
                Ok(None)
            }
        },
        Err(error) => {
            writeln!(output, "Error: {error}")?;
            Ok(None)
        }
    }
}

#[derive(Debug)]
enum SetupLoadError {
    MissingSetupLine(&'static str),
    InvalidSetup { player: Player, error: String },
    InvalidGame(String),
}

impl fmt::Display for SetupLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSetupLine(label) => {
                write!(formatter, "command history is missing {label}")
            }
            Self::InvalidSetup { player, error } => {
                write!(
                    formatter,
                    "invalid {} setup in command history: {error}",
                    player_label(*player)
                )
            }
            Self::InvalidGame(error) => {
                write!(
                    formatter,
                    "invalid homeworld setup in command history: {error}"
                )
            }
        }
    }
}

fn game_from_history_setup(history: &str) -> Result<(Game, String), SetupLoadError> {
    let mut commands = history_commands(history);
    let player_one_stars = next_history_setup_line(&mut commands, "Player 1 stars")?;
    let player_one_ship = next_history_setup_line(&mut commands, "Player 1 ship")?;
    let player_two_stars = next_history_setup_line(&mut commands, "Player 2 stars")?;
    let player_two_ship = next_history_setup_line(&mut commands, "Player 2 ship")?;

    let player_one =
        parse_setup(player_one_stars, player_one_ship, Player::One).map_err(|error| {
            SetupLoadError::InvalidSetup {
                player: Player::One,
                error: error.to_string(),
            }
        })?;
    let player_two =
        parse_setup(player_two_stars, player_two_ship, Player::Two).map_err(|error| {
            SetupLoadError::InvalidSetup {
                player: Player::Two,
                error: error.to_string(),
            }
        })?;
    let game = Game::new([player_one, player_two], Player::One)
        .map_err(|error| SetupLoadError::InvalidGame(format_game_error(&error)))?;
    let commands = commands.collect::<Vec<_>>().join("\n");

    Ok((game, commands))
}

fn next_history_setup_line<'a>(
    commands: &mut impl Iterator<Item = &'a str>,
    label: &'static str,
) -> Result<&'a str, SetupLoadError> {
    commands
        .next()
        .ok_or(SetupLoadError::MissingSetupLine(label))
}

fn history_commands(history: &str) -> impl Iterator<Item = &str> {
    history.lines().filter_map(history_command)
}

fn history_command(line: &str) -> Option<&str> {
    let command = line
        .split_once('#')
        .map_or(line, |(command, _)| command)
        .trim();
    (!command.is_empty()).then_some(command)
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
    use std::{collections::VecDeque, fs, io::Cursor, path::PathBuf};

    use super::*;

    #[derive(Default)]
    struct RecordingLineInput {
        lines: VecDeque<PromptLine>,
        prompts: Vec<String>,
        history_entries: Vec<String>,
    }

    impl RecordingLineInput {
        fn new(lines: impl IntoIterator<Item = &'static str>) -> Self {
            Self::with_lines(
                lines
                    .into_iter()
                    .map(|line| PromptLine::Read(line.to_owned())),
            )
        }

        fn with_lines(lines: impl IntoIterator<Item = PromptLine>) -> Self {
            Self {
                lines: lines.into_iter().collect(),
                prompts: Vec::new(),
                history_entries: Vec::new(),
            }
        }
    }

    impl LineInput for RecordingLineInput {
        fn read_prompted_line<W: Write>(
            &mut self,
            prompt: &str,
            _output: &mut W,
        ) -> io::Result<PromptLine> {
            self.prompts.push(prompt.to_owned());
            Ok(self.lines.pop_front().unwrap_or(PromptLine::Eof))
        }

        fn add_history_entry(&mut self, line: &str) -> io::Result<()> {
            self.history_entries.push(line.to_owned());
            Ok(())
        }
    }

    #[test]
    fn line_input_prompts_and_records_shared_session_history() {
        let mut input = RecordingLineInput::new(["ys bm", "gs", "bl rl", "rm", "show", "q"]);
        let mut output = Vec::new();

        run_with_line_input(&mut input, &mut output).expect("session runs");

        assert_eq!(
            input.prompts,
            [
                "Player 1 stars> ",
                "Player 1 ship> ",
                "Player 2 stars> ",
                "Player 2 ship> ",
                "P1> ",
                "P1> ",
            ]
        );
        assert_eq!(
            input.history_entries,
            ["ys bm", "gs", "bl rl", "rm", "show", "q"]
        );
    }

    #[test]
    fn ctrl_c_quits_during_setup() {
        let mut input = RecordingLineInput::with_lines([PromptLine::Interrupted]);
        let mut output = Vec::new();

        run_with_line_input(&mut input, &mut output).expect("session exits cleanly");

        let output = String::from_utf8(output).expect("output is utf8");
        assert_eq!(input.prompts, ["Player 1 stars> "]);
        assert!(input.history_entries.is_empty());
        assert!(output.contains("Goodbye."));
        assert!(!output.contains("Game started."));
    }

    #[test]
    fn ctrl_c_quits_during_command_prompt() {
        let mut input = RecordingLineInput::with_lines([
            PromptLine::Read("ys bm".to_owned()),
            PromptLine::Read("gs".to_owned()),
            PromptLine::Read("bl rl".to_owned()),
            PromptLine::Read("rm".to_owned()),
            PromptLine::Interrupted,
        ]);
        let mut output = Vec::new();

        run_with_line_input(&mut input, &mut output).expect("session exits cleanly");

        let output = String::from_utf8(output).expect("output is utf8");
        assert_eq!(
            input.prompts,
            [
                "Player 1 stars> ",
                "Player 1 ship> ",
                "Player 2 stars> ",
                "Player 2 ship> ",
                "P1> ",
            ]
        );
        assert_eq!(input.history_entries, ["ys bm", "gs", "bl rl", "rm"]);
        assert!(output.contains("Game started."));
        assert!(output.contains("Goodbye."));
    }

    #[test]
    fn user_history_policy_filters_empty_and_save_history_lines() {
        assert!(should_record_user_history("ys bm"));
        assert!(should_record_user_history("show"));
        assert!(should_record_user_history("q"));

        assert!(!should_record_user_history(""));
        assert!(!should_record_user_history("   "));
        assert!(!should_record_user_history("save-history game.yaml"));
        assert!(!should_record_user_history("sh game.yaml"));
        assert!(!should_record_user_history("SH game.yaml"));
    }

    #[test]
    fn tab_completion_expands_exact_command_shorthands() {
        for (input, expected) in [
            ("h", "help"),
            ("e", "end"),
            ("q", "quit"),
            ("v", "save"),
            ("sh", "save-history"),
            ("l", "load"),
            ("b", "build"),
            ("t", "travel"),
            ("tr", "trade"),
            ("x", "trade"),
            ("sac", "sacrifice"),
            ("i", "invade"),
            ("c", "catastrophe"),
            ("B", "build"),
        ] {
            assert_eq!(
                command_completion(input, input.len()),
                Some((0, expected)),
                "{input} should complete to {expected}"
            );
        }
    }

    #[test]
    fn tab_completion_expands_unique_command_partials() {
        for (input, expected) in [
            ("a", "ai"),
            ("bu", "build"),
            ("cat", "catastrophe"),
            ("en", "end"),
            ("he", "help"),
            ("inv", "invade"),
            ("lo", "load"),
            ("qui", "quit"),
            ("sho", "show"),
            ("save-h", "save-history"),
            ("trav", "travel"),
        ] {
            assert_eq!(
                command_completion(input, input.len()),
                Some((0, expected)),
                "{input} should complete to {expected}"
            );
        }
    }

    #[test]
    fn tab_completion_preserves_text_after_first_token() {
        assert_eq!(command_completion("b 0 gs", 1), Some((0, "build")));
        assert_eq!(command_completion("  b 0 gs", 3), Some((2, "build")));
    }

    #[test]
    fn tab_completion_ignores_ambiguous_partials_arguments_and_semicolons() {
        for input in [
            "", "   ", "s", "show", "build", "save", "sa", "sav", "b;", "b 0 gs;",
        ] {
            assert_eq!(command_completion(input, input.len()), None, "{input}");
        }

        assert_eq!(command_completion("b ", 2), None);
        assert_eq!(command_completion("b 0 gs", "b 0 gs".len()), None);
    }

    #[test]
    fn tab_completion_suggests_system_ids_for_system_arguments() {
        let snapshot = CompletionSnapshot::from_game(&Game::default(Player::One));

        assert_eq!(
            completion_candidates("b ", 2, &snapshot),
            Some((2, vec!["0".to_owned(), "1".to_owned()]))
        );
        assert_eq!(
            completion_candidates("s ", 2, &snapshot),
            Some((2, vec!["0".to_owned(), "1".to_owned()]))
        );
    }

    #[test]
    fn tab_completion_suggests_visible_piece_arguments() {
        let snapshot = CompletionSnapshot::from_game(&Game::default(Player::One));

        assert_eq!(
            completion_candidates("b 0 g", 5, &snapshot),
            Some((4, vec!["gs".to_owned()]))
        );
        assert_eq!(
            completion_candidates("i 1 r", 5, &snapshot),
            Some((4, vec!["rs".to_owned(), "rm".to_owned()]))
        );
    }

    #[test]
    fn tab_completion_suggests_travel_target_words_and_catastrophe_colors() {
        let snapshot = CompletionSnapshot::from_game(&Game::default(Player::One));

        assert_eq!(
            completion_candidates("t 0 gs ", 7, &snapshot),
            Some((
                7,
                vec![
                    "existing".to_owned(),
                    "new".to_owned(),
                    "x".to_owned(),
                    "n".to_owned(),
                ],
            ))
        );
        assert_eq!(
            completion_candidates("t 0 gs e", 8, &snapshot),
            Some((7, vec!["existing".to_owned()]))
        );
        assert_eq!(
            completion_candidates("c 0 r", 5, &snapshot),
            Some((4, vec!["red".to_owned()]))
        );
    }

    #[test]
    fn tab_completion_suggests_ai_arguments() {
        assert_eq!(
            completion_candidates("ai ", 3, &CompletionSnapshot::default()),
            Some((3, vec!["show".to_owned(), "p1".to_owned(), "p2".to_owned()]))
        );
        assert_eq!(
            completion_candidates("ai p2 p", 7, &CompletionSnapshot::default()),
            Some((6, vec!["priority".to_owned()]))
        );
        assert_eq!(
            completion_candidates("ai p2 s", 7, &CompletionSnapshot::default()),
            Some((6, vec!["search".to_owned()]))
        );
    }

    #[test]
    fn tab_completion_suggests_paths_for_file_arguments() {
        let temp_dir =
            std::env::temp_dir().join(format!("hw-cli-tab-completion-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("temp dir can be created");
        std::fs::write(temp_dir.join("game.yaml"), "").expect("temp file can be written");
        std::fs::write(temp_dir.join("history.hw"), "").expect("temp file can be written");

        let prefix = format!("{}/g", temp_dir.display());
        let line = format!("load {prefix}");
        assert_eq!(
            completion_candidates(&line, line.len(), &CompletionSnapshot::default()),
            Some((5, vec![temp_dir.join("game.yaml").display().to_string()]))
        );

        std::fs::remove_dir_all(&temp_dir).expect("temp dir can be removed");
    }

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
    fn setup_semicolon_prints_state_once_after_setup() {
        let output = run_script(
            "ys bm;
gs;
bl rl;
rm;
q
",
        );

        assert!(!output.contains("Error:"));
        assert!(output.contains("Game started."));
        assert_eq!(output.matches("Status: in progress").count(), 1);
        assert_eq!(output.matches("Current player: Player 1").count(), 1);
        assert_eq!(output.matches("Remaining actions: 1").count(), 1);
        assert!(output.contains("[0] homeworld Player 1"));
        assert!(output.contains("[1] homeworld Player 2"));
    }

    #[test]
    fn short_action_notation_drives_hot_seat_turns() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
b 0 gs
s
q
",
        );

        assert!(output.contains("Action applied."));
        assert!(output.contains("Turn ended."));
        assert!(output.contains("Current player: Player 2"));
        assert!(output.contains("Remaining actions: 1"));
        assert!(!output.contains("Error:"));
    }

    #[test]
    fn ai_command_reports_selectable_strategies() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
ai
ai p2 first
ai
ai p2 off
ai
q
",
        );

        assert!(output.contains("AI players: Player 1 human, Player 2 human"));
        assert!(output.contains("AI Player 2 set to first."));
        assert!(output.contains("AI players: Player 1 human, Player 2 first"));
        assert!(output.contains("AI Player 2 disabled."));
        assert!(!output.contains("Error:"));
    }

    #[test]
    fn ai_command_accepts_search_strategy() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
ai p2 search
ai
q
",
        );

        assert!(output.contains("AI Player 2 set to search."));
        assert!(output.contains("AI players: Player 1 human, Player 2 search"));
        assert!(!output.contains("Error:"));
    }

    #[test]
    fn enabling_ai_for_current_player_runs_immediately() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
ai p1 priority
q
",
        );

        assert!(output.contains("AI Player 1 set to priority."));
        assert!(output.contains("P1> b 0 rs"));
        assert!(output.contains("Action applied."));
        assert!(output.contains("Turn ended."));
        assert!(output.contains("Current player: Player 2"));
        assert!(!output.contains("Error:"));
    }

    #[test]
    fn ai_runs_after_human_turn_reaches_ai_player() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
ai p2 priority
b 0 rs
q
",
        );

        assert!(output.contains("AI Player 2 set to priority."));
        assert!(output.contains("P2> x 1 rm ym"));
        assert!(output.contains("Action applied."));
        assert!(output.contains("Turn ended."));
        assert!(output.contains("Current player: Player 1"));
        assert!(!output.contains("Error:"));
    }

    #[test]
    fn disabling_ai_stops_automatic_play() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
ai p2 priority
ai p2 off
b 0 rs
q
",
        );

        assert!(output.contains("AI Player 2 set to priority."));
        assert!(output.contains("AI Player 2 disabled."));
        assert!(!output.contains("P2> x "));
        assert!(output.contains("Current player: Player 2"));
        assert!(!output.contains("Error:"));
    }

    #[test]
    fn ai_configuration_survives_load_but_is_not_saved() {
        let save_path = temp_save_path("ai_configuration_survives_load_but_is_not_saved");
        let loaded_path = temp_save_path("ai_configuration_loaded_game");
        fs::write(
            &loaded_path,
            save::to_yaml(&Game::default(Player::Two)).expect("game serializes"),
        )
        .expect("loaded game fixture writes");
        let script = format!(
            "ys bm
gs
bl rl
rm
ai p2 priority
v {}
l {}
q
",
            save_path.display(),
            loaded_path.display()
        );

        let output = run_script(&script);
        let yaml = fs::read_to_string(&save_path).expect("save file exists");
        let _ = fs::remove_file(save_path);
        let _ = fs::remove_file(loaded_path);

        assert!(output.contains("Loaded from "));
        assert!(output.contains("P2> x 1 rm ym"));
        assert!(!yaml.contains("priority"));
        assert!(!yaml.contains("first"));
        assert!(!yaml.contains("ai:"));
        assert!(!output.contains("Error:"));
    }

    #[test]
    fn paid_action_auto_ends_when_no_catastrophe_is_possible() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
b 0 gs
q
",
        );

        assert!(output.contains("Action applied."));
        assert!(output.contains("Turn ended."));
        assert!(output.contains("Current player: Player 2"));
        assert!(output.contains("Remaining actions: 1"));
    }

    #[test]
    fn paid_action_does_not_auto_end_when_catastrophe_is_possible() {
        let output = run_script(
            "gs gm
gl
ys rm
bl
b 0 gs;
q
",
        );

        assert!(output.contains("Action applied."));
        assert!(!output.contains("Turn ended."));
        assert!(output.contains("Current player: Player 1"));
        assert!(output.contains("Remaining actions: 0"));
        assert!(output.contains("Stars: gs, gm"));
        assert!(output.contains("Ships: P1 gl, P1 gs"));
    }

    #[test]
    fn catastrophe_auto_ends_after_budget_is_spent_when_none_remain() {
        let output = run_script(
            "gs gm
gl
ys rm
bl
b 0 gs
c 0 g
q
",
        );

        assert!(output.contains("Action applied."));
        assert!(output.contains("Turn ended."));
        assert!(output.contains("Status: finished, winner Player 2"));
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
    fn semicolon_prints_state_after_a_parse_error() {
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
        assert!(output.contains("Status: in progress"));
        assert!(output.contains("Current player: Player 1"));
        assert!(output.contains("Remaining actions: 1"));
    }

    #[test]
    fn semicolon_prints_state_after_an_action_error() {
        let output = run_script(
            "ys bm
gs
bl rl
rm
b 9 gs;
q
",
        );

        assert!(output.contains("Error: Turn(InvalidAction(InvalidAction(UnknownSystem"));
        assert!(output.contains("Status: in progress"));
        assert!(output.contains("Current player: Player 1"));
        assert!(output.contains("Remaining actions: 1"));
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
        assert!(!yaml.contains("history:"));
        assert!(!yaml.contains("commands:"));
    }

    #[test]
    fn save_history_command_writes_typed_history_only() {
        let replay_path = temp_history_path("save_history_command_replay_fixture");
        let save_path = temp_save_path("save_history_command_writes_typed_history_only");
        fs::write(
            &replay_path,
            "show
",
        )
        .expect("history fixture writes");
        let script = format!(
            "ys bm
gs
bl rl
rm
l {}
bad;
sh {}
q
",
            replay_path.display(),
            save_path.display()
        );

        let output = run_script(&script);
        let yaml = fs::read_to_string(&save_path).expect("save file exists");
        let _ = fs::remove_file(replay_path);
        let _ = fs::remove_file(save_path);

        assert!(output.contains("Saved history to "));
        assert!(yaml.contains("history:"));
        assert!(yaml.contains("- ys bm"));
        assert!(yaml.contains("- gs"));
        assert!(yaml.contains("- bl rl"));
        assert!(yaml.contains("- rm"));
        assert!(yaml.contains("- l "));
        assert!(yaml.contains("- bad;"));
        assert!(!yaml.contains("- show"));
        assert!(!yaml.contains("commands:"));
        assert!(!yaml.contains("sh "));
    }

    #[test]
    fn load_yaml_replays_commands_but_not_history() {
        let path = temp_save_path("load_yaml_replays_commands_but_not_history");
        fs::write(
            &path,
            save::to_yaml_with_extras(
                &Game::default(Player::One),
                &save::SaveExtras {
                    history: vec!["bad;".to_owned()],
                    commands: vec!["show".to_owned()],
                },
            )
            .expect("game serializes"),
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
        assert!(output.contains("P1> show"));
        assert!(output.contains("Status: in progress"));
        assert!(!output.contains("unknown command `bad`"));
    }

    #[test]
    fn load_yaml_with_history_only_does_not_replay() {
        let path = temp_save_path("load_yaml_with_history_only_does_not_replay");
        fs::write(
            &path,
            save::to_yaml_with_extras(
                &Game::default(Player::One),
                &save::SaveExtras {
                    history: vec!["show".to_owned()],
                    commands: Vec::new(),
                },
            )
            .expect("game serializes"),
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
        assert!(!output.contains("P1> show"));
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
        assert!(output.contains("P1> b 0 gs"));
        assert!(output.contains("Action applied."));
        assert!(output.contains("Turn ended."));
        assert!(output.contains("P2> show"));
        assert!(output.contains("Current player: Player 2"));
        assert!(output.contains("Finished commands from "));
        assert!(!output.contains("Error:"));
    }

    #[test]
    fn history_load_ignores_blank_lines_and_comments() {
        let path = temp_history_path("history_load_ignores_blank_lines_and_comments");
        fs::write(
            &path,
            "# Build a ship, then the turn passes automatically.

b 0 gs # use green power

# Comments can occupy whole lines.
show # inspect the resulting state
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

        assert!(!output.contains("Error:"));
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

    #[test]
    fn load_command_at_setup_reads_history_setup_lines() {
        let path = temp_history_path("load_command_at_setup_reads_history_setup_lines");
        fs::write(
            &path,
            "gm ys
bl
ys rm
gl
show
",
        )
        .expect("history fixture writes");
        let script = format!(
            "l {}
q
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(output.contains("Running commands from "));
        assert!(output.contains("Game started."));
        assert!(output.contains("[0] homeworld Player 1"));
        assert!(output.contains("Stars: gm, ys"));
        assert!(output.contains("Ships: P1 bl"));
        assert!(output.contains("[1] homeworld Player 2"));
        assert!(output.contains("Stars: ys, rm"));
        assert!(output.contains("Ships: P2 gl"));
        assert!(output.contains("Finished commands from "));
    }

    #[test]
    fn setup_history_ignores_blank_lines_and_comments() {
        let path = temp_history_path("setup_history_ignores_blank_lines_and_comments");
        fs::write(
            &path,
            "# Setup starts below.

gm ys # Player 1 stars
bl # Player 1 ship

# Player 2 setup.
ys rm # Player 2 stars
gl # Player 2 ship

show # remaining commands still run
",
        )
        .expect("history fixture writes");
        let script = format!(
            "l {}
q
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(!output.contains("Error:"));
        assert!(output.contains("Game started."));
        assert!(output.contains("Stars: gm, ys"));
        assert!(output.contains("Ships: P1 bl"));
        assert!(output.contains("Stars: ys, rm"));
        assert!(output.contains("Ships: P2 gl"));
        assert!(output.contains("P1> show"));
    }

    #[test]
    fn load_command_at_setup_accepts_yaml_saves() {
        let path = temp_save_path("load_command_at_setup_accepts_yaml_saves");
        fs::write(
            &path,
            save::to_yaml(&Game::default(Player::Two)).expect("game serializes"),
        )
        .expect("save fixture writes");
        let script = format!(
            "load {}
q
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(output.contains("Loaded from "));
        assert!(output.contains("Game started."));
        assert!(output.contains("Current player: Player 2"));
    }

    #[test]
    fn failed_setup_load_keeps_prompting_for_setup() {
        let path = temp_history_path("failed_setup_load_keeps_prompting_for_setup");
        fs::write(
            &path,
            "ys bm
gs
",
        )
        .expect("history fixture writes");
        let script = format!(
            "l {}
ys bm
gs
bl rl
rm
show
q
",
            path.display()
        );

        let output = run_script(&script);
        let _ = fs::remove_file(path);

        assert!(output.contains("Error: command history is missing Player 2 stars"));
        assert!(output.contains("Game started."));
        assert!(output.contains("Stars: ys, bm"));
        assert!(output.contains("Stars: bl, rl"));
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
