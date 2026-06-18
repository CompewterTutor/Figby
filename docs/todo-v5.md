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

- [ ] `5.1.4` Collapsed/shared borders between adjacent panels
  - **Goal:** Adjacent panels currently draw overlapping borders (double-thick lines
    between panels). Use ratatui's border-sharing pattern: panels on shared edges
    only draw their own half, eliminating double borders. Reference:
    `Layout::default().spacing(0)` + `Borders` side selection per panel.
  - **Touches:** `figby-rs/src/tui/layout.rs`, all panel render calls in `mod.rs`
  - **Success:** No double-border lines anywhere. Layout looks tight and clean.
  - **Difficulty:** Medium

- [ ] `5.1.5` Phase merge: release/5.1 → main
  - **Difficulty:** Low

---

## Phase 5.2 — Layout Restructure

- [ ] `5.2.1` Palette moved under tools (left column)
  - **Goal:** Left column is currently just the toolbox. Stack toolbox on top,
    palette drawer below it. Use `Layout::vertical` with
    `[Constraint::Length(toolbox_h), Constraint::Min(0)]`. Palette gets
    remaining space.
  - **Touches:** `figby-rs/src/tui/layout.rs`, `figby-rs/src/tui/mod.rs`
  - **Success:** Palette visible below toolbox without opening a drawer.
    Right panel freed from palette duty.
  - **Difficulty:** Medium

- [ ] `5.2.2` Right panel: tabbed prop/info/library/effects drawer
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

- [ ] `5.2.3` Props tab: context-sensitive tool properties
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

- [ ] `5.2.4` Phase merge: release/5.2 → main
  - **Difficulty:** Low

---

## Phase 5.3 — Status Bar Redesign

- [ ] `5.3.1` Powerline-style three-section layout
  - **Goal:** Redesign status bar into three sections using `Flex`:
    - **Left:** `[mode_icon] ModeName` ▶ `[tool_icon] ToolName` ▶ `[status_position] X:n Y:n`
    - **Middle** (`Constraint::Fill(1)`): filename + unsaved indicator + font name
    - **Right:** `[status_git_branch] branch` ▶ `[status_fps] FPS` ▶ `[status_clock] HH:MM`
    Powerline separators (`` or ``) between sections using theme colors.
  - **Touches:** `figby-rs/src/tui/components/status_bar.rs`, `figby-rs/src/tui/mod.rs`
  - **Success:** Status bar looks like lazyvim powerline. All info visible at 80+ cols.
  - **Difficulty:** Medium

- [ ] `5.3.2` Responsive: drop low-priority items at narrow widths
  - **Goal:** Measure available width. Drop items right-to-left when space runs out:
    clock → FPS → git branch → zoom → font name. Mode + tool + position never drop.
  - **Touches:** `figby-rs/src/tui/components/status_bar.rs`
  - **Depends:** `5.3.1`
  - **Success:** At 80 cols: mode/tool/pos + filename + unsaved. At 40 cols: mode + pos.
  - **Difficulty:** Low

- [ ] `5.3.3` Phase merge: release/5.3 → main
  - **Difficulty:** Low

---

## Phase 5.4 — Image Editor Fix

- [ ] `5.4.1` Fix image editor mode switching
  - **Goal:** Switching to image editor mode currently non-functional (user reports
    can't switch to it). Identify mode-switch path, fix so Image Editor tab/mode
    activates cleanly from welcome screen image actions and from mode toggle keybind.
  - **Touches:** `figby-rs/src/tui/mod.rs`, mode switching logic
  - **Success:** Image editor mode reachable and canvas interactive.
  - **Difficulty:** Medium

- [ ] `5.4.2` Fix mouse events in image editor
  - **Goal:** Mouse clicks not reaching image editor canvas/toolbox. Audit
    `handle_mouse_event` routing — confirm image editor mode is handled in the same
    dispatch path as font editor. Fix any mode-check that gates mouse events.
  - **Touches:** `figby-rs/src/tui/mod.rs`
  - **Depends:** `5.4.1`
  - **Success:** All tools usable with mouse in image editor.
  - **Difficulty:** Medium

- [ ] `5.4.3` Image import dialog (rascii options)
  - **Goal:** "Convert Image to ASCII" action opens file picker. On file select,
    show options dialog: charset picker (block/smooth/full/braille/deluxe),
    output width slider, color toggle (truecolor / 256 / mono). Preview rendered
    result in dialog. Confirm → load into canvas as new layer.
  - **Touches:** `figby-rs/src/tui/mod.rs`, new dialog in `figby-rs/src/tui/dialogs/`
  - **Depends:** `5.4.1`, `5.0.4`
  - **Success:** Image converted with chosen options appears on canvas.
  - **Difficulty:** High

- [ ] `5.4.4` Phase merge: release/5.4 → main
  - **Difficulty:** Low

---

## Phase 5.5 — Animation Audit & Surface

- [ ] `5.5.1` Audit 4.5–4.8 implementation vs spec
  - **Goal:** Read `timeline.rs`, `player.rs`, `export.rs`. Document what is
    actually implemented vs what was spec'd. Write findings to
    `docs/animation-audit.md`: what works, what is stub/partial, what is missing.
  - **Touches:** `figby-rs/src/tui/timeline.rs`, `player.rs`, `export.rs` (read-only)
  - **Success:** Written audit. Gap list ready for 5.5.2.
  - **Difficulty:** Low

- [ ] `5.5.2` Surface timeline panel in main layout
  - **Goal:** Timeline panel is currently inaccessible from the main UI (no keybind,
    no menu entry visible). Add: `T` toggles timeline panel at bottom of canvas.
    Timeline expands to ~8 rows when open, collapses otherwise. Frame thumbnails,
    playhead, add/delete frame buttons.
  - **Touches:** `figby-rs/src/tui/layout.rs`, `figby-rs/src/tui/mod.rs`,
    `figby-rs/src/tui/timeline.rs`
  - **Depends:** `5.5.1`
  - **Success:** Press T → timeline appears. Add frames, switch, see content change.
  - **Difficulty:** Medium

- [ ] `5.5.3` Verify animation export end-to-end
  - **Goal:** Test GIF export, APNG export, ANSI export from a 5-frame animation.
    Fix any broken paths identified in `5.5.1`. Export dialog must reach all
    three formats.
  - **Touches:** `figby-rs/src/tui/export.rs`, `figby-rs/src/output.rs`
  - **Depends:** `5.5.1`
  - **Success:** All three export formats produce valid files from a test animation.
  - **Difficulty:** Medium

- [ ] `5.5.4` Phase merge: release/5.5 → main
  - **Difficulty:** Low
