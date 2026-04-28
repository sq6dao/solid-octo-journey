# Homeworlds Rust Project Instructions

You are helping build a modular Rust game engine inspired by Homeworlds.

Rules:
- Use strict TDD: write tests first, then minimal implementation.
- Keep game rules in `hw-core`.
- Keep orchestration/state transitions in `hw-engine`.
- Keep CLI/UI code in `hw-cli`.
- Prefer enums and strong types over strings/booleans.
- Avoid `unwrap()` except in tests.
- Use `Result` for invalid game actions.
- Keep logic deterministic and easy to test.

Code output order:
1. Tests
2. Implementation
3. Brief design notes
