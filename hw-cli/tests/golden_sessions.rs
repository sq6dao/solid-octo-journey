use std::io::Cursor;

struct GoldenCase {
    name: &'static str,
    history: &'static str,
    expect: &'static [&'static str],
    reject: &'static [&'static str],
}

const GOLDEN_CASES: &[GoldenCase] = &[
    GoldenCase {
        name: "partial opening",
        history: include_str!("../../tests/golden/partial_opening.hw"),
        expect: &[
            "Game started.",
            "Status: in progress",
            "Current player: Player 1",
            "[0] homeworld Player 1",
            "Ships: P1 gs",
        ],
        reject: &["Error:"],
    },
    GoldenCase {
        name: "short terminal game",
        history: include_str!("../../tests/golden/short_terminal_game.hw"),
        expect: &[
            "Game started.",
            "Action applied.",
            "Turn ended.",
            "Status: finished, winner Player 2",
        ],
        reject: &["Error:"],
    },
    GoldenCase {
        name: "comments and blanks",
        history: include_str!("../../tests/golden/comments_and_blanks.hw"),
        expect: &[
            "Game started.",
            "Action applied.",
            "Turn ended.",
            "Current player: Player 2",
            "Ships: P1 gs, P1 gs",
        ],
        reject: &["Error:", "Opening comment"],
    },
];

#[test]
fn golden_sessions_match_expectations() {
    for case in GOLDEN_CASES {
        assert_no_reserved_markers(case.name, case.history);
        let output = run_history(case.history);

        for expected in case.expect {
            assert!(
                output.contains(expected),
                "{} should contain {expected:?}\n\n{output}",
                case.name
            );
        }

        for rejected in case.reject {
            assert!(
                !output.contains(rejected),
                "{} should not contain {rejected:?}\n\n{output}",
                case.name
            );
        }
    }
}

fn run_history(history: &str) -> String {
    let input = command_stream(history);
    let mut output = Vec::new();
    hw_cli::session::run(Cursor::new(input), &mut output).expect("golden session runs");
    String::from_utf8(output).expect("golden output is utf8")
}

fn command_stream(history: &str) -> String {
    history
        .lines()
        .filter_map(history_command)
        .fold(String::new(), |mut commands, command| {
            commands.push_str(command);
            commands.push('\n');
            commands
        })
}

fn history_command(line: &str) -> Option<&str> {
    let command = line
        .split_once('#')
        .map_or(line, |(command, _)| command)
        .trim();
    (!command.is_empty()).then_some(command)
}

fn assert_no_reserved_markers(name: &str, history: &str) {
    for (index, line) in history.lines().enumerate() {
        let directive = line.trim_start().strip_prefix('#').unwrap_or(line).trim();
        assert!(
            !matches!(directive, marker if marker.starts_with("EXPECT:") || marker.starts_with("REJECT:")),
            "{name}: reserved marker directive on line {}",
            index + 1
        );
    }
}

#[test]
fn command_stream_ignores_blank_lines_and_comments() {
    let history = "
ys bm

# setup ship
gs # inline comment

show
";

    assert_eq!(command_stream(history), "ys bm\ngs\nshow\n");
}

#[test]
fn marker_directives_are_reserved_for_rust_expectations() {
    assert_no_reserved_markers("plain", "# Opening\nshow\n");

    let result = std::panic::catch_unwind(|| assert_no_reserved_markers("bad", "# EXPECT: show"));

    assert!(result.is_err());
}
