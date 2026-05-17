use std::{fs, path::Path};

use hw_ai::{AiDecision, FirstLegalStrategy, PriorityStrategy, SearchStrategy, Strategy};
use hw_cli::{
    parser::{AiCommand, AiStrategy, ParsedCommand, parse_input, parse_setup},
    render::{render_game, render_turn_summary},
};
use hw_core::{Color, Piece, Player, Size};
use hw_engine::{
    Action, Game, GameStatus, HomeworldSetup, TravelTarget, has_possible_catastrophe, save,
};

const MAX_AI_DECISIONS: usize = 512;
const MAX_LOAD_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppMode {
    Setup,
    Playing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SetupStep {
    PlayerOneStars,
    PlayerOneShip,
    PlayerTwoStars,
    PlayerTwoShip,
}

impl SetupStep {
    const fn prompt(self) -> &'static str {
        match self {
            Self::PlayerOneStars => "Player 1 stars",
            Self::PlayerOneShip => "Player 1 ship",
            Self::PlayerTwoStars => "Player 2 stars",
            Self::PlayerTwoShip => "Player 2 ship",
        }
    }
}

#[derive(Clone, Debug)]
struct SetupState {
    step: SetupStep,
    player_one_stars: Option<String>,
    player_one: Option<HomeworldSetup>,
    player_two_stars: Option<String>,
    show_after: bool,
}

impl Default for SetupState {
    fn default() -> Self {
        Self {
            step: SetupStep::PlayerOneStars,
            player_one_stars: None,
            player_one: None,
            player_two_stars: None,
            show_after: false,
        }
    }
}

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

