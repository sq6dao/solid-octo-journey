use hw_core::{Color, Piece, Player, Size, SystemId};
use hw_engine::{Action, HomeworldSetup, TravelTarget};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParsedCommand {
    Help,
    Show,
    End,
    Quit,
    Action(Action),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedInput {
    pub command: ParsedCommand,
    pub show_after: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for ParseError {}

pub fn parse_input(line: &str, current_player: Player) -> Result<ParsedInput, ParseError> {
    let (command_line, show_after) = strip_show_suffix(line)?;
    Ok(ParsedInput {
        command: parse_command(command_line, current_player)?,
        show_after,
    })
}

pub fn parse_command(line: &str, current_player: Player) -> Result<ParsedCommand, ParseError> {
    let tokens = tokenize(line);
    if tokens.is_empty() {
        return Err(ParseError::new("expected a command"));
    }

    match tokens[0].as_str() {
        "help" | "h" => require_no_args(&tokens, ParsedCommand::Help),
        "show" => require_no_args(&tokens, ParsedCommand::Show),
        "s" if tokens.len() == 1 => Ok(ParsedCommand::Show),
        "end" | "e" => require_no_args(&tokens, ParsedCommand::End),
        "quit" | "q" => require_no_args(&tokens, ParsedCommand::Quit),
        "build" | "b" => parse_build(&tokens, current_player),
        "travel" | "t" => parse_travel(&tokens, current_player),
        "trade" | "tr" | "x" => parse_trade(&tokens, current_player),
        "sacrifice" | "sac" | "s" => parse_sacrifice(&tokens, current_player),
        "invade" | "i" => parse_invade(&tokens, current_player),
        "catastrophe" | "c" => parse_catastrophe(&tokens),
        other => Err(ParseError::new(format!("unknown command `{other}`"))),
    }
}

fn strip_show_suffix(line: &str) -> Result<(&str, bool), ParseError> {
    let trimmed = line.trim();
    match trimmed.find(';') {
        Some(index) if index == trimmed.len() - 1 => Ok((&trimmed[..index], true)),
        Some(_) => Err(ParseError::new(
            "semicolon is only allowed at the end of a command",
        )),
        None => Ok((trimmed, false)),
    }
}

pub fn parse_setup(
    stars_line: &str,
    ship_line: &str,
    player: Player,
) -> Result<HomeworldSetup, ParseError> {
    let stars = parse_piece_list(&tokenize(stars_line))?;
    if !(1..=2).contains(&stars.len()) {
        return Err(ParseError::new("homeworld setup needs one or two stars"));
    }

    let ships = parse_piece_list(&tokenize(ship_line))?;
    let [ship] = ships.as_slice() else {
        return Err(ParseError::new("homeworld setup needs exactly one ship"));
    };

    Ok(HomeworldSetup::new(
        stars
            .into_iter()
            .map(|(color, size)| Piece::new(color, size))
            .collect(),
        Piece::owned(ship.0, ship.1, player),
    ))
}

fn parse_build(tokens: &[String], player: Player) -> Result<ParsedCommand, ParseError> {
    let system = parse_system(tokens.get(1), "system")?;
    let (ship, next) = consume_owned_piece(tokens, 2, player)?;
    require_end(tokens, next)?;

    Ok(ParsedCommand::Action(Action::Build {
        player,
        system,
        ship,
    }))
}

fn parse_travel(tokens: &[String], player: Player) -> Result<ParsedCommand, ParseError> {
    let from = parse_system(tokens.get(1), "source system")?;
    let (ship, target_index) = consume_owned_piece(tokens, 2, player)?;
    let target_word = tokens
        .get(target_index)
        .ok_or_else(|| ParseError::new("expected travel target"))?;

    let target = match target_word.as_str() {
        "existing" | "x" => {
            let to = parse_system(tokens.get(target_index + 1), "target system")?;
            require_end(tokens, target_index + 2)?;
            TravelTarget::Existing(to)
        }
        "new" | "n" => {
            let stars = parse_piece_list(&tokens[target_index + 1..])?;
            if !(1..=2).contains(&stars.len()) {
                return Err(ParseError::new("new travel target needs one or two stars"));
            }
            TravelTarget::New {
                stars: stars
                    .into_iter()
                    .map(|(color, size)| Piece::new(color, size))
                    .collect(),
            }
        }
        _ => return Err(ParseError::new("expected `existing`, `x`, `new`, or `n`")),
    };

    Ok(ParsedCommand::Action(Action::Travel {
        player,
        from,
        ship,
        target,
    }))
}

fn parse_trade(tokens: &[String], player: Player) -> Result<ParsedCommand, ParseError> {
    let system = parse_system(tokens.get(1), "system")?;
    let (from, next) = consume_owned_piece(tokens, 2, player)?;
    let (to, next) = consume_owned_piece(tokens, next, player)?;
    require_end(tokens, next)?;

    Ok(ParsedCommand::Action(Action::Trade {
        player,
        system,
        from,
        to,
    }))
}

fn parse_sacrifice(tokens: &[String], player: Player) -> Result<ParsedCommand, ParseError> {
    let system = parse_system(tokens.get(1), "system")?;
    let (ship, next) = consume_owned_piece(tokens, 2, player)?;
    require_end(tokens, next)?;

    Ok(ParsedCommand::Action(Action::Sacrifice {
        player,
        system,
        ship,
    }))
}

fn parse_invade(tokens: &[String], player: Player) -> Result<ParsedCommand, ParseError> {
    let system = parse_system(tokens.get(1), "system")?;
    let (color, size, next) = consume_piece(tokens, 2)?;
    require_end(tokens, next)?;

    Ok(ParsedCommand::Action(Action::Invade {
        player,
        system,
        target: Piece::owned(color, size, other_player(player)),
    }))
}

fn parse_catastrophe(tokens: &[String]) -> Result<ParsedCommand, ParseError> {
    let system = parse_system(tokens.get(1), "system")?;
    let color = parse_color(
        tokens
            .get(2)
            .ok_or_else(|| ParseError::new("expected color"))?,
    )?;
    require_end(tokens, 3)?;

    Ok(ParsedCommand::Action(Action::Catastrophe { system, color }))
}

fn tokenize(line: &str) -> Vec<String> {
    line.split_whitespace()
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn require_no_args(tokens: &[String], command: ParsedCommand) -> Result<ParsedCommand, ParseError> {
    require_end(tokens, 1)?;
    Ok(command)
}

fn require_end(tokens: &[String], next: usize) -> Result<(), ParseError> {
    if next == tokens.len() {
        Ok(())
    } else {
        Err(ParseError::new("unexpected extra input"))
    }
}

fn parse_system(token: Option<&String>, label: &str) -> Result<SystemId, ParseError> {
    let token = token.ok_or_else(|| ParseError::new(format!("expected {label}")))?;
    token
        .parse::<usize>()
        .map(SystemId::new)
        .map_err(|_| ParseError::new(format!("invalid {label} `{token}`")))
}

fn consume_owned_piece(
    tokens: &[String],
    index: usize,
    player: Player,
) -> Result<(Piece, usize), ParseError> {
    let (color, size, next) = consume_piece(tokens, index)?;
    Ok((Piece::owned(color, size, player), next))
}

fn consume_piece(tokens: &[String], index: usize) -> Result<(Color, Size, usize), ParseError> {
    let first = tokens
        .get(index)
        .ok_or_else(|| ParseError::new("expected piece"))?;

    if let Some((color, size)) = parse_compact_piece(first) {
        return Ok((color, size, index + 1));
    }

    let color = parse_color(first)?;
    let size = parse_size(
        tokens
            .get(index + 1)
            .ok_or_else(|| ParseError::new("expected piece size"))?,
    )?;
    Ok((color, size, index + 2))
}

fn parse_piece_list(tokens: &[String]) -> Result<Vec<(Color, Size)>, ParseError> {
    let mut pieces = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let (color, size, next) = consume_piece(tokens, index)?;
        pieces.push((color, size));
        index = next;
    }
    Ok(pieces)
}

fn parse_compact_piece(token: &str) -> Option<(Color, Size)> {
    let mut chars = token.chars();
    let color = parse_short_color(chars.next()?)?;
    let size = parse_short_size(chars.next()?)?;
    if chars.next().is_some() {
        return None;
    }
    Some((color, size))
}

fn parse_color(token: &str) -> Result<Color, ParseError> {
    match token {
        "red" | "r" => Ok(Color::Red),
        "yellow" | "y" => Ok(Color::Yellow),
        "green" | "g" => Ok(Color::Green),
        "blue" | "b" => Ok(Color::Blue),
        _ => Err(ParseError::new(format!("invalid color `{token}`"))),
    }
}

fn parse_short_color(token: char) -> Option<Color> {
    match token {
        'r' => Some(Color::Red),
        'y' => Some(Color::Yellow),
        'g' => Some(Color::Green),
        'b' => Some(Color::Blue),
        _ => None,
    }
}

fn parse_size(token: &str) -> Result<Size, ParseError> {
    match token {
        "small" | "s" => Ok(Size::Small),
        "medium" | "m" => Ok(Size::Medium),
        "large" | "l" => Ok(Size::Large),
        _ => Err(ParseError::new(format!("invalid size `{token}`"))),
    }
}

fn parse_short_size(token: char) -> Option<Size> {
    match token {
        's' => Some(Size::Small),
        'm' => Some(Size::Medium),
        'l' => Some(Size::Large),
        _ => None,
    }
}

fn other_player(player: Player) -> Player {
    match player {
        Player::One => Player::Two,
        Player::Two => Player::One,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_general_commands_and_short_aliases() {
        assert_eq!(parse_command("help", Player::One), Ok(ParsedCommand::Help));
        assert_eq!(parse_command("h", Player::One), Ok(ParsedCommand::Help));
        assert_eq!(parse_command("show", Player::One), Ok(ParsedCommand::Show));
        assert_eq!(parse_command("s", Player::One), Ok(ParsedCommand::Show));
        assert_eq!(parse_command("end", Player::One), Ok(ParsedCommand::End));
        assert_eq!(parse_command("e", Player::One), Ok(ParsedCommand::End));
        assert_eq!(parse_command("quit", Player::One), Ok(ParsedCommand::Quit));
        assert_eq!(parse_command("q", Player::One), Ok(ParsedCommand::Quit));
    }

    #[test]
    fn parses_trailing_semicolon_as_show_shortcut() {
        assert_eq!(
            parse_input("b 0 gs;", Player::One),
            Ok(ParsedInput {
                command: ParsedCommand::Action(Action::Build {
                    player: Player::One,
                    system: SystemId::new(0),
                    ship: Piece::owned(Color::Green, Size::Small, Player::One),
                }),
                show_after: true,
            })
        );
        assert_eq!(
            parse_input("show;", Player::One),
            Ok(ParsedInput {
                command: ParsedCommand::Show,
                show_after: true,
            })
        );
        assert_eq!(
            parse_input("show", Player::One),
            Ok(ParsedInput {
                command: ParsedCommand::Show,
                show_after: false,
            })
        );
        assert_eq!(
            parse_input("t 0 ys x 1;", Player::One),
            Ok(ParsedInput {
                command: ParsedCommand::Action(Action::Travel {
                    player: Player::One,
                    from: SystemId::new(0),
                    ship: Piece::owned(Color::Yellow, Size::Small, Player::One),
                    target: TravelTarget::Existing(SystemId::new(1)),
                }),
                show_after: true,
            })
        );
    }

    #[test]
    fn rejects_semicolons_that_do_not_finish_the_command() {
        assert!(parse_input("show; show", Player::One).is_err());
        assert!(parse_input("show;;", Player::One).is_err());
    }

    #[test]
    fn parses_build_with_verbose_and_compact_pieces() {
        assert_eq!(
            parse_command("build 0 green small", Player::One),
            Ok(ParsedCommand::Action(Action::Build {
                player: Player::One,
                system: SystemId::new(0),
                ship: Piece::owned(Color::Green, Size::Small, Player::One),
            }))
        );
        assert_eq!(
            parse_command("b 0 gs", Player::Two),
            Ok(ParsedCommand::Action(Action::Build {
                player: Player::Two,
                system: SystemId::new(0),
                ship: Piece::owned(Color::Green, Size::Small, Player::Two),
            }))
        );
    }

    #[test]
    fn parses_travel_with_existing_and_new_targets() {
        assert_eq!(
            parse_command("travel 0 yellow small existing 1", Player::One),
            Ok(ParsedCommand::Action(Action::Travel {
                player: Player::One,
                from: SystemId::new(0),
                ship: Piece::owned(Color::Yellow, Size::Small, Player::One),
                target: TravelTarget::Existing(SystemId::new(1)),
            }))
        );
        assert_eq!(
            parse_command("t 0 ys x 1", Player::One),
            Ok(ParsedCommand::Action(Action::Travel {
                player: Player::One,
                from: SystemId::new(0),
                ship: Piece::owned(Color::Yellow, Size::Small, Player::One),
                target: TravelTarget::Existing(SystemId::new(1)),
            }))
        );
        assert_eq!(
            parse_command("t 0 ys n rm bl", Player::Two),
            Ok(ParsedCommand::Action(Action::Travel {
                player: Player::Two,
                from: SystemId::new(0),
                ship: Piece::owned(Color::Yellow, Size::Small, Player::Two),
                target: TravelTarget::New {
                    stars: vec![
                        Piece::new(Color::Red, Size::Medium),
                        Piece::new(Color::Blue, Size::Large),
                    ],
                },
            }))
        );
    }

    #[test]
    fn parses_trade_with_exchange_alias() {
        assert_eq!(
            parse_command("x 0 bs rs", Player::One),
            Ok(ParsedCommand::Action(Action::Trade {
                player: Player::One,
                system: SystemId::new(0),
                from: Piece::owned(Color::Blue, Size::Small, Player::One),
                to: Piece::owned(Color::Red, Size::Small, Player::One),
            }))
        );
    }

    #[test]
    fn parses_s_as_show_or_sacrifice_by_context() {
        assert_eq!(parse_command("s", Player::One), Ok(ParsedCommand::Show));
        assert_eq!(
            parse_command("s 0 gm", Player::One),
            Ok(ParsedCommand::Action(Action::Sacrifice {
                player: Player::One,
                system: SystemId::new(0),
                ship: Piece::owned(Color::Green, Size::Medium, Player::One),
            }))
        );
    }

    #[test]
    fn parses_invade_with_opponent_owned_target() {
        assert_eq!(
            parse_command("i 1 gs", Player::One),
            Ok(ParsedCommand::Action(Action::Invade {
                player: Player::One,
                system: SystemId::new(1),
                target: Piece::owned(Color::Green, Size::Small, Player::Two),
            }))
        );
    }

    #[test]
    fn parses_catastrophe_with_short_color() {
        assert_eq!(
            parse_command("c 1 r", Player::One),
            Ok(ParsedCommand::Action(Action::Catastrophe {
                system: SystemId::new(1),
                color: Color::Red,
            }))
        );
    }

    #[test]
    fn parses_setup_lines() {
        assert_eq!(
            parse_setup("ys bl", "gm", Player::One),
            Ok(HomeworldSetup::new(
                vec![
                    Piece::new(Color::Yellow, Size::Small),
                    Piece::new(Color::Blue, Size::Large),
                ],
                Piece::owned(Color::Green, Size::Medium, Player::One),
            ))
        );
    }

    #[test]
    fn rejects_invalid_commands() {
        assert!(parse_command("", Player::One).is_err());
        assert!(parse_command("b 0", Player::One).is_err());
        assert!(parse_command("t 0 ys maybe 1", Player::One).is_err());
        assert!(parse_command("catastrophe 0 purple", Player::One).is_err());
        assert!(parse_setup("ys bl rm", "gm", Player::One).is_err());
    }
}
