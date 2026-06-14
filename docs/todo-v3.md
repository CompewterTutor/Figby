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

- [ ] `3.2.0` Custom ratatui widget: `AnimationTimeline`
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
