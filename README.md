# Homeworlds Rust

A modular Rust engine for a two-player Homeworlds-style game.

## Crates

- `hw-core`: core domain types and invariants.
- `hw-engine`: actions, validation, transitions, turns, and game flow.
- `hw-cli`: simple text hot-seat interface.

## Run Tests

```sh
cargo test
```

## Run The CLI

From the repository root:

```sh
cargo run -p hw-cli
```

The CLI prompts for Player 1 and Player 2 homeworld setup. Enter one or
two stars, then one starting ship:

```text
Player 1 stars> ys
Player 1 ship> gs
Player 2 stars> bl
Player 2 ship> rm
```

After setup, use commands such as:

```text
show
b 0 gs
end
s
x 0 bs rs
c 1 r
q
```

Append `;` to a successful command to print the game state afterward:

```text
b 0 gs;
```

See [docs/cli.md](docs/cli.md) for the full command reference.
