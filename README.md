# Homeworlds Rust

A modular Rust engine for a two-player Homeworlds-style game.

## Crates

- `hw-core`: core domain types and invariants.
- `hw-engine`: actions, validation, transitions, turns, and game flow.
- `hw-cli`: simple text hot-seat interface.

## Current Features

- Strongly typed core model for pieces, banks, systems, and players.
- Deterministic engine game loop with turns, sacrifice budgets, automatic
  turn end when safe, and terminal game status.
- Hand-editable YAML save/load support.
- Text hot-seat CLI with short commands, command replay files, session
  history saves, and interactive arrow-key editing.

## Run Tests

```sh
cargo test
```

## Run The CLI

From the repository root:

```sh
cargo run -p hw-cli
```

In an interactive terminal, use the arrow keys to browse current-session
history and edit the current line before pressing Enter.

Press Tab at the end of a short command or unique partial command to
expand it to the full command word, such as `b` to `build`, `x` to
`trade`, `bu` to `build`, or `save-h` to `save-history`. Ambiguous
partials such as `s` or `sa`, and lines containing `;`, are left
unchanged.

Interactive history is session-only. It includes setup lines and typed
commands, but not commands replayed from files and not `save-history` /
`sh` itself.

The CLI prompts for Player 1 and Player 2 homeworld setup. Enter exactly
two stars, then one starting ship:

```text
Player 1 stars> ys bm
Player 1 ship> gs
Player 2 stars> bl rl
Player 2 ship> rm
```

Append `;` to any setup line to print the full game state once after
setup completes.

You can also enter `load <path>` or `l <path>` at the first setup prompt.
For command history files loaded during setup, the first four non-empty
lines are Player 1 stars, Player 1 ship, Player 2 stars, and Player 2
ship; remaining lines replay as normal commands.

After setup, use commands such as:

```text
show
b 0 gs
v game.yaml
sh game-with-history.yaml
l game.yaml
l scenario.txt
end
s
x 0 bs rs
c 1 r
q
```

When a paid action spends the last action and no catastrophe is possible,
the CLI ends the turn automatically.

Append `;` to a command to print the game state afterward, including
after errors:

```text
b 0 gs;
```

Save files use hand-editable YAML. `save-history` or `sh` writes typed
session history into the YAML metadata, while normal `save` stays compact.
The same `load` command can also replay a plain text command history file
with one CLI command per line. History files ignore empty lines and `#`
comments. See [docs/cli.md](docs/cli.md) for the full command reference
and [docs/save-format.md](docs/save-format.md) for the save format.
