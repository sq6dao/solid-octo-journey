# YAML Save Format

Save files use YAML v1. They are intended to be readable and editable by
hand, then validated on load.

## Example

```yaml
version: 1
players: [p1, p2]

turn:
  current_player: p1
  remaining_actions: 1
  required_action: null

status: in_progress

homeworlds:
  p1: 0
  p2: 1

bank:
  red: { small: 2, medium: 2, large: 2 }
  yellow: { small: 1, medium: 3, large: 2 }
  green: { small: 2, medium: 1, large: 3 }
  blue: { small: 2, medium: 2, large: 2 }

systems:
  - stars: [ys, bm]
    ships: ["p1:gs"]
  - stars: [rs, bl]
    ships: ["p2:rm"]
  - stars: [gm]
    ships: ["p1:ys", "p2:bs"]
  - stars: [rl]
    ships: ["p1:gm", "p2:yl"]

history:
  - "ys bm"
  - "gs"
  - "bl rl"
  - "rm"

commands:
  - "show"
```

## Fields

- `version`: must be `1`.
- `players`: v1 supports `p1` and `p2`.
- `turn.current_player`: player whose turn it is.
- `turn.remaining_actions`: current action budget, from `0` to `3`.
- `turn.required_action`: `null`, `build`, `travel`, `trade`, or
  `invade`.
- `status`: `in_progress`, `draw`, or `{ winner: p1 }`.
- `homeworlds`: maps player IDs to system IDs.
- `bank`: remaining pieces by color and size.
- `systems`: ordered list of systems; the list index is the system ID.
- `history`: optional archival CLI input history. The CLI does not replay
  this field.
- `commands`: optional CLI commands to replay after loading the saved
  game state.

Pieces use compact color/size notation such as `ys`, `bm`, or `rl`.
Ships prefix a piece with the owner, such as `"p1:gs"`.

Normal `save` output omits `history` and `commands`. `save-history` or
`sh` writes typed session input to `history` and leaves `commands`
omitted. Hand-authored saves can use `commands` for replay steps.

## Validation

Loading rejects unsupported versions, unknown players, malformed pieces,
invalid systems, bad homeworld IDs, impossible turn state, and bank counts
that disagree with the pieces on the board.
