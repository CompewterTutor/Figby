# Feiglet — Memory Index

Master memory index. Detailed entries live in versioned files below.

## Versioned Memory Files

| Milestone | File | Status |
|-----------|------|--------|
| v1 — Port | [memory-v1.md](memory-v1.md) | Active |

## Architectural Decisions

### UTF-8 Native Encoding
Feiglet uses Rust `char`/`String` natively (UTF-8), not `wchar_t`.
FIGlet C used `typedef long inchr` for internal char representation.
We map directly to `char` (Unicode scalar value) and use `String` for
output rows. TLF fonts already UTF-8; FIGfont ASCII is valid UTF-8.

### Smushing Engine as Pure Functions
The C `smushem()` function uses global `smushmode`, `previouscharwidth`,
`currcharwidth`, `hardblank`, `right2left`. Rust version takes these
explicitly as parameters — no globals. `SmushMode` bitflags struct
replaces `int smushmode`.

### No ZIP Reading in Initial Port
The original zipio.c/inflate.c/crc.c stack reads `.flf` from ZIP archives.
Rust equivalent uses the `zip` crate. Deferred to Phase 1.1.7 — initial
port reads plain files only.

### Error Handling
No `unwrap()` in production paths. All parse errors return `Result`.
Parser is lenient: hard errors on bad magic, but silently corrects minor
issues (truncation, extreme values) as C does.

### CLI Library
Use `clap` with derive macros for CLI parsing, replacing `getopt`.
FIGlet flag semantics preserved exactly.

## Task History

### 1.1.1 — Create `feiglet` crate in workspace
Added `[lib]` section to `feiglet-rs/Cargo.toml` (name=feiglet, path=src/lib.rs,
crate-type=[lib]). Added `#![doc]` crate-level attribute to `src/lib.rs` with
description. Sorted `pub mod` declarations alphabetically (rustfmt preference).
Five module stubs (font, render, smush, control, input) compile as-is.

### 1.1.2 — Define core types: FIGfont, FIGcharacter, FCharnode
Added `FIGfont`, `FIGcharacter`, `FCharnode` structs in `font.rs` with serde
derive (Serialize/Deserialize). `FIGcharacter` wraps `Vec<String>` rows with
`width()` and `rows()` accessors. `FCharnode` maps `u32` code → `FIGcharacter`.
`FIGfont` owns font metadata (hardblank, charheight, baseline, maxlength,
old_layout, full_layout, print_direction, comment_lines, codetag_count) +
`HashMap<u32, FIGcharacter>` chars. All types derive `Default` (FIGfont: -1
for print_direction, rest zero/empty). Round-trip serde tests via serde_json.

### 1.1.3 — FIGfont magic number + header line parser
Added `parse_header()` in `font.rs` — validates `flf2a` magic, extracts hardblank
char, parses 5 required numeric fields (height, baseline, max_length, old_layout,
comment_lines) and 3 optional fields (print_direction, full_layout, codetag_count).
Missing optionals defaulted per C logic: print_direction → -1, full_layout derived
from old_layout, codetag_count → 0. Defined `FontError` enum with `InvalidMagic`
and `ParseError` variants. 11 fixture + error tests.

### 1.1.4 — FIGcharacter data parser
Added `parse_char_data()` in `font.rs` — reads 95 required ASCII FIGcharacters
(codes 32–126) and 7 Deutsch chars (196, 214, 220, 228, 246, 252, 223). Added
`strip_endmarks()` private helper that follows `figlet.c:1155-1165` algorithm:
trim trailing whitespace, identify endmark as last remaining char, remove all
consecutive endmarks from the right. Trailing whitespace before endmarks is
preserved for width correctness. Added `DEUTSCH_CHARS` constant matching C
`inchr deutsch[7]`. Function returns unconsumed line slice for subsequent
codetag parsing (1.1.5). 10 tests covering endmark stripping edge cases,
102-char parse, endmark removal verification, width consistency, error on
truncated input, and unconsumed line return.

### 1.1.6 — TLF font support (TOIlet format)

Added `FontFormat` enum (`Figfont`/`Tlf`), `format` field on `FIGfont`,
`parse_tlf_font()` entry point. `parse_header()` now accepts both `flf2a`
and `tlf2a` magic numbers. Reuses all FIGfont parsing infrastructure
(endmark stripping, char data, codetagged). TLF rows are UTF-8 natively
(Rust `String` handles this without special treatment).

### 1.1.7 — Compressed font support (zip/deflate)

Added `load_font()` as FIGopen() equivalent: try `fontdir/name.flf`, bare
`name.flf`, `fontdir/name.tlf`, then bare `name.tlf`. Each candidate reads
bytes, detects ZIP magic (`PK\x03\x04`), extracts first entry if ZIP, parses
via existing `parse_tlf_font()`. `FontError` gained `IoError(std::io::Error)`
and `ZipError(String)` variants. `zip = "2"` and `flate2 = "1"` added to
Cargo.toml. ZIP is only read from — `ZipWriter` used solely in tests.
`PartialEq` on `FontError` is now manual (cannot derive with `std::io::Error`).

### 1.1.8 — Phase merge: release/1.1 → master

