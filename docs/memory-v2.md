# Figby v2 — Memory

## Phase 2.0 — CLI Polish

### 2.0.1 — CLI `--help` output

Added `help = "..."` to every `#[arg()]` field and `long_about` to `#[command()]`
on `CliArgs` struct in `main.rs`. Every FIGlet flag now has a descriptive help
string. Tests verify `--help` exits with `DisplayHelp` and output contains
key flags and descriptions. `--long-help` not a built-in clap 4 flag — tested
via `render_long_help()` directly.

Pre-existing clippy issue in `render.rs`: `calc_smush_amount()` missing
`#[allow(clippy::too_many_arguments)]` — added to pass clippy gate.
Pre-existing bench bug: `calc_smush_amount` call had wrong argument order —
fixed to pass clippy `--all-targets`.

### 2.0.5 — `.ftmp` template file format design + CLI

Created `figby-rs/src/template.rs` — `.ftmp` template parser and renderer.
Format uses TOML frontmatter delimited by `---` lines with two sections:
- `[canvas]`: width, height, keep_ratio, margin, padding
- `[variables.varname]`: text, font, x, y, z, align, overlap, plus
  border/shadow fields (stubbed, deferred to 2.0.7).

Parser (`parse_ftmp`) reads frontmatter via `toml::from_str`, extracts
`{{varname}}` placeholders from body via simple string scanning.
Renderer (`render_template`) sorts layers by z-index (ascending),
loads fonts with dedup cache, renders each layer's text through FIGlet
pipeline (`add_char`/`render_line`), places onto canvas at (x,y) with
overwrite or flow overlap mode. Flow layers stack vertically via cursor.
Margin and padding applied to final output.

CLI flag `--render-template` (`-T`) added to `CliArgs`. When specified,
reads `.ftmp`, parses, renders, prints output rows. Font directory
resolved from `-d`, `FIGLET_FONTDIR`, or default `/usr/share/figlet`.

`toml = "0.8"` added to Cargo.toml dependencies for TOML frontmatter parsing.

### 2.0.9 — Builtin template functions: date + repo-data (syntax + reserve)

Added `TemplateBuiltin` enum with `Date(String)` and `RepoData(String)` variants
to `template.rs`. Added `builtin: Option<TemplateBuiltin>` field to `Layer` struct
(default `None`). `parse_ftmp()` recognizes `{{date:format}}` and
`{{repo-data:field}}` tags before the variables lookup. `render_template()` skips
builtin layers with `continue` (no-op, deferred to 2.1). No `.unwrap()` in
production — all new code uses proper Option handling. fmt and clippy pass clean.

### 2.0.7 — Border and shadow rendering for template output

Added three private helper functions to `template.rs`:
- `content_bbox()` — scans rendered rows at canvas position for non-space chars,
  returns `Option<(top, bottom, left, right)>` bounding box.
- `fill_border()` — fills a ring of `'.'` chars around the content bbox, only
  overwriting space cells. Border ring = expanded rect minus content rect.
- `fill_shadow()` — fills `'.'` chars in a rectangular region offset down-right
  from content bbox, only overwriting space cells.

Wired into `render_template()` in both image and text branches — after
`place_on_canvas()`, computes bbox from each layer's rendered rows + placement,
applies border then shadow. Only activates when `border_width`/`shadow_size` is
`Some` and `> 0`.

4 new tests: border-only, shadow-only, border+shadow, border-no-overwrite-other-layer.
Plus 5 direct unit tests for the helpers (content_bbox basic/no-content/multi-row,
fill_border ring/shadow offset/border preserves content).

### 2.0.x — Fix broken template tests

Fixed 6 failing unit tests in `template.rs`:

1. **TOML quoting bug** in `make_border_shadow_ftmp()`: Raw string `r#"border_color = "."#`
   consumed the closing `"` of the TOML value into the `"#` raw string delimiter, producing
   `border_color = ".` (invalid TOML). Fixed with regular string `"border_color = \".\""`.

2. **`test_render_overwrite_mode`**: Assertion expected `starts_with("BB  ")` but test font
   renders `" BB "` (leading space). Changed text from `"AA"/"BB"` to single `"A"/"B"` and
   assertion to `starts_with(" BB")`.

3. **`test_render_z_order`**: Placed layers in order z=2, z=0, z=1 without actual sorting —
   last-placed (z=1) won. Fixed to place in ascending z-order so highest z overwrites last.

