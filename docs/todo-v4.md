# Figby v3 — TUI Refinement & Animation

Milestone goal: Complete the TUI editor with full layer support, animation
timeline, particle effects, animation export/playback, and a polished UI
with theming, modern status bar, and component architecture.

---

## Phase 3.0 — Manual Testing & Audit

- [ ] `3.0.1` Full manual test of all v2.x features
  - **Goal:** Test every feature from v2.0 through v2.9 manually. Identify
    bugs, UX issues, missing polish. Log findings for 3.x triage.
  - **Touches:** N/A (testing pass)
  - **Success:** Documented list of all issues found. Go/no-go for 3.x.
  - **Tests:** Manual verification.
  - **Difficulty:** Medium

- [ ] `3.0.2` Triage issues into 3.x tasks
  - **Goal:** Take findings from 3.0.1 and create/amend tasks in the
    appropriate 3.x phase. Prioritize: crashes > data loss > visual bugs
    > UX polish.
  - **Touches:** `docs/todo-v3.md`
  - **Success:** All issues assigned to a 3.x phase task.
  - **Tests:** N/A (doc-only).
  - **Difficulty:** Low

- [ ] `3.0.3` Phase merge: release/3.0 → main
  - **Difficulty:** Low

---

## Phase 3.1 — Layers, Blending & Compositing

- [ ] `3.1.1` Layer system
  - **Goal:** Layer panel: list of layers, visibility toggle, lock toggle,
    opacity slider, drag-to-reorder. New/delete/duplicate/merge layers.
    Each layer is an independent ASCII buffer.
  - **Touches:** `figby-rs/src/tui/layers.rs`
  - **Success:** Layers render stacked. Layer operations work.
  - **Tests:** Create, delete, reorder, merge layers.
  - **Difficulty:** High

- [ ] `3.1.2` Blending modes
  - **Goal:** Per-layer blend mode: Normal, Multiply, Overlay, Screen,
    Add, Subtract. Render composited output in real time. Preview
    thumbnail per layer showing blend effect.
  - **Touches:** `figby-rs/src/tui/layers.rs`
  - **Success:** Blend modes produce correct composed output.
  - **Tests:** Multiply + Overlay blend with known test patterns.
  - **Difficulty:** High

- [ ] `3.1.3` Layer groups + masks
  - **Goal:** Group layers into folders. Layer mask: paint on mask to
    hide/reveal parts of layer. Mask thumbnail in layer panel.
  - **Touches:** `figby-rs/src/tui/layers.rs`
  - **Success:** Groups collapsible. Mask hides painted areas.
  - **Tests:** Group create, mask paint, verify composited result.
  - **Difficulty:** Medium

- [ ] `3.1.4` Export with layers
  - **Goal:** Export flattened composite. Export individual layers as
    separate files. Export with transparency (space = transparent).
  - **Touches:** `figby-rs/src/tui/export.rs`
  - **Success:** Flattened export matches canvas. Layer exports correct.
  - **Tests:** Export composite vs manual layer merge.
  - **Difficulty:** Low

- [ ] `3.1.5` Phase merge: release/3.1 → main
  - **Difficulty:** Low

---

## Phase 3.2 — Animation Timeline & Playback

- [x] `3.2.0` Custom ratatui widget: `AnimationTimeline`
  - **Goal:** Create `AnimationTimeline` widget following ratatui best
    practices: `Widget for &AnimationTimeline` (reference-based, not
    consuming), separate `TimelineState` implementing
    `StatefulWidget`. Layout constraints via `Constraint`. Should
    support: frame thumbnails in horizontal scroll, keyframe markers,
    playhead position, time ruler. Reusable as a standalone ratatui
    custom widget following the patterns in
    `docs.rs/ratatui/latest/ratatui/widgets/#authoring-custom-widgets`.
  - **Touches:** `figby-rs/src/tui/timeline.rs`
  - **Success:** Widget renders correctly with sample data. Can be
    embedded in any Layout. State persists across frames.
  - **Tests:** Unit test widget rendering width/height constraints.
    Verify playhead position updates.
  - **Difficulty:** Medium

- [ ] `3.2.1` Frame management
  - **Goal:** Timeline panel with frame thumbnails, add/delete/duplicate/
    reorder frames using `AnimationTimeline` widget. Each frame stores
    full layer state. Onion skinning (semi-transparent overlay of
    prev/next frame).
  - **Touches:** `figby-rs/src/tui/timeline.rs`
  - **Success:** Frames addable, reorderable. Onion skin overlay renders.
  - **Tests:** Create frames, switch between them, verify state isolation.
  - **Difficulty:** High

