# Figby Codebase Review — 2026-07-08

Scope: general health check + deep dive on the animation subsystem (timeline,
keyframes, tweening, GIF import/export, in-terminal playback), prompted by the
question "can Figby play an animated banner in the terminal for an app intro?"

Method: direct build/test/clippy runs + manual code reading, plus three Haiku
subagents for parallelizable survey work (gif_import.rs audit, TODO/FIXME
sweep, docs/version consistency check). Findings below were independently
re-verified against current `HEAD` (release/6.6) rather than trusted from
subagent output alone.

---

## 1. Build health (verified, current)

- `cargo build` — clean, 9.3s.
- `cargo test` — **1256 passed, 0 failed** across 8 test binaries (lib 994,
  main 67, fuzz 4, regression_export 13, regression_image 15, regression_tui 9,
  run_tests 48/7 ignored, tui.rs 106).
- `cargo clippy --all-targets` — **0 warnings**.
- Version: `figby-rs/Cargo.toml` = 6.0.3, matches `CHANGELOG.md` latest entry
  (2026-06-22).
- `docs/todo-v6.md`: all 41 top-level items checked off — the v6 hardening
  milestone is fully complete on this branch.

The crate is in good shape mechanically. This review is about feature
completeness and drift, not about broken builds.

## 2. Direct answer: is inline animation playback implemented?

**Partially, and only inside the full TUI.** There is no lightweight,
standalone way to "play an animated banner" in a terminal today:

- `main.rs` is a flat figlet-compatible flag parser (`clap::Parser`, no
  subcommands). There is no `figby play <file>` or equivalent.
- The welcome screen (`tui/welcome.rs:583` `render_title_with_font`) renders a
  **static** FIGlet "FIGBY" title once — no animation, no intro sequence.
- The only way to watch an animation play is: launch `--tui` → open/build a
  file with timeline frames → open the Export dialog → press a key to invoke
  the in-TUI player (`player::play_fullscreen`, `tui/mod.rs:3963`).

The pieces to build a real "animated intro banner" feature already exist but
are disconnected — see §4.

## 3. Animation subsystem — verified current state

`figby-rs/src/tui/timeline.rs` (2225 lines), `player.rs` (1151 lines),
`export.rs` (1476 lines), `gif_import.rs` (263 lines). A prior audit
(`docs/animation-audit.md`, 2026-06-18) covers this ground but is now **stale
in places** — commits `5.5.2` (timeline panel surfaced in main layout),
`5.7.1` (GIF import → timeline), and `5d5bd80` (playback deadlock/corruption
fix, 2026-06-22) closed several gaps it lists as open. Re-verified findings
below.

### Works well
- Timeline CRUD (add/insert/remove/duplicate/reorder), onion skinning,
  keyframing (`LayerKeyframe`: position/opacity/blend, linear interpolation),
  tweening (4 easing functions, preview/commit/discard flow) — all present and
  covered by tests in `timeline.rs`.
- Timeline panel is now wired into the main editor layout and is scrollable
  (`6.8.8`), toggled via `self.animation.timeline_visible`.
- GIF import (`gif_import.rs`) correctly handles disposal methods
  (Background/Previous/Any), per-frame vs. global palette fallback,
  transparency (indexed + grayscale), and extracts real per-frame delays
  (`frame.delay` → `GifImportResult.frame_delays`, line 152/259).
- GIF/APNG/ANSI export dispatch, memory guards, and the recent Esc-deadlock /
  alt-screen-corruption fixes are solid.

### Confirmed open gaps

**P0 — Real bug: imported GIF timing is silently discarded on export.**
`tui/mod.rs:3657` stores the real per-frame delays from an imported GIF into
`export_dialog.frame_delays`. But `GA::Export` (`tui/mod.rs:3331-3344`)
unconditionally calls `export_dialog.set_timeline(fps, count)` whenever the
Export dialog is opened for Gif/Apng mode with a non-empty timeline —
and `set_timeline` (`export.rs:135-139`) **overwrites `frame_delays` with a
uniform value derived from FPS**, clobbering the real GIF timing. Concretely:
import a GIF with variable per-frame delays → open Export → the export uses
flat uniform timing, not the source GIF's actual timing. `set_per_frame_delays`
(`export.rs:153`) exists and would fix this, but it is called **only from
tests** (`export.rs:1353,1389,1434`) — never from production code.

**P0 — Terminal-content capture is still a stub.**
`try_query_terminal_cells` (`player.rs:373`) takes `_w`/`_h` (unused, still
underscore-prefixed) and unconditionally returns `Err(Unsupported)`. "Capture
current terminal output as frame 0" is not implemented; the captured frame is
always blank. This was flagged in the 2026-06-18 audit and is unchanged.

**P1 — ANSI multi-frame export has no timing at all.**
`export_cells_to_ansi_multi` (`output.rs:367-380`) takes a `_frame_delays_cs`
parameter that is **never read** (underscore-prefixed) — it just concatenates
`\x1b[2J\x1b[H` + frame content per frame with zero delay/sleep encoding. If a
user exports "ANSI animation" and `cat`s it to a terminal, all frames render
instantaneously with no perceptible animation. This is the most relevant gap
to the "play a banner in the terminal" ask — the export format that looks
terminal-native doesn't actually replay as one.

