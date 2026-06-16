# Figby v3 — Ratatui Refactor & UX Fixes

Milestone goal: Fix broken TUI interactions (menu mouse, font editor focus,
glyph selection), add extended charsets, define a default keymap, and fully
refactor the TUI architecture to idiomatic ratatui patterns (StatefulWidget,
WidgetRef, proper Layout usage, event-driven component protocol).

---

## Phase 3.0 — Priority Bug Sprint

> Fix these before any architecture work. They block daily use.

- [x] `3.0.1` Fix menu mouse: can't select dropdown items
  - **Goal:** Clicking a menu header opens the dropdown (current). Clicking a
    dropdown item must navigate to and select that item. Currently
    `handle_mouse_event` in `menu.rs` only hit-tests the header row
    (`row == self.menu_area.y`). Dropdown items rendered below the bar have no
    mouse handler. `tui_menu::MenuState` has no mouse API.
  - **Root cause:** `tui_menu` crate is render-only; dropdown hit positions are
    unknown at event time. The `MouseEventKind::Down` path calls
    `self.state.select()` immediately on header click (line 171-173 of
    `menu.rs`) before any sub-item has focus, which is a no-op or wrong.
  - **Fix plan:**
    1. Replace `tui_menu` dependency with a custom `MenuBar` widget
       (`figby-rs/src/tui/menu.rs`) that owns its dropdown render pass and
       records item screen rects during draw.
    2. On `Down` click on header: open dropdown only (no `select()`).
    3. On `Down` click inside open dropdown rect: compute item index from
       `row - dropdown_origin_y`, call navigate + `select()`.
    4. On `Down` click outside any menu area while active: close menu.
    5. On `Up` click: no-op (select happens on Down for snappier feel).
  - **Touches:** `figby-rs/src/tui/menu.rs`, `figby-rs/Cargo.toml`
    (remove `tui-menu` dep)
  - **Success:** Mouse click on any dropdown item triggers its action.
    Keyboard nav still works (existing Alt+key, arrow key paths unchanged).
  - **Tests:** Manual — open each menu, click each item, verify action fires.
  - **Difficulty:** Medium

- [x] `3.0.2` Fix font editor: search not activatable / behavior unclear
  - **Goal:** After opening a `.flf` file the font editor overview renders but
    the user cannot tell how to filter glyphs. Pressing `/` activates search
    (`search_active = true`) but there is no visible cursor and no prominent
    hint. Any other printable key does nothing unless `search_active` is already
    true.
  - **Root cause (UX):** `handle_key_overview` requires `/` before accepting
    printable chars. The search field shows static placeholder text with no
    cursor indicator. No status line explains how to interact.
  - **Fix plan:**
    1. Change any printable char (when `!search_active && !code_input_active`)
       to auto-activate search and append the typed char in one keypress.
    2. Render a `|` cursor at the end of the search query string when
       `search_active`.
    3. Add a one-line key-hint footer inside `render_overview`:
       `"↑↓←→ Navigate  / Search  Enter Edit  A Add  D Del  H Header  Esc Close"`.
    4. Set `search_active = false` when Esc is pressed with empty query
       (already done), and clear search on font load in `load_font`.
  - **Touches:** `figby-rs/src/tui/font_editor.rs` (render_overview,
    handle_key_overview, load_font)
  - **Success:** User can type immediately to filter; cursor visible; hints
    always visible at bottom of overview.
  - **Tests:** Open font, type "A", verify search filters to glyphs with
    code 65 or char 'A'. Press Esc, verify search clears.
  - **Difficulty:** Low