#[derive(Clone, Debug)]
pub struct TuiApp {
    mode: AppMode,
    setup: SetupState,
    game: Option<Game>,
    ai_control: AiControl,
    input: String,
    messages: Vec<String>,
    show_help: bool,
    should_quit: bool,
    typed_history: Vec<String>,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Setup,
            setup: SetupState::default(),
            game: None,
            ai_control: AiControl::default(),
            input: String::new(),
            messages: vec!["Homeworlds TUI".to_owned()],
            show_help: false,
            should_quit: false,
            typed_history: Vec::new(),
        }
    }

    pub const fn mode(&self) -> AppMode {
        self.mode
    }

    pub fn prompt(&self) -> &'static str {
        match self.mode {
            AppMode::Setup => self.setup.step.prompt(),
            AppMode::Playing => self
                .game
                .as_ref()
                .map(|game| prompt_label(game.turn().current_player()))
                .unwrap_or("P1"),
        }
    }

    pub fn prompt_text(&self) -> String {
        format!("{}> ", self.prompt())
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }

    pub fn game(&self) -> Option<&Game> {
        self.game.as_ref()
    }

    pub const fn show_help(&self) -> bool {
        self.show_help
    }

    pub const fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn push_char(&mut self, ch: char) {
        self.input.push(ch);
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    pub fn submit_input(&mut self) {
        let line = std::mem::take(&mut self.input);
        self.submit_line(&line);
    }

    pub fn submit_line(&mut self, line: &str) {
        self.submit_line_with_depth(line, 0, true);
    }

    fn submit_line_with_depth(&mut self, line: &str, load_depth: usize, record_history: bool) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }

        if record_history && should_record_user_history(line) {
            self.typed_history.push(line.to_owned());
        }
        match self.mode {
            AppMode::Setup => self.handle_setup_line(line, load_depth),
            AppMode::Playing => self.handle_game_line(line, load_depth),
        }
    }

    fn handle_setup_line(&mut self, line: &str, load_depth: usize) {
        if let Some(load) = parse_load_command(line) {
            match load {
                Ok((path, show_after)) => self.load_from_setup(&path, show_after, load_depth),
                Err(error) => self.push_message(format!("Error: {}", error.message())),
            }
            return;
        }

        self.setup.show_after |= command_requests_state_after_error(line);
        match self.setup.step {
            SetupStep::PlayerOneStars => {
                self.setup.player_one_stars = Some(line.to_owned());
                self.setup.step = SetupStep::PlayerOneShip;
            }
            SetupStep::PlayerOneShip => {
                let stars = self.setup.player_one_stars.clone().unwrap_or_default();
                match parse_setup(&stars, line, Player::One) {
                    Ok(setup) => {
                        self.setup.player_one = Some(setup);
                        self.setup.step = SetupStep::PlayerTwoStars;
                    }
                    Err(error) => {
                        self.push_message(format!("Error: {}", error.message()));
                        self.setup.player_one_stars = None;
                        self.setup.step = SetupStep::PlayerOneStars;
                    }
                }
            }
            SetupStep::PlayerTwoStars => {
                self.setup.player_two_stars = Some(line.to_owned());
                self.setup.step = SetupStep::PlayerTwoShip;
            }
            SetupStep::PlayerTwoShip => {
                let stars = self.setup.player_two_stars.clone().unwrap_or_default();
                let player_one = self.setup.player_one.clone();
                match (
                    player_one,
                    parse_setup(&stars, line, Player::Two).map_err(|error| error.to_string()),
                ) {
                    (Some(player_one), Ok(player_two)) => {
                        match Game::new([player_one, player_two], Player::One) {
                            Ok(game) => self.start_game(game),
                            Err(error) => {
                                self.push_message(format!(
                                    "Error: invalid homeworld setup: {}",
                                    format_game_error(&error)
                                ));
                                self.setup = SetupState::default();
                            }
                        }
                    }
                    (_, Err(error)) => {
                        self.push_message(format!("Error: {error}"));
                        self.setup.player_two_stars = None;
                        self.setup.step = SetupStep::PlayerTwoStars;
                    }
                    (None, Ok(_)) => {
                        self.push_message("Error: missing Player 1 setup".to_owned());
                        self.setup = SetupState::default();
                    }
                }
            }
        }
    }

    fn start_game(&mut self, game: Game) {
        let show_after = self.setup.show_after;
        self.game = Some(game);
        self.mode = AppMode::Playing;
        self.setup = SetupState::default();
        self.push_message("Game started.");
        if show_after {
            self.push_game_render();
        } else {
            self.push_turn_summary();
        }
    }

    fn load_from_setup(&mut self, path: &Path, show_after: bool, load_depth: usize) {
        if load_depth >= MAX_LOAD_DEPTH {
            self.push_message("Error: load nesting limit exceeded");
            return;
        }

        match read_load_source(path) {
            Ok(LoadSource::Save(saved)) => {
                self.game = Some(saved.game);
                self.mode = AppMode::Playing;
                self.setup = SetupState::default();
                self.push_message(format!("Loaded from {}.", path.display()));
                if show_after {
                    self.push_game_render();
                } else {
                    self.push_turn_summary();
                }
                if !saved.commands.is_empty() {
                    self.push_message(format!("Running commands from {}.", path.display()));
                    self.run_history_commands(&saved.commands.join("\n"), load_depth + 1);
                    self.push_message(format!("Finished commands from {}.", path.display()));
                }
                self.run_ai_turns();
            }
            Ok(LoadSource::History(history)) => match game_from_history_setup(&history) {
                Ok((game, commands)) => {
                    self.game = Some(game);
                    self.mode = AppMode::Playing;
                    self.setup = SetupState::default();
                    self.push_message(format!("Loaded setup from {}.", path.display()));
                    self.push_turn_summary();
                    self.push_message(format!("Running commands from {}.", path.display()));
                    self.run_history_commands(&commands, load_depth + 1);
                    self.push_message(format!("Finished commands from {}.", path.display()));
                    self.run_ai_turns();
                }
                Err(error) => self.push_message(format!("Error: {error}")),
            },
            Err(error) => self.push_message(format!("Error: {error}")),
        }
    }

    fn handle_game_line(&mut self, line: &str, load_depth: usize) {
        if self.game.is_none() {
            self.push_message("Error: game is not started");
            return;
        }

        let Some(current_player) = self.game.as_ref().map(|game| game.turn().current_player())
        else {
            self.push_message("Error: game is not started");
            return;
        };
        let show_after_error = command_requests_state_after_error(line);
        match parse_input(line, current_player) {
            Ok(parsed) => match parsed.command {
                ParsedCommand::Help => {
                    self.show_help = !self.show_help;
                    self.push_message(if self.show_help {
                        "Help shown."
                    } else {
                        "Help hidden."
                    });
                    self.push_render_after_semicolon(parsed.show_after);
                }
                ParsedCommand::Show => self.push_game_render(),
                ParsedCommand::Quit => {
                    self.should_quit = true;
                    self.push_message("Goodbye.");
                }
                ParsedCommand::Ai(command) => {
                    self.run_ai_command(command);
                    self.push_render_after_semicolon(parsed.show_after);
                    self.run_ai_turns();
                }
                ParsedCommand::Save(path) => {
                    let Some(game) = self.game.as_ref() else {
                        self.push_message("Error: game is not started");
                        return;
                    };
                    match save::save_file(game, &path) {
                        Ok(()) => self.push_message(format!("Saved to {}.", path.display())),
                        Err(error) => self.push_message(format!("Error: {error}")),
                    }
                    self.push_render_after_semicolon(parsed.show_after);
                }
                ParsedCommand::SaveHistory(path) => {
                    let extras = save::SaveExtras {
                        history: self.typed_history.clone(),
                        commands: Vec::new(),
                    };
                    let Some(game) = self.game.as_ref() else {
                        self.push_message("Error: game is not started");
                        return;
                    };
                    match save::save_file_with_extras(game, &extras, &path) {
                        Ok(()) => {
                            self.push_message(format!("Saved history to {}.", path.display()))
                        }
                        Err(error) => self.push_message(format!("Error: {error}")),
                    }
                    self.push_render_after_semicolon(parsed.show_after);
                }
                ParsedCommand::Load(path) => {
                    self.load_in_game(&path, parsed.show_after, load_depth)
                }
                ParsedCommand::End => {
                    let Some(game) = self.game.as_ref() else {
                        self.push_message("Error: game is not started");
                        return;
                    };
                    match game.end_turn() {
                        Ok(next) => {
                            self.game = Some(next);
                            self.push_message("Turn ended.");
                            self.push_turn_summary();
                            self.push_render_after_semicolon(parsed.show_after);
                            self.run_ai_turns();
                        }
                        Err(error) => {
                            self.push_message(format!("Error: {}", format_game_error(&error)));
                            self.push_render_after_semicolon(parsed.show_after);
                        }
                    }
                }
                ParsedCommand::Action(action) => {
                    self.apply_user_action(action);
                    self.push_render_after_semicolon(parsed.show_after);
                    self.run_ai_turns();
                }
            },
            Err(error) => {
                self.push_message(format!("Error: {}", error.message()));
                self.push_render_after_semicolon(show_after_error);
            }
        }
    }

    fn load_in_game(&mut self, path: &Path, show_after: bool, load_depth: usize) {
        if load_depth >= MAX_LOAD_DEPTH {
            self.push_message("Error: load nesting limit exceeded");
            return;
        }

        match read_load_source(path) {
            Ok(LoadSource::Save(saved)) => {
                self.game = Some(saved.game);
                self.push_message(format!("Loaded from {}.", path.display()));
                self.push_turn_summary();
                if !saved.commands.is_empty() {
                    self.push_message(format!("Running commands from {}.", path.display()));
                    self.run_history_commands(&saved.commands.join("\n"), load_depth + 1);
                    self.push_message(format!("Finished commands from {}.", path.display()));
                }
                self.push_render_after_semicolon(show_after);
                self.run_ai_turns();
            }
            Ok(LoadSource::History(history)) => {
                self.push_message(format!("Running commands from {}.", path.display()));
                self.run_history_commands(&history, load_depth + 1);
                self.push_message(format!("Finished commands from {}.", path.display()));
                self.push_render_after_semicolon(show_after);
                self.run_ai_turns();
            }
            Err(error) => {
                self.push_message(format!("Error: {error}"));
                self.push_render_after_semicolon(show_after);
            }
        }
    }

    fn run_history_commands(&mut self, history: &str, load_depth: usize) {
        for command in history_commands(history) {
            if self.should_quit {
                break;
            }
            if let Some(game) = &self.game {
                self.push_message(format!(
                    "{}> {command}",
                    prompt_label(game.turn().current_player())
                ));
            }
            self.submit_line_with_depth(command, load_depth, false);
        }
    }

    fn run_ai_command(&mut self, command: AiCommand) {
        match command {
            AiCommand::Show => self.push_message(render_ai_control(&self.ai_control)),
            AiCommand::Set { player, strategy } => {
                self.ai_control.set_strategy(player, strategy);
                self.push_message(format!(
                    "AI {} set to {}.",
                    player_label(player),
                    ai_strategy_label(strategy)
                ));
            }
            AiCommand::Off { player } => {
                self.ai_control.disable(player);
                self.push_message(format!("AI {} disabled.", player_label(player)));
            }
        }
    }

    fn apply_user_action(&mut self, action: Action) {
        let Some(game) = self.game.as_ref() else {
            self.push_message("Error: game is not started");
            return;
        };

        match game.apply_action(&action) {
            Ok(next) => {
                self.game = Some(next);
                self.push_message("Action applied.");
                self.auto_end_turn_if_ready();
                self.push_turn_summary();
            }
            Err(error) => self.push_message(format!("Error: {}", format_game_error(&error))),
        }
    }

    fn run_ai_turns(&mut self) {
        for _ in 0..MAX_AI_DECISIONS {
            let Some(game) = &self.game else {
                return;
            };
            if game.status() != GameStatus::InProgress {
                return;
            }

            let player = game.turn().current_player();
            let Some(strategy) = self.ai_control.strategy(player) else {
                return;
            };
            let Some(decision) = choose_ai_decision(strategy, game) else {
                self.push_message("AI has no legal decision.");
                return;
            };

            self.push_message(format!(
                "{}> {}",
                prompt_label(player),
                ai_decision_command(&decision)
            ));
            self.apply_ai_decision(decision);
        }

        self.push_message(format!("AI stopped after {MAX_AI_DECISIONS} decisions."));
    }

    fn apply_ai_decision(&mut self, decision: AiDecision) {
        match decision {
            AiDecision::Action(action) => {
                let Some(game) = self.game.as_ref() else {
                    self.push_message("Error: game is not started");
                    return;
                };
                match game.apply_action(&action) {
                    Ok(next) => {
                        self.game = Some(next);
                        self.push_message("Action applied.");
                        self.auto_end_turn_if_ready();
                        self.push_turn_summary();
                    }
                    Err(error) => {
                        self.push_message(format!("Error: {}", format_game_error(&error)))
                    }
                }
            }
            AiDecision::EndTurn => {
                let Some(game) = self.game.as_ref() else {
                    self.push_message("Error: game is not started");
                    return;
                };
                match game.end_turn() {
                    Ok(next) => {
                        self.game = Some(next);
                        self.push_message("Turn ended.");
                        self.push_turn_summary();
                    }
                    Err(error) => {
                        self.push_message(format!("Error: {}", format_game_error(&error)))
                    }
                }
            }
        }
    }

    fn auto_end_turn_if_ready(&mut self) {
        let Some(game) = &self.game else {
            return;
        };
        if game.status() != GameStatus::InProgress
            || game.turn().remaining_actions() != 0
            || has_possible_catastrophe(game.turn().state())
        {
            return;
        }

        match game.end_turn() {
            Ok(next) => {
                self.game = Some(next);
                self.push_message("Turn ended.");
            }
            Err(error) => self.push_message(format!("Error: {}", format_game_error(&error))),
        }
    }

    fn push_turn_summary(&mut self) {
        if let Some(game) = &self.game {
            self.push_message_lines(&render_turn_summary(game));
        }
    }

    fn push_game_render(&mut self) {
        if let Some(game) = &self.game {
            self.push_message_lines(&render_game(game));
        }
    }

    fn push_render_after_semicolon(&mut self, show_after: bool) {
        if show_after {
            self.push_game_render();
        }
    }

    fn push_message(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
        if self.messages.len() > 200 {
            let drop_count = self.messages.len() - 200;
            self.messages.drain(0..drop_count);
        }
    }

    fn push_message_lines(&mut self, message: &str) {
        for line in message.lines() {
            self.push_message(line.to_owned());
        }
    }
}

