# Feiglet v1 — C-to-Rust Port

Milestone goal: Feature-complete Rust port of FIGlet 2.2.5 supporting all
FIGfont (.flf) and TOIlet (.tlf) fonts with kerning, smushing, control files,
and multi-byte input.

---

## Phase 1.1 — Crate Scaffold & Font Parser

- [x] `1.1.1` Create `feiglet` crate in workspace
  - **Goal:** Rust crate `feiglet` added to workspace. Compiles clean.
  - **Touches:** `feiglet-rs/Cargo.toml`, `feiglet-rs/src/lib.rs`
  - **Success:** `cargo build -p feiglet` succeeds. Clippy clean.
  - **Tests:** Empty crate compiles.
  - **Difficulty:** Low

- [ ] `1.1.2` Define core types: FIGfont, FIGcharacter, FCharnode
  - **Goal:** Port C structs (`fcharnode`, `inchr`, `outchr`) to Rust types.
    `FIGfont` struct owns parsed font data. `FCharnode` maps char code to
    rows of sub-character strings. Use `Vec<Vec<&str>>` or `Vec<String>`.
    `Hardblank` tracked as `char`. `CharHeight`, `Baseline`, `MaxLength`,
    `OldLayout`, `FullLayout`, `PrintDirection`, `CommentLines` fields.
  - **Touches:** `feiglet-rs/src/font.rs`
  - **Success:** All types defined + documented. Round-trip serde tests.
  - **Tests:** Type construction + default tests.
  - **Difficulty:** Low

- [ ] `1.1.3` FIGfont magic number + header line parser
  - **Goal:** Parse `flf2a$ 6 5 20 15 3 0 143 229` header. Validate magic
    number (`flf2a`). Extract hardblank char, height, baseline, max_length,
    old_layout, comment_lines, print_direction, full_layout, codetag_count.
    Handle missing optional fields. Reject invalid magic.
  - **Touches:** `feiglet-rs/src/font.rs` — `parse_header()`
  - **Success:** Parse known headers correctly. Error on bad magic.
  - **Tests:** Fixture-based header parse tests. Error case tests.
  - **Difficulty:** Low

- [ ] `1.1.4` FIGcharacter data parser (required ASCII + Deutsch chars)
  - **Goal:** Read the 95 required ASCII FIGcharacters (codes 32-126) plus 7
    Deutsch chars (196, 214, 220, 228, 246, 252, 223). Remove trailing
    endmark characters (last block of identical chars per line). Store rows
    as `Vec<String>`. Handle `\r\n` and `\n` line endings.
  - **Touches:** `feiglet-rs/src/font.rs` — `parse_char_data()`
  - **Success:** All required chars parsed. Endmarks stripped. Widths consistent.
  - **Tests:** Parse known FIGfont fixture. Verify char count = 102.
  - **Difficulty:** Medium

- [ ] `1.1.5` Code-tagged FIGcharacter parser
  - **Goal:** After required chars, read variable-length code-tagged chars.
    Each has a numeric code tag line followed by height rows. Build
    `HashMap<inchr, FIGcharacter>`. Handle negative codes. Skip code -1
    (reserved).
  - **Touches:** `feiglet-rs/src/font.rs` — `parse_codetagged()`
  - **Success:** All codetagged chars parsed. Map complete.
  - **Tests:** Parse known FIGfont with codetagged chars. Count matches `codetag_count`.
  - **Difficulty:** Medium

- [ ] `1.1.6` TLF font support (TOIlet format)
  - **Goal:** Support `tlf2a` magic number. UTF-8 encoded rows instead of
    raw bytes. Shared parser infrastructure with FIGfont, differing only in
    magic check and row encoding.
  - **Touches:** `feiglet-rs/src/font.rs` — TLF detection + UTF-8 decode
  - **Success:** TLF font parses identically to C output.
  - **Tests:** Parse `emboss.tlf` fixture. Compare with C output.
  - **Difficulty:** Medium

