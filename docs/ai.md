# AI

`hw-ai` contains deterministic, reusable AI support built on `hw-core`
and `hw-engine`.

The first supported API is legal decision generation:

```rust
use hw_ai::{AiDecision, legal_decisions};
use hw_engine::Game;

let game = Game::default(hw_core::Player::One);
let decisions: Vec<AiDecision> = legal_decisions(&game);
```

`AiDecision` currently represents either an engine `Action` or an
explicit turn end. `legal_decisions` returns an empty list for terminal
games and includes `EndTurn` only when the engine accepts ending the
current turn.

Build actions are generated for bank-available current-player ships.
Travel actions are generated for owned ships to existing systems and
one-star new discoveries. Trade actions are generated from owned ships to
same-size bank-available ships of other colors. Sacrifice actions are
generated for owned ships. Invade actions are generated for opponent
ships. Catastrophe actions are generated for overpopulated system/color
pairs even when no paid action budget remains.

The engine filter removes illegal candidates before they are returned.
Equivalent decisions are deduplicated while preserving deterministic
order.

Strategy implementations use the same legal-decision stream:

```rust
use hw_ai::{FirstLegalStrategy, PriorityStrategy, Strategy};
use hw_engine::Game;

let game = Game::default(hw_core::Player::One);
let decision = FirstLegalStrategy.choose(&game);
let stronger_decision = PriorityStrategy.choose(&game);
```

`FirstLegalStrategy` returns the first generated legal decision that does
not immediately produce a terminal non-win for the current player, or
`None` for terminal games and positions with no safe legal decisions.

`PriorityStrategy` is still deterministic, but it groups legal decisions
before choosing. It prefers:

1. Immediate wins for the current player
2. Legal catastrophes
3. Paid actions
4. Legal turn end

Tie-breaking within each group preserves `legal_decisions` order. An
action is treated as an immediate win when applying it, then legally
ending the turn if possible, produces `GameOutcome::Winner` for the
current player. `EndTurn` is treated as an immediate win when ending the
turn directly produces that outcome.

Both strategies skip decisions that would immediately produce a
`GameOutcome::Winner` for the opponent or `GameOutcome::Draw`, either
directly or after legally ending the turn from the resulting position.

## CLI Integration

`hw-cli` can assign either baseline strategy to a player during a
session:

```text
ai p2 priority
ai p1 first
ai p2 off
ai
```

AI control is session-local CLI state and is not written to YAML saves.
After any command that reaches an AI-controlled turn, the CLI repeatedly
asks the selected strategy for decisions until the turn returns to a
human player or the game ends. AI decisions are printed with replay-style
short commands such as `b 0 rs`, `x 1 rm ym`, or `e`.
