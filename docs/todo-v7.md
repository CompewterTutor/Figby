# Figby v7 — Animation Editor, Playback & Architecture

Milestone goal: Clear every unchecked finding from `docs/manual-testing-v7.md`.
The v6 pass shipped UI plumbing (layers, timeline, brushes, transport bar) but
left the animation editor *non-functional* — edits vanish on frame switch,
play/pause is disconnected from the timeline, and `mod.rs` still weighs in at
5281 LOC. v7 fixes the data-loss and playback blockers first, then does the
keymap overhaul + props-panel overhaul requested in the same notes, then the
architectural split that unblocks all later work.

Source: `docs/manual-testing-v7.md` (the unchecked `[ ]` items at lines 20–29).
Severity: 🔴 blocker, 🟠 arch, 🟡 smell.

**Fix order is intentional — 7.0 gates everything.** Confirm
`cargo test --manifest-path figby-rs/Cargo.toml` GREEN + `cargo clippy -p figby
--all-targets -- -D warnings` clean after every task.

---

## Phase 7.0 — Critical Animation Editor Bugs (🔴 do first)

> These three tasks make the animation editor actually usable. Do them in order:
> 7.0.1 (commit-on-switch) must land before any test that edits + advances a
> frame can be written; 7.0.2 (false GIF error) is small and isolated; 7.0.3
> (play/pause cursor) depends on the same `TimelineState::current_frame` field
> validated by 7.0.1.

