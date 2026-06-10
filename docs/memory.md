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