- [ ] `1.1.7` Compressed font support (zip/deflate)
  - **Goal:** Read `.flf` files inside ZIP archives using `zip` crate.
    Implement `FIGopen()` equivalent: try font directory + suffix, then
    bare path. Fall back to ZIP reading. Use `flate2` for raw deflate if
    needed.
  - **Touches:** `feiglet-rs/src/font.rs`, `Cargo.toml` (add `zip`, `flate2`)
  - **Success:** Font loads from ZIP and from plain file.
  - **Tests:** ZIP font loading test.
  - **Difficulty:** Medium

- [ ] `1.1.8` Phase merge: release/1.1 → main
  - **Goal:** Phase review passes, branch merges cleanly.
  - **Touches:** CI, todo-v1.md checkboxes
  - **Success:** All 1.1.x tasks checked. Review approved.
  - **Difficulty:** Low

---

## Phase 1.2 — Render Engine (Kerning + Smushing)

- [ ] `1.2.1` Character lookup + width calculation
  - **Goal:** Implement `getletter()`: given inchr code, find the
    `FIGcharacter` in the font map. Return char rows + width
    (length of first row). Fall back to missing-char (code 0).
    Track `previouscharwidth`.
  - **Touches:** `feiglet-rs/src/render.rs` — `lookup_char()`
  - **Success:** Known chars resolve correctly. Unknown chars fall back.
  - **Tests:** Lookup tests for all required chars. Fallback test.
  - **Difficulty:** Low

- [ ] `1.2.2` Smushing rules engine
  - **Goal:** Port `smushem()` — all 6 horizontal + 5 vertical smushing
    rules. Enum-based rule selection. Return `Option<char>` (None = no
    smush). Handle universal smushing (no rules = overlap).
    Algorithm identical to C: blank→other, hardblank→hardblank,
    equal chars, underscore, hierarchy, pair, big X.
  - **Touches:** `feiglet-rs/src/smush.rs`
  - **Success:** All smushing rules produce identical output to C.
  - **Tests:** Unit test per rule. Golden output comparison.
  - **Difficulty:** Medium

- [ ] `1.2.3` Smush amount calculation
  - **Goal:** Port `smushamt()` — max overlap between current char and
    output line. For each row, find last non-space in output line and
    first non-space in current char. Minimum across all rows determines
    smush amount. Handle left-to-right and right-to-left.
  - **Touches:** `feiglet-rs/src/render.rs` — `calc_smush_amount()`
  - **Success:** Smush amount matches C for known inputs.
  - **Tests:** Known-fixture smush amount tests.
  - **Difficulty:** Medium

- [ ] `1.2.4` Character addition with smushing
  - **Goal:** Port `addchar()` — append char to output line. Apply smush
    amount. For overlapping columns, call `smushem()`. Handle RTL by
    building char on left side. Bail if `outlinelen` exceeds limit.
  - **Touches:** `feiglet-rs/src/render.rs` — `add_char()`
  - **Success:** Lines build correctly with kerning and smushing.
  - **Tests:** Single-word render test. Compare output to C.
  - **Difficulty:** Medium

- [ ] `1.2.5` Output line printing
  - **Goal:** Port `putstring()` / `printline()` — render output rows with
    justification (left/center/right). Replace hardblanks with spaces.
    Respect `outputwidth` for line truncation.
  - **Touches:** `feiglet-rs/src/render.rs` — `render_line()`
  - **Success:** Output matches C for simple cases.
  - **Tests:** Justification tests. Width limit tests.
  - **Difficulty:** Low

- [ ] `1.2.6` Line breaking and word splitting
  - **Goal:** Port `splitline()` + main loop logic — break lines at word
    boundaries. Handle paragraph mode (`-p`). Edge cases: char wider than
    outputwidth, multiple spaces, forced breaks.
  - **Touches:** `feiglet-rs/src/render.rs` — `split_line()`
  - **Success:** Multi-word text renders with correct line breaks.
  - **Tests:** Multi-word render tests. Compare to C output.
  - **Difficulty:** High

- [ ] `1.2.7` Phase merge: release/1.2 → main
  - **Difficulty:** Low