- [ ] `3.2.2` Keyframing
  - **Goal:** Keyframeable properties per layer: position offset, opacity,
    blend mode. Keyframe markers on timeline. Interpolation between
    keyframes (linear). Keyframe editor panel.
  - **Touches:** `figby-rs/src/tui/timeline.rs`
  - **Success:** Keyframes set. Playback interpolates between them.
  - **Tests:** Set keyframes, play, verify interpolation.
  - **Difficulty:** High

- [ ] `3.2.3` Tweening
  - **Goal:** Auto-tween: select start/end keyframes, generate
    intermediate frames. Easing functions: linear, ease-in, ease-out,
    bounce. Preview tween before committing.
  - **Touches:** `figby-rs/src/tui/timeline.rs`
  - **Success:** Tween generates intermediate frames with correct easing.
  - **Tests:** Tween between known keyframes, verify frame sequence.
  - **Difficulty:** Medium

- [ ] `3.2.4` GIF export from timeline
  - **Goal:** Render animation timeline to animated GIF. Frame delay per
    frame or global FPS setting. Loop count. Preview playback in TUI.
  - **Touches:** `figby-rs/src/tui/export.rs`
  - **Success:** GIF matches timeline playback.
  - **Tests:** Export GIF, verify frame count + timing.
  - **Difficulty:** Medium

- [ ] `3.2.5` Phase merge: release/3.2 → main
  - **Difficulty:** Low

---

## Phase 3.3 — Particle Effect Creator

- [ ] `3.3.1` Particle system design
  - **Goal:** Design particle data model: emitter position, spawn rate,
    lifetime, velocity (x,y), acceleration, size, color, character,
    opacity, blend mode. Config file format (TOML). Runtime particle
    state: active particles array, per-particle remaining lifetime.
  - **Touches:** `figby-rs/src/tui/particles.rs`
  - **Success:** Particle system spec documented. Rust types defined.
  - **Tests:** Particle lifecycle: spawn → update → expire.
  - **Difficulty:** Medium

- [ ] `3.3.2` Particle emitter UI
  - **Goal:** Toolbox tool: place emitter on canvas. Emitter config panel:
    spawn rate (particles/sec), lifetime range, velocity range, acceleration,
    spread angle, emission shape (point/circle/rect). Preview emission in
    real time on canvas.
  - **Touches:** `figby-rs/src/tui/particles.rs`, `figby-rs/src/tui/toolbox.rs`
  - **Success:** Emitter placed. Particles animate on canvas.
  - **Tests:** Emit particles, verify count and motion.
  - **Difficulty:** High

- [ ] `3.3.3` Particle-to-layer baking
  - **Goal:** Bake current particle frame(s) to a canvas layer. Generate
    frame-by-frame layer stack from particle animation. Toggle between
    live preview and baked layers.
  - **Touches:** `figby-rs/src/tui/particles.rs`, `figby-rs/src/tui/layers.rs`
  - **Success:** Baked layers match frozen particle state.
  - **Tests:** Bake 10 frames, verify each is independent.
  - **Difficulty:** Medium

- [ ] `3.3.4` Phase merge: release/3.3 → main
  - **Difficulty:** Low

---

## Phase 3.4 — Animation Exporter

- [ ] `3.4.1` Frame-by-frame terminal capture
  - **Goal:** Capture raw terminal output of each animation frame.
    Render each frame to a buffer, capture the rendered cells as styled
    text (char + FG + BG). Store as frame array in memory.
  - **Touches:** `figby-rs/src/tui/export.rs`
  - **Success:** Frames captured to memory. Cell data matches on-screen.
  - **Tests:** Capture frame, compare cell-by-cell with live render.
  - **Difficulty:** Medium

- [ ] `3.4.2` APNG export
  - **Goal:** Export animation as animated PNG (APNG). Each frame is a
    PNG of the rasterized ASCII canvas at that point. Frame delay
    metadata. Loop count.
  - **Touches:** `figby-rs/src/tui/export.rs`, `figby-rs/Cargo.toml`
  - **Success:** APNG plays back correctly in browser/image viewer.
  - **Tests:** Export APNG, verify frame count + timing.
  - **Difficulty:** Medium

