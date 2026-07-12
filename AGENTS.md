# AGENTS.md

Guidance for AI agents working in the Figby repository.

## Project Overview

Figby is a Rust port of FIGlet (Frank, Ian & Glenn's Letters) — the classic
ASCII art banner generator. The original C implementation (v2.2.5) renders text
in large characters using FIGfont (.flf) and TOIlet (.tlf) font files with
kerning, smushing, and multi-byte character support.

This repo preserves the original C source in `c-figlet/` while the Rust port lives
in `figby-rs/`. The goal is a **feature-complete, safe, modern Rust rewrite**
that supports all FIGlet 2.2.5 features plus UTF-8 natively.

## Development Commands

```bash
# Build Rust crate
cargo build -p figby

# Run tests
cargo test -p figby

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -p figby --all-targets --all-features -- -D warnings

# Test with fonts
cargo run -p figby -- -f fonts/standard "Hello"

# Run figby from repo root (binary is in figby-rs/target)
../figby-rs/target/debug/figby < tests/input.txt

# Compare against system figlet (built with TLF support)
diff <(figlet < tests/input.txt) <(figby-rs/target/debug/figby < tests/input.txt)

# Single input line test
printf "unexpected token \`}'" | figby-rs/target/debug/figby
```

### Ralph Loop

```bash
# Run autonomous task loop (multi-phase from main)
./scripts/ralph.sh

# Run single task
./scripts/ralph.sh 1.1.1

# Dry run to preview
./scripts/ralph.sh --dry-run

# Time-bounded run
./scripts/ralph.sh --minutes=30

# Run until specific task
./scripts/ralph.sh --until=1.1.3

# Log session to file
./scripts/ralph.sh --log=/tmp/figby-ralph.log

# Quiet mode (suppress agent stderr)
./scripts/ralph.sh --quiet
```

Ralph supports multi-phase loops, per-task agent selection via `Agent:`/`Difficulty:` fields in task blocks, phase completion review with auto-fix cycles, and major release (X.0) gates with RC branch creation. Stop gracefully: `kill -TERM $(cat /tmp/ralph.pid)` or `touch scripts/STOP.md`.

## Key Conventions

### Task Workflow

- Tasks defined in `docs/todo-v*.md` with format `- [ ] `X.Y.Z` description`
- Each task maps to branch `task-X.Y.Z` off `release/X.Y`
- Phase completion triggers review before merge to main
- Major versions (X.0) require human sign-off

### Post-Task Checklist

After completing any implementation work, do ALL of the following (or ask the user to confirm, unless in auto mode):

1. **Summarize** what changed — files touched, behavior added/fixed/removed.
2. **Verify** — run before committing (fail = stop, report, fix first):
   ```bash
   cargo build --manifest-path figby-rs/Cargo.toml
   cargo test --manifest-path figby-rs/Cargo.toml
   cargo clippy --manifest-path figby-rs/Cargo.toml --all-targets -- -D warnings
   cargo fmt --manifest-path figby-rs/Cargo.toml --check
   ```
   If `cargo fmt --check` fails, run `cargo fmt` and re-verify before committing.
3. **Commit** — write a proper conventional-commit message (`feat:`, `fix:`, `refactor:`, etc.) with a concise subject and body that explains *why*, not just *what*. Co-author line required.
4. **Version bump** — increment the patch version (or minor/major if scope warrants) in:
   - `figby-rs/Cargo.toml` (`version = "X.Y.Z"`)
   - `README.md` (if it mentions a version)
   - Any `--version` / `--help` output strings in `main.rs`
5. **Changelog** — prepend an entry to `CHANGELOG.md` (create it if absent) in Keep a Changelog format:
   ```
   ## [X.Y.Z] - YYYY-MM-DD
   ### Added / Changed / Fixed / Removed
   - One-line description per change
   ```
6. **Check off task** in `docs/todo-v*.md`.
7. Add entries to `docs/memory.md` and `docs/learnings.md` if there's anything non-obvious to preserve.

**Auto mode:** do steps 1–7 without asking. **Interactive mode:** summarize + verify first, then ask "Commit, bump version, and update changelog?" before proceeding.

### Memory Updates

After completing any implementation work:
1. Add entries to `docs/memory.md`
2. Add insights to `docs/learnings.md`
3. Check off the task in `docs/todo-v*.md`

### Architectural Invariants

1. No `unwrap()` in production paths — use proper error handling
2. FIGfont spec compliance is non-negotiable — test against original C output
3. Support all FIGlet 2.2.5 command-line flags
4. UTF-8 is the native encoding — no wchar_t hacks

## File Structure

```
figby-rs/         # Rust crate (library + TUI binary)
├── Cargo.toml
└── src/
    ├── main.rs           # CLI entry point + TUI launch
    ├── lib.rs            # Crate root (re-exports)
    ├── font.rs           # FIGfont/TLF parser + ZIP font support
    ├── font_gen.rs       # TTF/OTF → FIGfont conversion (fontdue)
    ├── render.rs         # Character rendering, kerning, smushing
    ├── smush.rs          # Smushing rules engine
    ├── control.rs        # FIGlet control file parser (.flc)
    ├── config.rs         # CLI config / settings persistence
    ├── input.rs          # Input pipeline
    ├── output.rs         # Output formatting
    ├── image_input.rs    # Image → ASCII luminance/RGB matrix
    ├── gif_import.rs     # Animated GIF import
    ├── palette_import.rs # Palette import (Paletty/ASE/WezTerm)
    ├── template.rs       # .ftmp template engine
    ├── web.rs            # WASM bindings
    └── tui/              # TUI application (ratatui)
        ├── mod.rs        # re-exports + render pipeline + shared helpers (~774 LOC)
        ├── app_state.rs  # TuiApp + EditorState/AnimationState/LightingState + new
        ├── event_loop.rs # run(), handle_event(), tick/async completion
        ├── dispatch.rs   # handle_key_event/handle_mouse_event + perform_* actions
        ├── canvas.rs     # ASCII canvas buffer
        ├── layers.rs     # Layer stack + compositing
        ├── palette.rs    # Color palette widget
        ├── palette_editor.rs
        ├── toolbox.rs    # Tool selection widget
        ├── font_editor.rs # FIGfont glyph editor
        ├── image_editor.rs
        ├── file_ops.rs   # File open/save dialogs
        ├── layout.rs     # Layout computation
        ├── keymap.rs     # Global keybindings
        ├── events.rs     # App event types
        ├── theme.rs      # Color theme system
        ├── welcome.rs    # Welcome screen
        ├── lighting.rs   # Dynamic lighting engine
        ├── side_panel.rs # Right drawer panel
        ├── timeline.rs   # Animation timeline
        ├── export.rs     # Export formats
        ├── tools/        # Tool implementations
        │   ├── brush.rs, eraser.rs, fill.rs, line.rs
        │   ├── selection.rs, spray.rs, text.rs, eyedropper.rs
        └── components/   # Reusable widgets
            ├── canvas.rs
            └── status_bar.rs
fonts/                # FIGlet font files (.flf/.tlf)
assets/               # Icons, images, templates
scripts/
├── ralph.sh          # Autonomous task loop (bash)
└── ralph-monitor.sh  # Rate-limit monitor cron
docs/
├── todo.md           # Master todo index
├── todo-v6.md        # v6 pre-release hardening tasks
└── codebase-audit-2026-06-18.md
```
