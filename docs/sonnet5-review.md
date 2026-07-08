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

**~~P0 — Terminal-content capture is still a stub~~ — resolved 2026-07-08
(6.0.8), verdict: not a bug, docs were wrong.** `try_query_terminal_cells`
still unconditionally returns `Err(Unsupported)` (blank captured frame) —
but investigating this properly (rather than carrying the 2026-06-18 audit's
framing forward again) turned up that the surrounding doc comments were
actively misleading, not just incomplete. They suggested DECRQCRA was a
viable future implementation path. It isn't: DECRQCRA's response (DECCKSR)
is a terminal-defined *checksum* of a region, used by conformance suites
(vttest) to verify already-known content renders correctly — it cannot be
inverted to recover arbitrary unknown screen content. No standard escape
sequence can do that; a couple of terminals (kitty, iTerm2) have proprietary,
non-portable extensions crossterm doesn't wrap. So `Err(Unsupported)` +
blank frame is the **correct, final** behavior, not an unfinished stub. Fixed
the doc comments to say so (and fixed a separate, unrelated staleness in the
same doc comment: it referenced `enter_player_mode()`/`exit_player_mode()`,
methods that no longer exist).

**P1 — ANSI multi-frame export has no timing at all.**
`export_cells_to_ansi_multi` (`output.rs:367-380`) takes a `_frame_delays_cs`
parameter that is **never read** (underscore-prefixed) — it just concatenates
`\x1b[2J\x1b[H` + frame content per frame with zero delay/sleep encoding. If a
user exports "ANSI animation" and `cat`s it to a terminal, all frames render
instantaneously with no perceptible animation. This is the most relevant gap
to the "play a banner in the terminal" ask — the export format that looks
terminal-native doesn't actually replay as one.

**~~P1 — Playback still blocks the TUI event loop~~ — investigated
2026-07-08, closed as not-a-bug (user-confirmed).** `play_animation` still
calls `player::play_fullscreen` synchronously. The original 4.8.3 spec called
for a "separate thread," but that's not actually beneficial here: the TUI and
the player both write directly to the same exclusive terminal fd via
crossterm/ratatui, so nothing useful runs concurrently while the player owns
the terminal and its own keyboard input (pause/seek/Esc/q) anyway. A
background thread would add a channel + lifecycle surface purely to
reproduce identical blocking behavior, with a real risk of the two writers
racing on stdout if not synchronized carefully. Left as a direct call, with a
new doc comment at the call site (`tui/mod.rs`) explaining why, so a future
contributor doesn't "fix" this without the context.

**~~P1 — A fully-built standalone player is orphaned~~ — fixed 2026-07-08
(6.0.7).** `player::play_raw()` is now wired up as `figby --play <path.gif>`.
Wiring it up surfaced a second, real bug in `play_raw()` itself: its exit
condition required the player to already be *paused* to end playback, but
nothing ever paused it automatically — an unattended, non-looping animation
played forever on its last frame instead of returning control. Fixed to
auto-exit once the final frame has had its natural on-screen interval.
Verified end-to-end over a real pty (tmux, not just unit tests): a 4-frame
and a 14-frame GIF both play through and exit 0 unattended; an oversized GIF
is still rejected cleanly by the existing memory guard.

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

## 4. Path to an actual "animated intro banner" feature — mostly done

Three already-written, currently-disconnected pieces were identified as
covering ~80% of this:

1. ~~Fix `export_cells_to_ansi_multi`~~ — ✅ done (6.0.4).
2. ~~Wire `player::play_raw()` up as a real entry point~~ — ✅ done (6.0.7):
   `figby --play <path.gif>`.
3. **Still open (optional):** call that same playback path once before
   rendering the welcome screen at TUI startup, using a bundled `.gif` as the
   app's own intro banner. Not done — `--play` is a separate one-shot CLI
   command for now, not wired into `--tui` startup. Deliberately left open:
   auto-playing an animation before every TUI launch is a UX/product call
   (does it block startup? is it skippable? does everyone want it every
   time?) that's better made explicitly than assumed.

## 5. Documentation drift — ✅ fixed 2026-07-08 (6.0.6)

- **README undersold the animation feature.** The only animation-related
  line in `README.md` used to list "animation timeline" under **Roadmap**
  (i.e., "planned"), with no mention of GIF import, keyframes, tweening, or
  playback — despite all of it being implemented and the v6 milestone (which
  covered surfacing the timeline panel) being 100% complete. Now: `--tui` is
  documented in the CLI flag table, the Features list describes the
  animation timeline/keyframing/GIF import-export capabilities (linking back
  to this doc for known limitations), Project Status reflects v6, and
  Roadmap now lists only genuinely-outstanding deferred work.
