# Figby v5 — UI Overhaul & Feature Completion

Milestone goal: Address all manual testing findings from v4 RC review.
Redesign welcome screen, overhaul layout, fix image editor, surface animation
timeline, wire NerdFont icons throughout, powerline status bar.

Source: `docs/4.0-manual-testing-notes.md`

---

## Phase 5.0 — Welcome Screen Redesign

- [x] `5.0.1` Banner: center + wrap Computerist-20 FIGBY title
  - **Goal:** Center the FIGBY FIGfont banner horizontally in the welcome box.
    If terminal too narrow for Computerist-20, fall back to Computerist-12.
    Mascot art left, title right — both vertically centered in banner row.
  - **Touches:** `figby-rs/src/tui/welcome.rs`
  - **Success:** Banner looks balanced at 80, 120, and 200 col terminals.
  - **Difficulty:** Low

- [x] `5.0.2` Two-column layout: Recent Files (left) + Actions (right)
  - **Goal:** Below banner, split into two columns using `Layout::horizontal`
    with `Flex::Start`. Left: Recent Files panel (scrollable numbered list,
    click or press number to open). Right: two labeled boxes — Font and Image.
  - **Touches:** `figby-rs/src/tui/welcome.rs`
  - **Success:** Both columns render with labeled borders. Recent list scrollable.
  - **Difficulty:** Low

- [x] `5.0.3` Font action panel with NerdFont icons
  - **Goal:** Right column top box labeled "Font". Five actions, each with
    NerdFont icon from `icons.yaml`, key shortcut in bracket, and label:
    - `[N]` `file_new` New Font from System Font
    - `[I]` `file_import` New Font from File
    - `[B]` `font_header` New Blank Font
    - `[O]` `file_open` Open Font
    - `[D]` `edit_duplicate` Open Duplicate
    Highlighted row on hover/selection.
  - **Touches:** `figby-rs/src/tui/welcome.rs`, `WelcomeAction`
  - **Success:** All five actions visible, keyboard shortcuts work, icons render.
  - **Difficulty:** Low

- [x] `5.0.4` Image action panel with NerdFont icons
  - **Goal:** Right column bottom box labeled "Image". Four actions:
    - `[C]` `image_import` New Image (blank canvas)
    - `[T]` `nav_forward` New from Template (shows grid of templates; ship with: Terminal Banner 80×24, Wide Banner 120×30, Square 40×40)
    - `[V]` `image_import` Convert Image to ASCII (opens file picker → rascii dialog)
    - `[F]` `file_open` Open Figmap (layered .figmap file)
    Highlighted row on hover/selection.
  - **Touches:** `figby-rs/src/tui/welcome.rs`, `WelcomeAction`
  - **Success:** All four actions visible and triggerable.
  - **Difficulty:** Medium

- [x] `5.0.5` Mouse click support on all welcome items
  - **Goal:** Clicking on any action row in Font or Image panel, or any item
    in Recent Files list, triggers the corresponding `WelcomeAction`.
    Use `MouseEventKind::Down` hit-testing against rendered row areas.
  - **Touches:** `figby-rs/src/tui/welcome.rs`, `figby-rs/src/tui/mod.rs`
  - **Success:** Mouse clicks on welcome screen items work identically to keyboard.
  - **Difficulty:** Medium

- [x] `5.0.6` Esc = dismiss only; Q / Ctrl+C = quit
  - **Goal:** `Esc` anywhere in TUI means "back out one level" — dismiss dialog,
    close panel, return to canvas. Never quits. Quit only via `Q` (no modifier,
    not in text input) or `Ctrl+C`. Update welcome, canvas, all dialogs.
  - **Touches:** `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/welcome.rs`,
    all dialog `handle_key` fns
  - **Success:** Esc from welcome → dismiss to editor. Esc from canvas → no-op.
    Q from canvas → quit. Ctrl+C anywhere → quit.
  - **Difficulty:** Low

- [x] `5.0.7` Phase merge: release/5.0 → main
  - **Difficulty:** Low

---

## Phase 5.1 — Toolbox & Canvas Polish

- [x] `5.1.1` Toolbox NerdFont icons
  - **Goal:** Toolbox widget currently uses 2-char `display_name()` abbrevs.
    Replace with `icon_key()` lookup from `App::icons` BTreeMap. Display:
    `[icon] FullName` per row. Pass icons map into `Toolbox` render just as
    `LayerPanel` and `StatusBar` already do.
  - **Touches:** `figby-rs/src/tui/toolbox.rs`, `figby-rs/src/tui/mod.rs`
  - **Success:** Every tool shows its NerdFont icon. Fallback to abbrev if icon missing.
  - **Difficulty:** Low

