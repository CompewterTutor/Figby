# Figby Overnight Handoff — 2026-06-16

## What ran

Ralph ran ~3 hours (04:41–06:23) autonomously. 10 tasks completed.

## What got done

### v3 milestone — COMPLETE
- `3.2.1` Glyph grid mouse scroll (we did this before ralph started)
- `3.2.2` Glyph cursor overlay with arrow nav + cell toggle
- `3.2.3` Font preview strip in overview
- `3.2.4` Phase merge release/3.2 → master → **v3 phase 3.2 done**
- `3.3.1` Full regression test suite vs v2.x baseline
- `3.3.2` v3.0.0 RC cut → **v3 COMPLETE, rc/3.0.0-rc.1 tag exists**

### v4 milestone started
- `3.2.0` AnimationTimeline custom ratatui widget (timeline.rs, ~400 lines)
- `3.2.1` Frame management (add/delete/reorder frames in timeline)
- `3.0.1` Custom MenuBar widget (ralph re-did v3 work due to ID collision)
- Phases 3.2 + 3.3 merged. Phase 3.0 hit major RC gate → ralph stopped correctly.

## Current git state

- Branch: `release/3.0`
- RC branches exist: `rc/2.0.0-rc.1`, `rc/3.0.0-rc.1`
- v3 is done on master

## Critical bug discovered: ralph task collision

`task_block()` in `scripts/ralph.sh` matches `[x]` tasks too (awk pattern `[.]`).
When todo-v4.md has unchecked `3.X.Y` and todo-v3.md has the same ID done `[x]`,
ralph always implements v3's description instead of v4's, loops infinitely.

**Fix needed in ralph.sh before running v4 tasks:**
Change `task_block()` awk pattern from `"^- \\[.\\] \`"` to `"^- \\[ \\] \`"`.

**Current unchecked v4 tasks that will collide:**
- `3.1.1`–`3.1.5` (Layers/blending) — same IDs as done v3 3.1.x
- `3.3.1`–`3.3.4` (Particles) — same IDs as done v3 3.3.x

## What you need to do

1. **Human sign-off on v3 RC**: review `rc/3.0.0-rc.1` branch, run e2e tests, merge to master if good.
2. **Fix ralph.sh** before running v4 tasks (see above).
3. **Pre-mark deconflicted v4 tasks** in todo-v4.md (or let the fix handle it):
   - `3.3.1`–`3.3.4` show as `[ ]` in release/3.0 (mid-flight fix didn't survive branch switch)
   - `3.1.1`–`3.1.5` need either the ralph.sh fix or manual `[x]` marks

## Files changed this session (major)

- `figby-rs/src/tui/font_editor.rs` — scroll overview, glyph cursor overlay, preview strip
- `figby-rs/src/tui/mod.rs` — mouse scroll, StatefulWidget for MenuBar, global dispatch
- `figby-rs/src/tui/menu.rs` — full rewrite to StatefulWidget + MenuBarState
- `figby-rs/src/tui/keymap.rs` — GlobalAction enum + GLOBAL_DISPATCH table
- `figby-rs/src/tui/timeline.rs` — NEW: AnimationTimeline + TimelineState + frame mgmt
- `figby-rs/tests/tui.rs` — 12 test fixes
- `docs/todo-v3.md` — all tasks marked [x]
- `docs/todo-v4.md` — 3.0.x, 3.2.x done; 3.1.x, 3.3.x still open (collision tasks)
- `.gitignore` — added STOP.md, font pipeline dirs

## Resume command (next session)

```bash
# Human sign-off on RC first, then:
git checkout master
# Fix ralph.sh task_block() regex, commit
# Then:
./scripts/ralph.sh --hours=8 --log=docs/ralph-log.md
```
