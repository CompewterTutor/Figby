# Figby — Memory Index

Master memory index. Detailed entries live in versioned files below.

## Versioned Memory Files

| Milestone | File | Status |
|-----------|------|--------|
| v1 — Port | [memory-v1.md](memory-v1.md) | Active |

## Architectural Decisions

### UTF-8 Native Encoding
Figby uses Rust `char`/`String` natively (UTF-8), not `wchar_t`.
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
### 1.1.1 — Create `figby` crate in workspace

Added `[lib]` section to `figby-rs/Cargo.toml` (name=figby, path=src/lib.rs,
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

### 1.3.1 — CLI argument parsing

Rewrote `main.rs` from scaffold to full CLI parser using clap derive.
`CliArgs` struct parses all 27 FIGlet flags (`-A` through `-F`) plus
positional message. `CliConfig` holds all 11 globals (`smushmode`,
`smushoverride`, `justification`, `right2left`, `paragraphflag`,
`deutschflag`, `cmdinput`, `outputwidth`, `fontdirname`, `fontname`,
`multibyte`). `SmushOverride` enum mirrors C `SMO_NO`/`SMO_YES`/`SMO_FORCE`.
Flag normalization matches C switch-case semantics: `-m` value mapping,
`-s`/`-k`/`-S`/`-o`/`-W` smush overrides, `-x`/`-l`/`-c`/`-r` justification,
`-X`/`-L`/`-R` text direction, `-D`/`-E` deutsch, `-p`/`-n` paragraph,
`-A`/positional cmdinput. `-F` prints error and exits. `run()` is a
placeholder for 1.3.4. `#[allow(non_snake_case)]` needed on `CliArgs`
struct because uppercase flags (e.g. `-L` vs `-l`, `-S` vs `-s`) would
collide in snake_case. 20 unit tests covering every flag + defaults.

### 1.3.2 — Info codes (`-I` flag)

Added `printinfo()` and `printusage()` functions ported from C. Six infocodes
mapped exactly: 0 (copyright + usage), 1 (version int), 2 (font dir), 3 (font),
4 (output width), 5 (font formats). Both functions take `&mut impl Write` for
testability — production passes `io::stdout().lock()`, tests pass `Vec<u8>`.
Constants added: `VERSION_INT=20205`, `VERSION="2.2.5"`, `DATE="31 May 2012"`,
`FONTFILE_MAGIC="flf2"`, `TOILETFILE_MAGIC="tlf2"`. `-I` handler in `main()`
prints info and exits with code 0, matching C behavior. 6 unit tests covering
all infocodes (0–5) with byte-exact output assertions.

### 1.3.3 — Terminal width detection (`-t`)

Added `get_columns()` using `termion::terminal_size()` — wraps `ioctl(TIOCGWINSZ)`
on Unix, returns `Option<u16>`. `-t` flag in `CliConfig::from_args()` calls
`get_columns()` and sets `outputwidth` to detected width if successful. `-t`
handling placed before `-w` so explicit `-w` flag overrides `-t` when both given.
`termion = "4"` added to `Cargo.toml`. No `unwrap()` in production — returns
`None` gracefully on non-TTY or error, falling back to default 80. 3 unit tests:
parsing, width update, `-w` override, `get_columns()` never panics.

### 1.3.4 — Main event loop

Full `run()` implementation porting C's `main()` loop (figlet.c:2003-2134).
`InputIter` enum handles both stdin and argv-based input (`Agetchar` equivalent)
with `unget` support for paragraph mode peeking. `flush_output_line()` helper
calls `render_line()` then writes rows to stdout, then clears output state.

Main loop matches C exactly:
- Paragraph mode (`-p`): peek at next char after `\n`, map to space if non-ws
- `last_was_eol_flag` tracking for `\n`/`\v`/`\f`/`\r` (not tab/space)
- Deutsch re-routing (`-D`): `[\]` → umlauts, `{|}~` → lowercase umlauts+ß
- `handlemapping()` identity (Phase 1.4.2 will integrate control files)
- Space normalization: tab/space→space, other ws→newline
- Control char skip: 1-31 (except `\n`) and 127 (DEL)
- Inner retry loop matching C's `do {} while (char_not_added)`:
  - `wordbreakmode == -1`: absorb spaces/newlines after forced break
  - `c == '\n'`: flush line unconditionally
  - `addchar` success: track wordbreakmode (0/1/2/3 per C spec)
  - `addchar` fail + `outlinelen == 0`: raw-char path (print char directly)
  - `addchar` fail + `c == ' '`: split (if wordbreakmode>=2) or flush, then absorb space
  - `addchar` fail + else: split/flush, set wordbreakmode, retry
- EOF flush: if `outlinelen != 0`, flush remaining line

`DEUTSCH_CHARS` visibility changed from `pub(crate)` to `pub` (needed by binary crate).
`io::Read` import added to `main.rs` for `BufReader::bytes()`.
12 unit tests for `InputIter`: empty args, single/multi word, empty words, unget.

### 1.3.5 — Phase merge: release/1.3 → master

Merged all Phase 1.3 work into default branch (master). Phase 1.3 complete:
CLI argument parsing (all 27 FIGlet flags), info codes (-I flag with 6 codes),
terminal width detection (-t flag via termion), main event loop with full
FIGlet 2.2.5 input processing pipeline. All 4 subtasks (1.3.1–1.3.4)
implemented, tested, merged. Phase 1.4 (Control Files & Character Mapping)
is next.

### 1.4.1 — Control file parser

Added `ControlCommand`, `ControlState`, `ControlError` types and `read_control()`
function in `control.rs`. Byte-level parser mirrors C `readcontrol()`:
- `t` (translate) with single char, range (`-` separator), and escape sequences
- Mapping table entries (lines starting with `0-9` or `-`)
- `f` (freeze) command
- `b`/`u`/`h`/`j` (multibyte modes) set `state.multibyte`
- `g` (ISO 2022 charset) with `charset_define()` for G0-G3,
  `gl`/`gr` selection, `96`/`94`/`94x94` char set variants
- `#` comments and blank lines silently consumed
- `ByteReader` wrapper struct provides multi-byte unget/pushback matching
  C `Zgetc`/`Zungetc` pattern
- 35 unit tests covering all command types, escape sequences, numeric
  formats (decimal, octal, hex), CRLF handling, fixture file parsing

### 1.4.2 — Character remapping via control files

Added `remap_char()` in `control.rs` — port of C `handlemapping()`. Iterates
`ControlState.commands` sequentially. For each translate command (thecommand=1),
checks if char `c` is in `[rangelo, rangehi]` range. On match: applies offset
via `wrapping_add` (matching C's signed `inchr` overflow semantics), then skips
remaining translate commands in current block (stops at freeze or end). Freeze
commands (thecommand=0) act as block boundaries — only first matching translate
within each block applies. Multiple blocks apply sequentially across freeze
boundaries. `remap_char()` returns modified char; identity if no match.

Wired into `run()` in `main.rs`: `-C` flag loads control file via `read_control()`
into `ControlState` after font loading. `remap_char()` called in main event loop
after Deutsch re-routing, replacing placeholder identity. Added `controlfile:
Option<String>` to `CliConfig`.

14 unit tests: empty commands, single char, range, no match, negative offset,
out of range, freeze-block skip (second match in same block skipped),
two-block sequential apply, three-block chain, mapping table entry, upper.flc
fixture (a→A, z→Z). All tests define state via `build_remap_state()` helper
that reads control file content then runs `remap_char`. No `.unwrap()` in
production — error handling via `Result` + `process::exit(1)` on control file
load failure.

### 1.4.3 — ISO 2022 character set handling

Added `CharReader` trait with `next()`/`unget()` methods to `control.rs` —
abstracts input for `iso2022()` to work with both `InputIter` and test
`MockReader`. `ControlState::iso2022()` port of C `iso2022()` (figlet.c:1745-1875):
processes ESC sequences for G0-G3 set designation, SO/SI GL shift, SS2/SS3
temporary invocation, LS2/LS3 permanent, GL/GR zone processing with double-byte
combining. `InputIter` implements `CharReader`. Main event loop dispatches via
`config.multibyte == 0` to `control_state.iso2022()`. All `.unwrap_or(0)` calls
match C's lenient second-byte read behavior. 15 unit tests covering all ISO 2022
escape types, edge cases, plain passthrough.

### 1.4.4 — Phase merge: release/1.4 → main

Merged all Phase 1.4 work into default branch (master). Phase 1.4 complete:
control file parser (`.flc` parsing with translate, freeze, multibyte mode,
ISO 2022 charset commands), character remapping via `remap_char()` with freeze-
block semantics, ISO 2022 character set handling with G0-G3/GL/GR/SS2/SS3.
All 3 subtasks (1.4.1–1.4.3) implemented, tested, merged. Phase 1.5
(Multi-byte Input) is next.

### 1.5.1 — UTF-8 input mode

Added `read_utf8_char()` in `input.rs` — port of C `getinchr()` case 2
(UTF-8 decoder). Decodes 1-6 byte sequences using `std::str::from_utf8`
for validation with explicit continuation byte checks. Error sentinel
`0x0080` for invalid sequences (matching C). Wired into main event loop
via `config.multibyte == 2` dispatch. Initial leading byte dispatch
uses bitmask matching. Continuation bytes validated (`b & 0xC0 == 0x80`).
No `.unwrap()` in production — `from_utf8` error path returns `Some(0x0080)`.
12 unit tests covering ASCII, 2/3/4 byte valid, overlong C0/C1, surrogate,
invalid lead bytes (0xFE/0xFF), F5+ codepoints, truncated sequences, bad
continuation, EOF on first byte, multiple mixed chars.

### 1.5.2 — DBCS, HZ, Shift-JIS input modes

Added `read_dbcs_char()` in `input.rs` — port of C `getinchr()` cases 1/4
(DBCS/SJIS). Lead byte 0x80-0x9F or 0xE0-0xEF combines with trail byte as
`(lead << 8) | trail`. Non-lead bytes pass through. EOF after lead byte
returns lead byte alone. `HZState` struct tracks HZ escape mode (`~{` enter,
`}~` leave, `~~` = tilde). `read_hz_char()` uses recursive approach:
`~{` sets mode + recurses, `}~` clears mode + recurses, `~~` returns `~`,
`~x` skips + recurses. Wired into main event loop via `config.multibyte`
match: modes 1/4 → `read_dbcs_char`, mode 3 → `read_hz_char`. `HZState`
passed as third parameter to `next_char` closure. 14 unit tests: 5 DBCS,
9 HZ covering all edge cases. `HZState` derives `Default`.

### 1.5.3 — Deutsch flag character re-routing

Added `deutsch_reroute()` in `input.rs` — port of C's inline deutsch
re-routing logic. Refactored from `main.rs` into a standalone function
for testability. Maps `[\]` (0x5B-0x5D) to Ä/Ö/Ü (196/214/220) and
`{|}~` (0x7B-0x7E) to ä/ö/ü/ß (228/246/252/223). Uses `DEUTSCH_CHARS`
constant from `font.rs`. Called before `remap_char()` in main event loop.
9 unit tests covering all 7 mappings, disabled flag, and out-of-range.

### 1.5.4 — Phase merge: release/1.5 → master

Merged all Phase 1.5 work into default branch (master). Phase 1.5 complete:
UTF-8 input mode (1.5.1), DBCS/HZ/Shift-JIS input modes (1.5.2), Deutsch
flag character re-routing (1.5.3). All 3 subtasks implemented, tested, merged.
Phase 1.6 (Test Suite & Verification) is next.

### 1.6.2 — Font fuzz testing

Added property-based fuzz tests for font parser in `tests/fuzz.rs` using `proptest`.
All 4 public parser functions (`parse_header`, `parse_tlf_font`, `parse_char_data`,
`parse_codetagged`) exercised with random malformed strings — no panics, only
`Result` returns. Height bounds prevent infinite loops at height=0. `proptest`
dev-dependency added.

### 1.6.3 — Rename project: Feiglet → Figby

Renamed every instance of `Feiglet`/`feiglet` to `Figby`/`figby` across the
entire repository. Includes: `figby-rs/` directory rename, Cargo package name,
CLI command name (`figby`), lib name, module imports (`use figby::...`),
all documentation files, scripts, and skills. Version 1.6.3 task added to
todo-v1.md with renumbering of subsequent tasks (benchmarks → 1.6.4, phase
merge → 1.6.5). Build, fmt, clippy, and all 273 tests pass clean.

### 1.6.4 — Performance benchmarks

Added Criterion benchmark suite in `figby-rs/benches/render_bench.rs` with 9 benches:
font_load, lookup_char (1000x), smush_horizontal (10000x), calc_smush_amount (1000x),
add_char_kerning (100x "Hi World"), add_char_smushing (100x), render_line (4
justifications), split_line, full_pipeline (~5KB Lorem Ipsum text). Font loaded lazily
via `OnceLock` to avoid re-parsing. `criterion = "0.5"` dev-dependency added,
`[[bench]]` entry with `harness = false`. No C binary available for baseline
comparison — Rust baseline established; manual C comparison needed separately.
`target/criterion/` already covered by `target/` in `.gitignore`.

### 1.6.5 — Phase merge: release/1.6 → main

Merged all Phase 1.6 work into default branch (master). Phase 1.6 complete:
port of C test harness (27 test cases, 1.6.1), font fuzz testing via proptest
(1.6.2), project rename Feiglet→Figby including `figby-rs/` directory (1.6.3),
Criterion performance benchmarks (1.6.4). All 4 subtasks implemented, tested,
merged. Phase 1.7 (Major Release: end-to-end verification + RC) is next.
