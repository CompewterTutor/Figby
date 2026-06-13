# Figby — Memory Index

Master memory index. Detailed entries live in versioned files below.

## Versioned Memory Files

| Milestone | File | Status |
|-----------|------|--------|
| v1 — Port | [memory-v1.md](memory-v1.md) | Active |
| v2 — Templates, Images & TUI | [memory-v2.md](memory-v2.md) | Active |

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

### 1.6.5 — Fix rendering pipeline bug

Identified and fixed root cause of `wordbreakmode` condition mismatch in
space-char failure path of main event loop. C figlet uses `wordbreakmode == 2`,
Rust used `wordbreakmode >= 2`. When `wordbreakmode == 3` (after a space, now
in a word), a failing space causes `printline()` (simple flush), not
`splitline()`. The `>= 2` check incorrectly called `split_line()` which
prematurely split on word boundaries, causing output divergence for
multi-line stdin input at default terminal width.

Additional fixes:
- `char_buffer.truncate(part2_start)` → `char_buffer.drain(..part2_start)` —
  `split_line` returns start index of part2, not length of part1
- Font parser: `String::from_utf8` → `String::from_utf8_lossy` for non-UTF-8
  font bytes (bubble font had embedded 0xFF bytes)

Status: 17/27 integration tests pass (was 4/27 before fixes). Remaining 10
failures involve RTL, TLF fonts, paragraph mode — separate issues from the
wordbreakmode bug.

Phase 1.7 (Major Release: end-to-end verification + RC) is next.

### 2.0.2 — Port make-examples script to CLI

Created `scripts/make-examples.sh` — POSIX shell script that generates
example output for every font file in `fonts/`. Supports `--sample-text`
(default `"hello figby"`), `--fonts` (comma-separated whitelist),
`--exclude` (comma-separated blacklist), and `--categories` (parsed but
deferred). Resolves `figby` binary via PATH, `figby-rs/target/debug/figby`,
or builds if missing. Uses `-d fonts/` flag so font resolution works from
repo root. Output goes to `examples/` with a `.gitkeep` sentinel file.

### 2.0.4 — Repo cleanup: move C source to c-figlet/

Moved all C FIGlet 2.2.5 source files (`figlet.c`, `chkfont.c`, `inflate.c`,
`zipio.c`, `utf8.c`, `getopt.c`, `crc.c`, headers, `Makefile*`) from root into
`c-figlet/`. Removed stale `.o` build artifacts. Updated references in README.md,
AGENTS.md, skills/ralph.md, .travis.yml, snapcraft.yaml, and run-tests.sh.
Root is now clean of loose C source files.

### 2.0.3 — Update README with proper documentation

Complete README rewrite covering: what Figby is, installation methods
(cargo, build from git, pre-built, package managers), CLI usage with
all 27 flags and examples, font directory setup and resolution order,
getting fonts (bundled + external sources), comparison with C FIGlet
(feature parity table), contributing guide with setup and quality gates,
project status with v1/v2 milestone references, roadmap, and license.
`cargo fmt --check` and `cargo clippy` pass clean.

### 2.0.8 — `--to-file` output flag (CLI arg only, no-op)

Added `--to-file <path>` long flag to `CliArgs` struct, `to_file: Option<String>` field
on `CliConfig` with default `None`, assignment in `from_args()`. No file I/O — deferred
to 2.1. One parse test (`test_flag_to_file`). `cargo fmt` and `cargo clippy` pass clean.

### 2.0.9 — Builtin template functions: date + repo-data (syntax + reserve)

Added `TemplateBuiltin` enum with `Date(String)` and `RepoData(String)` variants
to `template.rs`. Added `builtin: Option<TemplateBuiltin>` field to `Layer` struct
(default `None`). `parse_ftmp()` recognizes `{{date:format}}` and
`{{repo-data:field}}` tags before the variables lookup. `render_template()` skips
builtin layers with `continue` (no-op, deferred to 2.1). No `.unwrap()` in
production — all new code uses proper Option handling. fmt and clippy pass clean.

### 2.0.10 — Phase merge: release/2.0 → master

Merged all Phase 2.0 work into default branch (master). Phase 2.0 complete:
CLI `--help` output (2.0.1), make-examples.sh (2.0.2), comprehensive README
(2.0.3), repo cleanup / C source relocation (2.0.4), `.ftmp` template format
+ renderer (2.0.5), border and shadow rendering (2.0.7), `--to-file` CLI arg
(2.0.8), builtin template functions (2.0.9). All 10 subtasks (2.0.1–2.0.10)
implemented, tested, merged. Phase 2.1 (Image-to-ASCII Pipeline) is next.