- [x] `5.1.2` Toolbox dynamic width
  - **Goal:** Toolbox column width currently hardcoded. Compute at render time:
    `max(icon_width + longest_full_name + 2 padding, brush_size_preview_width)`.
    Clamp between 10 and 20 cols. Layout constraint updated to `Constraint::Length`.
  - **Touches:** `figby-rs/src/tui/toolbox.rs`, `figby-rs/src/tui/layout.rs`
  - **Success:** Toolbox always wide enough for content. Large brush size preview fits.
  - **Difficulty:** Low

- [x] `5.1.3` Canvas visible border
  - **Goal:** Draw a distinct border around the active canvas area so the user
    can see exactly where the canvas starts and ends. Use a different border
    style (e.g. double or thick) or color (accent) from surrounding panels.
    Show canvas dimensions (WxH) in the border title.
  - **Touches:** `figby-rs/src/tui/mod.rs` canvas render section
  - **Success:** Canvas boundary immediately obvious. Dimensions visible in title.
  - **Difficulty:** Low

- [x] `5.1.4` Collapsed/shared borders between adjacent panels
  - **Goal:** Adjacent panels currently draw overlapping borders (double-thick lines
    between panels). Use ratatui's border-sharing pattern: panels on shared edges
    only draw their own half, eliminating double borders. Reference:
    `Layout::default().spacing(0)` + `Borders` side selection per panel.
  - **Touches:** `figby-rs/src/tui/layout.rs`, all panel render calls in `mod.rs`
  - **Success:** No double-border lines anywhere. Layout looks tight and clean.
  - **Difficulty:** Medium

- [x] `5.1.5` Phase merge: release/5.1 → main
  - **Difficulty:** Low

---

## Phase 5.2 — Layout Restructure

- [x] `5.2.1` Palette moved under tools (left column)
  - **Goal:** Left column is currently just the toolbox. Stack toolbox on top,
    palette drawer below it. Use `Layout::vertical` with
    `[Constraint::Length(toolbox_h), Constraint::Min(0)]`. Palette gets
    remaining space.
  - **Touches:** `figby-rs/src/tui/layout.rs`, `figby-rs/src/tui/mod.rs`
  - **Success:** Palette visible below toolbox without opening a drawer.
    Right panel freed from palette duty.
  - **Difficulty:** Medium

- [x] `5.2.2` Right panel: tabbed prop/info/library/effects drawer
  - **Goal:** Replace right panel with a tabbed component. Tabs (NerdFont icons):
    - `Layers` `layer_new` — layer stack (already exists, just move here)
    - `Props` `settings_open` — tool properties, brush settings, image/font info
    - `Text` `tool_text` — text tool input, font picker, justify, size
    - `Libraries` `brush_shape_custom` — brush stamps, saved selections
    - `Effects` `image_contrast` — global effects, filters
  - **Touches:** `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/layout.rs`,
    new `figby-rs/src/tui/side_panel.rs`
  - **Success:** Right panel shows correct content per active tab.
    Tab switching via keyboard and mouse click.
  - **Difficulty:** High

- [x] `5.2.3` Props tab: context-sensitive tool properties
  - **Goal:** Props tab content changes based on active tool:
    - Brush/Spray/Eraser: size, shape, density sliders
    - Text: font name, size, justify
    - Eyedropper: sample info
    - Fill: threshold
    - Emitter: emission config (already exists as EmitterConfigPanel, embed here)
    Image/font info always shown at bottom of Props tab.
  - **Touches:** `figby-rs/src/tui/side_panel.rs`
  - **Depends:** `5.2.2`
  - **Success:** Switching tools updates Props tab content immediately.
  - **Difficulty:** Medium

- [x] `5.2.4` Phase merge: release/5.2 → main
  - **Difficulty:** Low

---

## Phase 5.3 — Status Bar Redesign

- [x] `5.3.1` Flat item-based status bar with section grouping
  - **Goal:** Replace fixed-width p1-p4 panels with flexible flat item list. Group info into three informal sections: left (mode/tool/pos/zoom), middle (font/unsaved/glyph), right (branch/FPS/clock/render/layer/undo/throbber). Use `│` pipe separators between items with theme colors. No powerline layout.
  - **Touches:** `figby-rs/src/tui/components/status_bar.rs`
  - **Success:** Status bar shows all items at 80+ cols. Sections visually grouped with pipe separators.
  - **Difficulty:** Medium