---

## Phase 1.3 — CLI Interface

- [ ] `1.3.1` CLI argument parsing (all FIGlet flags)
  - **Goal:** Parse all FIGlet 2.2.5 flags using `clap`:
    `-A`, `-D`, `-E`, `-X`, `-L`, `-R`, `-x`, `-l`, `-c`, `-r`, `-p`,
    `-n`, `-s`, `-k`, `-S`, `-o`, `-W`, `-t`, `-v`, `-I`, `-m`, `-w`,
    `-d`, `-f`, `-C`, `-N`, `-F` (error).
    Set globals: `smushmode`, `smushoverride`, `justification`,
    `right2left`, `paragraphflag`, `deutschflag`, `cmdinput`,
    `outputwidth`, `fontdirname`, `fontname`, `multibyte`.
  - **Touches:** `feiglet-rs/src/main.rs` — CLI struct + parse
  - **Success:** All flags parsed. Defaults match C. `-F` prints error.
  - **Tests:** Flag parse tests. Default value tests.
  - **Difficulty:** Low

- [ ] `1.3.2` Info codes (`-I` flag)
  - **Goal:** Implement `printinfo()`: infocode 0 (copyright), 1 (version),
    2 (fontdir), 3 (font), 4 (outputwidth), 5 (formats).
    Output format must match C exactly.
  - **Touches:** `feiglet-rs/src/main.rs`
  - **Success:** Info output matches C byte-for-byte.
  - **Tests:** All infocodes tested.
  - **Difficulty:** Low

- [ ] `1.3.3` Terminal width detection (`-t`)
  - **Goal:** Implement `get_columns()` using `termion` or `crossterm`.
    Fall back to `DEFAULTCOLUMNS` (80) if terminal unavailable.
  - **Touches:** `feiglet-rs/src/main.rs`, `Cargo.toml` (add `termion`)
  - **Success:** Terminal width detected. Falls back gracefully.
  - **Tests:** Mock terminal width test.
  - **Difficulty:** Low

- [ ] `1.3.4` Main event loop
  - **Goal:** Port `main()` loop — read chars via `getinchr()`, process
    through `handlemapping()` + Deutsch re-routing, build lines with
    `addchar()`, handle line breaking. End-of-file exits.
  - **Touches:** `feiglet-rs/src/main.rs` — `run()` function
  - **Success:** Full pipeline: input→font→render→output.
  - **Tests:** End-to-end CLI test with known input/output.
  - **Difficulty:** High

- [ ] `1.3.5` Phase merge: release/1.3 → main
  - **Difficulty:** Low

---

## Phase 1.4 — Control Files & Character Mapping

- [ ] `1.4.1` Control file parser
  - **Goal:** Port `readcontrol()` — parse `.flc` control files.
    Commands: `t` (translate), digits/mapping table entries,
    `f` (freeze), `b`/`u`/`h`/`j` (multibyte modes),
    `g` (ISO 2022 charset), `#` (comments).
    Build linked list of `comnode` commands.
  - **Touches:** `feiglet-rs/src/control.rs`
  - **Success:** All control file commands parsed correctly.
  - **Tests:** Parse each command type. Known .flc fixture tests.
  - **Difficulty:** Medium

- [ ] `1.4.2` Character remapping via control files
  - **Goal:** Port `handlemapping()` — iterate control file commands.
    Translate chars via range+offset. Freeze commands halt translates
    until next unfreeze. Sequential apply.
  - **Touches:** `feiglet-rs/src/control.rs` — `remap_char()`
  - **Success:** Mapped chars transform correctly. Freeze works.
  - **Tests:** Known mapping test cases from C test suite.
  - **Difficulty:** Medium

- [ ] `1.4.3` ISO 2022 character set handling
  - **Goal:** Port `iso2022()` — process ISO 2022 escape sequences.
    G0/G1/G2/G3 set selection, double-byte flag, GL/GR invocation.
    Port `charset()` for charset definition.
  - **Touches:** `feiglet-rs/src/control.rs` — `iso2022()`
  - **Success:** ISO 2022 sequences processed correctly.
  - **Tests:** Escape sequence tests.
  - **Difficulty:** High

