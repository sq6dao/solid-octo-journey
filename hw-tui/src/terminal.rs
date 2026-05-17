use std::{
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use hw_cli::render::render_help;
use hw_core::{Color as GameColor, Piece, Player, Size, SystemId};
use hw_engine::{Game, GameOutcome, GameStatus};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{AppMode, TuiApp};

type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub fn run_stdio() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_app(&mut terminal, TuiApp::new());
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_app(terminal: &mut TuiTerminal, mut app: TuiApp) -> io::Result<()> {
    while !app.should_quit() {
        terminal.draw(|frame| draw(frame, &app))?;
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key)
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && matches!(key.code, KeyCode::Char('c')) =>
                {
                    break;
                }
                Event::Key(key) => match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => app.submit_input(),
                    KeyCode::Backspace => app.backspace(),
                    KeyCode::Char(ch) => app.push_char(ch),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &TuiApp) {
    let area = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(8),
            Constraint::Length(3),
        ])
        .split(area);

    frame.render_widget(status_panel(app), vertical[0]);
    draw_main(frame, app, vertical[1]);
    frame.render_widget(messages_panel(app), vertical[2]);
    frame.render_widget(input_panel(app), vertical[3]);

    if app.show_help() {
        let popup = centered_rect(72, 70, area);
        frame.render_widget(Clear, popup);
        frame.render_widget(help_panel(), popup);
    }
}

fn draw_main(frame: &mut ratatui::Frame<'_>, app: &TuiApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(area);
    frame.render_widget(board_panel(app), chunks[0]);
    frame.render_widget(bank_panel(app), chunks[1]);
}

fn status_panel(app: &TuiApp) -> Paragraph<'static> {
    let title = match app.mode() {
        AppMode::Setup => "Homeworlds TUI - setup",
        AppMode::Playing => "Homeworlds TUI - game",
    };
    let status = app
        .game()
        .map(render_status)
        .unwrap_or_else(|| format!("Setup prompt: {}", app.prompt()));
    Paragraph::new(status)
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
}

fn board_panel(app: &TuiApp) -> Paragraph<'static> {
    let lines = app
        .game()
        .map(board_lines)
        .unwrap_or_else(|| vec![Line::from("Enter setup lines or load <path>.")]);
    Paragraph::new(lines)
        .block(Block::default().title("Board").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}

fn bank_panel(app: &TuiApp) -> Paragraph<'static> {
    let lines = app
        .game()
        .map(bank_lines)
        .unwrap_or_else(|| vec![Line::from("No game loaded.")]);
    Paragraph::new(lines)
        .block(Block::default().title("Bank").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}

fn messages_panel(app: &TuiApp) -> Paragraph<'_> {
    let start = app.messages().len().saturating_sub(6);
    let lines = app.messages()[start..]
        .iter()
        .map(|message| Line::from(message.as_str()))
        .collect::<Vec<_>>();
    Paragraph::new(lines)
        .block(Block::default().title("Messages").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}

fn input_panel(app: &TuiApp) -> Paragraph<'_> {
    let prompt = app.prompt_text();
    let line = Line::from(vec![
        Span::styled(prompt, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(app.input()),
    ]);
    Paragraph::new(line).block(Block::default().title("Command").borders(Borders::ALL))
}

fn help_panel() -> Paragraph<'static> {
    Paragraph::new(render_help())
        .block(Block::default().title("Help").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}

fn render_status(game: &Game) -> String {
    match game.status() {
        GameStatus::InProgress => format!(
            "Current player: {:?} | actions: {}",
            game.turn().current_player(),
            game.turn().remaining_actions()
        ),
        GameStatus::Finished(outcome) => format!("Finished: {outcome:?}"),
    }
}

fn board_lines(game: &Game) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("Status: {}", status_label(game.status()))),
        Line::from(format!(
            "Current player: {}",
            player_label(game.turn().current_player())
        )),
        Line::from(format!(
            "Remaining actions: {}",
            game.turn().remaining_actions()
        )),
        Line::from("Systems:"),
    ];

    let state = game.turn().state();
    for (index, system) in state.systems().iter().enumerate() {
        let system_id = SystemId::new(index);
        lines.push(Line::from(format!(
            "[{index}] {}",
            system_label(game, system_id)
        )));
        lines.push(piece_list_line("  Stars: ", system.stars(), false));
        lines.push(piece_list_line("  Ships: ", system.ships(), true));
    }

    lines
}

fn bank_lines(game: &Game) -> Vec<Line<'static>> {
    let state = game.turn().state();
    GameColor::ALL
        .into_iter()
        .map(|color| {
            let counts = Size::ALL
                .into_iter()
                .map(|size| format!("{}={}", size_label(size), state.bank().count(color, size)))
                .collect::<Vec<_>>()
                .join(" ");
            let style = Style::default().fg(tui_piece_color(color));
            Line::from(vec![
                Span::styled(format!("{}:", color_label(color)), style),
                Span::styled(format!(" {counts}"), style),
            ])
        })
        .collect::<Vec<_>>()
}

