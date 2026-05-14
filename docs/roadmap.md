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
- [x] Interactive arrow-key history and line editing
- [x] Tab-expand exact command shorthands
- [x] Tab-expand partial command names such as `bu` and `save-h`
- [x] Tab-expand command arguments such as system IDs, pieces, and paths

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
Deterministic reusable computer player support.

AI support should start as a reusable engine-adjacent layer, not as
CLI-only logic. The first implementation should live in a new `hw-ai`
crate that depends on `hw-core` and `hw-engine`.

The first public AI unit is one decision at a time:

- `AiDecision::Action(Action)`
- `AiDecision::EndTurn`
- `legal_decisions(game: &Game) -> Vec<AiDecision>`
- `Strategy`, with a deterministic `FirstLegalStrategy`

The UI driver can repeatedly ask a strategy for the next decision until
the turn passes or the game ends.

Rule constraints for legal generation:

- New discoveries are one-star systems only
- Two-star systems are homeworlds only
- Both homeworlds exist from game start
- Legal generation must never emit a two-star discovery action

#### AI-1: Legal Decisions

- [ ] Add the `hw-ai` workspace crate
- [ ] Generate deterministic legal `AiDecision` values from `Game`
- [ ] Include `EndTurn` only when `game.end_turn()` succeeds
- [ ] Generate legal catastrophes even at zero action budget
- [ ] Generate paid actions only when budget and sacrifice action-kind
  restrictions allow them
- [ ] Filter candidate actions through existing `Game::apply_action`
- [ ] Use deterministic ordering by system ID, action kind, color, size,
  and piece order
- [ ] Remove duplicate equivalent decisions
- [ ] Test that every generated action applies successfully
- [ ] Test that no generated decision discovers a two-star system

Initial action coverage:

- [ ] Build from every plausible bank ship candidate
- [ ] Travel owned ships to existing systems
- [ ] Travel owned ships to new one-star discoveries
- [ ] Trade owned ships to same-size bank ships of other colors
- [ ] Sacrifice owned ships
- [ ] Invade opponent ships in shared systems
- [ ] Catastrophe for every overpopulated system/color

#### AI-2: Baseline Strategies

- [ ] Add `FirstLegalStrategy`
- [ ] Add a simple deterministic priority strategy
- [ ] Prefer immediate wins, then necessary catastrophes, then paid
  actions, then legal turn end
- [ ] Test deterministic tie-breaking
- [ ] Test terminal-state handling

#### AI-3: Heuristic Evaluation

- [ ] Add a board-position evaluator
- [ ] Score homeworld safety
- [ ] Score ship count and ship size
- [ ] Score color access and action flexibility
- [ ] Score opponent pressure and invasion threats
- [ ] Account for bank scarcity
- [ ] Keep the evaluator deterministic and explainable in tests

#### AI-4: Shallow Search

- [ ] Add one-ply move selection over `AiDecision`
- [ ] Add configurable depth-limited search
- [ ] Use deterministic tie-breaking for equal scores
- [ ] Keep randomness out of v1 search
- [ ] Test search on small tactical fixtures

#### AI-5: UI Integration

- [ ] Add human-vs-AI support to the CLI
- [ ] Let the CLI repeatedly apply AI decisions until the AI turn ends
- [ ] Print AI decisions in the same notation as replayed commands where
  practical
- [ ] Reuse the same `hw-ai` APIs from the future TUI

#### AI-6: Stronger AI

- [ ] Consider alpha-beta search
- [ ] Consider seeded randomized tie-breaking
- [ ] Consider opening books
- [ ] Add replay/history tooling for AI regression games

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