Second merge (this commit) brings 3 post-initial-merge commits from `release/2.0`:
fix broken template tests, redesign `.ftmp` format (YAML frontmatter, defer to TUI),
add `assets/tui/icons.yaml` for Phase 2.2, renumber 2.2.5→2.2.6.

### 2.1.2 — Luminance-to-ASCII character mapping

Added ASCII art conversion pipeline in `image_input.rs`:
- `DEFAULT_CHAR_MAP` constant (` .-:=+*#%@`) — darkest to brightest
- `luminance_to_ascii()` — converts luminance matrix to ASCII string with
  bilinear resize to target width, auto aspect-ratio correction (0.5× height
  for terminal char ~2:1 aspect), and configurable char map
- `image_to_ascii()` — convenience wrapper: loads image, converts to ASCII,
  defaults to terminal width (80 fallback) and default char map
- `bilinear_resize()` (private) — bilinear interpolation for arbitrary
  width/height scaling
- `luminance_to_char()` (private) — maps u8 luminance to char via linear
  index into char_map: `idx = luminance * (len - 1) / 255`
- 22 new tests: luminance→char mapping (black/white/mid/custom/empty/single),
  bilinear resize (identity/upscale/downscale/empty/single-pixel), ASCII
  output (all-white/all-black/custom-map/empty/zero-width), image→ASCII
  integration (PNG/custom-map/width/nonexistent/temp-image)

No `.unwrap()` in production — all fallible paths return `Result` or handle
edge cases with early returns. Terminal width detection falls back to 80.

### 2.1.3 — Colored ASCII output (24-bit ANSI)

Added 24-bit ANSI color support in `image_input.rs`:
- `RgbPixel` type alias `(u8, u8, u8)` for RGB triples
- `load_rgb_matrix()` / `rgb_from_dynamic()` — load image preserving original color via `to_rgba8()`
- `apply_grayscale()` — in-place BT.709 luminance conversion on RGB matrix
- `apply_negative()` — in-place invert: `(255-r, 255-g, 255-b)`
- `bilinear_resize_rgb()` — bilinear interpolation on `&[Vec<RgbPixel>]`
- `ansi_color_code(r, g, b)` — returns `"\x1b[38;2;{r};{g};{b}m"`
- `ansi_reset_code()` — returns `"\x1b[0m"`
- `ImageColorConfig` struct — `colored`, `grayscale`, `negative`, `char_map`, `target_width`
- `color_matrix_to_ascii()` — resizes, applies transforms, wraps chars in ANSI codes
- `image_to_colored_ascii()` — convenience wrapper loading image with config
- 10 new tests: RGB load, pixel preservation, grayscale in-place, negative in-place,
  ANSI format, reset code, colored output, grayscale flag, negative flag, bilinear resize RGB

No `.unwrap()` in production — all fallible paths return `Result`.
fmt and clippy pass clean.

### 2.1.4 — Braille art + dithering

Added braille art pipeline in `image_input.rs`:
- `BRAILLE_BASE` constant (U+2800) — Unicode braille starting codepoint
- `pixels_to_braille_char()` — maps 2×4 pixel block to single braille char via 8-dot bit ordering
- `floyd_steinberg_dither()` — error diffusion dithering with 7/16, 3/16, 5/16, 1/16 fractions
- `luminance_to_braille()` — converts luminance matrix to braille string, with optional dithering
- `image_to_braille()` — convenience wrapper: load image → grayscale → braille
- 10 new tests: all-blank, all-filled, each dot individually, multiple blocks, partial/odd-sized, empty, dither binary output, no-dither vs dither output, file integration

`.expect()` in `pixels_to_braille_char` for `char::from_u32` is a safe invariant (bits=0..255, base=0x2800, code always in valid Unicode range), following existing codebase convention.
fmt and clippy pass clean.

### 2.1.5 — Image CLI flags integration

Added `ImageOptions` struct, image CLI flags (`--image`/`-i`, `--map`, `--braille`/`-b`,
`--color`, `--grayscale`, `--negative`, `--dither`, `--width`, `--height`, `--dimensions`,
`--flipX`, `--flipY`), image mode dispatch, and `run_image()` entry point to `main.rs`.
Flip helpers for luminance and RGB matrices reside in `main.rs` (private functions).
17 flag parse tests + 2 integration tests covering every flag, defaults, short aliases,
multiple paths, and mode detection. No `.unwrap()` in production — all error paths use
`match`/`continue`. fmt and clippy pass clean.

