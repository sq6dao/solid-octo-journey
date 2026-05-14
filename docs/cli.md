# Text CLI

`hw-cli` provides a simple two-player hot-seat interface.

## Start

Run from the repository root:

```sh
cargo run -p hw-cli
```

The game reads commands from standard input and writes prompts, turn
summaries, errors, and state renders to standard output.

## Homeworld Setup

At startup, each player enters one or two stars and one starting ship.
Player 1's homeworld is system `0`; Player 2's homeworld is system `1`.

```text
Player 1 stars> ys
Player 1 ship> gs
Player 2 stars> bl
Player 2 ship> rm
```

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

## Semicolon Shortcut

Append `;` as the final character of a command to print the game state
after the command succeeds:

```text
b 0 gs;
e;
```

If the command fails, the CLI prints the error and does not print state.
Semicolons are only accepted as the last character:

```text
show;    # accepted
show; q  # rejected
```

`show;` still prints the state only once.

## Sample Session

```text
cargo run -p hw-cli
Player 1 stars> ys
Player 1 ship> gs
Player 2 stars> bl
Player 2 ship> rm
P1> show
P1> b 0 gs;
P1> e
P2> q
```

Invalid commands and illegal actions are reported as errors and leave the
game at the same turn.