Merged all Phase 1.1 work into default branch (master). Phase 1.1 complete:
crate scaffold, core types, FIGfont/TLF header parser, FIGcharacter data
parser, code-tagged character parser, TLF support, ZIP/deflate compressed
font loading. All 7 subtasks (1.1.1–1.1.7) implemented, tested, merged.
Phase 1.2 (render engine) is next.

### 1.2.1 — Character lookup + width calculation

Added `lookup_char()` in `render.rs` — font char lookup with fallback to
char code 0. Updates `current_width` (via `&mut usize`) so caller captures
previous width before next call. Uses `expect()` for char 0 invariant
(FIGfont spec requires it). Three tests: known char, unknown fallback,
previous-width tracking.

### 1.2.2 — Smushing rules engine

Full smushing rules engine in `smush.rs`. `SmushMode` newtype over `u32`
with bitmask constants matching FIGfont `full_layout` encoding.
`smush_horizontal()` mirrors `figlet.c:smushem()` — all 6 horizontal rules
(H1-H6) plus universal overlap and kerning. `smush_vertical()` implements
V1-V5 vertical smushing rules. Hardblank treated as space for vertical ops.
Hierarchy helpers shared between H3/V3. No `.unwrap()` in production — all
fallible paths use `Option<char>`. 34 unit tests covering every rule.

### 1.2.3 — Smush amount calculation

Added `calc_smush_amount()` in `render.rs` — port of C `smushamt()`.

### 1.2.4 — Character addition with smushing

Added `add_char()` in `render.rs` — port of C `addchar()`. Function takes
font, char code, mutable output rows, outlinelen, prev_width, smush mode,
RTL flag, and outlinelen_limit. Returns `bool` (true if char added, false
if limit exceeded). Saves and restores `prev_width` on failure. LTR: builds
char on right side of output with kerning/smushing overlap. RTL: builds char
on left side with reversed smush dominance. Post-loop updates `outlinelen`
from `output_rows[0]` character count. Uses `#[allow(clippy::too_many_arguments)]`
(8 params, mirrors C's global-based approach). Uses iterator-style loops to
avoid `needless_range_loop` clippy lint. 9 tests: first-char, two-char kerning,
two-char smush, RTL smush, limit bail, prev_width restore, single-word
("Hi!"), multi-row, and boundary smush.
Two private helpers: `last_non_space()` (RTL scan for last non-space) and
`first_non_space()` (LTR scan for first non-space), each with fallback
position/char parameters matching C sentinel behavior (null terminator
for forward scans, position 0 for backward scans). Main function iterates
over row pairs, computes overlap between last non-space of output and
first non-space of current char, applies edge adjustment (boundary char
smush or space), and returns minimum across all rows. Handles LTR and RTL,
KERN-only and SMUSH modes. Uses `saturating_sub` for safe unsigned
arithmetic matching C signed-int boundary behavior. 9 unit tests covering
guards, LTR/RTL basics, row-min, boundary smush/no-smush, and all-spaces
edge cases.

### 1.2.5 — Output line printing

Added `Justification` enum (`Left`/`Center`/`Right`) with `from_i32()`
conversion matching C `justification` global (0/1/2). Added `render_line()`
in `render.rs` — port of C `putstring()`/`printline()` figlet.c:1553-1610.
Processes each row: (1) replace hardblank with space, (2) truncate to
`outputwidth - 1` if `outputwidth > 1`, (3) prepend spaces for Center/Right
per C formula. Center formula: `2*i + len - 1 < outputwidth`. Right formula:
`i + len < outputwidth`. No `clearline()` port — Rust returns fresh
`Vec<String>` each call; caller manages lifecycle. 13 tests: hardblank
replacement, left/center/right justification, width truncation, truncation
with center, `outputwidth <= 1` bypass, multi-row, C formula trace tests,
hardblank+truncation combination, zero outputwidth, empty rows.

### 1.2.6 — Line breaking and word splitting

Added `split_line()` in `render.rs` — port of C `splitline()` (figlet.c:1623-1658).
Scans char_buffer backward for last run of consecutive spaces, splits into
part1 (before space run) and part2 (after). Rebuilds both parts from scratch
via `add_char()` calls, matching C's clearline->addchar->printline->addchar
sequence. Returns `Option<(Vec<String>, usize)>` — part1 rows to caller and
part2_start index for caller to truncate its buffer. `output_rows` is mutated
in-place to contain only part2. Eight tests: basic multiword split, multiple
spaces consumed, no-word-break (None), single char after space, leading spaces
consumed, all-spaces buffer, multi-row font, empty buffer. All 8 tests use
`build_expected()` helper that calls `add_char()` independently to verify
part1/part2 output consistency.

### 1.2.7 — Phase merge: release/1.2 → master

Merged all Phase 1.2 work into default branch (master). Phase 1.2 complete:
character lookup + width calculation, smushing rules engine (all 6 horizontal
+ 5 vertical rules), smush amount calculation, character addition with
smushing, output line printing with justification, line breaking and word
splitting. All 6 subtasks (1.2.1–1.2.6) implemented, tested, merged.
Phase 1.3 (CLI Interface) is next.
