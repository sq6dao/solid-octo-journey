# Decision Log

## Format
- Date: YYYY-MM-DD
- Decision
- Context
- Alternatives
- Consequences

---

## 2026-04-28 – Workspace Architecture

**Decision**
Use a Cargo workspace with:
- `hw-core`
- `hw-engine`
- `hw-cli`

**Context**
Separation of concerns and testability are critical.

**Alternatives**
- Single crate → simpler but less modular
- Feature flags → adds complexity

**Consequences**
+ Clear boundaries
+ Easier testing
- Slight setup overhead

---

## 2026-04-28 – Strict TDD

**Decision**
All features must start with tests.

**Context**
Game rules are complex and error-prone.

**Alternatives**
- Implementation-first → faster initially
- Hybrid → inconsistent discipline

**Consequences**
+ High correctness
+ Regression safety
- Slower early development

---

## 2026-04-28 – Domain-Driven Design

**Decision**
Model domain explicitly in `hw-core`.

**Context**
Game rules should be independent of execution.

**Alternatives**
- Logic in engine → harder to test
- Data-only core → weak invariants

**Consequences**
+ Strong invariants
+ Reusable logic
- More upfront design

---

## 2026-04-28 – Strong Typing

**Decision**
Use enums and structs instead of primitives.

**Context**
Game rules depend on discrete states.

**Alternatives**
- Strings/ints → flexible but unsafe

**Consequences**
+ Compile-time safety
+ Self-documenting code
- Slight verbosity

---

## 2026-04-28 – Pure State Transitions

**Decision**
Engine functions are pure:
`GameState -> Action -> Result<GameState>`

**Context**
Determinism and testability are priorities.

**Alternatives**
- Mutable state → simpler but error-prone

**Consequences**
+ Easy testing
+ Predictable behavior
- Requires cloning or careful ownership

---

## 2026-04-28 – Error Handling

**Decision**
Use `Result` for invalid actions.

**Context**
Invalid moves are expected in gameplay.

**Alternatives**
- Panic → unsafe
- Silent ignore → unclear behavior

**Consequences**
+ Explicit failure handling
+ Better debugging
- Slight boilerplate

---

## 2026-04-28 – Cargo.lock Policy

**Decision**
Commit `Cargo.lock`.

**Context**
Workspace includes a binary (`hw-cli`).

**Alternatives**
- Ignore lockfile → less reproducibility

**Consequences**
+ Deterministic builds
- Occasional merge conflicts

---

## Future Decisions (To Be Made)

- Action representation (command pattern vs enum-only)
- AI architecture
- Serialization format (JSON vs binary)
- Networking approach
