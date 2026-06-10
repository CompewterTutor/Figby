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