- `docs/animation-audit.md` (2026-06-18) now carries a superseded notice at
  the top pointing here — several items it lists as gaps were closed within
  days of being written; readers relying on it would otherwise think the
  timeline panel still isn't in the main layout, which is no longer true.

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
| 3 | Wire `player::play_raw()` to a CLI entry point; reuse it for an intro-banner flag | Medium | ✅ Fixed 2026-07-08 (6.0.7) — `figby --play <gif>`; also fixed a hang bug found while wiring it up |
| 4 | Implement real `try_query_terminal_cells` (DECRQCRA or drop the "capture terminal as frame 0" claim from docs) | Medium | ✅ Resolved 2026-07-08 (6.0.8) — docs corrected, no functional change was possible/needed (see below) |
| 5 | Move playback off the TUI event-loop thread | Medium | ✅ Resolved 2026-07-08 — investigated, closed as not-a-bug (user-confirmed) |
| 6 | Add unit tests to `gif_import.rs` (currently 0) | Low | ✅ Fixed 2026-07-08 (6.0.5) — 7 tests: round-trip, delays, disposal, malformed/oversized input |
| 7 | Update README animation section + mark `animation-audit.md` superseded | Low | ✅ Fixed 2026-07-08 (6.0.6) |
| 8 | Remove dead `impl Widget for &ExportDialog` | Low | ✅ N/A — already removed upstream (5.5.3), review claim was stale |

Everything above is additive/fix-only — no architectural rework needed. The
animation subsystem's core data model and TUI editing experience (timeline,
keyframes, tweening) are genuinely solid; the gaps are concentrated entirely
in the "get pixels onto a real terminal with correct timing outside the full
editor" path, which lines up exactly with the intro-banner idea that prompted
this review.

**Update 2026-07-08: all 8 items resolved** (6.0.4–6.0.9, one commit per
item). Net result for the original question — "can Figby play an animated
banner in the terminal?" — yes: `figby --play <path.gif>` now does exactly
that, GIF-imported timing survives to export, and ANSI multi-frame export
actually encodes real per-frame delays instead of discarding them. Two items
(4, 5) turned out not to be bugs on closer inspection and were closed with
corrected documentation instead of code changes — see their entries above.

**Follow-up 2026-07-08 (6.0.10): scaled playback.** `--play` originally
imported GIFs at native pixel resolution (1 pixel = 1 terminal cell), which
meant real-world GIFs bigger than roughly 1000x1000 total pixels (or just
several-hundred-pixel GIFs with enough frames) got rejected outright by the
animation import size cap, and anything wider/taller than the terminal would
overflow it regardless. Added `gif_import::GifScaleTarget` /
`import_gif_scaled()` (bilinear resize, same convention already used by
`image_input`'s image-to-ASCII path) and wired `--play` to scale to the
terminal by default, or to `--play-width <N>` explicitly. The size cap now
checks scaled output size rather than native size. Verified against a real
480x360, 90-frame GIF that previously failed with "too large" — it now
imports and plays to completion.

**Follow-up 2026-07-08 (6.0.11): loop until dismissed.** `--play` only ever
played once and auto-exited (by design, per the 6.0.7 fix). Added
`--loop`: repeats indefinitely, any keypress dismisses (interactive
pause/seek/speed controls are bypassed in this mode, since there's no
natural end to wait for — this was a deliberate simplification, confirmed
with the user, over trying to preserve those controls alongside a bail-on-
any-key affordance). `player::play_raw()` gained a `loop_playback: bool`
parameter. Verified end-to-end over a real pty: playback wraps past the
last frame back to frame 1 repeatedly and exits cleanly on an arbitrary
keypress.

**Follow-up 2026-07-08 (6.0.12): TUI GIF import also scales.** The scaling
work above only covered `--play`; the TUI's own File > Import GIF path
(`perform_import_gif()`, `tui/mod.rs`) had the identical problem — imports
at native pixel resolution, same size-cap rejection risk, plus an
unusably huge canvas for anything but small GIFs. Now computes the actual
canvas viewport via `layout::FrameLayout::compute()` (the same call
already used for mouse hit-testing, so it reflects the real toolbox/side-
panel/timeline chrome) and scales with `GifScaleTarget::FitBox`. Verified
by driving the real TUI over a pty: File > Import GIF > the same 480x360,
90-frame GIF that fails outright via unscaled `import_gif` now produces a
40x15 canvas (fit to the 120x40 terminal used for the test) with all
frames populated in the timeline.

*Aside noticed during this verification, not yet investigated:* after the
GIF import dialog's Enter-to-confirm succeeded (canvas correctly sized,
timeline populated, mode switched to Image Editor), the same keypress
also appeared to trigger a stray "Open Font" dialog with a "stream did not
contain valid UTF-8" error, reusing the GIF's path as if it were a font
file. This looks like a pre-existing double-dispatch issue in the generic
`AppEvent::OpenRequested` handling (unrelated to the scaling change here —
nothing in this fix touches dialog-closing/mode-transition code), not
something introduced by this session's changes. Flagging it rather than
silently leaving it out; not fixed as part of this task.