**P1 — Playback still blocks the TUI event loop.**
`play_animation` (`tui/mod.rs:3947`) calls `player::play_fullscreen` (line
3963) synchronously. The 2026-06-22 fix removed the double alt-screen
management bug, but did not add threading — spec'd as "separate thread" in
the original 4.8.3 task. Still a direct, blocking call.

**P1 — A fully-built standalone player is orphaned.**
`player::play_raw()` (`player.rs:566-619`) is a complete raw-mode playback
engine — pre-computed ANSI, sleep-based timing, full keyboard controls
(space/arrows/+/-/l/L/Esc/q). It is **never called** from anywhere in the TUI
or CLI. This is effectively 100+ lines of working, tested, dead code that is
the natural basis for a standalone `figby play` command.

**~~P2 — `gif_import.rs` has zero tests~~ — fixed 2026-07-08 (6.0.5).** Added
7 unit tests covering single/multi-frame round-trip, per-frame delay
preservation, `Background` disposal-region clearing, malformed-file and
nonexistent-path errors, and oversized-dimension rejection.

**~~P2 — Duplicate/dead render path~~ — correction, already fixed upstream.**
This review carried the 2026-06-18 audit's claim forward without
re-verifying it: `impl Widget for &ExportDialog` does **not** exist in the
current tree. `git log -S` shows it was introduced in `955ad65` (4.3.2) and
removed again in `e4e6f1b` (5.5.3, "Verify animation export end-to-end") —
i.e. it was cleaned up well before the audit that still listed it as dead
code. No action needed; noted here only so the punch list below is accurate.

## 4. Path to an actual "animated intro banner" feature

Three already-written, currently-disconnected pieces cover ~80% of this:

1. **Fix `export_cells_to_ansi_multi`** to actually emit per-frame delay as a
   literal sleep, or switch the ANSI export target to an asciinema-cast-style
   format with embedded timestamps (asciinema export is already listed, unbuilt,
   in `todo-v6.md`'s deferred section — this is the same feature).
2. **Wire `player::play_raw()`** up as a real entry point — either a CLI
   subcommand (`figby play <exported-file>`) or a `--intro` flag on the
   existing flat CLI, since `main.rs` has no subcommand infra yet and adding
   one is a larger change than adding a flag.
3. **Optionally** call that same playback path once before rendering the
   welcome screen at TUI startup, using a bundled `.gif`/`.txt` animation as
   the app's own intro banner.

This reuses fully-implemented, tested code (`play_raw`) rather than building
new playback machinery — the gap is wiring and the ANSI-timing bug above, not
missing capability.

## 5. Documentation drift

- **README undersells the animation feature.** The only animation-related
  line in `README.md` (line 214) lists "animation timeline" under **Roadmap**
  (i.e., "planned"), with no mention of GIF import, keyframes, tweening, or
  playback — despite all of it being implemented and the v6 milestone (which
  covered surfacing the timeline panel) being 100% complete. Worth moving out
  of Roadmap and into the feature list, and worth documenting what does *not*
  work yet (terminal capture, per-frame GIF export timing) so users don't hit
  the gaps in §3 unwarned.
- `docs/animation-audit.md` (2026-06-18) should be marked superseded/updated —
  several items it lists as gaps were closed within days of being written;
  readers relying on it now will think the timeline panel still isn't in the
  main layout, which is no longer true.

## 6. Everything else (TODO/FIXME sweep)

Only 3 incomplete-work markers in the entire `figby-rs/src` tree, none in the
animation subsystem:
- `tui/mod.rs:1574` — system font picker / duplicate flow not implemented in
  welcome screen.
- `tui/mod.rs:1584` — template picker flow stub (v5.0.4 feature).
- `main.rs:634` — `http(s)://` path input not implemented (explicit error).

All minor, non-blocking, unrelated to animation.

## 7. Prioritized punch list

| # | Item | Effort | Status |
|---|------|--------|--------|
| 1 | Fix `GA::Export` clobbering GIF-imported `frame_delays` via `set_timeline` — only reset delays if not already populated from a GIF import | Low | ✅ Fixed 2026-07-08 (6.0.4) |
| 2 | Make `export_cells_to_ansi_multi` actually use its delay parameter (encode real timing) | Low | ✅ Fixed 2026-07-08 (6.0.4) — emits a self-playing `sh` script |
| 3 | Wire `player::play_raw()` to a CLI entry point; reuse it for an intro-banner flag | Medium | Open |
| 4 | Implement real `try_query_terminal_cells` (DECRQCRA or drop the "capture terminal as frame 0" claim from docs) | Medium | Open |
| 5 | Move playback off the TUI event-loop thread | Medium | Open |
| 6 | Add unit tests to `gif_import.rs` (currently 0) | Low | ✅ Fixed 2026-07-08 (6.0.5) — 7 tests: round-trip, delays, disposal, malformed/oversized input |
| 7 | Update README animation section + mark `animation-audit.md` superseded | Low | Open |
| 8 | Remove dead `impl Widget for &ExportDialog` | Low | ✅ N/A — already removed upstream (5.5.3), review claim was stale |

Everything above is additive/fix-only — no architectural rework needed. The
animation subsystem's core data model and TUI editing experience (timeline,
keyframes, tweening) are genuinely solid; the gaps are concentrated entirely
in the "get pixels onto a real terminal with correct timing outside the full
editor" path, which lines up exactly with the intro-banner idea that prompted
this review.
