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
- Travel with a target of an existing or new system
- Trade
- Sacrifice
- Invade
- Catastrophe

Start with non-mutating validation over `&GameState`.

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
- Yellow for Travel
- Blue for Trade
- Red for Invade

Build also validates that the requested ship is the smallest available
bank piece of that color.

**Context**
Action validation now covers per-action rule legality for supported
actions while remaining non-mutating. Turn sequencing, sacrifice action
budgets, ownership changes, and state transitions are still deferred.

**Alternatives**
- Keep validation limited to ownership and presence checks
- Add turn sequencing and sacrifice budgets immediately
- Wait for mutation APIs before validating color powers

**Consequences**
+ Invalid action-power attempts return structured errors
+ Build validation now matches the smallest-available-piece rule
- Turn-aware validation remains part of future engine work

---

## 2026-05-12 – Travel Target Unification

**Decision**
Model discovery as travel to a new system. `Action::Travel` carries a
target that is either an existing `SystemId` or a new system described by
unowned star pieces.

**Context**
Homeworlds treats both movement to an existing system and discovery of a
new system as yellow actions. Keeping them under one action variant makes
the engine action model match that rule more directly.

**Alternatives**
- Keep separate Travel and Discover action variants
- Use optional destination fields on Travel
- Delay the merge until state-transition APIs exist

**Consequences**
+ Existing-system movement and discovery share one validation path
+ Callers no longer need a separate Discover action shape
- Existing callers must migrate to `TravelTarget::Existing` or
  `TravelTarget::New`

---

## 2026-05-12 – Sacrifice Validation

**Decision**
Validate sacrifice as ownership plus presence at the selected system. The
sacrificed ship must belong to the acting player and be present in the
referenced system.

Sacrifice validation does not remove the ship or grant extra actions.

**Context**
Sacrifice can now be validated before turn sequencing and action-budget
support exist.

**Alternatives**
- Allow sacrificing any present ship
- Implement size-based action budgets immediately
- Keep sacrifice unsupported until the turn system exists

**Consequences**
+ Sacrifice is now a real validated action shape
+ Invalid sacrifice attempts reuse existing structured action errors
- Size-based action budgets remain part of future turn-system work

---

## 2026-05-12 – Invade Validation

**Decision**
Represent the red action as `Invade`. Validate invade as a red-powered
action targeting an opponent-owned ship at the selected system.

Invade validation does not change ownership of the target ship.

**Context**
The engine now has a typed shape for all four color powers before pure
state-transition support exists.

**Alternatives**
- Name the action Attack
- Implement ownership mutation immediately
- Keep the red action absent until state transitions exist

**Consequences**
+ Red action legality is covered by non-mutating validation
+ Invalid own-ship invasions return a structured error
- Ownership changes remain part of future state-transition work

---

## 2026-05-12 – Action Validation Module Layout

**Decision**
Keep one validation module per action under `hw-engine`. The Travel
validation module is stored in `travel.rs` and declared as `mod travel`.

**Context**
Action validation had grown into one large file. Splitting by action keeps
each rule implementation small while preserving action-name-based file
names.

**Alternatives**
- Keep all validation in one file
- Use longer file names like `travel_action.rs`

**Consequences**
+ Validation files now align with action names
- Action modules avoid Rust keyword overlap

---

## 2026-05-12 – Transition System States

**Decision**
Allow `StarSystem` values with zero ships, and derive size ordering from
`Small < Medium < Large`.

**Context**
Pure state transitions need to represent intermediate homeworld states
after the last ship leaves. Invade validation also needs an explicit ship
size ordering so same-or-larger invasion checks are deterministic.

**Alternatives**
- Keep rejecting zero-ship systems
- Add a separate transition-only system representation
- Compare sizes with ad hoc match expressions in the engine

**Consequences**
+ Homeworld loss can be detected later without changing `GameState`
+ Engine validation can compare ship sizes directly
- Non-homeworld cleanup must be handled by engine transitions

---

## 2026-05-12 – Discovery Star Bank Validation

**Decision**
Validate `TravelTarget::New` discovery stars against the bank, including
duplicate color/size requests.

**Context**
Discovery consumes stars from the bank once transitions exist. Validating
availability before transitions keeps invalid actions from reaching state
mutation code.

