# Figby v2 — Memory

## Phase 2.0 — CLI Polish

### 2.0.1 — CLI `--help` output

Added `help = "..."` to every `#[arg()]` field and `long_about` to `#[command()]`
on `CliArgs` struct in `main.rs`. Every FIGlet flag now has a descriptive help
string. Tests verify `--help` exits with `DisplayHelp` and output contains
key flags and descriptions. `--long-help` not a built-in clap 4 flag — tested
via `render_long_help()` directly.

Pre-existing clippy issue in `render.rs`: `calc_smush_amount()` missing
`#[allow(clippy::too_many_arguments)]` — added to pass clippy gate.
Pre-existing bench bug: `calc_smush_amount` call had wrong argument order —
fixed to pass clippy `--all-targets`.
