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
- [ ] Star = 1–2 pieces
- [ ] Ships orbiting
- [ ] Owner presence
- [ ] Validation rules

#### 5. GameState (domain only)
- [ ] Systems collection
- [ ] Player homeworlds
- [ ] Bank state

---

## Phase 2 – Rules Engine (hw-engine)

### Goal
Encode all legal moves and transitions.

### Features

#### 1. Actions
- [ ] Enum:
  - Build
  - Move
  - Trade
  - Discover
  - Sacrifice
  - Catastrophe

#### 2. Action Validation
- [ ] Rule checking per action
- [ ] Invalid move → Result::Err

#### 3. Turn System
- [ ] Current player tracking
- [ ] Turn switching
- [ ] Multi-action turns (via sacrifice)

#### 4. State Transitions
- [ ] Pure functions: `GameState -> Action -> GameState`
- [ ] No side effects

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
- [ ] Piece invariants
- [ ] System rules
- [ ] Action validation

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
