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
