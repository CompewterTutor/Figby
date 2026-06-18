# Changelog

## [5.3.0] - 2026-06-18
### Added
- Phase 5.3 status bar redesign — flat item-based layout with StatusItem priority system
- Three informal sections (left/middle/right) with pipe separators
- Responsive dropping of low-priority items at narrow widths

## [5.2.0] - 2026-06-18
### Added
- Phase 5.2 layout restructure — palette under tools, tabbed right panel (Layers/Props/Text/Libraries/Effects), context-sensitive tool properties in Props tab

## [5.0.0] - 2026-06-18
### Added
- Welcome screen Phase 5.0 — complete overhaul
- Banner: Computerist-20 FIGfont title with Computerist-12 fallback; both mascot
  and title vertically centered in banner row; title horizontally centered
- Two-column content layout: Recent Files (left, scrollable, ↑↓) + Font/Image
  action panels (right)
- Font panel: 5 actions with NerdFont icons in `[K]ey` inline format
  (N/I/B/O/D shortcuts)
- Image panel: 4 actions with NerdFont icons (C/T/V/F shortcuts)
- Mouse hover highlight and click support on all welcome items via stored
  hit-test rects updated each render cycle
- `dispatch_welcome_action()` shared by keyboard and mouse paths
- Esc → dismiss/back-out only (never quits); Q / q → quit from canvas;
  Ctrl+C → quit via SIGINT

## [3.0.0-rc.4] - 2026-06-18
### Added
- Multi-directory font search: `load_font()` now accepts `&[&str]`, searches
  `DEFAULT_FONT_DIRS` (`/usr/local/share/figlet`, `/usr/share/figlet`) as
  fallback when a font is not found in the user-specified directory.
- `full` charset preset for `--create-font`: ASCII printable + block elements
  with `█` (full block) as the darkest character.
- ChicagoFLF system font generated to `figby-fonts/new_fonts/`.
### Changed
- `print_direction` in generated FIGfonts changed from `-1` to `0` (explicit LTR).
  Header generation now uses the struct's field value instead of hardcoding.
- Generated fonts default to `full` charset (from `smooth`) for richer output.

## [3.0.0-rc.2] - 2026-06-17
### Changed
- Phase merge: release/4.1 → main. Phase 4.1 complete: welcome screen (4.1.4),
  ZIP font browsing (4.1.5), various polish fixes.

## [3.0.0-rc.1] - 2026-06-16
### Added
- v3.0.0 release candidate cut. RC branch `rc/3.0.0-rc.1`, annotated tag
  `v3.0.0-rc.1`. Full Phase 3.3 regression complete.

## [2.5.4] - 2026-06-16
### Changed
- Phase merge: release/3.2 → master. Phase 3.2 complete: glyph grid mouse
  click+double-click, glyph char editor cursor+cell toggle, font preview strip.

## [2.5.3] - 2026-06-16

### Added
- Font editor overview: mouse wheel scroll through glyph grid (`handle_mouse_scroll_overview`)

## [2.5.2] - 2026-06-16

### Changed
- `keymap.rs` now owns a `GLOBAL_DISPATCH` table mapping `(KeyModifiers, KeyCode)` to
  `GlobalAction` variants; `lookup_global()` does exact-match lookup
- `TuiApp::handle_key_event` global if-chain replaced with `dispatch_global()` match arm;
  eliminates ~70 lines of repetitive modifier/key-code guards
- Undo/redo and undo-panel toggle also routed through dispatch table (early global pass)

## [2.5.1] - 2026-06-16

### Changed
- `MenuBar` refactored to `StatefulWidget for &MenuBar` with separate `MenuBarState`
- All mutable menu state (active_menu, focused_item, header/item rects, pending action)
  moved to `MenuBarState`; `MenuBar` retains only static config (menus, theme)
- Key/mouse handlers now take `&mut MenuBarState` instead of `&mut self`
- Render uses `frame.render_stateful_widget` for proper ratatui StatefulWidget pattern

## [2.5.0] - 2026-06-15

### Added
- `FrameLayout` struct: single-pass layout computation stored on `TuiApp` for mouse hit-testing
- `DrawerMode` enum: collapsible right drawer cycling Palette → BrushKeys → Closed (`?` key)
- Zen mode (`F11`): canvas expands to full frame area with dim hint overlay
- `Ctrl+K`: toggle full keybindings overlay panel
- Brush panel now shows `Shape:` label alongside Char/Size fields
- Extended keymap entries for all tool shortcuts, brush controls, and new global commands

### Changed
- Layout refactored to `tui/layout.rs`; canvas uses `Constraint::Fill(1)` instead of `Min`
- Collapsed borders between toolbox/canvas/right-panel (ratatui recipe — no double lines)
- `Tab` / `Shift+Tab` now cycle modes from any context (was `Ctrl+Tab` only)
- Font editor Overview auto-search exclusion expanded to protect all tool/global shortcuts
- Status bar zoom format changed from `Zoom:{n}x` to `{icon} {n}x`
- Settings dialog (`S`) now only opens when not in FontEditor mode (where `S` opens Smushing)

### Fixed
- Font editor Overview mode intercepting tool shortcuts (b/e/l/v/etc.) for auto-search
- Collapsed `if` blocks flagged by clippy (mod.rs mouse handler)
- Integration tests updated for new layout, status bar format, and EditorState field paths

## [Unreleased] — Rust Port

### Added

- Rust project scaffold (`figby-rs/`)
- Cargo workspace configuration
- FIGlet font submodule for test fixtures
- CI configuration (fmt + clippy + test)

### Porting Progress

- [ ] Phase 1.1 — Crate scaffold, font parser
- [ ] Phase 1.2 — Render engine (kerning + smushing)
- [ ] Phase 1.3 — CLI interface (all FIGlet flags)
- [ ] Phase 1.4 — Control files + character mapping
- [ ] Phase 1.5 — Multi-byte input (UTF-8, DBCS, Shift-JIS)
- [ ] Phase 1.6 — TLF (TOIlet) font support
- [ ] Phase 1.7 — Full test suite against original C
- [ ] Phase 1.8 — Optimization + polish
