# AGENTS.md

Guidance for AI agents working in the Figby repository.

## Project Overview

Figby is a Rust port of FIGlet (Frank, Ian & Glenn's Letters) — the classic
ASCII art banner generator. The original C implementation (v2.2.5) renders text
in large characters using FIGfont (.flf) and TOIlet (.tlf) font files with
kerning, smushing, and multi-byte character support.

This repo preserves the original C source in the root while the Rust port lives
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
figby-rs/         # Rust crate (the port)
├── Cargo.toml
└── src/
    ├── main.rs
    ├── font.rs       # FIGfont/TLF parser
    ├── render.rs     # Character rendering, kerning, smushing
    ├── smush.rs      # Smushing rules engine
    └── util.rs       # Utilities, IO helpers
fonts/                # FIGlet font files
scripts/
├── ralph.sh         # Autonomous task loop (bash)
└── install-hooks.sh
skills/
└── ralph.md         # Self-review checklist for tasks
docs/
├── todo.md          # Master todo index
├── todo-v1.md       # v1 tasks (port plan)
├── memory.md        # Memory index
└── memory-v1.md     # v1 memory entries
```