### 2.2.1 — System font enumeration via font-kit

Created `font_gen.rs` with system font enumeration using `font-kit` crate:
- `FontFamilyInfo` struct with `family: String` and `styles: Vec<String>`
- `FontGenError` enum wrapping `SelectionError` and `FontLoadingError`
- `list_system_fonts()` — enumerates all installed font families via `SystemSource::all_families()`, loads first handle per family via `select_family_by_name()`, extracts style descriptions from font properties
- `list_monospace_fonts()` — filters system fonts using name heuristic ("Mono" substring) + `Font::is_monospace()` check
- Private helpers: `describe_style()`, `family_is_monospace()`, `load_styles()`
- 3 unit tests: non-empty font list, monospace filter produces subset, styles are populated
- `font-kit = "0.14.3"` enabled in Cargo.toml (was commented out)

### 2.2.3 — FIGfont header from font metrics

Added `generate_figfont_header()` and `generate_figfont()` in `font_gen.rs`.
- `generate_figfont_header(font)` — generates FIGfont header line:
  `flf2a<hardblank> <height> <baseline> <max_length> 0 0 -1 <full_layout> 0`
  Always uses old_layout=0 (full-size), comment_lines=0, print_direction=-1,
  codetag_count=0. Uses `format!` macro (infallible, no unwrap).
- `generate_figfont(font)` — generates complete `.flf` content: header + 95 ASCII
  chars (32-126) + 7 Deutsch chars + codetagged chars. Missing required chars use
  space-padded rows of `maxlength` width. Each row terminated with `@` endmark.
  Codetagged chars sorted by code for deterministic output.
- 5 new tests: header round-trip, default full-size layout, smush layout
  preservation (191), hardblank multi-byte (DEL), full font round-trip with
  placeholder chars. No test failures from font-kit (tests use `parse_header`
  and `parse_tlf_font` directly, no system font dependency).

### 2.2.4 — CLI command: `--create-font`

Added `system_font_to_figfont()` in `font_gen.rs`:
- Loads system font by name via font-kit, renders all 102 required chars
  (32–126 + 7 Deutsch) to monochrome bitmaps via `rasterize_glyph()`
- Converts bitmaps to FIGcharacter rows with correct baseline positioning
  using raster bounds origin_y for padding calculation
- Computes charheight/baseline from font metrics (ascent/descent in
  design units scaled by `point_size / units_per_em`)
- `FontGenError` gains `GlyphLoading(GlyphLoadingError)`, `FontNotFound(String)`,
  `NoGlyph(u32)` variants
