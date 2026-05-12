# Homeworlds Rust Engine – Roadmap

## Guiding Principles
- Strict TDD (tests first, minimal implementation)
- Deterministic game logic
- Strong typing (enums over primitives)
- Clear separation:
  - `hw-core`: domain rules
  - `hw-engine`: state transitions
  - `hw-cli`: interface

---

## Current State
- Workspace crates exist for `hw-core`, `hw-engine`, and `hw-cli`.
- `hw-core` currently implements:
  - `Color`, `Size`, `Player`, and `Piece`
  - `Bank` with three copies of every color/size combination
  - `SystemId` as a typed identifier for systems in a `GameState`
  - `StarSystem` with 0-2 unowned stars and zero or more owned ships
  - `GameState` as a domain container for systems, player homeworlds,
    and bank state
  - validation errors for invalid bank, star system, and game state
    construction
- `StarSystem` exposes deterministic owner presence checks through
  `has_presence` and `owners_present`.
- `GameState` requires exactly one distinct homeworld per player, but
  homeworld loss checks are deferred.
- `hw-engine` currently defines typed actions and non-mutating validation
  for Build, Move, Trade, Sacrifice, Invade, and Catastrophe. Move
  targets can be existing systems or newly discovered systems. Discovery
  validation requires requested stars to be available in the bank, and
  Move rejects target systems that share a star size with the source.
  Invade requires a same-size-or-larger acting-player ship in the target
  system.
- `hw-engine` exposes pure `apply_action` state transitions. Build and
  Move are implemented; other actions currently validate and report
  unsupported transition execution.
- `hw-cli` is still a placeholder.
- The workspace test suite currently covers core piece, bank, and star
  system invariants, `GameState` container behavior, and initial action
  validation.

---

## Phase 1 – Core Domain (hw-core)

### Goal
Model all fundamental game concepts with no engine logic.

### Features

#### 1. Piece System
- [x] Color enum (Red, Yellow, Green, Blue)
- [x] Size enum (Small, Medium, Large)
- [x] Piece struct
- [x] Ownership (Player)

#### 2. Player
- [x] Player type
- [x] Validation (2 players only initially)

#### 3. Bank
- [x] Collection of all pieces
- [x] Draw/remove operations
- [x] Validation (no negative counts)

#### 4. Star System
- [x] Star = 0–2 pieces
- [x] Ships orbiting
- [x] Owner presence
- [x] Validation rules
- [x] `StarSystemError` for invalid construction

#### 5. GameState (domain only)
- [x] Systems collection
- [x] Player homeworlds
- [x] Bank state
- [x] `SystemId` for typed system references
- [x] `GameStateError` for invalid construction

---

## Phase 2 – Rules Engine (hw-engine)

### Goal
Encode all legal moves and transitions.

### Features

#### 1. Actions
- [x] Enum:
  - [x] Build
  - [x] Move (existing or new system target)
  - [x] Trade
  - [x] Sacrifice
  - [x] Invade
  - [x] Catastrophe

#### 2. Action Validation
- [x] Non-mutating validation for Build, Move, Trade, Sacrifice,
  Invade, and Catastrophe
- [x] Invalid move → Result::Err
- [x] Full rule validation

#### 3. Turn System
- [ ] Current player tracking
- [ ] Turn switching
- [ ] Multi-action turns (via sacrifice)

#### 4. State Transitions
- [ ] Pure functions: `GameState -> Action -> GameState`
- [ ] No side effects
- [x] Build
- [x] Move
- [ ] Trade
- [ ] Invade
- [ ] Sacrifice
- [ ] Catastrophe

---

## Phase 3 – Game Flow

### Goal
Playable game loop (engine-level, no UI yet)

- [ ] Game initialization
- [ ] Win condition detection
- [ ] Game termination

---

## Phase 4 – CLI Interface (hw-cli)

### Goal
Minimal playable interface

- [ ] Command parsing
- [ ] Text-based rendering
- [ ] Input validation
- [ ] Turn feedback

---

## Phase 5 – Testing Expansion

### Unit Tests
- [x] Piece invariants
- [x] System rules
- [x] Bank invariants
- [x] GameState container and homeworld behavior
- [x] Action validation

### Integration Tests
- [ ] Full turn sequences
- [ ] Known game scenarios
- [ ] Edge cases (catastrophes, sacrifices)

---

## Phase 6 – Extensions (Future)

- [ ] AI player (minimax / heuristics)
- [ ] Serialization (save/load)
- [ ] Network multiplayer
- [ ] GUI (optional)

---

## Development Workflow

For every feature:
1. Define behavior
2. Write failing tests
3. Implement minimal logic
4. Refactor safely
5. Expand test coverage

---

## Milestones

- M1: Core domain complete
- M2: All actions implemented
- M3: Fully playable via CLI
- M4: Stable test suite
