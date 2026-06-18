# Figby Handoff — 2026-06-18

## Status
Ralph is running autonomously (PID in `/tmp/ralph.pid`).
Currently on task `5.1.1` (Toolbox NerdFont icons) on branch `task-5.1.1` off `release/5.1`.

## What Was Done This Session

### Phase 5.0 — Welcome Screen Redesign (COMPLETE, v5.0.0)
All tasks 5.0.1–5.0.7 implemented and committed.

- **5.0.1** `welcome.rs`: Computerist-20 banner with Computerist-12 fallback; mascot+title vertically centered; title horizontally centered via `Alignment::Center`. Dynamic `mascot_width` from parsed ANSI spans.
- **5.0.2** Two-column content layout: Recent Files (left, scrollable ↑↓/scroll) + Font/Image panels (right, stacked).
- **5.0.3** Font panel: 5 actions with NerdFont icons, `[K]suffix` inline format (N/I/B/O/D).
- **5.0.4** Image panel: 4 actions with NerdFont icons (C/T/V/F). Format uses `FONT_ACTIONS`/`IMAGE_ACTIONS` const arrays.
- **5.0.5** Mouse click + hover: stored `recent_rects`/`font_rects`/`image_rects` updated each render cycle. `handle_mouse()` returns `(Option<WelcomeAction>, hover_dirty)`. `dispatch_welcome_action()` shared by keyboard + mouse.
- **5.0.6** Esc removed from global quit keymap (`keymap.rs`). Q/q = quit. Ctrl+C = SIGINT.
- **5.0.7** Committed as `v5.0.0`, `release/5.0` branch created.

### Ralph Setup
- All agents switched to `opencode/deepseek-v4-flash-free`
- `scripts/ralph-monitor.sh`: cron every 15 min, detects rate limits, auto-switches to `opencode-go/deepseek-v4-flash` and restarts ralph
- Cron installed and running

## Files Changed This Session
- `figby-rs/src/tui/welcome.rs` — complete overhaul
- `figby-rs/src/tui/mod.rs` — dispatch_welcome_action, mouse routing, render call updated
- `figby-rs/src/tui/keymap.rs` — Esc removed from quit, Q added
- `figby-rs/Cargo.toml` — version 4.0.0-rc.1 → 5.0.0
- `docs/todo-v5.md` — 5.0.1–5.0.7 all [x]
- `CHANGELOG.md` — v5.0.0 entry added
- `scripts/ralph.sh` — agents updated to free tier
- `scripts/ralph-monitor.sh` — NEW: rate-limit monitor

## Next Steps (ralph handles automatically)
- 5.1.1 Toolbox NerdFont icons (IN PROGRESS)
- 5.1.2 Toolbox dynamic width
- 5.1.3 Canvas visible border
- 5.1.4 Collapsed/shared borders
- 5.2–5.5 Layout restructure, status bar, image editor fix, animation surface

## Monitor
```bash
tail -f /home/hippo/git_repos/Figby/docs/ralph-log.md
```
Stop: `touch /home/hippo/git_repos/Figby/scripts/STOP.md`
