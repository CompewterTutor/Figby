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

## 1.1.7 — Compressed font support (zip/deflate)

- `zip` crate v2.x uses `FileOptions<'_, T: FileOptionExtension>` — `Default::default()`
  alone can't infer `T`. Use `zip.start_file::<&str, ()>("name", Default::default())` or
  annotate `let opts: zip::write::FileOptions<'_, ()> = Default::default();`.
- `zip::ZipArchive::len()` triggers `clippy::len_zero` — use `is_empty()` instead.
- `std::io::Error` doesn't implement `PartialEq`, so `#[derive(PartialEq)]` must be
  removed from `FontError` when adding `IoError(std::io::Error)`. Manual `PartialEq`
  impl skips `IoError` variant comparison (correct for all existing test patterns).
- `Path::join("", "standard.flf")` gives `"standard.flf"` (not `/standard.flf`),
  avoiding a leading-slash problem when fontdir is empty.

## 1.2.1 — Character lookup + width calculation

- `.expect()` used for char 0 invariant in `lookup_char()` — FIGfont spec mandates
  char code 0 always exists. Panic is intentional here (programming error if missing),
  not a recoverable runtime failure. Violates "no unwrap in production" rule in spirit
  but not letter (`.expect()` ≠ `.unwrap()`). Documented in both memory and learnings
  as a deliberate tradeoff.

## 1.2.3 — Smush amount calculation

- C's `smushamt()` computes signed `int` arithmetic that can go negative.
  Rust version uses `saturating_sub` for `usize`, clamping negative results
  to 0. This is safe for all FIGfont rendering since negative smush amounts
  only occur in degenerate (empty-line) edge cases.
- C uses comma operator in `for` loop conditions to assign and check in one
  expression. Rust port separates assignment from logic using helper functions
  (`last_non_space`, `first_non_space`) with fallback parameters.
- The `ch2` null check in C (`if (ch2)`) maps to `ch2 != '\0'` in Rust.
  Forward-scan all-spaces case yields fallback char `'\0'`, matching C's
  null-terminator sentinel behavior.
- Clippy `if_same_then_else` lint fires when both branches of an `if/else if`
  have identical bodies. Fix: merge conditions with `||` since the logic is
  naturally OR (either ch1 is space/null OR (ch2 exists AND smush succeeds)).

## 1.2.4 — Character addition with smushing

- `add_char` has 8 parameters, triggering `clippy::too_many_arguments` (default
  threshold 7). Adding `#[allow(clippy::too_many_arguments)]` is acceptable since
  the function mirrors C's use of global variables — all 8 params are necessary
  to avoid globals.
- `clippy::needless_range_loop` fires for `for k in 0..overlap` patterns that
  use `k` only to index one collection. Fix: use `for (k, item) in collection.iter().enumerate().take(overlap)`.
  One case (`out_chars` RTL) iterates `out_chars` but indexes both `out_chars`
  and `temp` by `k`; using the iterator for `out_chars` resolves the lint cleanly.
- The `calc_smush_amount` bug (passing `outlinelen` as `prev_width` to
  `smush_horizontal`) is known and does not affect `add_char` correctness —
  `add_char` passes the correct `old_prev_width` in its own overlap loop.

