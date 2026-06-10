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

## 1.2.6 — Line breaking and word splitting

- C's `splitline()` uses global `inchrline` (char buffer) and `outline`
  (rendered rows). Rust version takes `&[u32]` char_buffer and `&mut Vec<String>`
  output_rows as explicit parameters — no globals.
- C's `splitline()` always produces output (even if no word break found, it
  prints a blank line). Rust version returns `None` for no-break, letting the
  caller decide the fallback (forced break or blank line). This is more
  idiomatic and avoids silent blank-line generation.
- Return type `Option<(Vec<String>, usize)>` packs both the rendered part1 rows
  (for printing by caller) and the part2_start index (for caller to truncate
  its char_buffer). Cleaner than C's side-effect-only approach.
- The `#![allow(clippy::too_many_arguments)]` pattern from `add_char()` carries
  over to `split_line()` (9 params) — all necessary to avoid globals.
- Test pattern: `build_expected()` helper calls `add_char()` independently to
  compute reference output, then compares against `split_line()` result. This
  tests both the splitting logic and the rebuild correctness simultaneously.

## 1.2.7 — Phase merge review

Three bugs found in phase merge review:

1. **Width guard in wrong function**: C's `smushem()` (which Rust `smush_horizontal()` mirrors)
   has NO width guard. The guard `if (currcharwidth < 2 || old_prev_width < 2) smush = 0`
   belongs in `addchar()`/`add_char()`, not in `smushem()`/`smush_horizontal()`. Having it
   in `smush_horizontal()` caused `calc_smush_amount()` to fail because it passed
   `outlinelen` (not `old_prev_width`) as the width parameter, causing false `None`
   returns when outlinelen was small.

2. **Missing first-char optimization**: C's `addchar()` has `if (prev_width == 0)` short-circuit
   that copies the character directly without smush computation. Rust `add_char()` lacked
   this, causing incorrect overlap calculations for the first character.

3. **Wrong `contains()` usage for KERN|SMUSH check**: `calc_smush_amount()` used
   `!mode.contains(KERN | SMUSH)` which checks ALL bits set (AND), but C's
   `!(smushmode & (KERN | SMUSH))` checks ANY bit set (OR). Changed to
   `!mode.contains(KERN) && !mode.contains(SMUSH)` to match C semantics.

## 1.3.4 — Main event loop

- `pub(crate)` visibility in `font.rs` constants is NOT visible from binary crate
  (`main.rs`), since the binary depends on `feiglet` as a separate library crate.
  Changing `DEUTSCH_CHARS` to `pub` is required when the binary needs it.
- `std::io::Stdin::bytes()` requires `Read` trait in scope (`use std::io::Read`).
  Using `io::BufReader::new(io::stdin()).bytes()` avoids
  `clippy::unbuffered_bytes` lint.
- `clippy::never_loop` fires on one-shot `loop { return ... }` — replace with
  plain `match`/`if`.
- The inner retry loop in C uses `do {} while (char_not_added)` with a flag.
  Rust alternative: `loop { ... break; ... }` where every branch either `break`s
  (char handled) or falls through (retry after flush/split). Avoids
  `clippy::needless_late_init`.
- Clippy `ptr_arg` on `&mut Vec<String>` — use `#[allow(clippy::ptr_arg)]` when
  the function signature needs to match the calling convention (callers pass
  `Vec<String>` and mutate it). Changing to `&mut [String]` loses the ability
  to `clear()`.
- `flush_output_line` has 8 parameters triggering `clippy::too_many_arguments`.
  Acceptable mirror of C's global-based approach — suppressed with allow attr.

## 1.3.5 — Phase merge

- Merge `c7ab68d` is single-parent (fast-forward), unlike previous phase
  merges (1.1.8, 1.2.7) which used `--no-ff` (two parents). The fast-forward
  was likely due to `master` being directly on `release/1.3`'s linear history
  with no divergent commits.

## 1.3.1 — CLI argument parsing

- `#[allow(non_snake_case)]` is required on clap structs when flags have
  uppercase/lowercase collisions (e.g., `-L` vs `-l`). In snake_case, `flag_L`
  and `flag_l` collapse to the same name. Eight such collisions exist in FIGlet.
- `CliArgs::try_parse_from(["feiglet", "-A"])` — the array arg must be
  owned (no `&` prefix). Clippy `needless_borrows_for_generic_args` fires if
  you write `&["feiglet", "-A"]`; clap's `try_parse_from` accepts
  `impl IntoIterator` and `[&str; N]` already satisfies that without a borrow.
- `-m -1` parsing with clap: requires `#[arg(allow_hyphen_values = true)]` on
  the field. Without it, clap treats `-1` as an unknown flag. In clap 4 the
  `Option<i32>` parser alone does NOT allow leading hyphens — the attribute
  must be explicit.
- `smushoverride` for `-s` does NOT change `smushmode` — it only sets
  override to `SMO_NO`. This differs from `-W` which sets `smushmode = 0`
  AND `override = SMO_YES`. Matching C semantics precisely is critical.

## 1.4.1 — Control file parser

- C's `readcontrol()` outer switch reads the FIRST byte of each line.
  Lines starting with `\` (backslash) in `upper.flc` (e.g.
  `\0x037A \0x0399`) fall to `default:` and are silently skipped.
  These mapping entries are effectively documentation-only in the C parser.
  Only lines starting with `0-9` or `-` are parsed as mapping table entries.
- `read_tchar()` and `read_num()` are deeply coupled — `read_tchar` parses
  `\` escape prefix then delegates to `read_num` for numeric escapes
  (`\0x...`, `\377`, `\-6`). `read_num` uses the full hex digit set
  `"0123456789ABCDEF"` regardless of parsed base (decimal uses hex
  digit set too, matching C's `strchr` approach).
- C `charsetname()` has dead code: the `\n`/`\r` check is never hit
  because `readTchar` already returns 0 for newlines. The `Zungetc(0, fp)`
  bug (pushing back NUL byte) is also present — harmless since
  `skiptoeol` always follows.
- C `readcontrol()` has missing `break` before `case '\r': case '\n':`
  after the `case 'g'` inner switch — harmless fallthrough since the
  empty-line case just does `break`.
- The `94x94` double-byte charset path reads `x`, then `9`, then `4`,
  then `skipws` before `charsetname`. The `96` path has NO `skipws`
  before `charsetname` — a C bug never triggered in practice (no
  `.flc` uses `96` charset).

## 1.4.3 — ISO 2022 character set handling

- Closure-based dispatch (`next_char`) used to route between `iso2022()`
  and raw `input.next()` based on `multibyte` flag. Parameters for `input`
  and `state` avoid closure capturing them, preventing borrow conflicts
  with `remap_char` usage later in the function.
- `control_state` must be `mut` even though most phases use it immutably
  (`remap_char`), because `iso2022()` takes `&mut self` to update gl/gr/gndbl.
- C `iso2022()` uses `inchr` (long) for ch, so values can exceed u32 range
  on shift operations. Rust port uses `u32` which is sufficient since
  FIGlet's gn values are ASCII codes shifted by at most 24 bits.
- `b'B' as u32 + 0x100` patterns can't be used directly in match arms
  in Rust — use literal hex values like `0x128` instead. Hex is readable
  as `0x100 + byte_value` mapping.

