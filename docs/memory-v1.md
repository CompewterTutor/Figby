# Feiglet v1 — Port Memory

## Phase 1.1 Scaffold

### Crate Structure (1.1.1)
`feiglet-rs/` is the single crate in the workspace. Main binary + library
in one crate for simplicity. Library exposes `font`, `render`, `smush`,
`control`, `input` modules.

### Core Types (1.1.2)
- `FIGfont` owns all parsed font data
- `FCharnode` maps `u32` char code → `Vec<String>` rows
- `SmushMode` bitflags: `EQUAL_CHAR=1, UNDERSCORE=2, HIERARCHY=4, PAIR=8, BIGX=16, HARDBLANK=32`
- `LayoutMode` enum: `FullSize, Kerning, Smushing`
- Control commands: `Translate{range, offset}`, `Freeze`, `MultibyteMode`, `CharsetSelect`

### FIGcharacter Data Parser (1.1.4)
- `parse_char_data()` returns unconsumed slice of lines for codetag processing
- `strip_endmarks()` preserves trailing whitespace before endmarks — critical for width correctness
- Endmark = last non-whitespace char per `figlet.c:1155-1165`; removes all consecutive occurrences from right
- `DEUTSCH_CHARS` constant: `[196, 214, 220, 228, 246, 252, 223]` matching C array
- No `char 0` (missing char sentinel) parsing here — handled at render time via `lookup_char()` fallback (1.2.1)

### TLF Font Support (1.1.6)
- `FontFormat` enum (`Figfont`/`Tlf`) tracks font format variant
- `parse_header()` accepts `tlf2a` magic in addition to `flf2a`
- `parse_tlf_font()`: public entry point parses full TLF content (header → comments → char data → codetagged)
- TLF rows are UTF-8 — Rust's native `String` handles this without special decode
- Reuses `parse_char_data()`, `parse_codetagged()`, `strip_endmarks()` unchanged
- 5 tests: magic detection, full header fields, full font parse (102 chars), endmark stripping, invalid magic rejection

### Code-tagged FIGcharacter Parser (1.1.5)
- `parse_codetagged()` reads variable-length code-tagged chars after required chars
- `parse_codetag_integer()` mirrors C's `sscanf(fileline,"%li",&theord)` — handles `0x`/`0X` hex prefix
- Code `-1` is reserved/skipped (rows consumed, no insertion)
- Negative codes stored via two's complement (`code as u32`) matching C's `inchr` → `u32` map key
- Stops at first non-numeric line (end of section, no error)
- Truncated char data (tag + fewer rows than charheight) returns `FontError::ParseError`
- `parse_codetagged()` takes `&[String]` (the unconsumed slice from `parse_char_data()`)
- 12 unit tests: basic, skip -1, hex, negative, truncated, empty, count matching, endmarks, non-numeric stop, full integration flow

### Smushing Rules Engine (1.2.2)

Full smushing rules engine in `smush.rs`:
- `SmushMode` newtype over `u32` with `const` bitmask values matching FIGfont full_layout encoding
  - H1-H6 in lower 6 bits (values 1/2/4/8/16/32), SM_KERN=64, SM_SMUSH=128
  - V1-V5 in bits 8-12 (values 256/512/1024/2048/4096), V_FIT=8192, V_SMUSH=16384
- `smush_horizontal()` mirrors `figlet.c:smushem()` exactly: blank→other, width guard, kerning⇒None, universal overlap, H6→H5→H4→H3→H2→H1 cascade
- `smush_vertical()` implements V1-V5 rules (EQUAL, UNDERSCORE, HIERARCHY, LINE, SUPERSMUSH)
- Hardblank treated as space for vertical smushing per FIGfont spec
- Hierarchy helpers (`hierarchy_class`, `is_hierarchy_char`) shared between H3/V3
- `u32` newtype avoids `bitflags` crate dependency — no new Cargo.toml entries
- No `.unwrap()` in production — all fallible paths use `Option<char>`
- 34 unit tests covering every rule, edge cases (blanks, widths, kerning), universal overlap, RTL, vertical blank/hardblank semantics

### Phase 1.1 Merge (1.1.8)

Phase 1.1 complete — all 7 subtasks merged from `release/1.1` into `master`.
Phase 1.2 (render engine: kerning + smushing) begins.
- `release/1.1` branch contains all Phase 1.1 commits
- Default branch is `master` (not `main`) — task spec alias resolved

### Phase 1.2 Merge (1.2.7)

Phase 1.2 complete — all 6 subtasks merged from `release/1.2` into `master`.
- Render engine components: character lookup (`lookup_char`), smushing rules
  engine (6 horizontal + 5 vertical rules in `smush.rs`), smush amount
  calculation (`calc_smush_amount`), character addition with smushing
  (`add_char`), output line printing with justification (`render_line`),
  line breaking and word splitting (`split_line`)
- Phase 1.3 (CLI Interface) begins next

## Phase 1.3 CLI Interface

### 1.3.1 — CLI argument parsing

- `main.rs` rewritten: scaffold placeholder → full clap derive CLI parser
- `CliArgs` struct with `#[derive(Parser)]` + `#[command]` for program info
- `#[allow(non_snake_case)]` on struct due to uppercase flag collisions
  (`-L` vs `-l`, `-S` vs `-s`, `-W` vs `-w`, `-N` vs `-n`, `-F` vs `-f`,
  `-D` vs `-d`, `-C` vs `-c`, `-R` vs `-r` — eight conflicts)
