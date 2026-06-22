# Figby v6 — Pre-Release Hardening & Polish

Milestone goal: Clear every release blocker from the 2026-06-18 codebase audit
before UI/UX polish, public docs, and crates.io publish.

Source: `docs/codebase-audit-2026-06-18.md` (read it for full rationale; finding
IDs B0/B1/.../A1/S1 below map 1:1 to that doc). Severity: 🔴 blocker, 🟠 arch,
🟡 smell.

**Fix order is intentional — do phases in sequence.** 6.0 (security) and 6.1
(green tests) gate everything. Confirm `cargo test` GREEN + `cargo clippy
--all-targets` clean after every task.

---

## Phase 6.0 — Critical Security (🔴 do first)

- [x] `6.0.1` Remove `$(cmd)` command substitution from template resolver (B0/RCE)
  - **Goal:** `resolve_text_value` runs `sh -c <cmd>` for any `$(...)` in a
    template text value — rendering a shared `.ftmp` executes its embedded shell.
    Remove the `$(...)` branch entirely (recommended), OR gate behind a
    default-OFF `--unsafe-template-exec` flag that is NEVER honored for non-local
    templates. Treat `.ftmp` as untrusted data by default.
  - **Touches:** `figby-rs/src/template.rs:160-187` (the `strip_prefix('(')`
    branch in `resolve_text_value`); remove/rewrite the unit tests at
    `template.rs:1034-1058` that assert `$(echo …)` runs.
  - **Success:** New security test asserts `$(...)` does NOT execute by default
    (literal passthrough or error). `${VAR}` env expansion may stay but document
    the leak risk; consider gating it too.
  - **Difficulty:** Medium

- [x] `6.0.2` Sandbox `{{img:PATH}}` template image paths (B0 adjacent)
  - **Goal:** `render_template` reads an arbitrary local path via
    `rascii_art::render_to(&img.source)` — a shared template can read/exfil-render
    any file the user can read. Restrict image source to the template's own
    directory (reject absolute paths and `..` traversal).
  - **Touches:** `figby-rs/src/template.rs:571-590`.
  - **Success:** Template referencing `/etc/passwd` or `../../x` is rejected;
    same-dir relative image still renders. Add a test.
  - **Difficulty:** Medium

- [x] `6.0.3` Cap template canvas dimensions (B7/DoS)
  - **Goal:** `render_template` allocates `vec![vec![' '; width]; height]` from
    unvalidated `u32` frontmatter; a crafted `width=4000000000` → OOM. Clamp
    `width`/`height` (e.g. `width*height <= 1_000_000` cells) and `margin`/
    `padding` to sane maxima at the top of `render_template`.
  - **Touches:** `figby-rs/src/template.rs:517-526` (alloc), `:673-686`
    (margin/padding `repeat`).
  - **Success:** Oversize-dimension template returns an error instead of OOM;
    add a test asserting the cap.
  - **Difficulty:** Low

---

## Phase 6.1 — Green the Test Suite (🔴 blocker B3)

> All 10 failures are STALE TESTS, not app bugs (confirmed in audit). No
> production logic change expected. See audit B3 "Refined diagnosis."

- [x] `6.1.1` Fix welcome-gate stale tests (mode/tool cluster)
  - **Goal:** 4 tests press keys without dismissing the welcome screen, so keys
    route to welcome dispatch (gate at `mod.rs:2427`, `WelcomeScreen.show`
    defaults true). Add `app.welcome_screen.show = false;` after `TuiApp::new()`
    in each (pattern already used by the passing quit sub-test at `tui.rs:72`).
  - **Touches:** `figby-rs/tests/tui.rs` — `test_tui_mode_switching` (:60),
    `test_tool_selection_roundtrip` (:131),
    `test_image_editor_mode_switch_and_toggle` (:2139),
    `test_palette_fg_keyboard_shortcut` (:2457).
  - **Success:** All 4 pass; no production code touched.
  - **Difficulty:** Low