- [x] `5.3.2` Responsive: drop low-priority items at narrow widths
  - **Goal:** Measure available width. Drop items right-to-left when space runs out:
    clock → FPS → git branch → zoom → font name. Mode + tool + position never drop.
  - **Touches:** `figby-rs/src/tui/components/status_bar.rs`
  - **Depends:** `5.3.1`
  - **Success:** At 80 cols: mode/tool/pos + filename + unsaved. At 40 cols: mode + pos.
  - **Difficulty:** Low

- [x] `5.3.3` Phase merge: release/5.3 → master
  - **Difficulty:** Low

---

## Phase 5.4 — Image Editor Fix

- [x] `5.4.1` Fix image editor mode switching
  - **Goal:** Switching to image editor mode currently non-functional (user reports
    can't switch to it). Identify mode-switch path, fix so Image Editor tab/mode
    activates cleanly from welcome screen image actions and from mode toggle keybind.
  - **Touches:** `figby-rs/src/tui/mod.rs`, mode switching logic
  - **Success:** Image editor mode reachable and canvas interactive.
  - **Difficulty:** Medium

- [x] `5.4.2` Fix mouse events in image editor
  - **Goal:** Mouse clicks not reaching image editor canvas/toolbox. Audit
    `handle_mouse_event` routing — confirm image editor mode is handled in the same
    dispatch path as font editor. Fix any mode-check that gates mouse events.
  - **Touches:** `figby-rs/src/tui/mod.rs`
  - **Depends:** `5.4.1`
  - **Success:** All tools usable with mouse in image editor.
  - **Difficulty:** Medium

- [x] `5.4.3` Image import dialog (rascii options)
  - **Goal:** "Convert Image to ASCII" action opens file picker. On file select,
    show options dialog: charset picker (block/smooth/full/braille/deluxe),
    output width slider, color toggle (truecolor / 256 / mono). Preview rendered
    result in dialog. Confirm → load into canvas as new layer.
  - **Touches:** `figby-rs/src/tui/mod.rs`, new dialog in `figby-rs/src/tui/dialogs/`
  - **Depends:** `5.4.1`, `5.0.4`
  - **Success:** Image converted with chosen options appears on canvas.
  - **Difficulty:** High

- [x] `5.4.4` Phase merge: release/5.4 → main
  - **Difficulty:** Low

---

## Phase 5.5 — Animation Audit & Surface

- [x] `5.5.1` Audit 4.5–4.8 implementation vs spec
  - **Goal:** Read `timeline.rs`, `player.rs`, `export.rs`. Document what is
    actually implemented vs what was spec'd. Write findings to
    `docs/animation-audit.md`: what works, what is stub/partial, what is missing.
  - **Touches:** `figby-rs/src/tui/timeline.rs`, `player.rs`, `export.rs` (read-only)
  - **Success:** Written audit. Gap list ready for 5.5.2.
  - **Difficulty:** Low

- [x] `5.5.2` Surface timeline panel in main layout
  - **Goal:** Timeline panel is currently inaccessible from the main UI (no keybind,
    no menu entry visible). Add: `T` toggles timeline panel at bottom of canvas.
    Timeline expands to ~8 rows when open, collapses otherwise. Frame thumbnails,
    playhead, add/delete frame buttons.
  - **Touches:** `figby-rs/src/tui/layout.rs`, `figby-rs/src/tui/mod.rs`,
    `figby-rs/src/tui/timeline.rs`
  - **Depends:** `5.5.1`
  - **Success:** Press T → timeline appears. Add frames, switch, see content change.
  - **Difficulty:** Medium

- [x] `5.5.3` Verify animation export end-to-end
  - **Goal:** Test GIF export, APNG export, ANSI export from a 5-frame animation.
    Fix any broken paths identified in `5.5.1`. Export dialog must reach all
    three formats.
  - **Touches:** `figby-rs/src/tui/export.rs`, `figby-rs/src/output.rs`
  - **Depends:** `5.5.1`
  - **Success:** All three export formats produce valid files from a test animation.
  - **Difficulty:** Medium

- [x] `5.5.4` Phase merge: release/5.5 → main
  - **Difficulty:** Low

---

## Phase 5.6 — Palette UX & Editor

- [x] `5.6.1` Color name tooltip on hover
  - **Goal:** When mouse hovers a palette swatch, show a one-line indicator below
    the swatch grid with the standard terminal colour name (e.g. "Blue",
    "Bright Cyan", or "#1A2B3C" for RGB). Track hover index separately from
    selected index. Clear on mouse-out.
  - **Touches:** `figby-rs/src/tui/palette.rs`, `figby-rs/src/tui/mod.rs`
    (add `Moved` arm to palette mouse handler)
  - **Success:** Hovering any swatch shows its name; moving away clears it.
  - **Difficulty:** Low

- [x] `5.6.2` 5-per-row hue-grouped palette layout
  - **Goal:** Replace the current 8-per-row flat grid with a hue-grouped layout:
    5 swatches per row, rows grouped by hue family (Reds, Oranges, Yellows,
    Greens, Cyans, Blues, Purples, Neutrals). Group membership computed via
    HSL hue bucketing; user can override group assignment later. This is
    groundwork for the Marker brush (5.6.5).
  - **Touches:** `figby-rs/src/tui/palette.rs`
  - **Success:** Palette renders with hue-grouped rows, 5 swatches wide.
    Arrow navigation still works row-by-row.
  - **Difficulty:** Medium

- [x] `5.6.3` Palette editor panel (save / load / duplicate)
  - **Goal:** New `PaletteEditor` panel (accessible from a keybind or right-panel
    tab). Shows palette name, list of swatches with hex values. Actions:
    - Save palette to `~/.config/figby/palettes/<name>.json`
    - Load palette from file picker (filters `.json` palette files)
    - Duplicate current palette under a new name
  - **Touches:** new `figby-rs/src/tui/palette_editor.rs`,
    `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/layout.rs`
  - **Success:** Can round-trip a palette through save → load and get identical
    colours. Duplicate produces independent copy.
  - **Difficulty:** Medium

- [x] `5.6.4` Palette import: common formats
  - **Goal:** Import palette swatches from popular formats:
    - **Paletty JSON** — array of `{hex, name}` objects (see paletty.dev export)
    - **Adobe ASE** — binary `ASEF` format; parse swatch blocks
    - **WezTerm JSON** — `colors` object with named terminal colour keys
    - **Windows Terminal JSON** — `schemes[].background/foreground/…` fields
    Show a format picker in the load dialog; auto-detect where unambiguous.
  - **Touches:** new `figby-rs/src/palette_import.rs`,
    `figby-rs/src/tui/palette_editor.rs`
  - **Depends:** `5.6.3`
  - **Success:** Each format imports to the correct swatch list without data loss.
  - **Difficulty:** Medium

- [x] `5.6.5` Marker brush mode (Aseprite-style shading)
  - **Goal:** New brush sub-mode "Marker". Requires 2+ colours selected in the
    palette. Painting on a non-empty pixel steps its colour forward one position
    in the selected-colour array by the brush's per-pixel hit strength (0.0–1.0,
    accounting for brush falloff). Accumulate fractional steps per pixel per
    stroke; commit integer steps on mouse-up. Auto-masks empty pixels (no effect
    on transparent/space cells). Clamp at the last selected colour.
  - **Touches:** `figby-rs/src/tui/brush.rs` (BrushSubMode enum, BrushState field),
    `figby-rs/src/tui/palette.rs` (multi-select state),
    `figby-rs/src/tui/tools/brush.rs`, `figby-rs/src/tui/mod.rs`,
    `figby-rs/src/tui/side_panel.rs` (Mode display)
  - **Depends:** `5.6.2` (hue-grouped palette for ergonomic colour stepping)
  - **Success:** Paint over a mid-tone pixel repeatedly → colour steps toward the
    darkest selected swatch and clamps. No effect on blank cells.
  - **Difficulty:** High

