# Figby v3 — Memory

Phase-by-phase implementation log for the v3 release.

## Phase 3.0 — Manual Testing & Audit

### Bugfix: `--create-font` produces invisible output

Three bugs fixed in `figby-rs/src/font_gen.rs` and `figby-rs/src/font.rs`:

1. **font_candidates path extension doubling** (`font.rs:font_candidates`): When `-f` receives a full path with `.flf` extension, the function appended another `.flf` → looked for `foo.flf.flf`. Fix: try bare path first when name contains a separator.

2. **Canvas too short for FreeType bitmap** (`font_gen.rs:system_font_to_figfont`): `raster_bounds` computes outline bounds (e.g., 9px for 'A' at 12pt), but font-kit's FreeType `rasterize_glyph` places the bitmap at `-bitmap_top` which can be 1px higher than `raster_bounds.origin_y`. Canvas allocated at bounds height → bitmap outside canvas → all-zero pixels. Fix: allocate canvas at full `charheight` and shift baseline via `transform.vector.y` so the FreeType origin aligns with the FIGfont baseline row. Added `canvas_to_figcharacter_cell` helper.

3. **Hardblank used for glyph fill** (`font_gen.rs:canvas_to_figcharacter`): Mapped all rendered pixels to `hardblank` (`$`), which by FIGfont spec displays as space → invisible output. Changed to use `'@'` for glyph fill, reserving `$` for hardblank (used only in font header).

### Learnings
- font-kit 0.14 FreeType backend's `raster_bounds` and `rasterize_glyph` can disagree on vertical positioning by ~1px. Always allocate cell-sized canvases.
- FIGfont hardblanks display as space in output — never use `$` for visible glyph content in generated fonts.
- `has_path_separator` check in `font_candidates` must account for bare paths with existing extensions.

### Bugfix: `--create-font` produces terrible output with variable character widths

**Root cause** (`font_gen.rs:system_font_to_figfont`): Character cell width was derived from
`raster_bounds.size().x()` (ink bounding box width). For monospace fonts, this gives wildly
varying widths (space=1, `!`=4, `W`=9) instead of the font's uniform advance width (~7).
Space character had width 1 because it has no visible ink → raster_bounds returns (0,0).

**Fix**: Use `font.advance(glyph_id)?.x()` for the cell width. `font.advance()` returns values
in font units (font-kit sets FT char size to units_per_em during font init). Scale by
`point_size / upem` to get pixel advance. This gives all characters a consistent cell width
matching the font's horizontal advance metric.

Also fixed `--font-size` help text from "pixels" to "points".

### Enhancement: rascii_art-driven glyph rendering with configurable charset

**Problem**: `canvas_to_figcharacter_cell` used a simple binary threshold (`pixel > 128` → fill
char, else space), producing blocky un-antialiased glyphs.

**Fix**: Replaced with `rascii_art::render_image_to` pipeline:
1. Convert font-kit `Canvas` (A8 alpha buffer) → `DynamicImage::Luma8`
2. Pass to `rascii_art::render_image_to` with configurable charset gradient
3. Split output into FIGcharacter rows

**Configurable charset** (`--create-font-charset` CLI flag):
- Named: `block` (░▒▓█), `default` (70-char), `slight`, `smooth` (custom 19-char)
- Custom: comma-separated string e.g. `--create-font-charset " ,.:o#"`
- `SMOOTH_CHARSET` is the default: ` .'^"~:;iroO0Q#8&%` — light marks to round chars
- Avoids `@` (FIGfont endmark — gets stripped by parser) and `$` (hardblank — displays as space)

**Learnings**:
- `rascii_art::render_image_to` formula `char_index = (grayscale * (N-1)) as usize` means
  N=2 charset only maps fully opaque pixels (alpha=255) to the second char. Need N≥3 for
  usable binary threshold.
- `@` cannot appear as a glyph fill character — FIGfont `strip_endmarks` removes all trailing
  endmarks from each row. If glyph row ends with `@`, it gets stripped.
