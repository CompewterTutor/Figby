# Animation Audit: 4.5–4.8 Implementation vs Spec

> **⚠️ Superseded — 2026-07-08.** Several gaps this audit lists as open were
> fixed within days of it being written (e.g. "no timeline panel in main
> layout" → closed by `5.5.2`; "GIF import missing" → closed by `5.7.1`), and
> the "dead `impl Widget for &ExportDialog`" item below was actually removed
> even earlier, in `5.5.3`, before this audit was written. Treat the
> per-phase tables below as a historical snapshot, not current status. See
> [docs/sonnet5-review.md](sonnet5-review.md) for a re-verified pass against
> current `HEAD` (and its punch list for what's genuinely still open).

Audit date: 2026-06-18
Files examined: `figby-rs/src/tui/timeline.rs`, `player.rs`, `export.rs`
Spec source: `docs/todo-v4.md` phases 4.5–4.8

---

## Per-Phase Summary

### Phase 4.5 — Animation Timeline & Playback

| Task | Status | Notes |
|------|--------|-------|
| `4.5.0` AnimationTimeline widget | ✅ Works | `Widget for &AnimationTimeline` + `StatefulWidget` + `TimelineState`. Frame thumbnails, keyframe markers, playhead, time ruler all present. Ruler renders frame indices. Playhead renders `▼`. |
| `4.5.1` Frame management | ✅ Works | `add/insert/remove/duplicate/reorder_frame` all present. `layer_state: Option<CanvasBuffer>` on TimelineFrame. Onion skinning toggle renders prev frame thumbnail dimly. |
| `4.5.2` Keyframing | ✅ Works | `LayerKeyframe` struct (`position_offset`, `opacity`, `blend_mode`). `TimelineFrame.layer_keyframes`. `set/remove/get_keyframe`. `get_interpolated_properties` with linear interpolation. `KeyframeEditState` + editor panel + `K` toggle. |
| `4.5.3` Tweening | ✅ Works | `EasingFunction` (Linear/EaseIn/EaseOut/Bounce). `TweenConfig`/`TweenPreview`. `open/compute/commit/discard_tween`. `render_tween_panel` + `handle_tween_key`. Ghost thumbnails in stateful render. `T` key opens panel. |
| `4.5.4` GIF export | ✅ Works | `fps`/`loop_count`/`frame_delays`/`preview_*` fields. `set_timeline`/`populate_from_timeline`. `F`/`L`/`P`/`V`/Space keys. Preview tick cycling. Layers/Alpha lines hidden in animation modes. |

### Phase 4.7 — Animation Exporter

| Task | Status | Notes |
|------|--------|-------|
| `4.7.1` Frame capture | ⚠️ Partial | `capture_timeline_frames()` works — composites timeline frames with keyframe interpolation. BUT `try_query_terminal_cells()` in player.rs always returns `Unsupported` (DECRQCRA stub). No actual terminal-content capture implemented. |
| `4.7.2` APNG export | ✅ Works | `ExportMode::Apng`. `export_cells_to_apng()` from output.rs. Same timeline frame composition as GIF. FPS/loop/delay support. |
| `4.7.3` ANSI export | ✅ Works | `ExportMode::Ansi`. `export_cells_to_ansi()`/`export_cells_to_ansi_multi()` from output.rs. Layers/Alpha toggles gated in ANSI mode. |

### Phase 4.8 — Animation Player (Standalone Widget)

| Task | Status | Notes |
|------|--------|-------|
| `4.8.0` AnimationPlayer widget | ✅ Works | `Widget for &AnimationPlayer` with `Cell` interior mutability. Play/pause/seek/loop/speed (0.25x–4x). Progress bar with frame counter + speed indicator. |
| `4.8.1` Terminal capture | ⚠️ Stub | `TerminalSession::capture()` + `enter/exit_player_mode()` present. `prepend_frame()` works. BUT `try_query_terminal_cells()` always returns `Err(Unsupported)` — captured frame is always blank. |
| `4.8.2` Raw mode playback | ✅ Works | `play_raw()` + `render_frame_raw()` + `color_fg_ansi()`/`color_bg_ansi()`. Pre-computed ANSI strings, sleep-based timing. All keyboard controls. Not called from TUI path. |
| `4.8.3` TUI integration | ⚠️ Partial | `P` in export dialog sets `play_requested` → `launch_player_from_export()`. Enter in timeline → `play_animation()`. BUT playback runs synchronously in event-loop handler (blocks TUI), not in a separate thread as spec'd. |

---

## Per-File Breakdown

### `timeline.rs` (1072 lines + 373 lines of tests)

**What works:**
- `AnimationTimeline` + `TimelineState` + `StatefulWidget` + `Widget` patterns per ratatui best practices
- All 5 frame CRUD operations with bounds checking
- Keyframing: set, remove, get, interpolate with before/after keyframe clamping
- Tweening: full flow from open → compute → preview → commit/discard
- Onion skinning: renders previous frame dimly behind active frame thumbnail
- Keyframe editor panel: navigation, numeric edit, blend mode cycling
- Tween panel: field navigation, value adjustment, easing cycling
- All easing functions (Linear, EaseIn, EaseOut, Bounce) including standard 4-phase bounce

**Gaps:**
- `AnimationTimeline` is only rendered through `StatefulWidget` — the bare `Widget for &AnimationTimeline` only draws a ruler line (timeline.rs:842-854). The widget cannot render frames without `StatefulWidget`. This is acceptable per the stateful-widget pattern but the standalone `Widget` impl is essentially a no-op.

### `player.rs` (1140 lines, 683 lines of tests)

**What works:**
- `AnimationPlayer` with full Cell-based interior mutability
- `Widget for &AnimationPlayer` renders frame cells + progress bar
- `advance()` accumulator-based frame timing, loop support
- Key handling: Space/Left/Right/Up/Down/+/-/l/L/Esc/Enter
- `play_fullscreen()`: capture → enter alt screen → render → handle keyboard → restore
- `play_raw()`: raw mode, pre-computed ANSI, sleep timing, keyboard controls
- `render_frame_raw()`: CUP + ANSI codes, blank-cell skip optimization
- `color_fg_ansi()`/`color_bg_ansi()`: all named + RGB + Indexed colors
- `TerminalSession` lifecycle management

**Gaps:**
- `try_query_terminal_cells()` (line 374) always returns `Err(Unsupported)` — no actual terminal content capture via DECRQCRA or other protocol. The captured "terminal content" is always a blank frame sized to terminal dimensions.
- `write_playback_progress_bar()` (line 638) uses `terminal::size().unwrap_or((80, 24))` — `.unwrap_or()` on `Result` is fine but the unwrap-or pattern is used in a non-production-critical path.

### `export.rs` (1497 lines, ~500 lines of tests)

**What works:**
- `ExportMode` 5-variant enum (Png/Apng/Gif/Txt/Ansi)
- `ExportDialog` with full GIF/APNG timeline fields
- `set_timeline()` / `populate_from_timeline()` / `preview_tick()` / `clear_timeline()`
- All GIF key handlers (`F`/`L`/`P`/`V`/Space)
- `play_requested` flag + close reset
- `perform_export()` dispatches to correct output function per format
- ANSI mode gates: Layers/Alpha toggles disabled in Ansi mode
- `capture_timeline_frames()`: full composite with keyframe interpolation (position offset, opacity, blend mode)
- `Widget for &ExportDialog` and classic `render()` method (both present — the struct has both a `render(&self, frame, area)` method AND a `Widget for &ExportDialog` impl, which is redundant but harmless)

**Gaps:**
- `ExportDialog` has TWO rendering implementations: `render(&self, frame, area)` at line 458 and `impl Widget for &ExportDialog` at line 719. These are nearly identical but separate code paths. The Widget impl is never registered in mod.rs — `self.dialogs.export_dialog.render(frame, area)` is called directly. The Widget impl is dead code.
- `populate_from_timeline()` always recalculates frame delays from FPS rather than using per-frame delay values (spec says "Frame delay per frame or global FPS setting" — global FPS is used, per-frame delays are not supported).

---

## Consolidated Gap List (Ready for `5.5.2` / `5.5.3`)

### P0 — Missing Feature

1. **Terminal content capture is stubbed** (`player.rs:374`) — `try_query_terminal_cells()` always returns `Err(Unsupported)`. Cannot capture real terminal output as frame 0 of animation. Blocks spec'd "Capture current terminal output as the first frame" (4.8.1).

### P1 — Deviations from Spec

2. **Playback blocks TUI event loop** — `play_animation()` (mod.rs:3126) calls `player::play_fullscreen()` synchronously. Spec says "separate thread/event loop" (4.8.3). On return, re-enters alt screen which is a visible flicker. Should run in background thread or use async.

3. **Per-frame delays not supported** — `populate_from_timeline()` (export.rs:153) always sets uniform delays from FPS. Spec says "Frame delay per frame or global FPS setting" (4.5.4). Only global FPS path exists.

### P2 — Dead Code / Cleanup

4. **Duplicate render code in ExportDialog** — `impl Widget for &ExportDialog` (export.rs:719-907) is never used. `ExportDialog::render()` method (line 458) is used instead via direct call in mod.rs. The Widget impl is dead code.

5. **`play_raw()` not integrated** — Full raw-mode engine exists (player.rs:566-619) but is never called from TUI. The TUI always uses `play_fullscreen()` (ratatui-based rendering). `play_raw()` is an independent entry point, usable from CLI but not wired.

### P3 — Polish / Minor

6. **No timeline panel in main layout** — Despite widgets existing, there is no `T`-toggle timeline panel at bottom of canvas (this is exactly task `5.5.2`'s goal). Currently the timeline state exists but has no visible panel in the main editor layout.

7. **AnimationTimeline standalone `Widget` is trivial** — The non-stateful Widget impl (timeline.rs:842-854) only draws a ruler line. All actual frame rendering is in `StatefulWidget`. This is correct but minimal.

---

## Architectural Observations

- All three files properly avoid `.unwrap()` in production — use `Result`, `Option`, `Cell`, and match/if-let patterns.
- FIGfont spec compliance is not relevant to these files (they deal with TUI animation, not FIGlet rendering).
- UTF-8 encoding is native (Rust `char`/`String`) — no wchar_t issues.
- No path traversal or security concerns found in read-only paths.
- Error handling is thorough in export paths (`perform_export` returns `Result`, keyboard handlers check bounds).