4. **`test_fill_shadow_offset`**: Asserted `canvas[3][3] == '.'` but shadow rect is a single
   cell at (row=3, col=4) — col 3 is outside shadow. Fixed assertion to expect `' '`.

### 2.0.x — README header template (redesign + defer)

After review:
- Template format redesigned: YAML frontmatter (not TOML), typed elements in
  `canvas.ftmp-elements`, body just `{{name}}` placeholders. Reference format
  in `assets/templates/figby-cli-h1.ftmp`.
- Image tag: `{{img:name}}` not `{{img:source:width:height:...}}` — all metadata
  in frontmatter under element's YAML block.
- **Implementation deferred.** The template system will be rewritten when TUI
  infrastructure lands (Phase 2.3+ ratatui), which provides a proper canvas
  widget with per-cell color metadata. Current TOML-based parser is too
  restrictive and can't handle ANSI color in the text grid.
- Template file saved as reference design: `assets/templates/figby-30w.ftmp`.

### 2.0.10 — Phase merge: release/2.0 → master

Phase 2.0 merged into master. This merge brings 3 commits that landed on
`release/2.0` after the initial phase merge (10035c9):
- Fix broken template tests (TOML quoting, assertion fixes)
- Redesign `.ftmp` format: YAML frontmatter, defer template rendering to TUI
- Add `assets/tui/icons.yaml` for Phase 2.2, renumber 2.2.5→2.2.6

All subtasks 2.0.1–2.0.10 complete. Phase 2.1 (Image-to-ASCII Pipeline) is next.

## Phase 2.1 — Image-to-ASCII Pipeline

### 2.1.5 — Image CLI flags integration

Added `ImageOptions` struct, image CLI flags, image mode dispatch, and `run_image()`
entry point to `main.rs`. Flip helpers (`flip_horizontal`, `flip_vertical`,
`flip_horizontal_rgb`, `flip_vertical_rgb`) added to `image_input.rs` — necessary
supporting change since matrix types are defined there.

Flags added to `CliArgs`: `--image`/`-i`, `--map`, `--braille`/`-b`, `--color`,
`--grayscale`, `--negative`, `--dither`, `--width`, `--height`, `--dimensions`
(format `WxH`), `--flipX`, `--flipY`. All parse cleanly via clap derive.
`--width`/`--height` override `--dimensions` values (last-wins semantics).

`main()` dispatches on `is_image_mode()` (non-empty `--image` paths) after
template rendering but before FIGlet mode — image and FIGlet modes coexist.
`--width` uses long flag only (`-w` reserved for FIGlet output width).
URL paths detected but not yet supported (eprintln + skip).

17 flag parse tests + 2 integration tests covering every flag, defaults,
short aliases, multiple paths, and mode detection. fmt and clippy pass clean.

### 2.1.6 — Phase merge: release/2.1 → master

Merged all Phase 2.1 work into default branch (master). Phase 2.1 complete:
Image loading + grayscale luminance conversion (2.1.1), luminance-to-ASCII
character mapping (2.1.2), 24-bit ANSI colored ASCII output (2.1.3), braille
art + Floyd-Steinberg dithering (2.1.4), image CLI flags integration (2.1.5).
All 5 subtasks implemented, tested, merged. Phase 2.2 (System Font → FIGfont
Creation) is next.

Merge conflicts: docs/ralph-log.md had a conflict between master's post-2.0.10
log entries and release/2.1's 2.1.x entries. Resolution: kept both sets in
chronological order. Also committed the 2.1.5 ralph-log entry that was
previously uncommitted on task-2.1.5/2.1.6.

### 2.1.1 — Image loading + grayscale conversion via `rascii_art`

Added `image = { version = "0.24", features = ["jpeg", "png", "bmp", "webp"] }` to
Cargo.toml. `rascii_art = "0.4.5"` was already present (used by `template.rs`).

Created `image_input.rs` module with two public functions:
- `load_luminance_matrix(path)` — opens image from file path, returns `Result<Vec<Vec<u8>>, ImageError>`
- `luminance_from_dynamic(img)` — converts `&DynamicImage` to luminance matrix

Both delegate to `image::DynamicImage::to_luma8()` then extract pixel rows into
`Vec<Vec<u8>>` (outer=rows, inner=columns, each 0-255). No `.unwrap()` in
production — all errors propagate as `ImageError`.

7 unit tests: PNG fixture load, JPEG encode+load, BMP encode+load, WEBP encode+load,
known RGB luminance ordering (green > red > blue), luminance range (non-empty rows),
nonexistent file returns error. `lib.rs` updated with `pub mod image_input;`.

