# Text CLI

`hw-cli` provides a simple two-player hot-seat interface.

## Start

Run from the repository root:

```sh
cargo run -p hw-cli
```

The game reads commands from standard input and writes prompts, turn
summaries, errors, and state renders to standard output.

When standard input is an interactive terminal, the CLI supports in-line
editing and arrow-key history. Up/down browse setup lines and commands
typed in the current run. Replayed commands are not added to this
history, and history does not persist after the CLI exits.

Press Tab at the end of a short command or unique partial command to
expand it to the full command word:

```text
b   -> build
bu  -> build
t   -> travel
tr  -> trade
x   -> trade
sh  -> save-history
save-h -> save-history
sac -> sacrifice
```

Expansion applies only to the first command word in an interactive
terminal. It preserves any arguments already typed after that word.
Ambiguous partials such as `s` or `sa`, and lines containing `;`, are
left unchanged.

## Homeworld Setup

At startup, each player enters exactly two stars and one starting ship.
Player 1's homeworld is system `0`; Player 2's homeworld is system `1`.

```text
Player 1 stars> ys bm
Player 1 ship> gs
Player 2 stars> bl rl
Player 2 ship> rm
```

Setup lines may also end with `;`. If any setup line uses it, the CLI
prints the full game state once after setup completes.

You can also load a save or command history from the setup prompt:

```text
Player 1 stars> load scenario.hw
```

When a command history is loaded before setup is complete, the first four
non-empty lines are read as setup input in this order:

```text
Player 1 stars
Player 1 ship
Player 2 stars
Player 2 ship
```

Remaining lines are replayed as normal CLI commands after the game
starts.

Pieces can be written in compact or long form:

- Compact: `gs`, `rm`, `bl`
- Long: `green small`, `red medium`, `blue large`

Colors are `red`, `yellow`, `green`, `blue`, or `r`, `y`, `g`, `b`.
Sizes are `small`, `medium`, `large`, or `s`, `m`, `l`.

## State And System IDs

Use `show` or `s` to print the current game state. The render includes
system IDs, homeworld ownership, stars, ships, remaining actions, and
bank counts.

```text
show
s
```

System IDs are the bracketed numbers in the rendered state:

```text
[0] homeworld Player 1
[1] homeworld Player 2
```

Use those IDs in action commands.

## Commands

| Command | Short | Form |
| --- | --- | --- |
| Help | `h` | `help` |
| Show state | `s` | `show` |
| End turn | `e` | `end` |
| Quit | `q` | `quit` |
| Save | `v` | `save <path>` |
| Save with history | `sh` | `save-history <path>` |
| Load | `l` | `load <path>` |
| Build | `b` | `build <system> <piece>` |
| Travel existing | `t` | `travel <from> <piece> existing <to>` |
| Travel existing | `t` | `travel <from> <piece> x <to>` |
| Travel new | `t` | `travel <from> <piece> new <star> [<star>]` |
| Travel new | `t` | `travel <from> <piece> n <star> [<star>]` |
| Trade | `tr`, `x` | `trade <system> <from-piece> <to-piece>` |
| Sacrifice | `sac`, `s` | `sacrifice <system> <piece>` |
| Invade | `i` | `invade <system> <target-piece>` |
| Catastrophe | `c` | `catastrophe <system> <color>` |

Short commands use the same argument order:

```text
b 0 gs
t 0 ys x 1
t 0 ys n rm bl
x 0 bs rs
v game.yaml
sh game-with-history.yaml
l game.yaml
s 0 gm
i 1 gs
c 1 r
e
q
```

`s` by itself means `show`. `s <system> <piece>` means `sacrifice`.

`x` as the first word means trade or exchange:

```text
x 0 bs rs
```

Inside a travel command, `x` means an existing target system:

```text
t 0 ys x 1
```

When a paid action spends the last action and no catastrophe is possible,
the CLI ends the turn automatically. If a catastrophe is possible, the
turn stays with the current player at 0 actions so they can either run a
`catastrophe` command or explicitly `end`.

## Semicolon Shortcut

Append `;` as the final character of a command to print the game state
after the command runs. During setup, append `;` to any setup line to
print the initial game state once after setup completes:

```text
b 0 gs;
e;
```

If the command fails, the CLI prints the error and then prints the
current unchanged state.
Semicolons are only accepted as the last character:

```text
show;    # accepted
show; q  # rejected
```

`show;` still prints the state only once.

## Save And Load

Use `save <path>` or `v <path>` to write a compact YAML save file. Use
`save-history <path>` or `sh <path>` to write the same save with typed
session history metadata. Use `load <path>` or `l <path>` to replace the
current game with a saved game or replay a command history file.

```text
v game.yaml
sh game-with-history.yaml
l game.yaml
l game.yaml;
```

Save files store the board, bank, turn state, and game status. The loader
validates the file before replacing the current game; failed loads leave
the current game unchanged. Paths are single command tokens, so spaces in
paths are not supported by the v1 text parser.

`save-history` records typed setup and command lines from the current CLI
session, excluding the `save-history` or `sh` command itself. Commands
replayed from files are not recorded. See [save-format.md](save-format.md)
for the YAML v1 schema.

## Command History Files

`load <path>` also accepts plain text command history files. After setup,
each non-empty line is parsed as the same CLI command you would type at
the prompt. Empty lines are ignored. `#` starts a comment that runs until
the end of the line, so `#`-only lines are also ignored:

```text
# Opening turn
b 0 gs
show
```

The commands run against the current game state. Errors are printed just
like typed commands and later lines continue to run. Each replayed
command is printed with the current prompt before it runs. A `quit` or
`q` command in the history exits the session. Semicolon state printing
works inside history files and after the outer load command:

```text
b 0 gs;
```

If a YAML save contains `commands`, they are replayed after the saved game
state loads. YAML `history` is archival and is not replayed. If a file
parses as a YAML save, it is loaded as a save. If it looks like a v1 YAML
save but is invalid, the CLI reports the save error instead of replaying
it as history.

## Sample Session

```text
cargo run -p hw-cli
Player 1 stars> ys bm
Player 1 ship> gs
Player 2 stars> bl rl
Player 2 ship> rm
P1> show
P1> b 0 gs;
P2> v game.yaml
P2> q
```

Invalid commands and illegal actions are reported as errors and leave the
game at the same turn.