enum LoadSource {
    Save(save::SavedGame),
    History(String),
}

fn parse_load_command(
    line: &str,
) -> Option<Result<(std::path::PathBuf, bool), hw_cli::parser::ParseError>> {
    let first = line.split_whitespace().next()?;
    if !matches!(first.to_ascii_lowercase().as_str(), "load" | "l") {
        return None;
    }

    let parsed = match parse_input(line, Player::One) {
        Ok(parsed) => parsed,
        Err(error) => return Some(Err(error)),
    };

    match parsed.command {
        ParsedCommand::Load(path) => Some(Ok((path, parsed.show_after))),
        _ => None,
    }
}

fn read_load_source(path: &Path) -> Result<LoadSource, String> {
    let input = fs::read_to_string(path).map_err(|error| format!("I/O error: {error}"))?;
    match save::from_yaml_with_extras(&input) {
        Ok(saved) => Ok(LoadSource::Save(saved)),
        Err(error) if looks_like_yaml_save(&input) => Err(error.to_string()),
        Err(_) => Ok(LoadSource::History(input)),
    }
}

fn looks_like_yaml_save(input: &str) -> bool {
    input.lines().any(|line| line.trim_start() == "version: 1")
}

#[derive(Debug)]
enum SetupLoadError {
    MissingSetupLine(&'static str),
    InvalidSetup { player: Player, error: String },
    InvalidGame(String),
}

impl std::fmt::Display for SetupLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    let commands = history_commands(history).collect::<Vec<_>>();
    let player_one_stars = commands
        .first()
        .ok_or(SetupLoadError::MissingSetupLine("Player 1 stars"))?;
    let player_one_ship = commands
        .get(1)
        .ok_or(SetupLoadError::MissingSetupLine("Player 1 ship"))?;
    let player_two_stars = commands
        .get(2)
        .ok_or(SetupLoadError::MissingSetupLine("Player 2 stars"))?;
    let player_two_ship = commands
        .get(3)
        .ok_or(SetupLoadError::MissingSetupLine("Player 2 ship"))?;

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
    let remaining = commands.into_iter().skip(4).collect::<Vec<_>>().join("\n");

