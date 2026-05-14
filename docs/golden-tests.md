# Golden Session Tests

Golden session tests keep reusable CLI histories separate from the
assertions that define expected behavior.

History fixtures live in `tests/golden/*.hw`. They use the same
line-oriented command format as CLI history files: setup lines first,
then commands, one per line. Empty lines and `#` comments are allowed.
Do not put expected-output markers in history files.

Expectations live in `hw-cli/tests/golden_sessions.rs`. Each case imports
a history file with `include_str!`, runs it through `hw_cli::session::run`,
and checks expected or rejected output substrings in Rust.

To add a case:

```rust
GoldenCase {
    name: "opening",
    history: include_str!("../../tests/golden/opening.hw"),
    expect: &["Game started.", "Status: in progress"],
    reject: &["Error:"],
}
```

Run golden sessions with:

```sh
cargo test -p hw-cli --test golden_sessions
```
