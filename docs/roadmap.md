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
- `GameState` requires exactly one distinct homeworld per player. Core
  construction still allows empty homeworld states, while engine game
  flow detects homeworld loss at turn end.
- `hw-engine` currently defines typed actions and non-mutating validation
  for Build, Travel, Trade, Sacrifice, Invade, and Catastrophe. Travel
  targets can be existing systems or newly discovered systems. Discovery
  validation requires requested stars to be available in the bank, and
  Travel rejects target systems that share a star size with the source.
  Invade requires a same-size-or-larger acting-player ship in the target
  system.
- `hw-engine` exposes pure `apply_action` state transitions for Build,
  Travel, Trade, Invade, Sacrifice, and Catastrophe. It also provides
  `TurnState` for turn sequencing and `Game` for initialization,
  turn-end win detection, and terminal-state enforcement.
- `hw-cli` provides a text hot-seat interface with setup prompts,
  command parsing, state rendering, turn feedback, and the semicolon
  state-display shortcut. Game startup requires exactly two stars in
  each homeworld. See `docs/cli.md` for command details.
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
  - [x] Travel (existing or new system target)
  - [x] Trade
  - [x] Sacrifice
  - [x] Invade
  - [x] Catastrophe

#### 2. Action Validation
- [x] Non-mutating validation for Build, Travel, Trade, Sacrifice,
  Invade, and Catastrophe
- [x] Invalid action → Result::Err
- [x] Full rule validation

#### 3. Turn System
- [x] Current player tracking
- [x] Turn switching
- [x] Multi-action turns (via sacrifice)

#### 4. State Transitions
- [x] Pure functions: `GameState -> Action -> GameState`
- [x] No side effects
- [x] Build
- [x] Travel
- [x] Trade
- [x] Invade
- [x] Sacrifice
- [x] Catastrophe

---

## Phase 3 – Game Flow

### Goal
Playable game loop (engine-level, no UI yet)

- [x] Game initialization
- [x] Win condition detection
- [x] Game termination

---

## Phase 4 – Testing Expansion

### Unit Tests
- [x] Piece invariants
- [x] System rules
- [x] Bank invariants
- [x] GameState container and homeworld behavior
- [x] Action validation
- [x] Action state transitions

### Integration Tests
- [x] Full turn sequences
- [x] Known game scenarios
- [x] Edge cases (catastrophes, sacrifices)

---

## Phase 5 – CLI Interface (hw-cli)

### Goal
Minimal playable interface

- [x] Command parsing
- [x] Text-based rendering
- [x] Input validation
- [x] Turn feedback

---

## Phase 6 – Serialization

### Goal
Stable save/load support and state interchange.

- [x] Add YAML serialization support through stable save DTOs
- [x] Serialize and deserialize game state, systems, pieces, bank counts,
  turn state, and game status
- [x] Define a v1 YAML save file format
- [x] Add save/load support to the CLI
- [x] Add round-trip tests and invalid-file coverage
- [ ] Action/history serialization

---

## Phase 7 – AI Player

### Goal
Deterministic computer player support.

- [ ] Generate legal actions from a game state
- [ ] Add a basic heuristic evaluator
- [ ] Implement one-ply or shallow-search move selection
- [ ] Add human-vs-AI support to the CLI
- [ ] Test deterministic move selection and terminal-state handling

---

## Phase 8 – TUI Interface

### Goal
Richer playable terminal UI, probably with Ratatui.

- [ ] Add Ratatui-based application shell
- [ ] Render board and system state panels
- [ ] Add command/input panel
- [ ] Show turn, action budget, errors, and game outcome feedback
- [ ] Add help/reference view
- [ ] Integrate save/load once serialization exists

---

## Phase 9 – Future Plans

- [ ] Network multiplayer
- [ ] GUI (optional)
- [ ] Stronger AI search and heuristics
- [ ] Replay/history tooling

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
- M3: Engine game flow complete
- M4: Stable test suite
- M5: Fully playable via CLI
- M6: Save/load support
- M7: AI opponent
- M8: TUI playable interface