- `pathfinder_geometry = "0.5"` added as direct dependency for
  `Transform2F` (needed by font-kit's `rasterize_glyph` API)

CLI integration in `main.rs`:
- `--create-font <name>` generates .flf from system font
- `--font-size <f32>` (default 12.0) controls pixel size
- `--output <path>` writes to file instead of stdout
- Handler placed before `-F` check, early return after generation

5 new tests: roundtrip metrics, parseable output, render known char,
nonexistent name error, size scaling.

### 2.2.5 — Create TUI iconset YAML file

Verified `assets/tui/icons.yaml` — 201 icon entries across 23 categories
(modes, tools, cursor, canvas, brush, palette, status, file ops, edit,
font editor, smushing rules, font transforms, image editor, text tool,
layers, blending, timeline, keyframes, export, settings, navigation,
dialogs, misc UI). Every entry uses `nf-*` Nerd Font icon prefix.

Added `serde_yaml` dev-dependency and integration test
`test_icons_yaml_all_keys_present` in `tests/tui.rs`:
- Compile-time embedded via `include_str!`
- Parses as `BTreeMap<String, String>`
- Asserts ≥120 entries
- Asserts every key non-empty
- Asserts every value starts with `nf-`

### 2.2.6 — Phase merge: release/2.2 → main

Merged all Phase 2.2 work into default branch (master). Phase 2.2 complete:
system font enumeration via font-kit (2.2.1), glyph rasterization to FIGcharacter
rows (2.2.2), FIGfont header generation from font metrics (2.2.3), `--create-font`
CLI command (2.2.4), TUI iconset YAML file (2.2.5). All 6 subtasks (2.2.1–2.2.6)
implemented, tested, merged. Phase 2.3 (TUI Core & Canvas) is next.

### 2.3.1 — TUI scaffold with ratatui

Created `figby-rs/src/tui.rs` — TUI scaffold with ratatui + crossterm:
- `AppMode` enum: `FontEditor`, `ImageEditor`, `AsciiPreview` with `title()` and `next()` cycling
- `TuiApp` struct: holds mode state, quit flag, icons map (from `icons.yaml`)
- `run()` — raw mode, alternate screen, event loop with render + event handling
- `render()` — vertical layout: toolbar (Tabs with 3 modes), main area (canvas + palette sidebar), status bar
- `handle_event()` / `handle_key_event()` — Tab cycles mode, q/Esc quits
- `--tui` CLI flag added to `main.rs` dispatches to TUI on startup
- `ratatui = "0.30.1"`, `crossterm = "0.28"` dependencies added; `serde_yaml` promoted to regular dep
- 3 smoke tests: all panels render, mode switching cycles correctly, default mode is FontEditor
- No `.unwrap()` in production — `serde_yaml::from_str` uses `unwrap_or_default()` for graceful fallback

### 2.3.2 — Toolbox bar

Created `figby-rs/src/tui/toolbox.rs` — shared toolbar with 10 tool variants:
- `Tool` enum: Brush, Marquee, Lasso, CircleSelect, PolygonSelect, Fill, Line, Eraser, Eyedropper, Text
- Each tool has `display_name()` (2-char label), `full_name()`, `key_shortcut()` (KeyCode), `icon_key()` (icons.yaml lookup)
- `Toolbox` struct wraps `selected: Tool` with `handle_key()`, `next()`, `prev()`, `render()`
- Keyboard shortcuts: V(select), B(brush), L(lasso), C(circle), P(polygon), G(fill), I(line), E(eraser), D(eyedropper), T(text)
- Active tool highlighted via `List` widget with cyan bold style
- Converted `tui.rs` → `tui/mod.rs` directory module for sub-module organization
- 3 tests: default tool is Brush, round-trip selection via all shortcuts, tool names appear in rendered output

### 2.3.3 — Canvas widget

Created `figby-rs/src/tui/canvas.rs` — scrollable/zoomable canvas widget:
- `CanvasCell` struct: `ch: char`, `fg: Option<Color>`, `bg: Option<Color>` with `Default` (space, no color)
- `CanvasBuffer` struct: 2D grid of `CanvasCell` with bounds-checked `get()`, `get_mut()`, `set()`. No `unwrap()` — all bounds errors return `Option`.
- `CanvasWidget` struct: owns `CanvasBuffer`, cursor position `(u16, u16)`, scroll offset `(u16, u16)`, zoom level `u8` (1-8), grid toggle `bool`. `impl Widget for &CanvasWidget` renders buffer cells into terminal area:
  - At zoom=1, each buffer cell = 1 terminal cell
  - At zoom=N, each buffer cell fills N×N block with its char
  - Grid overlay with `│`/`─`/`┼` at cell boundaries (dim style)
  - Cursor highlight via reversed style (rendered last to win over grid)
- `handle_key()` dispatches arrows (move cursor), `+`/`=` (zoom in), `-`/`_` (zoom out), `G` (toggle grid). Returns `bool` (handled).
- `ensure_cursor_visible()` auto-scrolls to keep cursor in view.
- Canvas placed before toolbox in key dispatch order.
- 6 integration tests: empty render, cell rendering, cursor movement, zoom in/out, cursor highlight style, grid characters at zoom=2.
- Memory entry on `Buffer::cell_mut` returning `Option<&mut Cell>` (non-panicking, matches invariants).

### 2.3.4 — Color palette

Created `figby-rs/src/tui/palette.rs` — color palette sidebar widget:
- `ColorTarget` enum: `Foreground`/`Background` with `toggle()` method
- `ANSI_16_COLORS` constant: 16 standard indexed colors (0-15)
- `extended_color()` helper: computes 240-color extended grid via page/offset → `Color::Indexed(idx.min(255))`
- `Palette` struct: owns target, selected_color, recent colors (max 8), selected_index, custom_hex input, extended mode/page state
- Keyboard: arrows navigate grid, Enter selects, `x`/`X` toggles FG/BG, `f`/`F` sets FG, `h`/`H` enters hex mode, `z`/`Z` toggles extended grid
- `set_custom_hex()` — parses `#RRGGBB` string via `u8::from_str_radix`, returns `bool` on success
- `apply_to_cell()` — applies selected color to `CanvasCell.fg` or `.bg` based on target
- `render()` — renders FG/BG indicator, color swatches (2 rows of 8), hex display, recent colors strip
- `push_recent()` — deduplicates and rotates recent colors, capped at 8
- Registered as `palette` module in `tui/mod.rs`, added `palette: Palette` field to `TuiApp`
- 8 integration tests in `tests/tui.rs`: default target, FG/BG toggle, selection, recent push, hex apply, apply to cell (fg/bg), render labels

### 2.3.5 — Brush selection

Created `figby-rs/src/tui/brush.rs` — brush shape picker and size controls:
- `BrushShape` enum: Square, Circle, SprayPaint, Custom with `cycle()` method
- `BrushState` struct: `shape: BrushShape`, `size: u8` (1..=20, clamped), `set_size()`,
  `size_up()`, `size_down()`, `cycle_shape()`
- `render_preview(max_size)` returns `Vec<String>` showing brush tip at current size
- `render()` ratatui widget: shows shape name, size, and preview in toolbox column
- Integrated into `TuiApp`: `brush` field, key events (`[` size down, `]` size up,
  `'` cycle shape), preview rendered below toolbox
- Status bar updated to show current brush shape and size
- No `.unwrap()` in production — all paths use proper Option/clamp arithmetic
- SprayPaint uses fixed seed 42 for deterministic output across test runs
- fmt and clippy pass clean

### 2.3.6 — Status bar + canvas settings

Created `figby-rs/src/tui/status.rs` with two widgets:
- `StatusBar` — renders cursor X,Y, zoom level, current tool name, mode name,
  unsaved indicator using Nerd Font icons from `icons.yaml`. Static `render()`
  method takes all display data as parameters (no stored state).
- `CanvasSettings` struct — settings panel with canvas width/height, font size,
  grid toggle, snap-to-grid toggle. `pub settings_open: bool` controls visibility.
  `handle_key()` navigates fields via ↑/↓/←/→, toggles booleans via Enter, closes
  via Esc. `render()` shows labeled fields with highlighted selection.

Integrated into `TuiApp`:
- `unsaved: bool` field (default `false`), `settings: CanvasSettings` field
- Status bar constraint changed from `Length(1)` to `Length(3)` (needs room for
  borders + 1 content line)
- Settings panel replaces palette sidebar when `settings_open` is true
- `S` key opens/closes settings, loading canvas state on open
- `apply_settings()` syncs canvas width/height/grid on each settings key event
- Settings mode blocks all other key handlers (canvas, toolbox, palette)
- `apply_settings()` — recreates canvas widget when dimensions change, toggles
  grid to match settings

10 integration tests covering all status bar fields (cursor, zoom, tool, mode,
unsaved indicator) and settings panel (toggle, width change, grid toggle,
snap-to-grid toggle). fmt and clippy pass clean.

### 2.4.1 — Brush tool

Added `tools/` subdirectory under `tui/` with `mod.rs` module root and `brush.rs`
execution module. Three core functions:
- `stamp_offsets()` — computes relative (dx, dy) offsets for Square, Circle,
  SprayPaint, and Custom brush shapes. Square fills N×N block, Circle uses
  euclidean distance ≤ radius, SprayPaint uses deterministic hash (seed 42, 35%
  density), Custom stamps only center cell.
- `paint_stamp()` — applies brush stamp at (cx, cy), clips to buffer bounds,
  no `unwrap()` in production (uses `get_mut` → `Option`).
- `paint_line()` — Bresenham line interpolation with per-step stamp calls.

Integrated into TUI:
- Mouse capture via `EnableMouseCapture`/`DisableMouseCapture` (crossterm `event`
  module, not `terminal`). Left-click places stamp, drag draws line, release
  resets drag origin.
- Keyboard painting: Space/Enter paints stamp at cursor when Brush tool active.
- `screen_to_buffer()` maps terminal coords to buffer coords using scroll/zoom.
- `canvas_inner_rect` tracks canvas rendering area for mouse→buffer conversion.
- `CanvasCell` gained `Copy` derive (all fields are Copy types).
- `CanvasWidget` gained `set_cursor()` and `scroll_offset()` methods.

14 unit tests: square coverage, circle shape, spray determinism, bounds clipping,
cell attributes, line directions (horizontal/vertical/diagonal/reverse), endpoint
clipping, size-1 square, custom-only-center.

### 2.3.7 — Phase merge: release/2.3 → main

Merged all Phase 2.3 work into default branch (master). Phase 2.3 complete:
TUI scaffold with ratatui (2.3.1), toolbox bar with tool selection (2.3.2),
scrollable/zoomable canvas widget (2.3.3), color palette sidebar (2.3.4),
brush shape picker with size/preview (2.3.5), status bar + canvas settings
panel (2.3.6). All 6 subtasks (2.3.1–2.3.6) implemented, tested, merged.
Phase 2.4 (Drawing Tools) is next.

### 2.4.6 — Eyedropper tool

Added `tools/eyedropper.rs` with `sample()` — bounds-checked cell lookup returning
`Option<CanvasCell>`. Integrated into TUI mouse handler: click samples cell char +
foreground color, sets `self.brush.ch` and `self.palette.selected_color`, pushes
color to recent colors, switches target to Foreground. `BrushState` gained `ch: char`
field (default `'\u{2588}'`) — all 6 hardcoded `ch: '\u{2588}'` in drawing tools
replaced with `self.brush.ch`. `Palette::push_recent` changed from `fn` to `pub fn`
to allow external call. Eyedropper excluded from keyboard paint (Space/Enter) and
mouse early-return. 5 unit tests: cell data, empty defaults, out-of-bounds,
no-foreground cell, char sampling. fmt and clippy pass clean.

### 2.4.7 — Spray paint brush

Added `tools/spray.rs` with stochastic spray stamp and Bresenham-spray line.
- `spray_stamp()` — iterates bounding box `[-radius, +radius]`, circle-check with
  `dx² + dy² ≤ r²`, paints with probability `density / 100.0` via `rand::Rng::gen_bool()`
- `spray_line()` — Bresenham interpolation calling spray_stamp at each step
- Uses `StdRng::seed_from_u64(thread_rng().gen())` for fresh randomness per click
  (different pattern each click); tests pass seeded `StdRng` for determinism
- `rand = "0.8"` added to Cargo.toml
- `BrushState` gained `density: u8` field (1–100, default 35), `set_density()`,
  `density_up()`, `density_down()` methods
- Density UI: `;` density down, `'` density up, brush shape cycle moved to `\`
  (was `'`), Settings `S` check moved before toolbox handler to avoid conflict
  with Spray tool shortcut `a` (aerosol)
- Spray preview in brush UI now reads `self.density` instead of hardcoded 35
- 6 tests: within-circle, density distribution (200 stamps @50% ±10%), stochastic
  different, deterministic seed, bounds clip, density 0/100 extremes

### 2.4.8 — Phase merge: release/2.4 → main

Merged all Phase 2.4 work into default branch (master). Phase 2.4 complete:
brush tool (2.4.1), eraser tool (2.4.2), line tool (2.4.3), fill/flood fill
tool (2.4.4), selection tools (2.4.5), eyedropper tool (2.4.6), spray paint
brush (2.4.7). All 7 subtasks implemented, tested, merged. Phase 2.5
(Font Editor Mode) is next.

### 2.5.1 — Font mode scaffold: glyph grid overview

Created `figby-rs/src/tui/font_editor.rs` with `FontEditor` struct:
- `FontEditorView` enum: `Overview` (char grid) or `CharEditor(u32)` (single char editing)
- Glyph grid renders all 102 required FIGcharacters (32-126 + 7 Deutsch) plus codetagged chars
- Each cell shows code label + mini FIGcharacter preview (cells sized by `maxlength × charheight+1`)
- Search/filter by char code or char value via `/` key activator
- Arrow keys navigate grid, Enter selects char → switches to `CharEditor` view
- Esc clears search or returns from `CharEditor` to `Overview`
- Font loaded at `TuiApp::new()` from `fonts/standard.flf` (graceful `None` on failure)
- `sync_font_char_to_canvas()` populates canvas with FIGcharacter rows on char selection
- Status bar shows `"Font Editor [U+XXXX]"` when editing a specific char
- No `.unwrap()` in production — font loading uses `if let Ok(font)`
- `/` key activates search (avoids conflict with tool shortcuts `b`,`v`,`l`, etc.)
- 7 integration tests: grid, search by code, search by char, select+open, Esc return, empty font, grid navigation

### 2.5.4 — Smushing rule configuration

Added `FontEditorView::SmushRuleEditor` variant with visual toggle grid for all 6
horizontal smushing rule bits (EQUAL_CHARS, UNDERSCORE, HIERARCHY, PAIR, BIGX,
HARDBLANK). `SMUSH_RULE_LABELS` constant maps rule names → `SmushMode` bit constants.
`smush_selected: usize` tracks cursor position in toggle list (wraps at bounds).
`render_smush_editor()` — bordered panel showing `[X]`/`[ ]` checkboxes with
reverse-highlight cursor, live preview of `'/' + '\\'` smush via `smush_horizontal()`,
and layout value/binary display.
`handle_key_smush_editor()` — Up/Down wrap-navigate, Enter/Space XOR-toggles rule bit
in `font.full_layout`, Esc returns to Overview. `'S'` key in overview handler opens
smush editor (overrides settings panel `'S'` in FontEditor mode because font_editor
handler runs first in `mod.rs`). 5 integration tests: open/close, single toggle,
multiple toggles with cumulative bitmask, navigation wrap, preview changes on toggle.

### 2.5.3 — FIGfont header / layout editor

Added `FontEditorView::HeaderEditor` variant with inline field editor for all 7
FIGfont header properties:
- `HEADER_FIELD_LABELS` constant lists field names: Hardblank, Char Height,
  Baseline, Max Length, Full Layout, Print Direction, Comment Lines
- `editing_field` / `edit_buffer` / `error_message` state for inline text input
- `enter_header_editor()` method switches view, resets cursor to field 0
- `render_header_editor()` — bordered panel showing all fields with highlight
  cursor, editing state (green bold), and error messages (red)
- `handle_key_header_editor()` — Up/Down nav, Enter toggles edit, Esc cancels
  or returns to Overview, chars/Backspace edit buffer. Validation: height≥1,
  baseline≤height, hardblank single char, print_direction ∈ {-1,0,1}
- `save_current_field()` parses and validates via `parse::<u32>()`/`parse::<i32>()`
- `'H'` key in overview opens header editor
- `mod.rs` render dispatch changed from `Overview`-only to `!CharEditor` so
  HeaderEditor routes to font_editor render
- 10 integration tests: open/close, all 7 field edits (charheight, baseline,
  hardblank, full_layout, print_direction, comment_lines, maxlength),
  rejection of height=0 and baseline>height

### 2.5.5 — Add/remove codetagged characters

Added `CodeInputMode` enum (Add, CopySource, CopyDest, DeleteConfirm) and
state fields (`code_input_active`, `code_input_buffer`, `copy_source_code`)
to `FontEditor`. Four core methods:
- `add_char(code)` — creates space-padded FIGcharacter, inserts into font,
  rebuilds `all_codes`, selects new char in grid
- `delete_char(code)` — removes from font, ensures code 0 (missing char)
  still exists, rebuilds `all_codes`
- `copy_char(src, dst)` — clones rows from src to dst (or creates space-
  padded default if src missing), rebuilds `all_codes`
- `ensure_missing_char()` — creates space-padded code 0 if absent

Rendering: code input prompt shown above grid when `code_input_active`.
Key handlers: `A` starts add flow, `D` starts delete confirm (Y/N prompt),
`C` starts two-step copy flow (source → destination). Digit entry for code,
Backspace/Enter/Esc for standard editing. Codepoint validation rejects
surrogates (0xD800-0xDFFF) and values > 0x10FFFF.
14 new unit tests covering all operations, edge cases, and buffer management.

### 2.5.6 — Font-level transform tools

Added `FontEditorView::TransformEditor` variant with 6 font-level transforms:
- **Resize**: changes `charheight`, adds/removes rows from all glyphs, clamps baseline, recalculates `maxlength`
- **Italicize**: prepends row-index spaces to each row of every glyph, recalculates `maxlength`
- **Bold**: duplicates every character in each row (doubles width), recalculates `maxlength`
- **Mirror**: 3 submodes — Horizontal (reverse each row), Vertical (reverse row order), Both (compose both)
- **Copy Glyph**: loads external FIGfont by name via `load_font()`, extracts glyph by code, inserts into current font
- **Rename**: updates `font_storage_name` (in-memory only, no file I/O)

Transform editor UI: navigable list (`↑`/`↓`), Enter activates, parameter input for Resize/CopyGlyph/Rename, submenu for Mirror. `T` key in overview opens transform editor. Transforms clear undo/redo stacks (bulk operations incompatible with per-char undo).

`MirrorMode` enum with cycle/prev/next navigation. `transform_copy_glyph_from()` accepts `fontdir` parameter for testability.
32 new unit tests + 6 new integration tests covering all transforms, empty-font safety, parameter input flow, and multi-transform consistency. Only `font_editor.rs` and `mod.rs` touched. fmt and clippy pass clean.

### 2.5.7 — Phase merge: release/2.5 → main

<<<<<<< HEAD
Merged all Phase 2.5 work into default branch (master). Phase 2.5 complete:
font mode scaffold with glyph grid overview (2.5.1), per-character canvas
editing with drawing tools + undo/redo (2.5.2), FIGfont header/layout editor
(2.5.3), smushing rule configuration (2.5.4), add/remove codetagged characters
(2.5.5), font-level transform tools (2.5.6). All 6 subtasks (2.5.1–2.5.6)
implemented, tested, merged. Phase 2.6 (Image Editor Mode) is next.
=======
- **Merge commit `b6d340f`** — release/2.5 merged into main.
- Phase 2.5 complete: all 7 subtasks (2.5.1–2.5.7) implemented, tested, merged.
- **Documentation**: `docs/todo-v2.md` task checked off; this memory entry added.
- **No code changes** — merge was performed externally; only doc state synced.
- **Next up**: Phase 2.6 — Image Editor Mode.
>>>>>>> release/2.5

### 2.6.1 — Image import + canvas display

Created `figby-rs/src/tui/image_editor.rs` with `ImageEditor` struct and `AsciiMode`
enum (Color/Grayscale). `ImageEditor` supports:
- Path entry via keyboard (`o` to open, type path, Enter to load, Esc to cancel)
- Image loading via `load_from_path()` using `image_input`'s `load_rgb_matrix()`
  and `bilinear_resize_rgb()` with target_width=80 and aspect-corrected 0.5× height
- Color mode: per-cell ANSI RGB colors stored in `CanvasCell.fg` via `ratatui::Color::Rgb`
- Grayscale mode: luminance-only chars with `None` foreground
- Mode toggle via `c`/`C` key, re-renders from cached `original_rgb` matrix
- Block title shows path entry buffer and error messages
- Status bar shows current mode (Color/Grayscale)

Integration in `tui/mod.rs`:
- `ImageEditor` field on `TuiApp`, initialized in `new()`
- `sync_image_to_canvas()` resizes `CanvasWidget` to match image cells dimensions
- Image editor key dispatch placed before canvas/tools in `handle_key_event()`
- Render sync in canvas rendering block alongside font editor sync

Made `bilinear_resize`, `bilinear_resize_rgb`, `luminance_to_char` public in
`image_input.rs`. 8 unit tests in `image_editor.rs`: grayscale load, color load,
nonexistent path error, mode toggle, CLI output match, canvas render, path entry
key handling, key mode toggle. fmt and clippy pass clean.

### 2.6.2 — Text tool with FIGlet font overlay

Created `tools/text.rs` with `TextToolState` struct:
- `TextToolState::new(font_dir)` — scans fonts/ for `.flf`/`.tlf` files, builds list
- `list_available_fonts(font_dir)` — reads directory, returns sorted deduplicated names
- `load_selected_font()` — loads FIGfont by name from current `font_index`, stores in `font: Option<FIGfont>`
- `render_text_to_buffer()` — uses `add_char()` pipeline to render `text_buffer` as FIGlet rows with kerning/smushing (font's `full_layout` mode), stamps non-space cells into `CanvasBuffer` at `cursor_position` with scale and color
- `render_options()` — Paragraph widget showing font name, justification (L/C/R), scale (1-4), color, text entry status

Integration in `tui/mod.rs`:
- `text_tool` field on `TuiApp`, initialized in `new("fonts")`
- Render conditionally swaps brush panel for text options when `Tool::Text` selected
- Mouse click sets `cursor_position` + enters text entry mode (`entering_text = true`)
- Key entry mode: letters/space/punctuation → buffer, Enter → render+clear+exit, Esc → cancel, Backspace → pop
- Non-entry mode: Up/Down → font navigation, j/J → justification cycle, +/- → scale, Space/Enter → enter text mode
- Font nav handled before canvas (prevents arrow conflicts), tool settings handled after canvas

14 unit tests: single char render, multi char, left/center/right justification, color apply, font switch, scale factor, edge clipping, empty text noop, no-font panic, entering text state, font listing nonempty and nonexistent. fmt and clippy pass clean.