- [x] `5.6.6` Phase merge: release/5.6 → main
  - **Difficulty:** Low

---

## Phase 5.7 — Animation Enhancements

- [x] `5.7.1` Animated GIF import to timeline
  - **Goal:** Add an "Import GIF" action (welcome screen + File menu) that opens
    an animated GIF file, decodes each frame as a canvas layer snapshot, and
    populates the animation timeline so the frames are immediately playable and
    editable. Frame count, delay, and loop count should be read from the GIF
    metadata. If a GIF has a palette, map it to the nearest palette entries in
    the active Figby palette (or create a new one).
  - **Touches:** `figby-rs/src/tui/file_ops.rs` (new FileOpsMode::ImportGif),
    `figby-rs/src/tui/mod.rs` (dispatch + handler),
    `figby-rs/src/tui/timeline.rs` (populate frames from decoded GIF),
    new `figby-rs/src/gif_import.rs` (GIF decode → CanvasBuffer frames via
    the `image` crate or `gif` crate, map colors),
    `figby-rs/src/tui/welcome.rs` (Import GIF action in Image panel)
  - **Suggested model:** Pro
  - **Difficulty:** High

- [ ] `5.7.2` Phase merge: release/5.7 → main
  - **Difficulty:** Low

---

## Phase 5.8 — Dynamic Lighting System