### 2.1.6 — Phase merge: release/2.1 → master

Merged all Phase 2.1 work into default branch (master). Phase 2.1 complete:
Image loading + grayscale luminance conversion (2.1.1), luminance-to-ASCII
character mapping (2.1.2), 24-bit ANSI colored ASCII output (2.1.3), braille
art + Floyd-Steinberg dithering (2.1.4), image CLI flags integration (2.1.5).
All 5 subtasks implemented, tested, merged. Phase 2.2 (System Font → FIGfont
Creation) is next.

Merge conflicts: docs/ralph-log.md had a conflict between master's post-2.0.10
log entries and release/2.1's 2.1.x entries. Resolution: kept both sets in
chronological order. Also committed the 2.1.5 ralph-log entry that was
previously uncommitted on task-2.1.5/2.1.6.

## Phase 2.2 — System Font → FIGfont Creation

### 2.2.6 — Phase merge: release/2.2 → main

Merged all Phase 2.2 work into default branch (master). Phase 2.2 complete:
system font enumeration via font-kit (2.2.1), glyph rasterization to FIGcharacter
rows (2.2.2), FIGfont header generation from font metrics (2.2.3), `--create-font`
CLI command (2.2.4), TUI iconset YAML file (2.2.5). All 6 subtasks (2.2.1–2.2.6)
implemented, tested, merged. Phase 2.3 (TUI Core & Canvas) is next.

### 2.3.6 — Status bar + canvas settings

Created `figby-rs/src/tui/status.rs` with two widgets:
- `StatusBar` — renders cursor X,Y, zoom level, current tool name, mode name,
  unsaved indicator using Nerd Font icons from `icons.yaml`. Static `render()`
  method takes all display data as parameters (no stored state).
- `CanvasSettings` struct — settings panel with canvas width/height, font size,
  grid toggle, snap-to-grid toggle. `pub settings_open: bool` controls visibility.
  `handle_key()` navigates fields via ↑/↓/←/→, toggles booleans via Enter, closes
  via Esc. `render()` shows labeled fields with highlighted selection.

Integrated into `TuiApp`:
- `unsaved: bool` field (default `false`), `settings: CanvasSettings` field
- Status bar constraint changed from `Length(1)` to `Length(3)` (needs room for
  borders + 1 content line)
- Settings panel replaces palette sidebar when `settings_open` is true
- `S` key opens/closes settings, loading canvas state on open
- `apply_settings()` syncs canvas width/height/grid on each settings key event
- Settings mode blocks all other key handlers (canvas, toolbox, palette)
- `apply_settings()` — recreates canvas widget when dimensions change, toggles
  grid to match settings

10 integration tests covering all status bar fields (cursor, zoom, tool, mode,
unsaved indicator) and settings panel (toggle, width change, grid toggle,
snap-to-grid toggle). fmt and clippy pass clean.

### 2.3.5 — Brush selection

Created `figby-rs/src/tui/brush.rs` — brush shape picker and size controls:
- `BrushShape` enum: Square, Circle, SprayPaint, Custom with `cycle()` method
- `BrushState` struct: `shape: BrushShape`, `size: u8` (1..=20, clamped), `set_size()`,
  `size_up()`, `size_down()`, `cycle_shape()`
- `render_preview(max_size)` returns `Vec<String>` showing brush tip at current size:
  - Square: filled `size×size` block of `@`
  - Circle: filled circle via distance check within radius `size/2`
  - SprayPaint: deterministic pseudo-random dots via `(x*7 + y*31 + seed) % 100 < 35`
  - Custom: single `+` at center, rest spaces
- `render()` ratatui widget: shows shape name, size, and preview in toolbox column
- Integrated into `TuiApp`: `brush` field, key events (`[` size down, `]` size up,
  `'` cycle shape), preview rendered below toolbox
- Status bar updated to show current brush shape and size
- No `.unwrap()` in production — all paths use proper Option/clamp arithmetic
- SprayPaint uses fixed seed 42 for deterministic output across test runs

### 2.3.7 — Phase merge: release/2.3 → main

Merged all Phase 2.3 work into default branch (master). Phase 2.3 complete:
TUI scaffold with ratatui (2.3.1), toolbox bar with tool selection (2.3.2),
scrollable/zoomable canvas widget (2.3.3), color palette sidebar (2.3.4),
brush shape picker with size/preview (2.3.5), status bar + canvas settings
panel (2.3.6). All 6 subtasks implemented, tested, merged.
Phase 2.4 (Drawing Tools) is next.

