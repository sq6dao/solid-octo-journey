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

Action generation is being added incrementally as AI-1 matures.
