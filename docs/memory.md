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
