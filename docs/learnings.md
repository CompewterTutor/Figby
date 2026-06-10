# Feiglet — Learnings

## 1.1.2 — Core types

- Serde + serde_json needed for round-trip tests even though "Touches" only listed
  `font.rs`. Cargo.toml modification is a necessary supporting change.
- `print_direction` defaults to -1 (unset) in `FIGfont::default()`, matching C
  semantics where -1 detects CLI override.
- `#[derive(Default)]` on `FIGcharacter` works since `Vec<String>`'s default is empty.
  Manual impl triggered `clippy::derivable_impls`.
- Using struct literal `..Default::default()` pattern avoids `field_reassign_with_default`
  clippy lint in tests.

