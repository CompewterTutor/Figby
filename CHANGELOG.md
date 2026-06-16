# Changelog

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