- [ ] `1.4.4` Phase merge: release/1.4 → main
  - **Difficulty:** Low

---

## Phase 1.5 — Multi-byte Input

- [ ] `1.5.1` UTF-8 input mode
  - **Goal:** Port `getinchr()` case 2 — UTF-8 decoder. Handle 1-6 byte
    sequences. Validate: reject overlong sequences (0xC0/0xC1), reject
    surrogate halves (0xD800-0xDFFF), reject 0xFF/0xF5+. Map to `char`.
  - **Touches:** `feiglet-rs/src/input.rs`
  - **Success:** UTF-8 decoded correctly. Invalid sequences handled.
  - **Tests:** UTF-8 test vectors. Error case tests.
  - **Difficulty:** Low (use `std::str::from_utf8` or `char::from_u32`)

- [ ] `1.5.2` DBCS, HZ, Shift-JIS input modes
  - **Goal:** Port multibyte modes 1 (DBCS), 3 (HZ), 4 (Shift-JIS).
    DBCS: lead byte 0x80-0x9F/0xE0-0xEF + trail byte.
    HZ: `~{` enters, `}~` leaves, `~~` = tilde.
    Shift-JIS: same as DBCS byte ranges.
  - **Touches:** `feiglet-rs/src/input.rs`
  - **Success:** All multibyte modes produce correct inchr values.
  - **Tests:** Known-sequence tests per mode.
  - **Difficulty:** Medium

- [ ] `1.5.3` Deutsch flag (`-D`) character re-routing
  - **Goal:** Port deutsch re-routing: `[\]` → umlauted A/O/U,
    `{|}~` → lowercase umlauts + ess-zed. Applies before mapping.
  - **Touches:** `feiglet-rs/src/input.rs`
  - **Success:** Deutsch re-routing matches C.
  - **Tests:** Deutsch flag test cases.
  - **Difficulty:** Low

- [ ] `1.5.4` Phase merge: release/1.5 → main
  - **Difficulty:** Low

---

## Phase 1.6 — Test Suite & Verification

- [ ] `1.6.1` Port C test harness
  - **Goal:** Port `run-tests.sh` test cases to Rust. Each test: known
    input → expected output (from C). Verify byte-exact match.
  - **Touches:** `feiglet-rs/tests/`
  - **Success:** All 27 existing test cases pass.
  - **Tests:** All test cases from C suite.
  - **Difficulty:** Medium

- [ ] `1.6.2` Font fuzz testing
  - **Goal:** Fuzz font parser with malformed FIGfont files. No panics.
    Graceful error handling for all malformed inputs.
  - **Touches:** `feiglet-rs/tests/fuzz.rs`
  - **Success:** No panics on any malformed input.
  - **Tests:** `cargo fuzz` or property-based tests.
  - **Difficulty:** Medium

- [ ] `1.6.3` Performance benchmarks
  - **Goal:** Benchmark render pipeline. At minimum match C performance.
    Use `criterion` for regression tracking.
  - **Touches:** `feiglet-rs/benches/`
  - **Success:** Render throughput at or above C baseline.
  - **Difficulty:** Low

- [ ] `1.6.4` Phase merge: release/1.6 → main
  - **Difficulty:** Low

---

## Phase 1.7 — Major Release

- [ ] `1.7.1` End-to-end verification against C
  - **Goal:** Run full test suite, compare every output byte-for-byte
    with original FIGlet 2.2.5. No differences.
  - **Touches:** Test infrastructure
  - **Success:** 100% output compatibility.
  - **Difficulty:** Medium

- [ ] `1.7.2` v1 major milestone RC — human sign-off
  - **Goal:** Prepare RC for v1.0.0. Ralph halts. Human reviews.
  - **Touches:** RC branch, annotated tag
  - **Success:** `rc/1.0.0-rc.1` created. Human merges.
  - **Difficulty:** Low
  - **Model:** Human