## Phase 2.4 — Drawing Tools

### 2.4.2 — Eraser tool

Created `figby-rs/src/tui/tools/eraser.rs` — eraser execution module with two
public functions:
- `erase_stamp()` — sets cells to `CanvasCell::default()` (space, no fg/bg)
  within brush shape area, reusing `stamp_offsets` from `tools::brush` for
  identical brush geometry. No `unwrap()` — bounds clipping via `get_mut` → `Option`.
- `erase_line()` — Bresenham line interpolation with per-step `erase_stamp` calls.

Integrated into TUI:
- Mouse dispatch guard broadened from `Tool::Brush` to `Tool::Brush | Tool::Eraser`
  using `matches!()` macro.
- Eraser branch in mouse `Down`/`Drag` handlers calls `erase_stamp`/`erase_line`
  instead of `paint_stamp`/`paint_line`.
- Keyboard painting (Space/Enter) also dispatches to Eraser when selected.
- No new dependencies or `unwrap()` in production — structurally identical to brush.

9 tests: square clearance, circle shape, spray determinism, bounds clipping,
horizontal/vertical/diagonal/reverse lines.

### 2.4.3 — Line tool

Created `figby-rs/src/tui/tools/line.rs` — line drawing module with one public
function `draw_line_segment()` that delegates to `brush::paint_line` (shared
Bresenham implementation). Thin wrapper keeps the door open for future
line-specific features (arrow heads, dashed styles, etc.) without coupling to
the brush module.

Integrated into TUI:
- Mouse dispatch guard broadened from `Tool::Brush | Tool::Eraser` to include
  `Tool::Line` via `matches!()`.
- Line tool mouse Down saves `line_start` + clones canvas buffer into
  `saved_buffer` (no immediate draw).
- Line tool mouse Drag restores buffer from `saved_buffer`, draws preview line
  from `line_start` to current position using active brush shape/size/palette color.
- Line tool mouse Up clears `line_start` and `saved_buffer`.
- Keyboard painting (Space/Enter) stamps single point when Line tool selected.
- Two new fields on `TuiApp`: `line_start: Option<(i16, i16)>`,
  `saved_buffer: Option<CanvasBuffer>`.

5 tests: horizontal, vertical, diagonal, reverse direction, endpoint clipping.

### 2.4.4 — Fill / flood fill tool

Created `figby-rs/src/tui/tools/fill.rs` — flood fill tool with one public
function `flood_fill()`. Uses iterative BFS with `Vec<(usize, usize)>` stack
(not recursive, no stack overflow). Orthogonal-only filling (4-directional).

Key behaviors:
- Bounds-checked at every step via `buffer.get_mut()` → `Option` — no `unwrap()`.
- Short-circuits if replacement cell's char already matches target char (no-op).
- Boundary-aware: stops at cells with different characters.
- Tile correctly reads target char from start cell before mutating any cells.

Integrated into TUI:
- Mouse dispatch guard includes `Tool::Fill` alongside existing drawing tools.
- Single-click mouse Down fills at clicked position using active palette color.
- Keyboard Space/Enter fills at cursor position.
- No Drag/Up handling — Fill is single-click, like a paint bucket.

10 unit tests: small region, bounded region (X border), unbounded to edge,
single cell, no-match short-circuit, out-of-bounds safety, boundary crossing
(X wall between two @ regions), empty region (space fill), orthogonal-only
(diagonal cells not filled), foreground color preservation.

### 2.4.5 — Selection tools: marquee, lasso, circle, polygon

Created `figby-rs/src/tui/tools/selection.rs` — four selection shapes:

- **Marquee:** click-drag rectangle. Origin on Down, updates on Drag, finalizes
  on Up. Stores as `(x1,y1)-(x2,y2)` inclusive rectangle mask.
- **Circle:** click center on Down, Drag computes radius = distance from center,
  finalizes on Up. Uses horizontal-span fill via `√(r² - dy²)` for each scanline.
- **Lasso:** click starts path, Drag appends points, Up closes and runs polygon
  fill. Reuses `polygon()` with freehand points as vertices.
- **Polygon:** successive clicks add vertices, Enter closes polygon, Esc cancels.
  Close-on-click when distance < 3px from first vertex.

`Selection` struct owns `Vec<Vec<bool>>` mask (row-major), bounding box, and
bounds recomputation. Mask operations:

- `marquee(buffer, x1, y1, x2, y2)` — clamping, inclusive rectangle.
- `circle(buffer, cx, cy, r)` — midpoint scanline fill, handles r≤0 as single cell.
- `polygon(buffer, vertices)` — even-odd rule scanline fill with floating-point
  edge intersection.
- `lasso(buffer, points)` — delegates to `polygon()` with ≥3 guard.
- `copy_from(buffer)` → `Clipboard` (Vec<Vec<Option<CanvasCell>>>) — bounding-box
  aligned, None for unselected cells.
- `cut_from(buffer)` → `Clipboard` — copy then delete.
- `delete_from(buffer)` — sets all masked cells to default (space).
- `paste_into(buffer, clipboard, dx, dy)` — writes `Some(cell)` entries at offset.
- `move_selection(buffer, dx, dy)` — cut from old position, paste at translated
  bounds, remask at translated position.
- `perimeter()` → perimeter cells (selected cell with ≥1 unselected 4-neighbor).

Overlay rendering in `CanvasWidget`:
- `selection_perimeter: Option<Vec<(usize, usize)>>` — buffer-coordinate perimeter
  cells, rendered with alternating `▒`/space dash pattern at zoom level.
- `polygon_vertices: Vec<(i16, i16)>` — in-progress polygon vertices shown as `+`
  markers with cyan bold style.

TUI integration in `tui/mod.rs`:
- `handle_key_event` changed to accept `impl Into<KeyEvent>` for modifier support.
- Selection tools bypass the early mouse return guard for drawing tools.
- Arrow keys move active selection by 1 cell.
- Ctrl+C copies selection to clipboard, Ctrl+X cuts, Ctrl+V pastes at cursor.
- Delete/Backspace clears selection. Esc deselects.
- Polygon: Enter closes, Esc cancels. Switching tools clears polygon points.
- `CanvasWidget` gains `selection_perimeter` / `polygon_vertices` fields.

Key design decisions:
- Borrow-checker workaround: `self.selection.take()` instead of clone for
  mutable buffer access while extracting selection.
- `Clipboard` is `Vec<Vec<Option<CanvasCell>>>` — `None` = transparent, allows
  bounding-box storage with non-rectangular selections.
- Dashed border uses 2-cell alternating pattern along sorted perimeter order.
- Polygon even-odd fill uses `partial_cmp` fallback for vertical edge ties.

No `unwrap()` in production — all buffer accesses via `buffer.get()`/`set()`.

13 unit tests: marquee mask, reversed coords, circle mask, radius 0, polygon
triangle, too-few vertices, lasso, copy-paste, cut, delete, move with bounds
update, clip-to-bounds, perimeter detection, empty/inactive, paste off-canvas.

### 2.4.8 — Phase merge: release/2.4 → main

Merged all Phase 2.4 work into default branch (master). Phase 2.4 complete:
brush tool (2.4.1), eraser tool (2.4.2), line tool (2.4.3), fill/flood fill
tool (2.4.4), selection tools (marquee/lasso/circle/polygon) (2.4.5),
eyedropper tool (2.4.6), spray paint brush (2.4.7). All 7 subtasks
implemented, tested, merged. Phase 2.5 (Font Editor Mode) is next.

### 2.5.2 — Per-character canvas editing with drawing tools

Added real-time canvas→FIGcharacter sync and per-character undo/redo:
- `FIGcharacter::set_rows()` in `font.rs` — allows mutable row replacement
- `FontEditor` gained `undo_stack: Vec<Vec<String>>` and `redo_stack: Vec<Vec<String>>` (cleared on `load_font`)
- `handle_key` signature updated to accept `KeyModifiers`; `Ctrl+Z` → undo, `Ctrl+Y` → redo in CharEditor view
- `sync_from_canvas()` reads canvas buffer, builds rows, pushes undo snapshot on change, updates FIGcharacter
- `sync_canvas_to_font_char()` in `TuiApp` called every render frame when in CharEditor mode, pushing canvas edits back to font in real time
- Undo stack uses dedup check (`last() != Some(&old)`) to avoid flooding during repeated frames; redo cleared on each new edit
- All fallible paths use `Option` — no `.unwrap()` in production

**Self-review checklist:**
- Task completeness: all goals met (real-time sync, undo/redo per char)
- Code quality: clippy passes with `-D warnings`
- Formatting: `cargo fmt --check` passes clean
- No scope creep: only `font.rs`, `font_editor.rs`, `mod.rs` touched
- Security: no path traversal, unsafe writes, or secret exposure
- Error handling: all fallible paths use `Option`/`Result`