**Alternatives**
- Validate only the discovered system shape
- Defer bank availability to transition execution

**Consequences**
+ Discovery legality now accounts for bank exhaustion
+ Transition code can rely on discovery stars being drawable
- Travel validation needs to count duplicate requested stars

---

## 2026-05-12 – Travel Star Size Validation

**Decision**
Reject Travel actions when the source system and target system share any
star size. This applies to existing-system targets and new discovery
targets.

**Context**
Homeworlds systems are identified by star sizes. A ship cannot travel or
discover between systems that overlap in star size.

**Alternatives**
- Validate only system IDs and piece ownership
- Defer size conflicts to transition execution

**Consequences**
+ Invalid Travel actions fail before transition execution
+ Discovery uses the same star-size rule as existing-system movement
- Travel validation now depends on target star details

---

## 2026-05-12 – Invade Ship Size Validation

**Decision**
Require Invade actions to have an acting-player ship in the target system
whose size is greater than or equal to the target ship size.

**Context**
Red action power determines whether a player can invade in a system, but
the ship-size rule determines which opponent ships can be captured there.

**Alternatives**
- Let any red ship invade any opponent ship
- Apply the size check only during transition execution

**Consequences**
+ Oversized target ships are rejected during validation
+ Invade transition code can rely on size legality
- Invade validation now checks both color power and ship size

---

## 2026-05-12 – Transition API and Build Execution

**Decision**
Expose `apply_action(&GameState, &Action) -> Result<GameState,
TransitionError>` from `hw-engine`. Implement action transitions in
separate commits after adding the public API.

**Context**
The engine already validates actions without mutating state. State
transitions now need a pure public entry point that validates first and
returns a new `GameState`.

**Alternatives**
- Consume `GameState` and `Action` by value
- Add a larger engine or turn-state type immediately
- Implement all action transitions in one commit

**Consequences**
+ Callers get a deterministic transition API
+ Each action transition landed as a separate tested commit
- Turn sequencing remains outside this API

---

## 2026-05-12 – Travel Execution and Pruning

**Decision**
Implement Travel transitions for existing-system targets and new discovery
targets. Prune non-homeworld systems that have no ships after movement,
returning remaining pieces to the bank and compacting system IDs.

**Context**
Travel is the first transition that can remove a ship from one system and
therefore needs engine-level cleanup. Homeworld systems remain in the
state even when their last ship leaves.

**Alternatives**
- Keep empty non-homeworld systems indefinitely
- Use tombstones instead of compacting `SystemId` values
- Delay discovery execution until a later transition pass

**Consequences**
+ Travel produces game states without empty non-homeworld systems
+ Discovery consumes stars from the bank during transition execution
- Callers must use the returned state after ID compaction

---

## 2026-05-12 – Trade Execution

**Decision**
Implement Trade transitions by drawing the requested same-size bank piece,
returning the old ship to the bank unowned, and replacing the ship in the
target system.

**Context**
Trade validation already enforces ownership, presence, blue power, bank
availability, and size matching. The transition can therefore focus on the
bank exchange and system replacement.

**Alternatives**
- Return the old ship before drawing the new one
- Leave bank updates for a later consistency pass

**Consequences**
+ Trade now updates both board and bank state
+ Size preservation is enforced before execution
- Piece counts reflect the current loose `GameState` bank consistency

---

## 2026-05-12 – Invade Execution

**Decision**
Implement Invade transitions by replacing one matching opponent ship with
the same color and size ship owned by the acting player.

**Context**
Invade validation already checks red power, opponent ownership, target
presence, and same-or-larger acting-player ship size. The transition only
needs to change target ownership in the selected system.

**Alternatives**
- Remove and rebuild the whole system through a dedicated ownership API
- Delay ownership changes until turn sequencing exists

**Consequences**
+ Red actions now mutate board ownership through `apply_action`
+ Bank state remains unchanged by Invade
- Duplicate pieces are still represented by equal `Piece` values

---

## 2026-05-12 – Sacrifice Execution

**Decision**
Implement Sacrifice transitions by removing the selected owned ship,
returning it to the bank unowned, and pruning the system if it becomes an
empty non-homeworld.

**Context**
Sacrifice validation already checks ownership and presence. Turn budgets
from sacrifice size remain a later turn-system concern.