- [x] `3.0.3` Fix font editor: no way to enter glyph editor
  - **Goal:** After opening a font the user expects to click or press Enter on
    a glyph to edit it. Arrow navigation works (`selected_index` highlights in
    reverse-video) but there is no visible indication that Enter opens the
    editor, and no mouse click handler for the glyph grid.
  - **Root cause (UX + missing mouse handler):** No mouse click dispatch in
    overview. The hint bar from `3.0.2` partially helps, but clicking a glyph
    cell should also open it.
  - **Fix plan:**
    1. `render_overview` must record the screen `Rect` of each rendered glyph
       cell (index → Rect) in a `Vec<(u32, Rect)>` stored on `FontEditor`.
    2. Add `handle_mouse_click(col: u16, row: u16)` to `FontEditor`: hit-test
       `cell_rects`, set `selected_index`, if double-click open `CharEditor`.
       Single-click just moves selection.
    3. Wire `handle_mouse_event` in the font editor component and in `mod.rs`
       when `is_font_ui_mode`.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`,
    `figby-rs/src/tui/components/font_editor.rs`,
    `figby-rs/src/tui/mod.rs`
  - **Success:** Single click highlights a glyph cell. Double-click (or Enter)
    opens canvas editor for that glyph. Back to overview via Esc.
  - **Tests:** Open font, single click glyph → selection moves. Double-click →
    canvas editor opens showing that glyph. Esc → back to overview, same
    selected glyph highlighted.
  - **Difficulty:** Medium

- [x] `3.0.4` Default keymap — data-driven keybinding table
  - **Goal:** Define all TUI keybindings in one place as a static table
    (`Keymap`), not spread across match arms in `mod.rs`. The table drives:
    (a) event dispatch, (b) the Help → Keybindings dialog, (c) future
    user-config override. Keymap must cover every current binding.
  - **Keybindings to capture (non-exhaustive):**

    | Binding          | Scope           | Action                   |
    |------------------|-----------------|--------------------------|
    | `Ctrl+O`         | Global          | Open file                |
    | `Ctrl+S`         | Global          | Save file                |
    | `Ctrl+Shift+S`   | Global          | Save As                  |
    | `Ctrl+E`         | Global          | Export                   |
    | `Ctrl+Q`         | Global          | Quit                     |
    | `Ctrl+Z`         | Canvas          | Undo                     |
    | `Ctrl+Y`         | Canvas          | Redo                     |
    | `Ctrl+Shift+Z`   | Canvas          | Redo                     |
    | `Ctrl+Shift+H`   | Global          | Toggle undo panel        |
    | `Tab`            | Global          | Next mode                |
    | `Shift+Tab`      | Global          | Prev mode                |
    | `F5`             | Global          | Toggle render mode       |
    | `+` / `-`        | Canvas          | Zoom in / out            |
    | `Alt+F/E/V/T/H`  | Global          | Open menu (File/Edit…)   |
    | `Esc`            | Menu / Dialog   | Close / cancel           |
    | `↑↓←→`           | Font overview   | Navigate glyph grid      |
    | `Enter`          | Font overview   | Open glyph editor        |
    | `/`              | Font overview   | Activate search          |
    | `A` / `D` / `C`  | Font overview   | Add / Delete / Copy glyph|
    | `H` / `S` / `T`  | Font overview   | Header / Smush / Transform|
    | `↑↓←→`           | Font char editor| Move cursor in glyph     |
    | `Space`          | Font char editor| Toggle cell              |
    | `M`              | Font char editor| Mirror                   |
    | `F`              | Font char editor| Flip                     |
    | `G`              | Font char editor| Generate from system font|

  - **Implementation:**
    1. Create `figby-rs/src/tui/keymap.rs`. Define `KeyBinding { keys, scope,
       description }` and `KEYMAP: &[KeyBinding]` as a const slice.
    2. Scope enum: `Global, Canvas, FontOverview, FontCharEditor, Dialog`.
    3. Do NOT change dispatch yet (that's the 3.1 refactor). Just define the
       table and wire the Help → Keybindings dialog to render it.
  - **Touches:** new `figby-rs/src/tui/keymap.rs`, `figby-rs/src/tui/mod.rs`
    (expose it), `figby-rs/src/tui/font_editor.rs` (help dialog render)
  - **Success:** `Help → Keybindings` dialog shows full key table, grouped by
    scope, with current correct bindings.
  - **Tests:** Open Keybindings dialog, verify all entries from the table above
    are visible. Spot-check 5 bindings work as documented.
  - **Difficulty:** Low–Medium

---

## Phase 3.0-C — Extended Charsets

> Needed for font generation pipeline AND canvas palette.

- [x] `3.0-C.1` Braille charset (U+2800–U+28FF) in font_gen + palette
  - **Goal:** Add all 256 Unicode Braille Pattern characters as a named charset
    `"braille"` in `font_gen::resolve_charset`. Also expose as a selectable
    group in the palette picker (`tui/palette.rs`).
  - **Why:** Braille patterns produce excellent high-density ASCII art from
    rasterized glyphs (each cell encodes a 2×4 pixel grid).
  - **Touches:** `figby-rs/src/font_gen.rs` (resolve_charset), new const
    `BRAILLE_CHARSET`, `figby-rs/src/tui/palette.rs`
  - **Success:** `figby --create-font-path Foo.ttf --create-font-charset braille`
    produces a `.flf` using braille chars. Braille group selectable in palette.
  - **Tests:** Generate a font with braille charset; verify output file is valid
    `.flf` with braille chars in glyph rows.
  - **Difficulty:** Low

- [x] `3.0-C.2` Block elements charset (U+2580–U+259F) in font_gen + palette
  - **Goal:** Add named charset `"blocks"` covering:
    - Half-blocks: ▀▄█▌▐ (U+2580–U+2590)
    - Shade chars: ░▒▓ (U+2591–U+2593)
    - Quadrant blocks: U+2596–U+259F
    - Vertical eighths: ▁▂▃▄▅▆▇█ (U+2581–U+2588)
  - **Touches:** `figby-rs/src/font_gen.rs`, `figby-rs/src/tui/palette.rs`
  - **Success:** `--create-font-charset blocks` works. Blocks palette group
    shows all chars in the TUI.
  - **Difficulty:** Low

- [x] `3.0-C.3` Box drawing + geometric charset in font_gen + palette
  - **Goal:** Add named charset `"box"` covering:
    - Box drawing: ─│┌┐└┘├┤┬┴┼ and double/heavy variants (U+2500–U+257F)
    - Geometric shapes subset: ▪▫■□◆◇ (U+25A0–U+25FF selected subset)
  - **Touches:** `figby-rs/src/font_gen.rs`, `figby-rs/src/tui/palette.rs`
  - **Difficulty:** Low

- [x] `3.0-C.4` Ogham decorative charset in font_gen + palette
  - **Goal:** Add named charset `"ogham"` for Ogham script block (U+1680–U+169F).
  - **Touches:** `figby-rs/src/font_gen.rs`, `figby-rs/src/tui/palette.rs`
  - **Difficulty:** Low

- [x] `3.0-C.5` "Deluxe" meta-charset combining all above
  - **Goal:** Named charset `"deluxe"` = ASCII printable + blocks + box +
    shade + braille + Ogham. Single preset for maximum expressive range.
    Listed first in the palette charset picker as the default for new files.
  - **Touches:** `figby-rs/src/font_gen.rs`, `figby-rs/src/tui/palette.rs`
  - **Difficulty:** Low

---

## Phase 3.1 — Ratatui Architecture Refactor

> Idiomatic ratatui patterns throughout. Prerequisite: 3.0 bugs fixed.

- [x] `3.1.1` Audit: map all Widget / StatefulWidget usage vs ratatui best practices
  - **Goal:** Read every `impl Widget` and every `frame.render_widget` call in
    `figby-rs/src/tui/`. Document every deviation from the ratatui authoring
    guide:
    - Consuming `Widget` impl where `Widget for &T` is correct
    - `StatefulWidget` state held inside the widget struct instead of separate
    - Manual `Frame.area()` splits instead of `Layout + Constraint`
    - Borrow issues caused by `&mut self` in render (requires reborrowing)
    - Widgets that store `Rect` instead of computing it from layout
    - Components that call their own `draw` recursively or hold frames
  - **Touches:** `figby-rs/src/tui/` (read-only audit)
  - **Output:** `docs/tui-arch-audit.md` with file:line findings and severity.
  - **Difficulty:** Medium

- [x] `3.1.2` Split `TuiApp` god-struct into focused components
  - **Goal:** `mod.rs:TuiApp` is 2000+ lines with state from every subsystem.
    Extract into composable top-level component structs:
    - `AppState` — mode, theme, render_mode, dirty, fps, git_branch
    - `DialogState` — wraps file_ops_comp, export_comp, undo_panel_comp
    - `EditorState` — canvas, selection, clipboard, tool state, undo
    Each implements an `EventSink` trait with `handle_key`, `handle_mouse`,
    `render`. `TuiApp::run` calls them in priority order.
  - **Touches:** `figby-rs/src/tui/mod.rs` (major rewrite)
  - **Depends:** `3.1.1`
  - **Difficulty:** High

- [x] `3.1.3` Convert all widgets to `Widget for &T` (non-consuming)
  - **Goal:** Widgets currently consumed or behind `&mut self` in render should
    implement `Widget for &WidgetType`. Stateful widgets should split into
    `Widget` (render, borrows &State) and a `*State` struct passed via
    `render_stateful_widget`.
  - **Pattern to follow:**
    ```rust
    impl Widget for &MyWidget {
        fn render(self, area: Rect, buf: &mut Buffer) { ... }
    }
    // Stateful:
    impl StatefulWidget for &MyWidget {
        type State = MyWidgetState;
        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) { ... }
    }
    ```
  - **Touches:** `figby-rs/src/tui/canvas.rs`, `figby-rs/src/tui/palette.rs`,
    `figby-rs/src/tui/toolbox.rs`, `figby-rs/src/tui/status.rs`,
    `figby-rs/src/tui/font_editor.rs` (render_* methods → Widget impls)
  - **Depends:** `3.1.2`
  - **Difficulty:** High

- [x] `3.1.4` Replace `Component` trait with typed event protocol
  - **Goal:** The current `Component::handle_key_event(KeyEvent) -> Option<Action>`
    is weakly typed. Actions like `Action::FontEditorAction` carry no payload.
    Replace with a typed protocol:
    ```rust
    enum FontEditorEvent { GlyphOpened(u32), SearchChanged(String), ViewChanged(FontEditorView) }
    enum CanvasEvent { CellPainted(usize, usize), SelectionChanged, ... }
    ```
    Event handlers return `Option<AppEvent>` (top-level app events). Dispatch
    in `TuiApp` pattern-matches on `AppEvent` instead of `Action` enum.
  - **Touches:** `figby-rs/src/tui/action.rs` (replace with event types),
    `figby-rs/src/tui/component.rs` (redefine trait),
    all component files.
  - **Depends:** `3.1.2`
  - **Difficulty:** High

- [x] `3.1.5` Layout system: replace hardcoded `Rect` with `Constraint`-based layout
  - **Goal:** Several widgets store `Rect` fields that are set during render
    (`toolbox_area`, `palette_area`, `canvas_inner_rect`, `menu_area`). This
    couples render-time geometry to event-time hit-testing.
    Refactor to a single `layout()` fn on `TuiApp` that computes all areas
    once per frame using `Layout::default().constraints(...)` and caches them
    in a `FrameLayout` struct. Hit-testing reads from `FrameLayout`.
  - **Implemented:** `tui/layout.rs` — `FrameLayout` struct with `DrawerMode` enum.
    `Constraint::Fill(1)` for canvas. Canvas collapsed borders (`canvas_borders()`).
    Right drawer cycles Palette → BrushKeys → Closed with `?`.
    Zen mode (full-screen canvas) with `F11`. `Ctrl+K` for keybindings overlay.
    `render_canvas_area()`, `render_brush_keys_panel()`, `render_overlays()` helpers.
  - **Touches:** `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/layout.rs` (new),
    `figby-rs/src/tui/keymap.rs`
  - **Depends:** `3.1.2`
  - **Difficulty:** Medium

- [x] `3.1.6` Fix double key-dispatch in `mod.rs`
  - **Root cause:** `mod.rs:1313-1331` calls `font_editor_comp.handle_key_event(key)`
    (which calls `editor.handle_key` via Component trait), and if it returns None,
    calls `editor.handle_key` directly again. Double dispatch for every unhandled key.
  - **Fix:** After `3.1.4`, dispatch is through typed events. No double-call needed.
    Until then: remove the second `editor.handle_key` call at line 1322-1331;
    the Component trait impl at line 1313 is sufficient.
  - **Touches:** `figby-rs/src/tui/mod.rs:1319-1331`
  - **Difficulty:** Low

- [x] `3.1.7` Custom menu widget with full mouse support (supersedes 3.0.1 shim)
  - **Goal:** After the architecture refactor, implement `MenuBar` as a proper
    `StatefulWidget` using `MenuBarState`. State holds: active menu index
    (None = closed), focused item index within the open menu. Render pass
    records item `Rect`s into state for mouse hit-testing.
    ```rust
    impl StatefulWidget for &MenuBar {
        type State = MenuBarState;
        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            // Draw headers; record header rects in state.header_rects
            // If state.open_menu.is_some(), draw dropdown; record item_rects
        }
    }
    ```
    Mouse handler reads `state.header_rects` and `state.item_rects`.
  - **Touches:** `figby-rs/src/tui/menu.rs` (full rewrite)
  - **Depends:** `3.1.3`, `3.1.5`
  - **Difficulty:** Medium

- [ ] `3.1.8` Keymap-driven dispatch (uses 3.0.4 table)
  - **Goal:** Wire `KEYMAP` from `3.0.4` as the actual dispatch table.
    `TuiApp::handle_key_event` looks up `(scope, key)` in `KEYMAP` and calls
    the corresponding handler fn. Eliminates the chain of `if modifiers ==
    KeyModifiers::CONTROL && code == KeyCode::Char('o')` arms in `mod.rs`.
  - **Touches:** `figby-rs/src/tui/mod.rs`, `figby-rs/src/tui/keymap.rs`
  - **Depends:** `3.1.2`, `3.0.4`
  - **Difficulty:** Medium

- [ ] `3.1.9` Phase merge: release/3.1 → main
  - **Difficulty:** Low

---

## Phase 3.2 — Font Editor Polish

> Depends on 3.1. These require the refactored widget/event system.

- [ ] `3.2.1` Glyph grid: mouse click select + double-click open
  - **Goal:** Full mouse support for the glyph overview grid. Supersedes `3.0.3`
    which was a hotfix. Now uses `StatefulWidget` render-pass rect recording.
    Single click → move `selected_index`. Double-click → open `CharEditor`.
    Mouse wheel → scroll grid.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`
  - **Depends:** `3.1.3`, `3.1.7`
  - **Difficulty:** Medium