fn piece_list_line(prefix: &'static str, pieces: &[Piece], show_owner: bool) -> Line<'static> {
    let mut spans = vec![Span::raw(prefix)];
    if pieces.is_empty() {
        spans.push(Span::raw("none"));
        return Line::from(spans);
    }

    for (index, piece) in pieces.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(", "));
        }
        spans.extend(piece_spans(*piece, show_owner));
    }

    Line::from(spans)
}

fn piece_spans(piece: Piece, show_owner: bool) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    if show_owner {
        if let Some(player) = piece.owner() {
            spans.push(Span::raw(player_short(player)));
            spans.push(Span::raw(" "));
        }
    }
    spans.push(Span::styled(
        piece_identity(piece),
        Style::default().fg(tui_piece_color(piece.color())),
    ));
    spans
}

fn piece_identity(piece: Piece) -> String {
    format!("{}{}", color_short(piece.color()), size_short(piece.size()))
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

fn system_label(game: &Game, system: SystemId) -> String {
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

fn color_label(color: GameColor) -> &'static str {
    match color {
        GameColor::Red => "red",
        GameColor::Yellow => "yellow",
        GameColor::Green => "green",
        GameColor::Blue => "blue",
    }
}

fn color_short(color: GameColor) -> &'static str {
    match color {
        GameColor::Red => "r",
        GameColor::Yellow => "y",
        GameColor::Green => "g",
        GameColor::Blue => "b",
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

fn tui_piece_color(color: GameColor) -> Color {
    match color {
        GameColor::Red => Color::Red,
        GameColor::Yellow => Color::Yellow,
        GameColor::Green => Color::Green,
        GameColor::Blue => Color::Blue,
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

#[cfg(test)]
mod tests {
    use hw_core::{Color as GameColor, Piece, Player, Size};
    use hw_engine::{Game, HomeworldSetup};

    use super::*;

    #[test]
    fn board_lines_render_systems_without_bank_section() {
        let text = lines_text(&board_lines(&sample_game()));

        assert!(text.contains("Status: in progress"));
        assert!(text.contains("Current player: Player 1"));
        assert!(text.contains("Remaining actions: 1"));
        assert!(text.contains("[0] homeworld Player 1"));
        assert!(text.contains("Stars: ys, bm"));
        assert!(text.contains("Ships: P1 gs"));
        assert!(!text.contains("Bank:"));
        assert!(!text.contains("red: small="));
    }

    #[test]
    fn board_lines_color_piece_tokens_by_game_color() {
        let lines = board_lines(&sample_game());

        assert_eq!(span_color(&lines, "ys"), Some(Color::Yellow));
        assert_eq!(span_color(&lines, "bm"), Some(Color::Blue));
        assert_eq!(span_color(&lines, "gs"), Some(Color::Green));
        assert_eq!(span_color(&lines, "rm"), Some(Color::Red));
    }

    #[test]
    fn bank_lines_color_each_bank_row_by_game_color() {
        let lines = bank_lines(&sample_game());

        assert_eq!(line_color(&lines, "red:"), Some(Color::Red));
        assert!(line_spans_have_color(&lines, "red:", Color::Red));
        assert_eq!(line_color(&lines, "yellow:"), Some(Color::Yellow));
        assert!(line_spans_have_color(&lines, "yellow:", Color::Yellow));
        assert_eq!(line_color(&lines, "green:"), Some(Color::Green));
        assert!(line_spans_have_color(&lines, "green:", Color::Green));
        assert_eq!(line_color(&lines, "blue:"), Some(Color::Blue));
        assert!(line_spans_have_color(&lines, "blue:", Color::Blue));
    }

    fn sample_game() -> Game {
        Game::new(
            [
                HomeworldSetup::new(
                    vec![
                        Piece::new(GameColor::Yellow, Size::Small),
                        Piece::new(GameColor::Blue, Size::Medium),
                    ],
                    Piece::owned(GameColor::Green, Size::Small, Player::One),
                ),
                HomeworldSetup::new(
                    vec![
                        Piece::new(GameColor::Blue, Size::Large),
                        Piece::new(GameColor::Red, Size::Large),
                    ],
                    Piece::owned(GameColor::Red, Size::Medium, Player::Two),
                ),
            ],
            Player::One,
        )
        .expect("game starts")
    }

    fn lines_text(lines: &[Line<'_>]) -> String {
        lines.iter().map(line_text).collect::<Vec<_>>().join("\n")
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("")
    }

    fn span_color(lines: &[Line<'_>], content: &str) -> Option<Color> {
        lines.iter().find_map(|line| {
            line.spans
                .iter()
                .find(|span| span.content == content)
                .and_then(|span| span.style.fg)
        })
    }

    fn line_color(lines: &[Line<'_>], prefix: &str) -> Option<Color> {
        lines
            .iter()
            .find(|line| line_text(line).starts_with(prefix))
            .and_then(|line| line.spans.first())
            .and_then(|span| span.style.fg)
    }

    fn line_spans_have_color(lines: &[Line<'_>], prefix: &str, color: Color) -> bool {
        lines
            .iter()
            .find(|line| line_text(line).starts_with(prefix))
            .is_some_and(|line| {
                !line.spans.is_empty() && line.spans.iter().all(|span| span.style.fg == Some(color))
            })
    }
}