- [ ] `3.4.3` ANSI escape sequence export
  - **Goal:** Export animation as ANSI escape sequence file (`.ans` or
    `.txt` with escape codes). Each frame: cursor home + styled text.
    Frame separator (clear + delay escape). Compatible with `cat` + terminal.
  - **Touches:** `figby-rs/src/tui/export.rs`
  - **Success:** Exported `.ans` file plays back in terminal correctly.
  - **Tests:** Export → `cat` → visual comparison with TUI playback.
  - **Difficulty:** Low

- [ ] `3.4.4` Phase merge: release/3.4 → main
  - **Difficulty:** Low

---

## Phase 3.5 — Animation Player (Standalone Widget)

- [ ] `3.5.0` Custom ratatui widget: `AnimationPlayer`
  - **Goal:** Standalone ratatui widget that plays back captured animation
    frames on the alternate screen. Implements `Widget for &AnimationPlayer`.
    Takes `Vec<Frame>` (styled cell arrays). Plays at specified FPS.
    Supports: play/pause, seek (frame index), loop toggle, speed control
    (0.25x–4x). Renders progress bar (current frame / total frames).
  - **Touches:** `figby-rs/src/tui/player.rs`
  - **Success:** Widget renders frame sequence at correct speed.
    Play/pause/seek/hotkeys work.
  - **Tests:** Play 10-frame animation, verify each frame renders in order.
  - **Difficulty:** Medium

- [ ] `3.5.1` Terminal capture for playback
  - **Goal:** When player starts, capture current terminal output as the
    first frame. Switch to alternate screen. Play animation. On finish or
    exit, restore original terminal content from capture.
  - **Touches:** `figby-rs/src/tui/player.rs`
  - **Success:** Terminal content preserved before/after playback.
  - **Tests:** Capture → play → restore, verify content matches.
  - **Difficulty:** Medium

- [ ] `3.5.2` Raw mode playback engine
  - **Goal:** Enter raw mode for playback (no echo, no line buffering).
    Render frames by writing pre-computed escape codes directly to stdout
    (bypass ratatui diffing for speed). Frame timing via `sleep` or
    spin-wait. Keyboard: Space=pause, Esc=exit, Left/Right=seek, +/-=speed.
  - **Touches:** `figby-rs/src/tui/player.rs`
  - **Success:** Playback is smooth at target FPS. Controls responsive.
  - **Tests:** Play at 30fps, measure frame timing accuracy.
  - **Difficulty:** High

- [ ] `3.5.3` Player integration into TUI
  - **Goal:** Export dialog → "Play Animation" button triggers player.
    Timeline → play button triggers player from current frame. Player
    runs in separate thread/event loop, returns to TUI on exit.
  - **Touches:** `figby-rs/src/tui/export.rs`, `figby-rs/src/tui/mod.rs`
  - **Success:** Player launches from export/timeline. TUI restores cleanly.
  - **Tests:** Launch player, exit, verify TUI state preserved.
  - **Difficulty:** Medium

- [ ] `3.5.4` Phase merge: release/3.5 → main
  - **Difficulty:** Low

---

## Phase 3.0-P (Priority Bugs & Polish) — TUI Startup & Interaction

> **These are pre-3.1 blockers. Fix before any Phase 3.1+ work.**

- [x] `3.0-P.1` Remove auto-load of standard font on startup *(fixed)*
  - **Goal:** TUI starts with blank canvas instead of auto-loading `fonts/standard.flf`.
    Show a welcome prompt or blank editor instead.
  - **Touches:** `figby-rs/src/tui/mod.rs:169`
  - **Success:** TUI opens to blank state.
  - **Difficulty:** Low

- [x] `3.0-P.2` Fix OS Error 2 in file open dialog *(fixed)*
  - **Goal:** When user presses Enter in open dialog with empty path or a path
    that doesn't exist, show inline error instead of spawning async thread that
    fails with "Cannot read file: No such file or directory (os error 2)".
  - **Touches:** `figby-rs/src/tui/mod.rs` `handle_key_event` Open branch
  - **Success:** Empty Enter = cancel. Missing file = friendly error in dialog.
  - **Difficulty:** Low

- [x] `3.0-P.3` Block mouse fall-through when dialog is open *(fixed)*
  - **Goal:** When file-ops or export dialog is open, mouse events must not
    reach the canvas/toolbox. Dialogs previously captured keyboard only;
    clicks went to the editor underneath.
  - **Touches:** `figby-rs/src/tui/mod.rs` `handle_mouse_event`
  - **Success:** Clicking background while dialog open does nothing.
  - **Difficulty:** Low