- [x] `6.1.2` Fix layers-model stale tests (poke active layer, not composite)
  - **Goal:** Tests write/read `editor.canvas.buffer` (composite output), but the
    app now sources from `layer_stack` + `recomposite_canvas`. Rewrite to write
    to / read the active LAYER buffer.
  - **Touches:** `figby-rs/tests/tui.rs` — `test_fill_tool_keyboard` (:881),
    `test_selection_perimeter_delete` (:2552), `test_tui_smoke_all_panels_render`
    (:47), `test_palette_render_contains_labels` (:617),
    `test_settings_toggle_visibility` (:749).
  - **Success:** All 5 pass; `Selection`/`flood_fill` production logic unchanged
    (already proven correct by passing unit tests).
  - **Difficulty:** Medium

- [x] `6.1.3` Fix shadow round-vs-truncate lib test
  - **Goal:** `palette_editor::test_load_current_from_palette` asserts `#4D0000`
    but `default_shadow_hex` truncates (`255*0.3=76.4→76=#4C0000`). Pick one:
    fix the test to `#4C0000`, OR change impl to `.round()`. Recommend `.round()`
    (matches user expectation) + update test.
  - **Touches:** `figby-rs/src/palette_import.rs:38`,
    `figby-rs/src/tui/palette_editor.rs:889`.
  - **Success:** lib test green; `cargo test` fully GREEN (0 fail).
  - **Difficulty:** Low

---

## Phase 6.2 — CI & Merge Gate (🔴 blocker — stops RED ever merging again)

- [x] `6.2.1` Add hard `cargo test` gate to ralph merge phase
  - **Goal:** ROOT CAUSE of B3 — `phase_review_and_merge` auto-merges on an LLM
    "approved" string with no real test run. Add a literal
    `cargo test --manifest-path figby-rs/Cargo.toml || { abort merge; }` (plus
    clippy/fmt) BEFORE the LLM review and before each task merge.
  - **Touches:** `scripts/ralph.sh:540-606`.
  - **Success:** A deliberately-failing test blocks the merge step.
  - **Difficulty:** Low

- [x] `6.2.2` GitHub Actions CI (fmt + clippy -D warnings + test)
  - **Goal:** New workflow runs `cargo fmt --check`, `cargo clippy --all-targets
    -- -D warnings`, `cargo test` on push/PR. Must be green to merge. Delete
    legacy `.travis.yml`.
  - **Touches:** new `.github/workflows/ci.yml`; remove `.travis.yml`.
  - **Success:** CI green on a clean branch; red on an intentional break.
  - **Difficulty:** Low

---

## Phase 6.3 — Parser Hardening (🔴 META — security backbone)

> Copy `palette_import.rs`'s pattern (checked_add, per-block bounds, count caps).
> It is the model — bring the others up to that bar. Extend `tests/fuzz.rs`
> (currently fonts only) with a target per parser.

- [x] `6.3.1` Validate FIGfont header numerics (B1)
  - **Goal:** `height/baseline/maxlength` parsed as `i32` then `as u32` — negative
    height → huge `charheight`, `height==0` accepted. Validate `height` in
    `1..=255`, reject negative `baseline`/`maxlength`, clamp `maxlength`.
  - **Touches:** `figby-rs/src/font.rs:261-326`.
  - **Success:** Crafted `.flf` with negative/zero/huge height rejected; fuzz
    target added.
  - **Difficulty:** Medium

- [x] `6.3.2` Cap zip decompression size (B2/zip-bomb)
  - **Goal:** `extract_first_zip_entry` / `read_zip_entry` call `read_to_end()`
    with no limit. Cap via `entry.size()` check or `take(MAX)`. (Path-traversal
    already defended.)
  - **Touches:** `figby-rs/src/font.rs:464-486`, `:526`.
  - **Success:** Small zip-bomb font rejected before exhausting memory; test.
  - **Difficulty:** Low

- [x] `6.3.3` Fix GIF memory-guard timing (B4/DoS)
  - **Goal:** `MAX_TOTAL_CELLS` is checked AFTER the read loop already cloned every
    frame. Check `width*height <= CAP` before the loop (dims known at `:69-70`);
    track `frame_count` in-loop and bail the moment `w*h*count` exceeds cap. Add a
    defensive length check on `frame.buffer[idx]` (`:199,224`).
  - **Touches:** `figby-rs/src/gif_import.rs:69-95,199,224`.
  - **Success:** Oversize GIF bails during decode; gif fuzz/oversize test added
    (module currently has 0 tests — S4).
  - **Difficulty:** Medium