> Design spec: `docs/lighting-design.md`

- [ ] `5.8.1` Core lighting engine (`lighting.rs`)
  - **Goal:** Implement `figby-rs/src/tui/lighting.rs` with all core types and
    computation from the design spec: `Normal3`, `NormalMap`, `Light` (Ambient /
    Directional / Point), `Scene`, `LightingLut`, `LutEntry`,
    `compute_normal_map_figfont()`, `shade_canvas()`, `cast_shadow()` (DDA),
    and `intensity_to_char()`. Unit-test each component in isolation (normal
    generation from a synthetic heightfield, Lambertian diffuse values,
    shadow raycast through a known occluder, LUT round-trip).
  - **Touches:** new `figby-rs/src/tui/lighting.rs`
  - **Suggested model:** Pro
  - **Difficulty:** High

- [ ] `5.8.2` Canvas and layer integration
  - **Goal:** Wire the lighting engine into the canvas render pipeline.
    `CanvasCell` gains `height: Option<u8>` (default `None`). `Layer` gains
    `accepts_lighting: bool` and `casts_shadow: bool` (both default `true`).
    After layer compositing in `components/canvas.rs`, run the shading pass
    when a `Scene` is active: call `shade_canvas()` and map each cell through
    the `LightingLut` before committing to ratatui's `Buffer`. `TuiApp` gains
    a `lighting_scene: Option<Scene>` field; shading pass is skipped when
    `None`. Expose layer flags in the Layers panel with `L`/`S` toggles.
  - **Touches:** `figby-rs/src/tui/canvas.rs` (CanvasCell height field),
    `figby-rs/src/tui/layers.rs` (Layer flags),
    `figby-rs/src/tui/components/canvas.rs` (shading pass hook),
    `figby-rs/src/tui/mod.rs` (Scene field, no-op when None),
    `figby-rs/src/tui/layout.rs` (Layers panel toggle keys)
  - **Depends:** `5.8.1`
  - **Suggested model:** Pro
  - **Difficulty:** High

- [ ] `5.8.3` Light management UI
  - **Goal:** Add a "Lighting" mode (key `G`) with an in-canvas light editor.
    Show a light list panel (left column, like Toolbox) listing current lights
    with type/intensity. Arrow keys move the selected point light's (x, y)
    position (shown as a `✦` glyph on the canvas overlay). `+`/`-` adjust
    intensity. `A` adds an ambient light, `D` adds directional, `P` adds point.
    `Del` removes selected light. Escape exits lighting mode. Status bar shows
    current light type and intensity. Real-time re-shade on every edit.
  - **Touches:** `figby-rs/src/tui/mod.rs` (AppMode::Lighting, key dispatch),
    new `figby-rs/src/tui/light_panel.rs` (light list widget),
    `figby-rs/src/tui/layout.rs` (lighting mode layout),
    `figby-rs/src/tui/components/status_bar.rs` (lighting mode status)
  - **Depends:** `5.8.2`
  - **Suggested model:** Pro
  - **Difficulty:** High

- [ ] `5.8.4` Palette LUT integration
  - **Goal:** Extend palette entries with `lit_color` and `shadow_color` fields
    (optional; defaults: `lit_color = fg`, `shadow_color = fg * 0.3`). Add
    `specular: bool` and `shininess: f32` per entry. Generate `LightingLut`
    from the active palette when a Scene is set, and regenerate on palette
    swap. Palette editor (5.6.3) gains `lit`/`shadow` colour pickers per swatch
    visible when lighting mode is active.
  - **Touches:** `figby-rs/src/tui/palette.rs` (LUT generation, entry fields),
    `figby-rs/src/tui/palette_editor.rs` (lit/shadow pickers),
    `figby-rs/src/tui/lighting.rs` (LightingLut wired to palette)
  - **Depends:** `5.8.3`
  - **Suggested model:** Mid
  - **Difficulty:** Medium

- [ ] `5.8.5` Phase merge: release/5.8 → main
  - **Difficulty:** Low