**Alternatives**
- Wait for turn sequencing before removing sacrificed ships
- Keep empty non-homeworld systems after sacrifice

**Consequences**
+ Sacrifice now updates board and bank state
+ Empty homeworlds are preserved for later loss detection
- Extra action budgets remain unimplemented until turn-system work

---

## 2026-05-12 – Catastrophe Execution

**Decision**
Implement Catastrophe transitions for one selected system. Remove all
stars and ships of the selected color in that system, return them to the
bank unowned, and leave other systems unchanged.

If the selected non-homeworld system has no stars after the catastrophe,
prune it even when ships remain, returning those remaining ships to the
bank as well.

If the selected homeworld system has no stars after the catastrophe,
retain the homeworld system but return all remaining ships there to the
bank.

**Context**
Catastrophe validation already checks overpopulation in one system. The
transition must not cascade into other systems or auto-resolve unrelated
overpopulation.

**Alternatives**
- Remove matching pieces from every overpopulated system
- Keep starless non-homeworld systems with remaining ships
- Auto-run catastrophes after other action transitions

**Consequences**
+ Catastrophe execution is explicit and single-system scoped
+ Starless non-homeworld cleanup matches the board model
+ Starless homeworlds cannot retain ships before loss detection
- Automatic catastrophe resolution remains out of scope

---

## 2026-05-14 – Turn Orchestration

**Decision**
Track turn flow in `hw-engine` with `TurnState`, layered over the pure
`GameState -> Action -> Result<GameState>` transition API.

`TurnState` stores the current player, remaining paid actions, and any
sacrifice-granted action-kind limit. Normal turns start with one paid
action. `end_turn` is explicit and only succeeds once paid action budget
is exhausted.

Catastrophes remain playerless, explicit, and cost 0 actions. They can
be applied before or after paid actions, including when no paid actions
remain, and unresolved catastrophes do not block ending the turn.

**Context**
The roadmap calls for current player tracking, turn switching, and
multi-action turns via sacrifice. Existing action validation and
transitions already handle individual actions without knowing turn
state.

**Alternatives**
- Embed current-player and action-budget fields in `hw-core::GameState`
- Replace `apply_action` with a turn-only action application API
- Auto-resolve or require pending catastrophes before turn end

**Consequences**
+ Core state remains focused on board, homeworld, and bank data
+ Low-level pure transitions stay available for tests and future tools
+ Sacrifice budgets are enforced without duplicating action validation
- Callers that need legal play flow must use `TurnState`

---

## 2026-05-14 – Engine Game Flow

**Decision**
Add `Game` in `hw-engine` as the engine-level game loop wrapper over
`TurnState`.

`Game::new` builds a game from explicit player homeworld setup,
consuming the starting stars and ships from a fresh bank. `Game::default`
provides one deterministic valid setup for tests and future CLI work.

Loss is checked only by `Game::end_turn`. A player loses when they have
no ships at their own homeworld; the other player wins. If both players
have lost at the same turn end, the game is a draw. Terminal games reject
further actions and turn ending.

**Context**
Phase 3 calls for engine-level initialization, win detection, and game
termination without adding CLI parsing or rendering. `TurnState` already
handles action budgets and player switching.

**Alternatives**
- Put initialization and terminal status directly in `GameState`
- Detect loss immediately after every action
- Provide only a fixed default setup

**Consequences**
+ Engine callers now have one API for playable game flow
+ Core remains a reusable board-state model
+ Loss timing is explicit and testable at turn boundaries
- The CLI still needs separate command parsing and rendering

---

## 2026-05-14 – Travel Naming

**Decision**
Rename the yellow movement action to Travel across public API,
module names, tests, and documentation.

`Action::Travel`, `TravelTarget`, and `ActionKind::Travel` replace the
old movement-action names. The validation and transition modules are named
`travel.rs`.

**Context**
The previous name forced raw-identifier module declarations
because `move` is a Rust keyword. Travel avoids that overlap while still
describing movement to existing systems and discovery of new systems.

**Alternatives**
- Keep the old public name and use raw identifiers internally
- Add partial compatibility aliases

**Consequences**
+ The action API and module names no longer overlap Rust keywords
+ File names line up directly with action names
- Existing callers must migrate to the Travel names

---

## Future Decisions (To Be Made)

- AI architecture
- Serialization format (JSON vs binary)
- Networking approach