**Merge problem resolved:** `release/2.4` was 3 commits ahead of `master`
with review fixes (`fill.rs` test clarity improvements, docs reordering).
Merged `release/2.4` → `master`, resolved conflicts in `fill.rs` and
`ralph-log.md` (took release/2.4 versions). Also cleaned stale conflict
markers left from release/2.1 merge in `ralph-log.md` and removed
duplicate 2.4.8 entry in `memory.md`. 413 lib tests + 62 main tests pass,
clippy/fmt clean.

### 2.5.7 — Phase merge: release/2.5 → main

Merged all Phase 2.5 work into default branch (master). Phase 2.5 complete:
font mode scaffold with glyph grid overview (2.5.1), per-character canvas
editing with drawing tools + undo/redo (2.5.2), FIGfont header/layout editor
(2.5.3), smushing rule configuration (2.5.4), add/remove codetagged characters
(2.5.5), font-level transform tools (2.5.6). All 6 subtasks (2.5.1–2.5.6)
implemented, tested, merged. Phase 2.6 (Image Editor Mode) is next.

### 2.6.5 — Phase merge: release/2.6 → main

Merged all Phase 2.6 work into default branch (master). Phase 2.6 complete:
image import + canvas display (2.6.1), text tool with FIGlet font overlay
(2.6.2), text blocks selectable/movable/scalable/rotatable/re-editable (2.6.3),
image adjustments (brightness/contrast/threshold/dither/invert/resize) (2.6.4).
All 4 subtasks (2.6.1–2.6.4) implemented, tested, merged. Phase 2.7 (File
Operations & Persistence) is next.

### 2.7.2 — Open / recent files

Added `FileOpsMode::Open` variant. `RecentFiles` struct with push/load/save
(max 10 entries, dedup on push, persisted to XDG data dir). `enter_open()`
method populates dialog with recent file list for display. `handle_key_open()`
supports path typing, digit keys (1-9) for recent file selection, directory
navigation (Up/Down/Tab/Enter/Esc). `render_open()` shows "Open Font" dialog
with path entry, directory listing (`.flf`/`.tlf`+dirs), recent files section,
and key hints. `handle_paste()` for drag-and-drop path entry from file manager.

Bracketed paste mode enabled in `run()` (`EnableBracketedPaste`/`DisableBracketedPaste`).
`Event::Paste(text)` handled in event loop — when file ops dialog is active,
text is inserted into path buffer.

`Ctrl+O` key binding starts open dialog. `start_open()` sets mode and populates
recent files. `perform_open()` reads file, parses via `parse_tlf_font()`, loads
into font editor, pushes to recent files, saves to disk. `open_recent_file()`
(stubbed, removed as dead code — digit keys fill path buffer then Enter performs
open via `perform_open()`).

Fixed `is_dir` detection in `render_save_as()` — was using `PathBuf::from(entry).is_dir()`
(checking CWD instead of parent directory). Now joins with parent path before
checking. Same fix applied in `render_open()`.

Recent files stored as newline-separated paths at `~/.local/share/figby/recent.json`
(XDG) with fallback to `~/.figby/recent.json`. No dependency added — manual
serialization avoids `serde_json`.

17 unit tests: open dialog state management (enter/exit/type/paste/finalize),
recent files (push/max/dedup/roundtrip/missing/remove), known font verification
(95 ASCII + 7 Deutsch chars in standard.flf), recent file selection by digit key.

### 2.7.7 — Phase merge: release/2.7 → main

Merged v2.7 (file ops, config, undo) into main. Also included:
- `run_tests.rs` fix: set `FIGLET_FONTDIR` env var so integration tests find fonts
- Renamed `fonts-external/` submodule → `figby-fonts/`, repointed to fork at
  `github.com:CompewterTutor/figby-fonts.git`
- Standard FIGlet fonts (18 `.flf`) copied into `figby-fonts/fonts/`
- Created `docs/todo-v3.md` and `docs/memory-v3.md` for v3 milestone
- Moved layers/timeline tasks from v2.8/v2.9 → v3.1/v3.2
- Replaced v2.8 with TUI architecture & backend cleanup (Component arch, crossterm,
  ratatui init/run)
- Replaced v2.9 with UI polish & third-party widgets (tui-menu, throbber, theming,
  tab icons, brush preview)
