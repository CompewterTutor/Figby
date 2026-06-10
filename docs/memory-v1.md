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