- [ ] `3.0-P.4` Welcome screen on startup
  - **Goal:** On startup (no file loaded), show a centered welcome overlay with:
    recent files list (numbered shortcuts), keybindings for Open / New / Help / Config.
    Dismiss on any file load or Esc.
  - **Touches:** `figby-rs/src/tui/mod.rs`, new `figby-rs/src/tui/welcome.rs`
  - **Success:** App opens to welcome screen. Pressing 1-N opens recent file directly.
  - **Difficulty:** Medium

- [ ] `3.0-P.5` ZIP file browsing in file open dialog
  - **Goal:** In the open dialog, `.zip` files appear as navigable "directories".
    Selecting one lists the `.flf`/`.tlf` files inside. Selecting a font inside
    reads it from the ZIP via the existing compressed-font path.
  - **Touches:** `figby-rs/src/tui/file_ops.rs`, `figby-rs/src/font.rs`
  - **Success:** User can open fonts directly from a `.zip` archive in the browser.
  - **Difficulty:** Medium

---

## Phase 3.0-A (Architecture Audit)

- [ ] `3.0-A.1` TUI architecture deepdive vs ratatui best practices
  - **Goal:** Compare current component architecture in `tui/components/` and
    `tui/mod.rs` against ratatui documentation patterns:
    `Widget for &T` (reference-based, non-consuming), `StatefulWidget`,
    `WidgetRef`, proper `Layout` + `Constraint` usage, custom widget authoring
    guide. Identify deviations and gaps. Produce list of concrete refactors.
  - **Touches:** `figby-rs/src/tui/` (audit only, no changes)
  - **Success:** Written audit in `docs/tui-arch-audit.md` with specific
    file:line findings and proposed fixes.
  - **Difficulty:** Medium

- [ ] `3.0-A.2` Apply ratatui architecture fixes from audit
  - **Goal:** Implement fixes identified in `3.0-A.1`. Priority: widget
    ownership/borrow patterns first, then layout constraints.
  - **Touches:** `figby-rs/src/tui/`
  - **Depends:** `3.0-A.1`
  - **Difficulty:** Medium–High

---

## Phase 3.0-V (Visual Polish & TachyonFX)

- [ ] `3.0-V.1` Evaluate TachyonFX for UI animations
  - **Goal:** Spike: add `tachyonfx` crate, prototype one animation (e.g. dialog
    fade-in, screen transition). Assess ergonomics and perf. Reference:
    https://ratatui.rs/ecosystem/tachyonfx/
  - **Touches:** `figby-rs/Cargo.toml`, spike branch
  - **Success:** Working prototype with at least one animated transition.
  - **Difficulty:** Medium

- [ ] `3.0-V.2` Default panel theme inspired by TachyonFX aesthetic
  - **Goal:** Update `tui/theme.rs` default colors to match the dark, neon-accent
    aesthetic of the TachyonFX showcase. Panel borders, selection highlights,
    dialog chrome. Keep configurable.
  - **Touches:** `figby-rs/src/tui/theme.rs`
  - **Success:** Side-by-side comparison shows obvious visual improvement.
  - **Difficulty:** Low

- [ ] `3.0-V.3` App fade-in on launch (ratzilla-style)
  - **Goal:** On startup, play a brief fade-in effect (TachyonFX or custom) that
    reveals the canvas/UI. Reference: https://ratatui.rs/ecosystem/ratzilla/
    See the fade-in demo as inspiration.
  - **Touches:** `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/welcome.rs`
  - **Success:** Smooth fade-in visible on every cold launch.
  - **Difficulty:** Medium

- [ ] `3.0-V.4` New status bar redesign
  - **Goal:** Redesign the bottom status bar. Show: mode, current tool, cursor
    position, font name + glyph count, unsaved indicator, git branch, FPS/render
    mode. Responsive layout: drops low-priority items at narrow widths.
  - **Touches:** `figby-rs/src/tui/status.rs`, `figby-rs/src/tui/components/status_bar.rs`
  - **Success:** Status bar looks polished; all info visible at typical terminal widths.
  - **Difficulty:** Medium

---

## Phase 3.0-W (Web Target)