- [x] `6.3.4` Range-check control-file group indices (B5/panic)
  - **Goal:** `state.gl = d - b'0'` / `gr` with no range check → byte `l 9` or
    `< '0'` → index ≥4 into `[u32;4]` or underflow → panic. Validate `d` is
    `b'0'..=b'3'` before assigning (ignore/clamp otherwise).
  - **Touches:** `figby-rs/src/control.rs:544,551` (and the `gn[..]` indexing at
    `:204-215`).
  - **Success:** Crafted `.flc` no longer panics; fuzz target added.
  - **Difficulty:** Low

- [x] `6.3.5` Limit image decode dimensions (B6/DoS)
  - **Goal:** `image::open(path)?` applies no `Limits`. Use
    `image::io::Reader::open()?.limits(Limits::default())` / set max
    width·height for both image-to-ASCII and TUI image import.
  - **Touches:** `figby-rs/src/image_input.rs:25,48`.
  - **Success:** Huge/decompression-bomb image rejected; test.
  - **Difficulty:** Low

---

## Phase 6.4 — Stale Docs (🟠 A2 — fix before release)

- [x] `6.4.1` Rewrite CLAUDE.md to match current source layout
  - **Goal:** Says "Current milestone v3"; references deleted
    `tui/components/{file_ops,font_editor,canvas}` wrappers (only `canvas.rs`,
    `status_bar.rs` remain); lists `font.rs`/`render.rs` under `tui/` (they're
    crate root). Update milestone to v6, fix the source-layout tree.
  - **Touches:** `figby-rs/CLAUDE.md` (or repo-root `CLAUDE.md`).
  - **Success:** Every path in CLAUDE.md exists; milestone current.
  - **Difficulty:** Low

- [x] `6.4.2` Fix AGENTS.md file-structure tree
  - **Goal:** Lists `src/util.rs` (does not exist) + outdated tree.
  - **Touches:** `AGENTS.md`.
  - **Success:** Tree matches actual `figby-rs/src/`.
  - **Difficulty:** Low

---

## Phase 6.5 — Correctness / Robustness (🟡 nice-to-fix this milestone)

- [x] `6.5.1` Replace `render.rs:14` `.expect()` with blank-glyph fallback (S1)
  - **Goal:** `lookup_char` `.expect()`s on missing char 0 — only production
    expect in the crate. A hand-edited font (font editor) could violate the
    char-0 invariant → panic. Return a blank glyph instead.
  - **Touches:** `figby-rs/src/render.rs:11-15`.
  - **Success:** Font missing char 0 renders blank, no panic; test.
  - **Difficulty:** Low

- [x] `6.5.2` Compile-time validate embedded ICONS_YAML (A3)
  - **Goal:** `TuiApp::new` does `serde_yaml::from_str(ICONS_YAML)
    .unwrap_or_default()` — malformed embedded YAML silently drops all icons.
    Add a build/`const` test that parses ICONS_YAML and fails compilation/CI on
    error.
  - **Touches:** `figby-rs/src/tui/mod.rs:405` + a new test.
  - **Success:** Breaking the YAML fails a test, not silently empties icons.
  - **Difficulty:** Low

- [x] `6.5.3` Clamp `font_gen` point_size + add file-path tests (S5)
  - **Goal:** `point_size: f32` unbounded → `charheight`/canvas allocs scale with
    it. Clamp to e.g. `4.0..=200.0`. `font_file_to_figfont` (the .ttf/.otf path
    variant) has 0 tests — add a bundled-font smoke test + a malformed-bytes test.
  - **Touches:** `figby-rs/src/font_gen.rs:566-577` (+ `render_font_glyphs`).
  - **Success:** Out-of-range point_size clamped; both new tests pass.
  - **Difficulty:** Low

---

## Phase 6.6 — Architecture (🟠 A1 — LARGE, may slip past v6)