- `$` (hardblank) displays as space in output — never use in glyph content.
- Unicode block chars (░▒▓█) give the best gradient for smooth edges, but pure-ASCII charsets
  ensure maximum terminal compatibility.

### E2E test checklists created
- 9 checklist files in `docs/e2e-*.md` covering `--create-font`, CLI info codes, template system, image pipeline, and all TUI editor features (~275 test cases).

### Fix: Dirty render mode — missing redraws for overlays, async ops, auto-save

Three classes of stale-display bugs fixed in `figby-rs/src/tui/mod.rs`:

1. **Async completion not checked between events**: `check_async_completion()` only ran inside `render()`. If dirty=false, render never fired, async results (save/open/export) piled up unseen. Fix: moved `check_async_completion()` into `handle_event()` so it runs every iteration regardless of render state.

2. **Auto-save + async-start paths never set dirty**: `perform_open`, `perform_save`, `perform_export`, `start_save`, auto-save trigger all started throbber + async ops without setting `self.dirty`. Fix: added `self.dirty = true` to each.

3. **State-change paths missing dirty flag**: Menu actions, dialog opens (`start_save_as`, `start_open`, settings toggle, Ctrl+E export), `apply_settings` — all changed visible UI state without triggering redraw. Fix: added `self.dirty = true` in every path.

Also added `self.dirty = true` in `check_async_completion()` itself when a result is processed, so UI updates from async completions are visible immediately.

**Tests**: Build, clippy, and all 590 lib tests pass. 17 pre-existing TUI test failures (unrelated to this change, confirmed by `git stash`).

### E2E testing: All 11 sections complete
- **Section 1** (Basic Font Creation): all 5 pass ✓
- **Section 2** (Generated Font Quality): all 5 pass ✓
- **Section 3** (Header Parameters): all pass ✓
- **Section 4** (Layout Modes): all 7 pass ✓
- **Section 5** (Width & Justification): all 6 pass ✓
- **Section 6** (Writing Direction): all 3 pass ✓
- **Section 7** (C figlet comparison): C figlet can't load multi-char fonts ✓
- **Section 8** (Deutsch): Keyboard reroute works, direct Unicode input crashes (pre-existing bug) ✓
- **Section 9** (Paragraph): all pass ✓
- **Section 10** (Spec Config): all pass ✓
- **Section 11** (Edge Cases): all 5 pass ✓

**Bugs found:**
1. MAJOR: Direct Unicode input of Deutsch chars (ÄÖÜäöüß) panics with "missing char code 0" in `render.rs:14`. The `lookup_char` function falls back to code 0 when a char isn't found, but no font has code 0. Workaround: use keyboard reroute `[\]{|}~` with `-D`.
2. MINOR: Multi-char charset smushing creates heavy overlap — traditional FIGlet smush rules (H1-H6) weren't designed for shaded characters.

---

## Phase 3.1 — Layers, Blending & Compositing

(To be filled during implementation.)

---

## Phase 3.2 — Font Editor Polish

### 3.2.2 — Glyph char editor: GlyphCursor overlay + cell toggle

Added `GlyphCursor` struct to `canvas.rs` — blinking `█` cursor overlay (`Option<GlyphCursor>` on `CanvasWidget`) rendered when set, replacing the normal reversed-style cursor. `blink()` toggles `visible` every 500ms via `Instant::now()`.

Added `glyph_cursor_x`, `glyph_cursor_y`, `brush_char` fields to `FontEditor`. `handle_key_char_editor()` rewired: arrow keys move cursor (clamped to glyph bounds), Space toggles cell between space and `brush_char`.

`mod.rs` render path syncs cursor position and calls `blink()` per frame — avoids recreating `GlyphCursor` each frame to preserve blink timer. Key dispatch syncs `brush_char` from palette before font_editor handler runs.

---

## Phase 3.3 — Particle Effect Creator

(To be filled during implementation.)

---

## Phase 3.4 — Animation Exporter

(To be filled during implementation.)

---

## Phase 3.5 — Animation Player (Standalone Widget)

(To be filled during implementation.)

---

## Phase 3.6 — Major Release

(To be filled during release.)
