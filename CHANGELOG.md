# Changelog

## [6.0.1] - 2026-06-22

### Added
- Palette editor: add (A), delete (Del), edit hex (E), rename swatch (N), rename palette (R) operations; inline hex and name editing modes; View menu entry for Palette Editor; keymap documentation for Ctrl+Shift+P (6.8.4).

## [Unreleased]

### Added (2026-06-22 session)
- Quit-confirm dialog: pressing `q`/FileQuit when `editor.unsaved` is true now shows
  "Unsaved Changes" overlay with [Y]es save / [N]o discard / [C]ancel; `trigger_quit()`
  helper centralizes all three quit paths (6.7.2).
- Built-in palettes: `palette_import::builtin_palettes()` returns Grayscale (5),
  Primary (20), Warm (15), Cool (15) swatches; accessible via View → Palette: * menu
  entries which load into the palette editor panel (6.8.5).
- Keybinds popup now scrollable: ↑/↓/PgUp/PgDn/q all work; `keybindings_scroll` field
  on `TuiApp`; title updated to show controls (6.8.7).
- Added 20+ missing keybinds to KEYMAP: Layer Panel scope (n/d/x/l/m/M/+/-/Ctrl+G),
  Text Tool scope (↑↓/Enter/Esc/[/]), Canvas extras (Ctrl+A/X/C/V/Delete, r, H/V) (6.8.7).
- Layers menu (New/Duplicate/Delete/Merge Down/Move Up/Move Down/Toggle Visibility/
  Toggle Lock) and Animation menu (Add Frame/Delete Frame/Play/Toggle Timeline) added
  to menu bar with full action handlers (6.9.3).
- `handle_font_editor_key()` and `handle_image_editor_key()` methods extracted from
  `handle_key_event`; each mode now dispatched via a single-line call (6.6.1g, 6.6.1h).

- New image dialog: Width/Height fields (Tab/arrow navigation, numeric entry),
  palette dropdown (Left/Right to cycle), Enter confirms with canvas creation
  at specified size + selected palette (6.8.2).

### Fixed (2026-06-22 session)
- Unicode chars (Ä Ö Ü ä ö ü ß) typed in Text tool no longer panic; verified covered
  by 6.5.1 blank-glyph fallback; added `test_text_tool_unicode_no_panic` (6.7.3).

## [6.0.0] - 2026-06-22
### Security
- Remove `$(cmd)` shell command substitution from template resolver (`template.rs:160`);
  rendering a shared `.ftmp` can no longer execute arbitrary shell commands (B0/RCE).
- Sandbox `{{img:PATH}}` template image paths to template directory; absolute paths
  and `..` traversal are rejected (B0 adjacent).
- Cap template canvas dimensions: `width*height > 1_000_000` cells rejected,
  margin/padding clamped; prevents OOM from crafted frontmatter (B7/DoS).
- Validate FIGfont header numerics: `height` must be 1..=255, negative baseline/
  maxlength rejected; invalid header no longer accepted (B1).
- Cap zip decompression: `read_to_end()` replaced with size-checked read; zip-bomb
  fonts rejected before exhausting memory (B2).
- Fix GIF memory-guard timing: dimension check now runs before the frame decode loop;
  oversized GIF bails at first frame rather than after full decode (B4/DoS).
- Range-check FLC control-file group indices: `gl`/`gr` validated as `b'0'..=b'3'`
  before assignment; crafted `.flc` no longer panics (B5/panic).
- Limit image decode dimensions: `image::io::Reader` now uses `Limits::default()`;
  decompression-bomb images rejected (B6/DoS).

### Fixed
- Green test suite: 10 stale tests fixed — welcome-gate tests now dismiss welcome
  screen before key events; layer-model tests read/write active layer buffer not
  composite; palette shadow test updated to use `.round()` (B3).
- Replace `.expect()` in `render.rs:lookup_char` with blank-glyph fallback; fonts
  missing char 0 no longer panic (S1).
- Text tool: printable keys (`b`/`e`/`f` etc.) no longer switch tools while
  `entering_text=true`; `Char(c)` captured before toolbox-selector dispatch (6.7.1).

### Added
- `LightingState`, `AnimationState`, `InteractionState` sub-structs extracted from
  `TuiApp`; shrinks borrow surface and god-object field count by 20 (6.6.1a–c).
- `LightPanel::render()` method extracted from `TuiApp::render_light_panel()`;
  `tui/overlays.rs` extracted from `TuiApp::render_overlays()`; lighting key
  dispatch extracted to `LightingState::handle_key()` (6.6.1d–f).
- Compile-time test validates embedded `ICONS_YAML`; malformed YAML now fails CI
  instead of silently dropping all icons (A3/S2, 6.5.2).
- Clamp `font_gen` point_size to 4.0..=200.0; unbounded value no longer causes
  oversized canvas allocations (S5, 6.5.3).
- GitHub Actions CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`
  run on push/PR; legacy `.travis.yml` removed (6.2.2).
- Hard `cargo test` gate in `scripts/ralph.sh` merge phase; LLM self-attestation
  no longer the only merge check (6.2.1).
- Brush size/shape indicator in toolbox panel; updates live on `[`/`]` (6.9.5).

### Changed
- `CLAUDE.md` and `AGENTS.md` updated to reflect current source layout and v6
  milestone (A2, 6.4.1–6.4.2).
- Shadow computation uses `.round()` instead of truncation; `default_shadow_hex`
  now produces `#4D0000` for 30% shadow (6.1.3).

### Docs
- Pre-release codebase audit: `docs/codebase-audit-2026-06-18.md` (sessions 1+2,
  complete read-through). Every finding mapped to a task in `docs/todo-v6.md`.

## [5.8.0] - 2026-06-18
### Added
- Phase merge: release/5.8 → main. Phase 5.8 (Dynamic Lighting System) complete:
  core lighting engine (5.8.1), canvas and layer integration with shading pass,
  layer lighting/shadow flags (5.8.2), light management UI with in-canvas light
  editor and light list panel (5.8.3), palette LUT integration with per-swatch
  lit/shadow colors and specular highlights (5.8.4).

## [5.6.0] - 2026-06-18
### Added
- Phase merge: release/5.6 → master. Phase 5.6 (Palette Enhancement & Marker Brush)
  complete: color name hover tooltip (5.6.1), hue-grouped palette with 5 per row
  (5.6.2), palette editor with save/load/duplicate (5.6.3), palette import from
  Paletty/ASE/WezTerm/Windows Terminal (5.6.4), marker brush mode with colour-stepping
  shading (5.6.5).
### Fixed
- Marker brush: when cell color not in selected palette, consume 1 step to enter
  the array at index 0 instead of jumping to index 1.
- Marker brush: preserve 0.0 fractional remainder entries in accum map for future
  strokes.
- TUI dispatch: only launch TUI when stdin is a terminal and no FIGlet flags are
  provided; piped stdin now correctly triggers CLI mode.
- Palette editor test: fixed race condition on XDG_CONFIG_HOME env var by
  serializing concurrent tests with a mutex.

## [5.5.0] - 2026-06-18
### Added
- Phase merge: release/5.5 → master. Phase 5.5 (Animation Audit & Surface) complete:
  animation audit (5.5.1), timeline panel surface in main layout (5.5.2), export
  end-to-end verification with 5-frame GIF/APNG/ANSI tests (5.5.3).

## [5.4.0] - 2026-06-18
### Changed
- Phase merge: release/5.4 → master. Phase 5.4 (Image Editor Fix) complete:
  mode switching fix (5.4.1), mouse event routing fix (5.4.2), rascii import
  dialog with charset/width/color options (5.4.3).

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