- [ ] `6.6.1a` Group lighting fields into `LightingState` sub-struct
  - **Goal:** Extract 5 lighting fields from `TuiApp` into `pub struct LightingState`:
    `scene` (was `lighting_scene`), `lut` (was `lighting_lut`), `max_shadow_distance`,
    `height_scale`, `panel` (was `light_panel`). Add `pub lighting: LightingState` to
    `TuiApp`. Shrinks borrow surface; no behavior change.
  - **Touches:** `figby-rs/src/tui/mod.rs` only — 28 self-access rewrites + struct/new().
    No test changes needed (tests don't access these fields directly).
  - **Note:** Rust NLL handles field-split borrows (`self.lighting.scene` + `self.lighting.panel`
    in same block). If compiler rejects, introduce `let idx = self.lighting.panel.selected_index;`
    before the mutable scene borrow.
  - **Success:** Compiles clean; `cargo test` green; `TuiApp` has 5 fewer top-level fields.
  - **Difficulty:** Low

- [ ] `6.6.1b` Group animation/particle fields into `AnimationState` sub-struct
  - **Goal:** Extract from `TuiApp` into `pub struct AnimationState`:
    `particle_system`, `emitter_active`, `emitter_panel`, `show_live_particles`,
    `baked_layer_indices`, `timeline_state`, `timeline_visible`, `marker_accum`.
    Add `pub animation: AnimationState` to `TuiApp`.
  - **Touches:** `figby-rs/src/tui/mod.rs` only — ~95 self-access rewrites + struct/new().
  - **Success:** Compiles clean; `cargo test` green; 8 fewer top-level fields.
  - **Difficulty:** Low

- [ ] `6.6.1c` Group drag/interaction fields into `InteractionState` sub-struct
  - **Goal:** Extract from `TuiApp` into `pub struct InteractionState`:
    `selection_drag_origin`, `selection_polygon_points`, `selection_lasso_points`,
    `prev_mouse_buf`, `mouse_batch_active`, `line_start`, `saved_buffer`.
    Add `pub interaction: InteractionState` to `TuiApp`.
  - **Touches:** `figby-rs/src/tui/mod.rs` only.
  - **Success:** Compiles clean; `cargo test` green; 7 fewer top-level fields.
  - **Difficulty:** Low

- [ ] `6.6.1d` Extract `render_light_panel` → method on `LightPanel`
  - **Goal:** Move `fn render_light_panel(&self, frame, area)` (~62 LOC, `:1256-1317`)
    from `TuiApp` impl into `light_panel.rs` as `LightPanel::render(...)`. Call site
    in `TuiApp::render` becomes `self.lighting.panel.render(frame, area, &self.lighting.scene, &self.theme)`.
  - **Touches:** `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/light_panel.rs`.
  - **Success:** Compiles clean; render behavior identical; `mod.rs` -62 LOC.
  - **Difficulty:** Low

- [ ] `6.6.1e` Extract `render_overlays` → `tui/overlays.rs`
  - **Goal:** Move `fn render_overlays(&mut self, frame)` (~147 LOC, `:1318-1464`)
    from `TuiApp` impl to a new file `figby-rs/src/tui/overlays.rs` as a free function
    or `TuiApp` extension trait. Pure render logic, no state mutation beyond `dirty`.
  - **Touches:** `figby-rs/src/tui/mod.rs`, new `figby-rs/src/tui/overlays.rs`.
  - **Success:** Compiles clean; `mod.rs` -147 LOC.
  - **Difficulty:** Low

- [ ] `6.6.1f` Extract lighting-mode key dispatch → `LightingState::handle_key`
  - **Goal:** The lighting-mode block in `handle_key_event` (`:2848-3025`, ~177 LOC)
    reads/mutates almost exclusively `self.lighting.*`. Move it to
    `LightingState::handle_key(&mut self, key, w, h) -> bool` (returns true if consumed).
    Call site: `if self.lighting.handle_key(key, w, h) { return None; }`.
  - **Touches:** `figby-rs/src/tui/mod.rs` only (method on `LightingState` defined in same file
    or in `lighting.rs`).
  - **Note:** Requires 6.6.1a complete first (needs `LightingState` struct).
  - **Success:** Compiles clean; lighting behavior unchanged; `handle_key_event` -177 LOC.
  - **Difficulty:** Medium

- [ ] `6.6.1g` Extract font-editor key dispatch → `FontEditor::handle_key`
  - **Goal:** Identify the font-editor block in `handle_key_event` and move it to
    `font_editor.rs` as a method. Large block — map it first, then extract.
  - **Touches:** `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/font_editor.rs`.
  - **Success:** Compiles clean; font-editor behavior unchanged; `handle_key_event` shrinks.
  - **Difficulty:** High

- [ ] `6.6.1h` Extract image-editor key dispatch → `ImageEditor::handle_key`
  - **Goal:** Same pattern as 6.6.1g for the image-editor block.
  - **Touches:** `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/image_editor.rs`.
  - **Success:** Compiles clean; image-editor behavior unchanged; `handle_key_event` shrinks.
  - **Difficulty:** High

---

## Phase 6.7 — Critical UX Bugs (🔴 — fix before any release)

> Source: manual testing notes (4.0-manual-testing-notes.md, 5.0-manual-testing-notes.md).
> These are crashes and data-loss risks — gate release on all green.

- [ ] `6.7.1` Fix text tool: keybinds eat input keys (manual-note #16)
  - **Goal:** Typing in Text tool mode routes keystrokes through the global keybind
    handler before the text input buffer receives them. Most printable keys are
    consumed by shortcuts (e.g. `b`=brush, `e`=erase, `f`=fill), so users cannot
    type normally. Text tool must suppress all non-modifier keybinds while in text
    input state; only Esc/Enter/Arrow/Backspace/Delete should pass through as
    control keys.
  - **Touches:** `figby-rs/src/tui/mod.rs` — `handle_key_event` text-tool branch;
    ensure text-input state is checked BEFORE global tool/mode dispatch.
  - **Success:** User can type `"Hello world"` in text tool without letters being
    swallowed. Test: simulate `t` → type `"abc"` → assert canvas contains `abc`.
  - **Difficulty:** Medium

- [ ] `6.7.2` Prompt to save on exit if unsaved changes (manual-note #18)
  - **Goal:** Pressing `q`/`Esc` quits immediately, silently discarding unsaved work.
    Show a confirmation dialog ("Unsaved changes — save before quitting? [Y]es / [N]o /
    [C]ancel") when `app.dirty` is true. Honour the three choices.
  - **Touches:** `figby-rs/src/tui/mod.rs` quit path; add `dirty` flag tracking (set
    on any canvas/font/layer mutation, clear on save).
  - **Success:** Edit something, press `q` → dialog appears. Save → quit. No → quit
    without save. Cancel → stays in editor.
  - **Difficulty:** Medium

- [ ] `6.7.3` Fix panic on direct Unicode input of Deutsch chars (e2e-test-checklist #8.1)
  - **Goal:** Typing ÄÖÜäöüß directly (not via keyboard reroute `-D`) panics with
    "missing char code 0" — fixed for render path by 6.5.1 blank-glyph fallback,
    but the TUI may still panic on these code points. Verify 6.5.1 covers the TUI
    path; if not, apply same fallback in the TUI render call.
  - **Touches:** `figby-rs/src/tui/mod.rs` text rendering; `figby-rs/src/render.rs`.
  - **Success:** Typing or pasting ÄÖÜäöüß in TUI text tool renders blank/unknown
    glyph — no panic.
  - **Difficulty:** Low

---

## Phase 6.8 — Missing Core Features (🟠 — needed for v6)

> Features that are present in menus/welcome screen but either do nothing or are
> entirely absent from the UI. All are referenced in manual testing notes.

- [ ] `6.8.1` Open image: implement file dialog (manual-notes #8, #11)
  - **Goal:** "Open Image" on the welcome screen and `o`/`O` in image editor mode
    currently do nothing (welcome) or activate a bare path-entry prompt with no
    directory browser. Implement a proper file dialog: show current directory listing,
    allow arrow-key navigation into subdirectories, filter by image extensions
    (png/jpg/gif/bmp/webp), `Enter` to confirm, `Esc` to cancel. Reuse the save
    dialog's directory-browser widget if one exists.
  - **Touches:** `figby-rs/src/tui/file_ops.rs`; `figby-rs/src/tui/welcome.rs`;
    image-open handler in `tui/mod.rs`.
  - **Success:** Welcome screen "Open Image" → file dialog → select image → loads into
    image editor. Path entry mode also replaced or augmented with directory browser.
  - **Difficulty:** Medium

- [ ] `6.8.2` New image dialog: canvas size + palette selection (manual-note #10)
  - **Goal:** "New Image" creates a canvas with hard-coded defaults. Add a creation
    dialog with fields: Width, Height (default 80×24), and a palette dropdown
    (list of built-in palette names). Tab/arrow to navigate fields, Enter to confirm.
  - **Touches:** `figby-rs/src/tui/mod.rs` new-image handler; new dialog widget.
  - **Success:** New Image → dialog appears → user enters 120×40 → canvas created at
    that size. Palette selection populates the palette panel.
  - **Difficulty:** Medium

- [ ] `6.8.3` Canvas size: add Edit Canvas Size action (manual-notes #7, #9) *(status bar display already implemented)*
  - **Goal:** No part of the UI shows current canvas dimensions. (1) Add `WxH` to the
    status bar. (2) Add "Edit Canvas Size" to the Image menu (or View menu) plus a
    keybind; opens a small dialog to resize (with crop/pad options). Resize should not
    destroy existing content — pad with spaces or crop from edges.
  - **Touches:** `figby-rs/src/tui/components/status_bar.rs`; menu definitions in
    `tui/mod.rs`; canvas resize logic in `tui/canvas.rs`.
  - **Success:** Status bar shows dimensions. Resize action changes canvas and content
    is preserved (padded/cropped correctly).
  - **Difficulty:** Medium

- [ ] `6.8.4` Add palette editor UI (manual-note #23; original spec items 5.6.3–5.6.4)
  - **Goal:** No palette editor exists: no keybind, no menu entry, no icon buttons in
    the palette toolbox. Implement palette editor panel: add/remove/edit colors, name
    the palette, import from file. Add icon buttons to palette toolbox (new color,
    edit color, delete color). Add keybind and menu item (View → Palette Editor or
    palette toolbox context action).
  - **Touches:** `figby-rs/src/tui/palette.rs`; `figby-rs/src/tui/toolbox.rs`; new
    `tui/palette_editor_panel.rs` (or extend existing `palette_editor.rs`).
  - **Success:** Can open palette editor, add a new color, rename it, close. Color
    appears in palette panel and can be used for drawing.
  - **Difficulty:** High

- [ ] `6.8.5` Default palettes: ship ~5-shade-per-hue built-in palettes (manual-note #17)
  - **Goal:** No built-in palettes exist. Add at minimum: a grayscale ramp, a primary
    color palette, and one warm + one cool themed palette. Each should have ~5 shades
    per hue. Palettes available in the new-image dialog (6.8.2) and via View menu.
  - **Touches:** `figby-rs/src/palette_import.rs`; add palette definitions as
    embedded YAML/TOML constants.
  - **Success:** Palettes appear in dropdown; selecting one populates the palette panel.
  - **Difficulty:** Low

- [ ] `6.8.6` Lighting tool: surface or implement (manual-note #14)
  - **Goal:** Lighting tool referenced in original spec and docs/lighting-design.md
    but absent from TUI tool palette, keybinds, and menus. Either: (a) implement the
    tool with basic directional-lighting preview if the logic exists in
    `lighting-design.md`, or (b) add a clearly-disabled placeholder in the toolbox
    with a "not yet implemented" tooltip so it's discoverable. At minimum, add a
    menu item under Tools so users know it's planned.
  - **Touches:** `figby-rs/src/tui/toolbox.rs`; `figby-rs/src/tui/tools/`; menu.
  - **Success:** Lighting tool visible in toolbox (even if greyed). If implemented:
    select it, observe canvas lighting effect.
  - **Difficulty:** High (full impl) / Low (placeholder)

- [ ] `6.8.7` Keybinds popup: make scrollable + add missing keybinds (manual-note #15)
  - **Goal:** Keybinds help popup is not scrollable; many keybinds (layer operations,
    animation controls, palette editor, text tool sub-commands) are missing. Make
    the popup scrollable (arrow keys or PgUp/PgDn). Audit all keybinds in
    `handle_key_event` and add each to the help data structure.
  - **Touches:** `figby-rs/src/tui/mod.rs` keybinds popup render + event handler.
  - **Success:** Popup scrolls. All keybinds visible across all modes.
  - **Difficulty:** Low

- [ ] `6.8.8` Timeline and layer panel: make scrollable (manual-note #19)
  - **Goal:** With many layers or animation frames, timeline and layer panel overflow
    and clip. Add scroll support (arrow keys + mouse wheel) to both panels. Show a
    scroll indicator when content overflows.
  - **Touches:** `figby-rs/src/tui/layers.rs`; animation timeline widget in
    `tui/mod.rs`.
  - **Success:** Add 20+ layers — layer panel scrolls. Animate 20+ frames — timeline
    scrolls.
  - **Difficulty:** Medium

---

## Phase 6.9 — UX & Layer Panel Polish (🟡 — nice-to-have for v6)

> Visual and interaction improvements from manual testing. None block release but
> all significantly affect first-run experience.

- [ ] `6.9.1` Layer panel: icon-based layout with 2-row entries (manual-note #12)
  - **Goal:** Layer panel is text-heavy and confusing. Each entry should be 2 rows:
    row 1 = layer name (editable on double-click), row 2 = compact attributes (icon
    for visibility, lock icon, opacity %, blend mode abbreviation). Replace verbose
    text labels with icons. Panel should be resizable (drag the border) and layers
    draggable with mouse or via keybinds.
  - **Touches:** `figby-rs/src/tui/layers.rs`; icon definitions.
  - **Success:** Layer panel visually compact. Can drag-resize panel width. Can
    reorder layers by dragging the handle on left edge of each row.
  - **Difficulty:** Medium

- [ ] `6.9.2` Layers: reorder by drag handle (manual-note #21)
  - **Goal:** No way to reorder layers. Add a drag handle (left edge of each layer row
    in the panel); mouse-drag or `Shift+Up`/`Shift+Down` keybinds reorder layers.
    Layer stack recomposites immediately after reorder.
  - **Touches:** `figby-rs/src/tui/layers.rs`; `tui/mod.rs` mouse/key handlers.
  - **Success:** Drag or Shift+Arrow reorders layers. Canvas updates immediately.
  - **Difficulty:** Medium

- [ ] `6.9.3` Add Layers menu and Timeline menu with actions (manual-note #20)
  - **Goal:** Layer actions (add, delete, duplicate, merge, move up/down, rename,
    toggle visibility/lock) exist only via keybinds — no menu. Add a Layers menu
    in the menu bar. Similarly add an Animation/Timeline menu with frame actions.
  - **Touches:** Menu bar definitions in `figby-rs/src/tui/mod.rs`.
  - **Success:** Alt+L opens Layers menu; all layer actions accessible from menu.
  - **Difficulty:** Low

- [ ] `6.9.4` Move tool options to right sidebar (manual-note #13)
  - **Goal:** Tool options (brush size, shape, opacity etc.) currently displayed below
    the toolbox on the left. User expected them in the right sidebar. Move or
    duplicate tool options panel to right sidebar, or make the location configurable.
  - **Touches:** `figby-rs/src/tui/layout.rs`; `tui/toolbox.rs`.
  - **Success:** Tool options visible in right sidebar. Left sidebar toolbox cleaner.
  - **Difficulty:** Medium

- [x] `6.9.5` Brush size/shape: show current values clearly in UI (manual-note #3)
  - **Goal:** No visible indicator of current brush size or shape. Add a small
    brush-preview widget in the toolbox or status bar showing the current size
    (number) and shape (icon: circle/square/spray). Update live as user changes
    brush settings.
  - **Touches:** `figby-rs/src/tui/toolbox.rs`; `tui/components/status_bar.rs`.
  - **Success:** Changing brush size with `[`/`]` updates the visible indicator
    immediately.
  - **Difficulty:** Low

- [ ] `6.9.6` Better visual divider between tool palette and brush info (manual-note #22)
  - **Goal:** Weak visual separation between the last tool button and the brush
    info section below it in the toolbox. Add a separator line or padding to make
    the boundary clear.
  - **Touches:** `figby-rs/src/tui/toolbox.rs` render.
  - **Success:** Clear visual break between tool list and brush info.
  - **Difficulty:** Low

---

## Deferred to post-v6 (tracked, not blocking)

Color-depth fallback (C1), reduced-motion `--no-anim` (C2), panic-hook terminal
restore + autosave (C3), perf opts (O1/O2/O3), new exports (SVG/asciinema/sixel),
template starter library, 3rd-party crate extraction
(`ratatui-paint-canvas` etc.), release tooling (cargo-dist/release-plz/VHS),
onboarding (`?`-help, which-key, tutorial), DESIGN.md + zoid-ui-kit token
alignment, **Figby→ rename/de-brand** (copyrighted name). See audit doc
🔵 Suggestions and Branding sections.