- `SmushOverride` enum: `No=0`, `Yes=1`, `Force=2` matching C
  `SMO_NO`/`SMO_YES`/`SMO_FORCE`
- `CliConfig` struct holds 11 globals from task spec, `Default` impl matches C
- `CliConfig::from_args()` normalization:
  - Boolean flag groups: last-checked wins (e.g., `-s` overrides `-k` when both set)
  - `-m` mapping: `< -1`→override=No, `== -1`→mode=0, `== 0`→mode=64, `> 0`→`(val&63)|128`+override=Yes
  - `-A` + any positional → `cmdinput=true`
- `-F` handled in `main()` before config build: prints error, exits(1)
- `run()` is no-op placeholder (filled in 1.3.4)
- `#[arg(short = 'I')] infocode: Option<i32>` parsed but unused until 1.3.2
- `#[arg(short = 't')]` and `#[arg(short = 'v')]` parsed but unused until 1.3.3/1.3.2
- `#[arg(short = 'C')] controlfile: Option<String>` parsed but unused until 1.4.1
- 20 unit tests cover all flags, defaults, value flags, edge cases

## Phase 1.4 Control Files & Character Mapping

### 1.4.3 — ISO 2022 character set handling

- `CharReader` trait added to `control.rs` with `next()`/`unget()` methods —
  abstracts input reading for `iso2022()` to work with both `InputIter` and
  test `MockReader`
- `ControlState::iso2022()` (control.rs:64-218) port of C `iso2022()` (figlet.c:1745-1875):
  - Reads input bytes, detects ESC (0x1B) → adds 0x100 to next byte,
    detects ESC $ (0x124) → adds 0x200 to next byte
  - Switch cases: SO(0x0E)/SI(0x0F) → GL shift, SS2(0x8E/ESC N)/SS3(0x8F/ESC O) →
    temporary G2/G3 with save/restore, ESC letters for LS2/LS3/G2-3 into GR,
    ESC (/*/0+ → 94-char set designation (G0-G3), ESC -/./ → 96-char set (G1-G3),
    ESC $ (/*/0+ → 94x94 double-byte (G0-G3), deprecated ESC $ x fallback
  - GL zone (0x21-0x7E) and GR zone (0xA0-0xFF) processing with gndbl
    double-byte combining and gn offset ORing
- `InputIter` implements `CharReader` (main.rs:112-121)
- Main event loop: closure `next_char` dispatches to `control_state.iso2022()`
  when `config.multibyte == 0`, else raw `input.next()` — used for both
  main input and paragraph mode peek
- All `.unwrap_or(0)` calls match C's lenient second-byte read behavior
  (no EOF check on designation/combining read)
- 15 unit tests: SO/SI GL switch, 94-char set designation, 96-char set + GR,
  double-byte via ESC $, SS2 single invocation, LS2 permanent, GR invocation,
  ASCII reset via `ESC ( B`, EOF mid-escape, EOF mid-double-byte, ESC $ ( J
  (no B special case), gn state persistence, SS2 gl/gr restore, plain char
  passthrough, newline/space passthrough

### 1.4.4 — Phase merge: release/1.4 → main

Phase 1.4 complete — all 3 subtasks merged from `release/1.4` into `master`.
- Control file parser (1.4.1): `read_control()` port with translate, freeze,
  multibyte mode, ISO 2022 charset, and mapping table commands
- Character remapping (1.4.2): `remap_char()` with freeze-block semantics,
  range+offset matching C `handlemapping()`
- ISO 2022 handling (1.4.3): `iso2022()` with G0-G3 set designation, GL/GR
  shift, SS2/SS3 invocation, double-byte combining
- Phase 1.5 (Multi-byte Input) begins next

## Phase 1.5 Multi-byte Input

### 1.5.1 — UTF-8 input mode

- `read_utf8_char()` added to `input.rs` — port of C `getinchr()` case 2
  (UTF-8 decoder). Signature: `pub fn read_utf8_char(input: &mut impl CharReader) -> Option<u32>`
- Decodes 1-6 byte UTF-8 sequences using `std::str::from_utf8` for validation
- Error sentinel: `0x0080` for invalid sequences (matching C)
- Validation rules:
  - Overlong sequences (0xC0, 0xC1) rejected by `from_utf8`
  - Surrogate halves (0xD800-0xDFFF) rejected by `from_utf8`
  - 0xFE/0xFF caught by leading byte pattern check
  - 0xF5+ values decode to codepoints > U+10FFFF, rejected by `from_utf8`
  - Bad/missing continuation bytes handled by explicit check (`b & 0xC0 == 0x80`)
- Wired into main event loop in `main.rs`: `config.multibyte == 2` → `input::read_utf8_char(input)`
  inserted before the generic `input.next()` fallback
- No `.unwrap()` in production — `from_utf8`'s `Ok` arm uses `.map()` for safe char extraction
- 12 tests: ASCII, 2/3/4 byte valid sequences, overlong C0/C1, surrogate, invalid
  lead bytes (0xFE/0xFF), F5+ codepoints, truncated sequences, bad continuation,
  EOF on first byte, multiple mixed chars
