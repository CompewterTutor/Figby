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

## 1.1.3 — Header parser

- `figlet.c` skips baseline with `%*d` in sscanf and doesn't parse `codetag_count`,
  while `chkfont.c` includes both. Feiglet follows `chkfont.c` (parse all fields
  including baseline and codetag_count as 9th optional field).
- Full layout derivation rule (from `figlet.c:1231-1238`):
  `old_layout == 0 → 64 (SM_KERN)`, `old_layout < 0 → 0`, else `(old_layout & 31) | 128 (SM_SMUSH)`.
- `pub(crate)` triggers `dead_code` lint when function only called from tests.
  Made function `pub` instead — will be used by subsequent font parser stages.

## 1.1.5 — Code-tagged FIGcharacter parser

- C's `sscanf(fileline,"%li",&theord)` auto-detects hex (`0x` prefix) via `strtol` behavior.
  Rust `parse_codetag_integer()` must manually detect `0x`/`0X` and call `i64::from_str_radix`.
- C stores code `-1` as a normal entry in the linked list; Rust port skips it (reserved per task spec).
- `inchr` in C is `long` (64-bit on Linux), but Rust map uses `u32` keys. Negative codes
  stored via two's complement `(code as u32)` — preserves bit pattern.
- Codetagged section end is signaled by first non-numeric line (not EOF). No error raised.

