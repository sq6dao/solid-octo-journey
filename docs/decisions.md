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

## 2026-05-12 – Star System Domain Invariants

**Decision**
Represent a star system in `hw-core` as:
- 0-2 unowned star pieces
- one or more owned ship pieces
- deterministic owner presence derived from `Player::ALL`

Invalid star systems return `StarSystemError` instead of panicking.

**Context**
The roadmap's Star System step needs core-domain validation without
introducing engine actions, turn flow, movement, discovery, or cleanup
behavior. A 0-star system lets core represent a homeworld after its stars
are gone; loss validation is handled later.

**Alternatives**
- Require at least one star on every system
- Model stars and ships as separate wrapper types immediately
- Store owner presence separately from ships

**Consequences**
+ Homeworlds with no stars can be represented for later loss checks
+ Owner presence cannot drift from the ships in orbit
+ The core model stays deterministic and easy to test
- Future engine transitions may need to clean up non-homeworld 0-star
  systems
- Wrapper types may still be useful once action rules become richer

---

## 2026-05-12 – GameState System Identity and Homeworlds

**Decision**
Introduce `SystemId` and use it in `GameState` to track:
- ordered `StarSystem` values
- exactly one distinct homeworld per `Player`
- current `Bank` state

`GameState::new` validates that every homeworld ID points at an existing
system and that players do not share a homeworld.

**Context**
The roadmap's GameState step includes systems collection, player
homeworlds, and bank state. Engine actions will also need a strong way to
refer to systems.

**Alternatives**
- Use `usize` indices as temporary system identifiers
- Track player homeworlds by embedding flags in `StarSystem`
- Allow players to share one homeworld system
- Treat missing player presence or 0-star homeworlds as construction
  errors

**Consequences**
+ Core can now pass a full board-and-bank snapshot to future engine code
+ Future actions can target systems through a strong type
+ Invalid or duplicate homeworld assignments are rejected
- `SystemId` is currently index-backed
- Win/loss validation remains a later game-flow concern

---

## 2026-05-12 – Initial Action Representation

**Decision**
Represent engine actions as a typed enum in `hw-engine`:
- Build
- Move with a target of an existing or new system
- Trade
- Sacrifice
- Catastrophe

Start with non-mutating validation over `&GameState`. Sacrifice is present
as an explicit unsupported variant.

**Context**
Phase 2 needs action representation before pure state transitions can be
implemented. Current `hw-core` exposes immutable domain accessors, so this
step validates action shape and references without changing state.

**Alternatives**
- Add action structs plus a trait hierarchy
- Implement full state transitions immediately
- Omit sacrifice and catastrophe until their rules are implemented

**Consequences**
+ Engine callers can use one stable action type
+ Invalid actions return structured `ActionError` values
+ Validation can grow before mutation APIs are added to core
- Action execution remains deferred
- Sacrifice still needs real rule validation

---

## 2026-05-12 – Catastrophe Validation

**Decision**
Validate catastrophe as an overpopulation-only action. A catastrophe is
legal when the target system contains 4 or more pieces of the selected
color, counting both stars and ships.

Catastrophe validation does not remove pieces or return them to the bank.

**Context**
Catastrophe can now be represented and validated before the engine has
pure state-transition support.

**Alternatives**
- Allow catastrophe for any positive piece count
- Keep catastrophe unsupported until state mutation exists

**Consequences**
+ Catastrophe legality now matches the overpopulation rule
+ Invalid catastrophes return structured `NoCatastrophe` errors
- Applying catastrophe effects remains part of future state transitions

---

## 2026-05-12 – Full Action Validation

**Decision**
Validate action powers by requiring the acting player to have the matching
color ship at the relevant system:
- Green for Build
- Yellow for Move
- Blue for Trade

Build also validates that the requested ship is the smallest available
bank piece of that color.

**Context**
Action validation now covers per-action rule legality for supported
actions while remaining non-mutating. Turn sequencing, sacrifice action
budgets, red/capture behavior, and state transitions are still deferred.

**Alternatives**
- Keep validation limited to ownership and presence checks
- Add turn sequencing and sacrifice budgets immediately
- Wait for mutation APIs before validating color powers

**Consequences**
+ Invalid action-power attempts return structured errors
+ Build validation now matches the smallest-available-piece rule
- Turn-aware validation remains part of future engine work

---

## 2026-05-12 – Move Target Unification

**Decision**
Model discovery as a move to a new system. `Action::Move` now carries a
target that is either an existing `SystemId` or a new system described by
unowned star pieces.

**Context**
Homeworlds treats both movement to an existing system and discovery of a
new system as yellow actions. Keeping them under one action variant makes
the engine action model match that rule more directly.

**Alternatives**
- Keep separate Move and Discover action variants
- Use optional destination fields on Move
- Delay the merge until state-transition APIs exist

**Consequences**
+ Existing-system movement and discovery share one validation path
+ Callers no longer need a separate Discover action shape
- Existing callers must migrate to `MoveTarget::Existing` or
  `MoveTarget::New`

---

## Future Decisions (To Be Made)

- Homeworld loss and win-condition validation
- Mutation APIs needed for pure state transitions
- AI architecture
- Serialization format (JSON vs binary)
- Networking approach