    Ok((game, remaining))
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

fn command_requests_state_after_error(command: &str) -> bool {
    let trimmed = command.trim();
    matches!(trimmed.find(';'), Some(index) if index == trimmed.len() - 1)
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

fn render_ai_control(ai_control: &AiControl) -> String {
    format!(
        "AI players: {} {}, {} {}",
        player_label(Player::One),
        ai_control
            .strategy(Player::One)
            .map_or("human", ai_strategy_label),
        player_label(Player::Two),
        ai_control
            .strategy(Player::Two)
            .map_or("human", ai_strategy_label)
    )
}

fn choose_ai_decision(strategy: AiStrategy, game: &Game) -> Option<AiDecision> {
    match strategy {
        AiStrategy::First => FirstLegalStrategy.choose(game),
        AiStrategy::Priority => PriorityStrategy.choose(game),
        AiStrategy::Search => SearchStrategy::default().choose(game),
    }
}

fn ai_strategy_label(strategy: AiStrategy) -> &'static str {
    match strategy {
        AiStrategy::First => "first",
        AiStrategy::Priority => "priority",
        AiStrategy::Search => "search",
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
            TravelTarget::New { stars } => {
                let stars = stars
                    .iter()
                    .map(|star| compact_piece(*star))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("t {} {} n {}", from.index(), compact_piece(*ship), stars)
            }
        },
        Action::Trade {
            system, from, to, ..
        } => {
            format!(
                "x {} {} {}",
                system.index(),
                compact_piece(*from),
                compact_piece(*to)
            )
        }
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

mod terminal;

pub use terminal::run_stdio;

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use hw_core::Player;
    use hw_engine::{Game, save};

    use super::*;

    #[test]
    fn app_starts_at_first_setup_prompt() {
        let app = TuiApp::new();

        assert_eq!(app.mode(), AppMode::Setup);
        assert_eq!(app.prompt(), "Player 1 stars");
        assert!(app.game().is_none());
    }

    #[test]
    fn manual_setup_starts_a_game() {
        let mut app = TuiApp::new();

        app.submit_line("ys bm");
        assert_eq!(app.prompt(), "Player 1 ship");
        app.submit_line("gs");
        assert_eq!(app.prompt(), "Player 2 stars");
        app.submit_line("bl rl");
        assert_eq!(app.prompt(), "Player 2 ship");
        app.submit_line("rm");

        assert_eq!(app.mode(), AppMode::Playing);
        assert_eq!(
            app.game().expect("game started").turn().current_player(),
            Player::One
        );
        assert!(
            app.messages()
                .iter()
                .any(|message| message == "Game started.")
        );
    }

    #[test]
    fn invalid_setup_logs_error_and_reprompts_player() {
        let mut app = TuiApp::new();

        app.submit_line("ys");
        app.submit_line("gs");

        assert_eq!(app.mode(), AppMode::Setup);
        assert_eq!(app.prompt(), "Player 1 stars");
        assert!(app.messages().iter().any(|message| {
            message.contains("Error:")
                && message.contains("homeworld setup needs exactly two stars")
        }));
    }

    #[test]
    fn setup_load_accepts_yaml_save() {
        let path = temp_path("setup_load_accepts_yaml_save.yaml");
        fs::write(
            &path,
            save::to_yaml(&Game::default(Player::Two)).expect("game serializes"),
        )
        .expect("save fixture writes");
        let mut app = TuiApp::new();

        app.submit_line(&format!("load {}", path.display()));

        assert_eq!(app.mode(), AppMode::Playing);
        assert_eq!(
            app.game().expect("game loaded").turn().current_player(),
            Player::Two
        );
        assert!(
            app.messages()
                .iter()
                .any(|message| message.starts_with("Loaded from "))
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn setup_load_accepts_history_setup_and_replays_remaining_commands() {
        let path = temp_path("setup_load_accepts_history_setup.hw");
        fs::write(
            &path,
            "ys bm
gs
bl rl
rm
b 0 gs
",
        )
        .expect("history fixture writes");
        let mut app = TuiApp::new();

        app.submit_line(&format!("load {}", path.display()));

        assert_eq!(app.mode(), AppMode::Playing);
        assert_eq!(
            app.game().expect("game loaded").turn().current_player(),
            Player::Two
        );
        assert!(
            app.messages()
                .iter()
                .any(|message| message == "Action applied.")
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_history_keeps_typed_lines_and_omits_replayed_commands() {
        let replay_path = temp_path("save_history_keeps_typed_lines_replay.hw");
        let save_path = temp_path("save_history_keeps_typed_lines.yaml");
        fs::write(&replay_path, "b 0 gs\n").expect("replay fixture writes");
        let mut app = started_app();

        app.submit_line(&format!("load {}", replay_path.display()));
        app.submit_line(&format!("save-history {}", save_path.display()));

        let saved = save::from_yaml_with_extras(
            &fs::read_to_string(&save_path).expect("history save reads"),
        )
        .expect("history save loads");
        assert_eq!(
            saved.history,
            vec![
                "ys bm".to_owned(),
                "gs".to_owned(),
                "bl rl".to_owned(),
                "rm".to_owned(),
                format!("load {}", replay_path.display()),
            ]
        );
        let _ = fs::remove_file(replay_path);
        let _ = fs::remove_file(save_path);
    }

    #[test]
    fn valid_action_updates_game_and_logs_feedback() {
        let mut app = started_app();

        app.submit_line("b 0 gs");

        assert_eq!(
            app.game().expect("game started").turn().current_player(),
            Player::Two
        );
        assert!(
            app.messages()
                .iter()
                .any(|message| message == "Action applied.")
        );
        assert!(
            app.messages()
                .iter()
                .any(|message| message == "Turn ended.")
        );
    }

    #[test]
    fn invalid_command_logs_error_without_advancing() {
        let mut app = started_app();
        let before = app.game().expect("game started").clone();

        app.submit_line("bogus");

        assert_eq!(app.game(), Some(&before));
        assert!(
            app.messages()
                .iter()
                .any(|message| message.contains("unknown command"))
        );
    }

    #[test]
    fn help_toggles_help_panel() {
        let mut app = started_app();

        app.submit_line("help");
        assert!(app.show_help());
        app.submit_line("help");
        assert!(!app.show_help());
    }

    #[test]
    fn quit_marks_app_as_quitting() {
        let mut app = started_app();

        app.submit_line("quit");

        assert!(app.should_quit());
    }

    #[test]
    fn ai_search_runs_after_human_turn_reaches_ai_player() {
        let mut app = started_app();

        app.submit_line("ai p2 search");
        app.submit_line("b 0 gs");

        assert!(
            app.messages()
                .iter()
                .any(|message| message == "AI Player 2 set to search.")
        );
        assert!(
            app.messages()
                .iter()
                .any(|message| message.starts_with("P2> "))
        );
    }

    fn started_app() -> TuiApp {
        let mut app = TuiApp::new();
        app.submit_line("ys bm");
        app.submit_line("gs");
        app.submit_line("bl rl");
        app.submit_line("rm");
        app
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("hw-tui-{}-{name}", std::process::id()))
    }
}