- [x] `7.0.1` Commit timeline frame edits on switch (manual-note #6)
  - **Goal:** Editing a frame, advancing to another frame, then coming back
    loses all edits — the animation editor is useless. Two independent
    `CanvasBuffer` copies exist: the live `EditorState::layer_stack` active layer
    (where every brush/tool stroke lands) and `TimelineFrame::layer_state: Option<CanvasBuffer>`
    (the per-frame snapshot read by `load_current_timeline_frame`). Only the
    frame→layer direction is wired; the reverse direction does not exist, so
    `load_timeline_frame` at `mod.rs:181-184` unconditionally overwrites live
    edits on every switch. Introduce `commit_current_timeline_frame(&mut self)`
    which writes `self.animation.timeline_state.frames[cf].layer_state =
    Some(self.editor.layer_stack.composite())` (matches what
    `capture_timeline_frames` at `export.rs:683` consumes), recaptures the
    thumbnail, and sets `has_keyframe = true`. Call it at the **top** of the
    Left/Right/timeline-click handlers **before** mutating `current_frame`.
  - **Touches:** `figby-rs/src/tui/mod.rs` — new helper, then call sites at
    `:3374` (Left arm), `:3385` (Right arm), `:1992-2002` (timeline click).
    Reuse the existing `composite()` accessor on `LayerStack`.
  - **Success:** Edit frame 0 with brush → press Right → press Left → edits
    persist. Export shows edited frames (was stale before — `capture_timeline`
    frames also read `layer_state`, so it silently exported pre-edit snapshots).
    New regression test in `tests/tui.rs` simulates edit → switch → switch-back
    and asserts cell content survives.
  - **Open questions:** Decide whether to also commit on every tool-stroke end
    (cheaper export, costlier tick) vs only on switch. Decide whether
    `layer_keyframes` should be recaptured from `layer_stack` on commit. Decide
    whether undo/redo (`EditorState.undo` at `mod.rs:186-191`) should become
    per-frame rather than the current global stack that surfaces wrong snapshots
    after a frame switch. Park all three as follow-ups under 7.3 architecture.
  - **Difficulty:** Medium

- [x] `7.0.2` Fix false "Cannot read file: stream did not contain valid UTF-8" on GIF import (manual-note #1)
  - **Goal:** Importing an animated GIF (a) emits a bogus
    `Cannot read file: stream did not contain valid UTF-8` error and (b) leaves
    the file-open dialog open so the user must press Escape to dismiss it —
    even though `gif::Decoder` already successfully decoded the GIF. Root cause:
    `mod.rs:2868` calls `perform_import_gif(path)` (binary, correct) and then
    returns `Some(AppEvent::OpenRequested)`; the dispatcher at `mod.rs:1016`
    unconditionally forwards `OpenRequested` to `perform_open()`, which at
    `mod.rs:4025` calls `std::fs::read_to_string(&path)` on the *binary* GIF —
    UTF-8 decode fails → false error. Separately, `perform_import_gif`'s
    success path at `mod.rs:4158-4243` never resets
    `self.dialogs.file_ops.mode = FileOpsMode::Idle`, so the dialog stays open
    regardless.
  - **Touches:** `figby-rs/src/tui/mod.rs` — drop the
    `return Some(AppEvent::OpenRequested)` at `:2869` (the ImportGif arm should
    return `None`); in `perform_import_gif`'s `Ok(gif_data)` branch set
    `self.dialogs.file_ops.mode = FileOpsMode::Idle`. Optionally also stop the
    `OpenRequested` dispatcher from running for already-handled import paths.
  - **Success:** Import an animated GIF → no error appears; the open dialog
    dismisses itself; the timeline populates and the editor lands on frame 0.
    Manual test confirms no `Escape` press is needed. Existing GIF import
    tests still pass.
  - **Difficulty:** Low

- [x] `7.0.3` Reconcile playback cursor with timeline `current_frame` (manual-note #3)
  - **Goal:** Play/pause is "weirdly separated" from the timeline. The
    `AnimationPlayer::current_frame: Cell<usize>` at `player.rs:29` and the
    `TimelineState::current_frame: usize` at `timeline.rs:165` are two
    independent cursors that never reconcile. `play_inline` at `mod.rs:4574`
    seeds the player cursor from the timeline cursor exactly once at play
    start; the tick handler at `mod.rs:936` (`player.advance(elapsed)`) writes
    only the player's `Cell`; `pause()`/`seek()`/`set_speed()` likewise touch
    only the player. Net effect from the user's POV: timeline strip stays
    frozen on the play-start frame during playback, and on Esc/q/transport-Stop
    `stop_inline_playback` at `mod.rs:4603-4606` drops `inline_player = None`
    outright, after which `render_canvas_area` falls through at `mod.rs:1388`
    and composites whatever `timeline_state.current_frame`'s buffer holds —
    i.e. the play-start frame, **not** the frame the player was actually
    showing when the user paused. Make play advance the *shared* frame
    counter and display the frame it's on; make pause stop at that frame —
    like every other animation app in history.
  - **Touches:** `figby-rs/src/tui/mod.rs` — in `run()`'s tick branch
    (`:933-949`), after `player.advance(elapsed)`, set
    `self.animation.timeline_state.current_frame = player.current_frame();`
    so the timeline indicator tracks playback. In `stop_inline_playback`
    (`:4603-4606`), before nulling `inline_player`, copy
    `player.current_frame()` into `timeline_state.current_frame` and call
    `self.load_current_timeline_frame()` (`:4058`) so the canvas holds the
    last frame the player rendered. Drop the redundant `self.seek(0)` in the
    player's own Esc arm (`player.rs:228`) so an Esc-triggered dismiss also
    lands on the last-shown frame. Drop the vestigial `TimelineState::playing`
    field at `timeline.rs:167` (per the comment at `mod.rs:4900` nothing reads
    it).
  - **Success:** Start playback from frame 2 → pause mid-playback at frame 5
    → canvas still shows frame 5 and the timeline marker sits on frame 5.
    Press play → resumes from frame 5. Press Stop → canvas stays at last
    displayed frame. New test asserts `timeline_state.current_frame ==
    player.current_frame()` immediately after the first tick, and asserts the
    visible buffer equals the player's last frame after `stop_inline_playback`.
  - **Difficulty:** Medium

---

## Phase 7.1 — Dialog & Keymap Polish (🔴 — fix before any release)

- [x] `7.1.1` Fix quit-confirm dialog sizing + add mouse input (manual-note #2)
  - **Goal:** The "Unsaved changes — save before quitting?" dialog has two
    bugs. (a) Text is cut off: outer width is hard-coded to 52 (`overlays.rs:185`)
    but the hint line at `overlays.rs:211`
    `"  [Y] Save and quit   [N] Discard and quit   [C] Cancel"` is 55 chars;
    after `Borders::ALL` the inner width is 50, so the right portion of
    "[C] Cancel" clips. (b) Mouse clicks do nothing — `handle_mouse_event`
    at `mod.rs:1822` has no `quit_confirm_dialog` branch; only the keyboard
    handler at `mod.rs:2734-2754` consults `self.quit_confirm_dialog`, so
    clicks pass through to whatever widget the falling-through branches hit.
  - **Touches:** `figby-rs/src/tui/overlays.rs:182-214` — compute width from
    the longest line (or bump the constant to `>= 57`) and render three
    distinct button rects (`Y`/`N`/`C`) with stored geometry. `figby-rs/src/tui/mod.rs`
    — add an early `if self.quit_confirm_dialog { /* hit-test the three button
    rects and dispatch Save/Discard/Cancel, mirroring `:2737-2751` */ return; }`
    branch at the top of `handle_mouse_event`. Store the button rects into a
    field on `TuiApp` (e.g. `dialogs.quit_confirm_buttons: [Rect; 3]`) so the
    render site can publish them and the mouse site can read them.
  - **Success:** Quit dialog shows the full hint line at 80 and 60 col
    terminals. Click on `[Y]` → starts save + quit; `[N]` → discard + quit;
    `[C]` → cancel and stay in editor. Keyboard path unchanged.
  - **Difficulty:** Medium

- [x] `7.1.2` Rebind sidebar/layer-panel chrome to Alt+arrows; free bare arrows for timeline + canvas (manual-note #4)
  - **Goal:** Too many overlapping shortcuts. Bare Left/Right is consumed by
    the side-panel tab cycle at `mod.rs:3357/3362` whenever the sidebar is
    open, so timeline frame advance (`mod.rs:3374/3385`) silently does
    nothing while the sidebar is open. The investigate report enumerates 20+
    consumers of the bare arrow keys; the chrome (sidebar, layer panel,
    toolbox tab strip) must move to Alt-modified keys so bare arrows always
    mean timeline/canvas content.
  - **Touches:** `figby-rs/src/tui/mod.rs` — remove the bare-arrow
    side-panel tab-cycle block at `:3354-3365`; rebind to `Alt+Left`/`Alt+Right`
    unconditionally. `figby-rs/src/tui/layers.rs:856/862/924/933/959` (and
    `:848/852` Shift-up/down reorder) — gate every arrow/Tab handler behind
    `modifiers == KeyModifiers::ALT`; Shift-reorder becomes
    `Alt+Shift+Up/Down`. Move the palette-nav block (`mod.rs:3694-3697`)
    above the canvas block (`mod.rs:3476`) or gate canvas arrows on
    `!palette.has_focus` so palette arrows aren't dead. Fix the duplicate
    `T` handling (`keymap.rs:109` → `mod.rs:3609` direct) and the bare `S`
    collision (settings at `mod.rs:3574` vs layer-panel cast-shadow at
    `layers.rs:880`) by requiring Alt on the layer-panel path. Update
    `figby-rs/src/tui/keymap.rs` (KEYMAP display entries + the global action
    enum) to register the new Alt bindings and the lighting-mode bindings
    (`G` enter, `A/D/P` add light, `+/-` intensity, `Del` remove, arrows
    move point light) which are currently undocumented in-app.
  - **Success:** With sidebar open and Layers tab focused, bare Left/Right
    advances the timeline frame; Alt+Left/Right cycles the side-panel tabs;
    bare Up/Down on a layer panel does nothing (use the mouse or Alt+Up/Down).
    Tab restores its mode-cycle behaviour even when layer groups exist
    (satisfies the already-checked `[x]` trail of manual-note item 1 about
    Layers swallowing Tab — confirm no regression). New test asserts bare
    ArrowRight with the sidebar open advances the timeline and does NOT
    change the active side-panel tab. Keymap popup lists the Alt bindings
    and the lighting bindings.
  - **Difficulty:** Medium

---

## Phase 7.2 — Tool Props Panels Overhaul (🟠 — needed for v7)

> The right-sidebar Props tab is read-only in every case. Half the tools have
> no dedicated props builder and fall through `match active_tool { _ =>
> add_tool_keybinds(...) }` at `side_panel.rs:346-348`, which renders a static
> tool-shortcut list — the "old toolbox" the manual note complains about.
> This phase makes the Props tab actually editable and gives every tool a real
> props panel.

- [x] `7.2.1` Make Props tab editable: clickable +/- rects + typed-entry mode (manual-note #5/#8)
  - **Goal:** The brush props panel at `side_panel.rs:359` (`add_brush_props`)
    displays Size/Shape/Mode/Density/Char and four keybind hint lines, all as
    inert `Span::raw` text — no widget rects, no hit testing. The only edits
    come from the global `[ ] \ ; ' Shift+M` shortcuts at `mod.rs:3621-3653`
    which fire whether or not the Props tab is open. Mouse handling in
    `handle_mouse_event` (`mod.rs:1822`) recognises only the side-panel tab
    strip and the Layers tab content (`mod.rs:1949` gating on
    `TabId::Layers`); the Props/Text/Libraries/Effects tab bodies are
    completely inert. Build an input model for the Props tab analogous to
    `palette_editor.rs::PanelMode`: clickable `+/-` rects with stored
    geometry for numeric fields (size, density, opacity), cycle-on-click
    rects for enum fields (shape, mode), and a typed-entry mode for string
    fields (custom brush char). Publish widget geometry into a `TuiApp`
    field so `handle_mouse_event` can hit-test.
  - **Touches:** `figby-rs/src/tui/side_panel.rs` — refactor
    `add_brush_props` / `add_text_props` / `add_emitter_props` /
    `add_lighting_props` to return geometry structs. `figby-rs/src/tui/mod.rs`
    — extend `handle_mouse_event` to dispatch into the Props tab when
    `active_tab == TabId::Props`. May want a new
    `tui/props_panel.rs` module to host the input-mode state and the
    per-tool builder dispatch, shrinking `side_panel.rs` in passing.
  - **Success:** Click `+` next to "Size" → brush size goes up; click `-`
    → down. Click "Shape" → cycles. Click the brush-char field → enters
    text input mode, type a single char, Enter commits. The global keyboard
    shortcuts continue to work in parallel. New test clicks each rect and
    asserts the underlying `BrushState`/`TextToolState` field changes.
  - **Difficulty:** High

- [x] `7.2.2` Add dedicated props builders for the seven hollow tools (manual-note #8)
  - **Goal:** Move, Rotate, Marquee (Select), Lasso, CircleSelect,
    PolygonSelect, and Line all currently fall through
    `side_panel.rs:346-348` to `add_tool_keybinds` and render only the static
    tool-shortcut catalogue. Each needs a real props panel:
    Move (stride / snap / wrap), Rotate (step angle / direction / pivot),
    Marquee + Lasso + CircleSelect + PolygonSelect (feather / additive /
    subtractive / move-with-arrow-keys toggle), Line (width / arrowhead /
    curve mode). Also wire `fill_threshold` (displayed at `side_panel.rs:452`
    `add_fill_props` but `grep fill_threshold =` finds only initialisers at
    `mod.rs:838/:5027` — the value is never mutated anywhere) to a real
    +/- / typed-entry handler built in 7.2.1.
  - **Touches:** `figby-rs/src/tui/side_panel.rs` — new `add_move_props`,
    `add_rotate_props`, `add_select_props` (shared by the four selection
    tools), `add_line_props`; replace the `_ => add_tool_keybinds(...)`
    fallback with an "unknown tool" placeholder line. `figby-rs/src/tui/mod.rs`
    — wire the corresponding state fields to either existing handlers or
    new ones (Rotate already has `mod.rs:3192`; the selection tools have no
    per-tool state today — add `SelectionState { feather, additive, movable }`
    to `InteractionState` if 6.6.1c is merged, else on `TuiApp`).
  - **Success:** Switch to Move tool → Props tab shows stride/snap/wrap.
    Switch to Line tool → shows width / arrowhead / curve mode. Switch to any
    selection tool → shows feather / additive / subtractive. Switch to Fill
    → fill_threshold is editable from the panel. The "old toolbox" fallback
    never renders for any first-class Tool variant.
  - **Difficulty:** Medium

---

## Phase 7.3 — Architecture: Split `mod.rs` (🟠 A1 — explicit ask in manual-note #7)

> `mod.rs` is **5281 LOC** despite the v6.6 sub-struct / extraction work.
> Manual note #7 says verbatim "files are HUGE... state management on the
> whole looks so bolted on in different places, like amateur hour." Continue
> the v6.6 extraction strategy (sub-struct → method → new file) until
> `mod.rs` is under ~1500 LOC and state flows through grouped sub-structs
> rather than a flat ~80-field `TuiApp`.

- [x] `7.3.1` Extract `handle_key_event` mode blocks into per-mode `handle_key` methods
  - **Goal:** `handle_key_event` is one giant dispatch with mode-specific
    blocks for text tool (`mod.rs:3096-3187`), rotate (`:3192-3208`),
    selection move (`:3219-3247`), move tool (`:3297-3323`), timeline frame
    advance (`:3374-3400`), lighting (`:3438-...` already moved to
    `LightingState::handle_key` under 6.6.1f), and the canvas cursor
    fallback (`:3476-3490`). Continue the pattern: extract each non-trivial
    block into a `handle_key` method on the relevant sub-struct (extending
    `EditorState` / `InteractionState` / `AnimationState` as needed). The
    dispatcher in `mod.rs` should become a short ordered list of
    `if self.<state>.handle_key(...) { return None; }` arms.
  - **Touches:** `figby-rs/src/tui/mod.rs` only at first; may grow new
    methods on existing sub-structs. No new files strictly required, but
    the text-tool block at `:3096-3187` (91 LOC) could move into
    `tui/tools/text.rs` if a `TextToolState::handle_key` is added.
  - **Success:** `handle_key_event` shrinks by ~300 LOC. Compiles clean,
    `cargo test` green, every keybind still fires. No behaviour change.
  - **Difficulty:** Medium

- [x] `7.3.2` Extract `render_canvas_area` + `render_overlays` residual blocks
  - **Goal:** `render_canvas_area` at `mod.rs:1388-1418` mixes player-widget
    dispatch with normal canvas compositing; pull the player-dispatch arm
    into `AnimationState::render` (already used for the inline player path)
    and shrink the main function. Audit `overlays.rs` (extracted under
    6.6.1e) for residual overlay logic still living in `mod.rs::render` and
    move it. Goal is `mod.rs::render` becomes "build layout, dispatch to each
    sub-struct's render, draw chrome."
  - **Touches:** `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/overlays.rs`,
    `figby-rs/src/tui/player.rs` (add `AnimationPlayer::render` already
    exists — wire the dispatch).
  - **Success:** `mod.rs::render` shrinks by ~150 LOC. Compiles clean,
    `cargo test` green, visual output byte-identical (manual diff against
    C figlet for a known banner).
  - **Difficulty:** Medium

- [x] `7.3.3` Group remaining `TuiApp` fields into sub-structs
  - **Goal:** Post-6.6.1a/b/c, `TuiApp` still carries many top-level fields
    for dialogs (`quit_confirm_dialog`, `dialogs.file_ops.*`,
    `export_dialog`, `rascii_import`), editor (`editor: EditorState` already
    grouped but its children have loose coupling), menus, theme, etc. Group
    the dialog fields into `DialogState`, audit the others, and aim for
    `TuiApp` to hold ~10 sub-struct fields instead of ~80 flat fields.
  - **Touches:** `figby-rs/src/tui/mod.rs` primarily; new
    `tui/dialog_state.rs` likely. Touch every access site.
  - **Success:** `TuiApp` field count drops by ~30. Compiles clean,
    `cargo test` green. No behaviour change.
  - **Difficulty:** Medium

- [ ] `7.3.4` Split `mod.rs` into topical submodules
  - **Goal:** After 7.3.1 / 7.3.2 / 7.3.3, `mod.rs` should be small enough
    to keep only the top-level `TuiApp` definition, `new`, `run` (event
    loop), and the high-level dispatch. Move the rest into topical files:
    `tui/app_state.rs` (struct defs + `new`),
    `tui/event_loop.rs` (the `run` body + tick logic),
    `tui/dispatch.rs` (key/mouse dispatch), keeping `mod.rs` as the
    re-export glue. Target: `mod.rs` under 1500 LOC.
  - **Touches:** `figby-rs/src/tui/mod.rs` heavily; new files
    `tui/app_state.rs`, `tui/event_loop.rs`, `tui/dispatch.rs`. Update
    `figby-rs/src/tui/mod.rs` doc comment and AGENTS.md file-structure tree
    (per task 6.4.2 convention — keep tree in sync).
  - **Success:** `wc -l figby-rs/src/tui/mod.rs` reports ≤ 1500. `cargo
    build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
    `cargo fmt --check` all green. No behaviour change (manual diff against
    a known banner).
  - **Difficulty:** High

---

## Phase 7.4 — Particle System Extensions (🟡 — explicit ask in manual-note #9, big change)

> Manual note #9 asks for inertia, vector of travel, collision layers, and
> per-particle timelines. Investigation found that **velocity + acceleration
> already exist** (`particles.rs:177-178` `vx/vy`, config `velocity_x_min/max`
> + `acceleration_x/y`) — the user did not see inertia because there is no
> visible trajectory indicator and no collision makes particles vanish off
> the canvas. The real gaps are collision response, per-particle keyframe
> tracks, and spawn/death event hooks. This is additive, not a rewrite.

- [ ] `7.4.1` Add edge + layer-cell collision response to particles
  - **Goal:** `ParticleSystem::update` at `particles.rs:285-315` advances
    position by `vx*dt`/`vy*dt` but never tests bounds; `render_to_canvas`
    at `:341-357` and `bake_to_buffer` at `:359-377` only `continue` when
    `px<0 || py<0 || px>=w || py>=h` — particles fly off-canvas and keep
    moving in dead space until their lifetime expires. Add collision
    response: configurable edge mode (bounce / wrap / despawn), optional
    layer-cell collision (treat non-blank cells as solid, reflect velocity
    along the normal computed from the 4-neighbour cell occupancy).
  - **Touches:** `figby-rs/src/tui/particles.rs` — `ParticleConfig` gains
    `edge_mode: EdgeMode`, `collide_with_layer: bool` (and a `layer_mask:
    Option<&CanvasBuffer>` runtime borrow or precomputed solid-set); update
    loop grows a collision step between `vy += ay*dt` and `x += vx*dt`.
  - **Success:** Spawn particles in a closed canvas → they bounce off the
    edges instead of disappearing. Spawn particles over a paint-stroke layer
    with `collide_with_layer=true` → they deflect off the painted cells.
    Existing particle tests still pass; new tests assert reflection vectors.
  - **Difficulty:** Medium

- [ ] `7.4.2` Per-particle keyframe tracks + lifecycle hooks
  - **Goal:** Each particle today has only a scalar
    `remaining_lifetime: f64` (`particles.rs:179`) and a global
    `ParticleSystem::age` (`:269`). Per the manual note, particles should
    have "their own timelines at some point." Add an optional
    `keyframes: Vec<ParticleKeyframe>` to the `Particle` struct, where a
    keyframe pins `color / size / character / opacity` at a fraction of
    total lifetime; the render interpolates between adjacent keyframes.
    Also add spawn/death callbacks (so a dying particle can trigger a
    secondary emitter, e.g. a burst on impact). Design doc update to
    `docs/lighting-design.md` is out of scope; write a new
    `docs/particles-design.md` sketch.
  - **Touches:** `figby-rs/src/tui/particles.rs` — `Particle` struct,
    `ParticleSystem::update` interpolation, new `ParticleKeyframe` type.
    New `docs/particles-design.md`.
  - **Success:** Configure a particle that starts red, fades to blue at
    50% lifetime, then white at 100% — render shows the colour ramp along
    its trajectory. A dying particle triggers a small secondary burst.
    New tests assert interpolated colour at 25%/50%/75% of lifetime.
  - **Difficulty:** High
  - **Note:** This phase is large and may slip past v7. Track separately if
    the release needs to ship before 7.4.2 lands.

---

## Phase 7.5 — Lighting Comprehensibility (🟡 — manual-note #10)

> Lighting "makes no sense." Investigation: the lighting engine is a real
> Lambertian + specular + shadow system (`lighting.rs:1-170`,
    `components/canvas.rs:8-120`), but **the heightmap source is empty by
    default** — `compute_normal_map_figfont` at `lighting.rs:244` is never
    wired to actual non-zero height data, the design doc's "Canvas Path"
    height-paint tool (section 2.2) is unimplemented, and toggling lights /
    direction / intensity produces almost no visible change. Compounding
    this, the light panel (`light_panel.rs:110`) renders terse labels like
    `"Amb 0.50"` / `"Dir 0.80"` / `"Pnt 0.90 (x,y)"` with no tooltips, no
    help overlay, no keybind hints, and the lighting keybinds are absent
    from `keymap.rs` entirely. Two-pronged fix.

- [ ] `7.5.1` Add lighting help overlay + register lighting keybinds
  - **Goal:** Stop gap so users can at least read what lighting does and
    how to drive it before the behaviour fix lands. In `light_panel.rs:110`
    add a help block listing the bindings (`G` enter, `Up/Down` select,
    `Left/Right` + `Shift+arrows` move point light, `+/-` intensity,
    `A/D/P` add ambient/directional/point light, `Delete` remove, `Esc`
    exit). Register all of these in `keymap.rs` (global action enum has no
    Lighting entries today). Show a one-shot hint on enter-mode using the
    existing hint machinery at `mod.rs:1092`.
  - **Touches:** `figby-rs/src/tui/light_panel.rs:63` (`build_lines`) and
    `:110` (`render`); `figby-rs/src/tui/keymap.rs` — new
    `LightingAction` enum + KEYMAP entries; `figby-rs/src/tui/mod.rs` —
    one-shot hint on `Lighting` mode enter.
  - **Success:** Press `G` → enters Lighting mode and shows a hint listing
    every binding. Open the keybinds popup (`?`) → a Lighting section is
    present. The light panel now shows next to each light entry a tooltip
    line explaining what it does.
  - **Difficulty:** Low

- [ ] `7.5.2` Wire a real heightmap source (FIGfont density path OR height-paint tool)
  - **Goal:** The behaviour blocker. `components/canvas.rs:48-58` builds the
    heightfield from `cell.height.unwrap_or(0)` and nothing sets
    `cell.height` to non-zero, so every normal is flat `(0,0,127)` and the
    Lambertian term is constant → toggling lights is visually inert. Pick
    ONE path and ship it: (a) FIGfont density path per `docs/lighting-design.md`
    section 2.1 — derive height from glyph fill density for FIGfont-rendered
    canvases, OR (b) the height-paint tool per section 2.2 — add a new
    `tools/height.rs` brush that paints `cell.height` directly. Update the
    design doc's status line (currently "Deferred to v4.x" at
    `lighting-design.md:5`) once shipped.
  - **Touches:** `figby-rs/src/tui/lighting.rs:244`
    (`compute_normal_map_figfont` callers), `figby-rs/src/tui/components/canvas.rs:48-58`
    (heightfield build), and either `figby-rs/src/font.rs` (density path)
    or new `figby-rs/src/tui/tools/height.rs` (paint tool).
  - **Success:** Enable Lighting mode on a canvas that has either
    FIGfont-rendered text or a height-painted patch → moving a directional
    light across the canvas visibly shadows / highlights different regions.
    New test asserts `compute_normal_map_figfont` produces non-flat normals
    for a non-empty glyph.
  - **Difficulty:** High

---

## Deferred to post-v7 (tracked, not blocking)

- Frame-specific undo/redo (currently the `EditorState.undo` stack is shared
  across frames so switching frames surfaces the wrong snapshots; revisit
  after 7.0.1's commit-on-switch lands).
- Commit-on-stroke-end vs commit-on-switch policy (tradeoff noted in 7.0.1
  open questions).
- Animated GIF export optimisation (palette reduction / disposal method
  support) — current exporter is correct but not optimal.
- Particle system: vector-field emitters (wind / gravity wells beyond the
  global `acceleration_x/y`).
- Lighting: specular exponent editor, shadow softness, multi-light shadow
  blending — all in `docs/lighting-design.md`.