- [ ] `3.0-W.1` WASM / web target via Ratzilla
  - **Goal:** Add a `wasm32-unknown-unknown` build target using `ratzilla` crate.
    Render the TUI in-browser via the Ratzilla web backend. Start with read-only
    font preview; interactive editing is stretch goal.
    Reference: https://ratatui.rs/ecosystem/ratzilla/
  - **Touches:** `figby-rs/Cargo.toml`, new `figby-rs/src/web.rs`, CI config
  - **Success:** `cargo build --target wasm32-unknown-unknown` succeeds.
    App renders in browser.
  - **Difficulty:** High

---

## Phase 3.0-C (Extended Charsets) — TOP PRIORITY

> Needed for font-gen, canvas work, and braille/block art modes.

- [ ] `3.0-C.1` Braille charset block
  - **Goal:** Add a "Braille" charset group covering all 256 Unicode Braille
    Pattern characters (U+2800–U+28FF). Expose in font-gen and canvas charset
    picker. Reference: `throbber-widgets-tui` uses these for spinners.
  - **Touches:** `figby-rs/src/font_gen.rs`, `figby-rs/src/tui/palette.rs`
  - **Success:** All 256 braille cells available as canvas characters.
  - **Difficulty:** Low

- [ ] `3.0-C.2` Block elements charset
  - **Goal:** Add charset group for block elements:
    - Full/half blocks: U+2580–U+259F (▀▄█▌▐ etc.)
    - Quadrant blocks: U+2596–U+259F
    - Vertical eighths: U+2581–U+2588 (▁▂▃▄▅▆▇█)
    - Horizontal eighths / left-right blocks
  - **Touches:** `figby-rs/src/font_gen.rs`, `figby-rs/src/tui/palette.rs`
  - **Difficulty:** Low

- [ ] `3.0-C.3` Box drawing + dithered charset
  - **Goal:** Add:
    - Box drawing: U+2500–U+257F (─│┌┐└┘├┤┬┴┼ and double/heavy variants)
    - Legacy dither/shade: U+2591–U+2593 (░▒▓)
    - Geometric shapes subset: U+25A0–U+25FF (▪▫■□◆◇ etc.)
  - **Touches:** `figby-rs/src/font_gen.rs`, `figby-rs/src/tui/palette.rs`
  - **Difficulty:** Low

- [ ] `3.0-C.4` Ogham charset
  - **Goal:** Add Ogham script block U+1680–U+169F for decorative use.
    Ogham characters are used in some ASCII art and terminal art styles.
  - **Touches:** `figby-rs/src/font_gen.rs`, `figby-rs/src/tui/palette.rs`
  - **Difficulty:** Low

- [ ] `3.0-C.5` "Deluxe" meta-charset
  - **Goal:** Combine all of the above (ASCII printable + box drawing + block
    elements + dither + quadrants + braille + Ogham) into a single "Deluxe"
    preset selectable from the charset picker. Useful for maximum expressive
    range when generating or painting.
  - **Touches:** `figby-rs/src/font_gen.rs`, `figby-rs/src/tui/palette.rs`
  - **Difficulty:** Low

---

## Phase 3.0-L (Dynamic Lighting — Future/Exploratory)

> Long-horizon research item. No implementation until v4.x. Design only for now.

- [ ] `3.0-L.1` Dynamic lighting system — initial design
  - **Goal:** Design spec for a "dynamic lighting" system:
    - Normal-map generation for a FIGfont or canvas ASCII image
    - Per-palette LUT for light/shadow color remapping
    - Scene lights: point, directional, ambient; with position, color, intensity
    - Per-object flags: `accepts_lighting`, `casts_shadow`
    - Shadow casting (raycast on ASCII grid)
    - Output: live-updated palette swap + character intensity mapping
  - **Touches:** `docs/lighting-design.md` (new doc, no code)
  - **Success:** Written design document. Not implemented.
  - **Difficulty:** Low (design); Very High (implementation — v4+)

---

## Phase 3.6 — Major Release

- [ ] `3.6.1` Full regression against C FIGlet 2.2.5
  - **Goal:** All FIGlet features produce identical output. Image/TUI/
    animation verified via manual review.
  - **Touches:** Test infrastructure
  - **Success:** 100% FIGlet output compatibility.
  - **Difficulty:** Medium

- [ ] `3.6.2` v3 major milestone RC — human sign-off
  - **Goal:** RC for v3.0.0. Ralph halts. Human reviews.
  - **Touches:** RC branch, annotated tag
  - **Success:** `rc/3.0.0-rc.1` created. Human merges.
  - **Difficulty:** Low
  - **Model:** Human
