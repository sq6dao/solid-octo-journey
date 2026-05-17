# TUI

`hw-tui` provides a Ratatui-based playable terminal interface.

Run it from the repository root:

```sh
cargo run -p hw-tui
```

## Setup

The TUI starts with the same setup sequence as the CLI:

```text
Player 1 stars
Player 1 ship
Player 2 stars
Player 2 ship
```

Enter pieces in compact or long form, for example `ys bm`, `green
small`, or `red medium`.

During setup, `load <path>` or `l <path>` can load either a YAML save or
a command history file. For command history files, the first four
non-empty, non-comment lines are used as setup input and any remaining
lines are replayed as game commands.

## Play

After setup, type the same commands supported by `hw-cli`, including
actions, `end`, `show`, `help`, `save`, `load`, and AI control commands
such as `ai p2 search`.

The TUI shows the board, bank, status, command input, and recent
messages. Press `Esc` or `Ctrl-C` to leave the TUI.