- [ ] `3.2.2` Glyph char editor: proper canvas cursor + cell toggle
  - **Goal:** In `CharEditor` mode the canvas is the editing surface. Currently
    the canvas widget handles drawing tools, but the font editor glyph cell is
    simple toggle (space / non-space). Add a dedicated `GlyphCursor` overlay
    that renders a blinking `█` cursor on the active cell. Arrow keys move
    cursor; Space toggles cell. Brush char from palette determines the fill char.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`,
    `figby-rs/src/tui/canvas.rs`
  - **Difficulty:** Medium

- [ ] `3.2.3` Font preview strip in overview
  - **Goal:** Below the glyph grid, render a live preview strip showing a
    sample string ("AaBbCc123!?") rendered using the current font glyphs via
    `render.rs`. Updates on every glyph edit. Helps see font as a whole.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`,
    `figby-rs/src/render.rs`
  - **Difficulty:** Medium

- [ ] `3.2.4` Phase merge: release/3.2 → main
  - **Difficulty:** Low

---

## Phase 3.3 — Major Release

- [ ] `3.3.1` Full regression: all features vs v2.x baseline
  - **Goal:** Verify every feature from v2 is preserved after the 3.1 refactor.
    Canvas, tools, undo/redo, file ops, export, image editor, font editor.
  - **Difficulty:** Medium

- [ ] `3.3.2` v3.0.0 RC — human sign-off
  - **Touches:** RC branch, annotated tag
  - **Difficulty:** Low
  - **Model:** Human
