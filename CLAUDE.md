# Figby — Claude Code Context

See [README.md](README.md) for full project overview, CLI usage, feature matrix, and build instructions.

## Source layout

- `figby-rs/src/` — Rust source
  - `main.rs` — CLI entry point + TUI launch
  - `tui/mod.rs` — TUI app state, event loop, render
  - `tui/components/` — Component wrappers (file_ops, canvas, font_editor, etc.)
  - `tui/file_ops.rs` — File open/save dialogs + recent files
  - `tui/font_editor.rs` — FIGfont glyph editor
  - `tui/canvas.rs` — ASCII canvas buffer + rendering
  - `tui/theme.rs` — Color theme system
  - `font.rs` — FIGfont parser
  - `render.rs` — FIGlet render engine
- `docs/` — Task lists (`todo-v3.md` is current), learnings, e2e checklists
- `fonts/` — Bundled `.flf` fonts
- `assets/fonts/` — Font conversion pipeline

## Build & run

```bash
cargo build --manifest-path figby-rs/Cargo.toml
cargo run --manifest-path figby-rs/Cargo.toml -- --tui
```

## Current milestone

v3 — Ratatui Refactor & UX Fixes. See `docs/todo-v3.md`.
Animation/layers/particles milestone moved to `docs/todo-v4.md`.
