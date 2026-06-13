# Figby v2 — Templates, Image Input & Full TUI Editor

Milestone goal: Extend Figby beyond FIGlet 2.2.5 with a `.ftmp` template
system (variable substitution, image embedding, layering), image-to-ASCII
conversion, system font → FIGfont creation, and a comprehensive TUI editor
with drawing tools, font/charset editing, image editing with FIGlet text
overlay, layers, and animation timeline.

---

## Phase 2.0 — CLI Polish, README, Templates & Repo Cleanup

- [x] `2.0.1` Implement CLI --help output
  - **Goal:** `--help` is minimal/blank. Add `#[arg(help = "...")]` or
    `#[command(about, long_about)]` to every clap field so `figby --help`
    and `figby --long-help` show complete usage with all flags, descriptions,
    and examples.
  - **Touches:** `figby-rs/src/main.rs`
  - **Success:** `figby --help` exits 0 and shows all flags with descriptions.
  - **Tests:** Check `--help` output contains expected flags.
  - **Difficulty:** Low

- [x] `2.0.2` Port make-examples script to CLI
  - **Goal:** Create `scripts/make-examples.sh` that generates a single
    Markdown file (`examples/FIGLET_EXAMPLES.md`) with a `###` header
    per font (showing font name, filename, and height), and the rendered
    output in a fenced code block. Accept: `--sample-text` (default:
    `"hello figby"`), `--fonts` (comma-separated whitelist), `--exclude`
    (comma-separated blacklist).
  - **Touches:** `scripts/make-examples.sh`
  - **Success:** Running with defaults produces a single markdown file
    viewable on GitHub or any Markdown renderer. Custom text and font
    filters work.
  - **Tests:** Generate examples, verify markdown structure.
  - **Difficulty:** Medium

- [x] `2.0.3` Update README with proper documentation
  - **Goal:** Full README covering: what Figby is, installation (cargo,
    package managers, pre-built), CLI usage with examples, font directory
    setup, getting fonts, comparison with C FIGlet, contributing guide.
  - **Touches:** `README.md`
  - **Success:** README is comprehensive and useful to new users.
  - **Tests:** N/A (manual review).
  - **Difficulty:** Low

- [x] `2.0.4` Repo cleanup — move C source to subdirectory
  - **Goal:** Move all C FIGlet 2.2.5 source files (`figlet.c`, `chkfont.c`,
    `inflate.c`, `zipio.c`, `utf8.c`, `getopt.c`, `crc.c`, headers,
    `Makefile*`) into `c-figlet/` to clean root. Update references in
    README, AGENTS.md, scripts.
  - **Touches:** Move files to `c-figlet/`, update docs/scripts
  - **Success:** Root no longer has loose C files. All references updated.
  - **Tests:** Verify paths in AGENTS.md and scripts still resolve.
  - **Difficulty:** Low

- [x] `2.0.5` `.ftmp` template file format design + CLI
  - **Goal:** Design `.ftmp` (FIGby Template) file format. Template body
    is clean — just `{{varname}}` placeholders. All configuration lives
    in frontmatter (YAML/TOML) with two sections:
    - **Canvas settings:** target width (defaults to terminal width via
      `term_size`, overridable by `--width`), height, keep ratio, margin,
      padding.
    - **Variable bindings:** each `varname` maps to `{text, font, x, y,
      z, align, overlap, borderWidth, borderColor, shadowSize, shadowColor}`.
      z-index optional; rendering order = ascending z, last tagged = highest.
      `overlap` mode: `overwrite` (pixels replace) or `flow` (no overlap —
      falls to next line like normal FIGlet wrapping at target width).
    `--render-template` CLI flag reads `.ftmp`, renders layers sequentially.
  - **Touches:** `figby-rs/src/template.rs`, `figby-rs/src/main.rs`
  - **Success:** Template body `{{greeting}}` with frontmatter binding
    renders text in specified font. `flow` mode layers stack vertically.
  - **Tests:** Parse `.ftmp`, render, verify output. `flow` vs `overwrite`.
  - **Difficulty:** High

- [x] `2.0.6` Template tag value sources + rascii image tag
  - **Goal:** Tag `text` attribute accepts three source types: string
    literal, env var (`${VAR}`), command substitution (`$(cmd)`).
    Add rascii image tag: `{{img:source:width:height:color:pos:charset}}`
    that converts image to ASCII inline via `rascii_art`. Test with
    `assets/img/figby.png` at width 30 with text "figby" alongside.
  - **Touches:** `figby-rs/src/template.rs`
  - **Success:** `${HOME}` resolves to home dir. `$(date)` runs and
    captures output. Image tag renders ASCII art in output.
  - **Tests:** Env var resolve, command capture, image tag with known file.
  - **Difficulty:** Medium

- [x] `2.0.7` Border and shadow rendering for template output
  - **Goal:** `borderWidth,borderColor` renders a border around the text
    block using `.` characters. `shadowSize,shadowColor` renders a
    drop-shadow offset by shadowSize using `.` characters. Both only
    applied to `.` placeholder cells (not overwriting existing content).
  - **Touches:** `figby-rs/src/template.rs`
  - **Success:** Template with border and shadow produces framed output
    with visible shadow offset.
  - **Tests:** Border-only, shadow-only, border+shadow output tests.
  - **Difficulty:** Medium

- [x] `2.0.8` `--to-file` output flag (add CLI arg, defer implementation)
  - **Goal:** Add `--to-file <path>` to CLI arg struct so it parses
    cleanly. Actual file write is deferred to 2.1 — output is piped
    or redirected for now.
  - **Touches:** `figby-rs/src/main.rs`
  - **Success:** `--to-file` accepted without error but currently a no-op.
  - **Tests:** Flag parse test only.
  - **Difficulty:** Low

- [x] `2.0.9` Builtin template functions: date + repo-data (defer to 2.1)
  - **Goal:** Add `{{date:format}}` (strftime-style date) and
    `{{repo-data:author|email|name|release}}` builtins for template use.
    Implementation deferred — just define syntax and reserve keywords.
  - **Touches:** `figby-rs/src/template.rs`
  - **Success:** Design documented. No runtime yet.
  - **Tests:** N/A (deferred).
  - **Difficulty:** Low

- [x] `2.0.10` Phase merge: release/2.0 → main
  - **Difficulty:** Low

---

## Phase 2.1 — Image-to-ASCII Pipeline

- [x] `2.1.1` Image loading + grayscale conversion via `rascii_art`
  - **Goal:** Add `rascii_art` dep. Load JPEG, PNG, BMP, WEBP. Convert to
    grayscale luminance matrix.
  - **Touches:** `figby-rs/Cargo.toml`, `figby-rs/src/lib.rs`, `figby-rs/src/image_input.rs`
  - **Success:** Image loads, pixels normalized to 0-255 luminance.
  - **Tests:** Load each supported format. Verify luminance values.
  - **Difficulty:** Medium

- [x] `2.1.2` Luminance-to-ASCII character mapping
  - **Goal:** Map grayscale pixel grid to ASCII char grid. Configurable char
    map (default: ` .-:=+*#%@`). Bilinear resize to fit terminal width.
  - **Touches:** `figby-rs/src/image_input.rs`
  - **Success:** Image renders as ASCII art. Custom map works.
  - **Tests:** Known-image→expected-ASCII output tests.
  - **Difficulty:** Medium

- [x] `2.1.3` Colored ASCII output (24-bit ANSI)
  - **Goal:** 24-bit ANSI escape codes per char preserving original pixel
    color. Grayscale flag. Negative invert.
  - **Touches:** `figby-rs/src/image_input.rs`
  - **Success:** Colored ASCII renders in modern terminals.
  - **Tests:** Color output escape code correctness.
  - **Difficulty:** Medium

- [x] `2.1.4` Braille art + dithering
  - **Goal:** Map 2×4 pixel blocks to Unicode braille (U+2800–U+28FF).
    Threshold + optional Floyd-Steinberg dithering.
  - **Touches:** `figby-rs/src/image_input.rs`
  - **Success:** Braille art renders. Dithering improves visibility.
  - **Tests:** Known braille patterns match expected.
  - **Difficulty:** Medium

- [x] `2.1.5` Image CLI flags integration
  - **Goal:** `--image`/`-i`, `--map`, `--braille`/`-b`, `--color`/`--grayscale`/`--negative`,
    `--dither`, `--width`/`--height`/`--dimensions`, `--flipX`/`--flipY`.
    Multiple paths + URLs. Coexists with FIGlet mode.
  - **Touches:** `figby-rs/src/main.rs`
  - **Success:** All image flags parsed. Integration test with known image.
  - **Tests:** Flag parse tests.
  - **Difficulty:** Low

- [x] `2.1.6` Phase merge: release/2.1 → main
  - **Difficulty:** Low

---

## Phase 2.2 — System Font → FIGfont Creation

- [x] `2.2.1` System font enumeration via font-kit
  - **Goal:** Enumerate installed system fonts. List families + styles.
    Filter by monospace.
  - **Touches:** `figby-rs/src/font_gen.rs`, `Cargo.toml` (enable `font-kit`)
  - **Success:** System fonts enumerated. Monospace filter works.
  - **Tests:** Font listing test.
  - **Difficulty:** Low

- [x] `2.2.2` Glyph rasterization → FIGcharacter rows
  - **Goal:** Rasterize glyph at target cell size. Convert bitmap to
    FIGcharacter sub-character strings. Variable-width, baseline alignment.
  - **Touches:** `figby-rs/src/font_gen.rs`
  - **Success:** Rendered FIGcharacter matches glyph shape at cell resolution.
  - **Tests:** Known font→known FIGcharacter output.
  - **Difficulty:** High

- [x] `2.2.3` FIGfont header from font metrics
  - **Goal:** Build FIGfont header: hardblank, height, baseline, max_length,
    full_layout. Default old_layout=0 (full-size).
  - **Touches:** `figby-rs/src/font_gen.rs`
  - **Success:** Generated `.flf` loads in Figby, renders identically to
    original system font at same point size.
  - **Tests:** Round-trip: generate .flf, parse, compare rendered glyphs.
  - **Difficulty:** Medium

- [x] `2.2.4` CLI command: `figby --create-font`
  - **Goal:** `--create-font <name>` generates `.flf` from system font.
    Optional `--font-size`. Output to stdout or `--output` path.
  - **Touches:** `figby-rs/src/main.rs`, `figby-rs/src/font_gen.rs`
  - **Success:** System font exported as valid FIGfont. Loadable by FIGlet.
  - **Tests:** Generate font, load it, render known text. Compare with C.
  - **Difficulty:** Low

- [x] `2.2.5` Create TUI iconset YAML file
  - **Goal:** Create `assets/tui/icons.yaml` — plain YAML mapping every UI
    element (tools, panels, modes, buttons, menus) from the todo spec to
    Nerd Font icon names. Covers: tool icons, mode tabs, cursor states,
    brush shapes, color palette controls, status indicators, file ops,
    edit actions, font editor panels, smushing rule toggles, font
    transforms, image adjustments, text tool controls, layer operations,
    blend modes, timeline controls, keyframe tools, export options,
    settings, navigation, dialog buttons, and misc UI widgets (checkboxes,
    toggles, scrollbars).
  - **Touches:** `assets/tui/icons.yaml`
  - **Success:** File contains all 120+ elements with valid Nerd Font
    icon names. Each `nf-*` name maps to a real Nerd Font glyph.
  - **Tests:** Verify YAML parse + all keys present.
  - **Difficulty:** Low

- [x] `2.2.6` Phase merge: release/2.2 → main
  - **Difficulty:** Low

---

## Phase 2.3 — TUI Core & Canvas

- [x] `2.3.1` TUI scaffold with ratatui
  - **Goal:** Ratatui app with mode switching: Font Editor, Image Editor,
    ASCII Preview. Shared layout: toolbar top, canvas center, status bar
    bottom, palette sidebar right.
  - **Touches:** `figby-rs/Cargo.toml` (enable `ratatui`), `figby-rs/src/tui.rs`
  - **Success:** TUI launches. Mode tabs switch between editors.
  - **Tests:** Smoke test: TUI renders all panels without panic.
  - **Difficulty:** Medium

- [x] `2.3.2` Toolbox bar
  - **Goal:** Vertical/horizontal toolbar with tool icons. Shared tool
    set across all modes: brush, selection tools (marquee, lasso, circle,
    polygon), fill, line, eraser, eyedropper, text tool. Active tool
    highlighted. Keyboard shortcuts (V=select, B=brush, E=eraser, etc.).
  - **Touches:** `figby-rs/src/tui/toolbox.rs`
  - **Success:** All tools render. Click/keyboard selects active tool.
  - **Tests:** Tool selection round-trip tests.
  - **Difficulty:** Medium

- [x] `2.3.3` Canvas widget
  - **Goal:** Scrollable/zoomable canvas widget. Renders the current
    working buffer (font glyph, image, or ASCII preview) as a grid of
    characters with optional color. Grid overlay. Cursor tracking.
  - **Touches:** `figby-rs/src/tui/canvas.rs`
  - **Success:** Canvas renders buffer. Arrow keys move cursor. Zoom
    in/out works.
  - **Tests:** Canvas render + cursor movement tests.
  - **Difficulty:** Medium

- [x] `2.3.4` Color palette
  - **Goal:** Color palette sidebar: 16 standard ANSI colors + 240-color
    extended grid. Foreground/background selector. Custom color picker
    (RGB sliders or hex input). Recent colors strip.
  - **Touches:** `figby-rs/src/tui/palette.rs`
  - **Success:** Click selects color. FG/BG toggle works. Custom color
    saved.
  - **Tests:** Color selection + apply tests.
  - **Difficulty:** Medium

- [x] `2.3.5` Brush selection
  - **Goal:** Brush shape picker: square, circle, spray paint, custom
    (user-drawn pattern). Size slider (1-20 chars). Preview of brush
    shape in toolbox.
  - **Touches:** `figby-rs/src/tui/brush.rs`
  - **Success:** Brush shapes render. Size changes reflected in preview.
  - **Tests:** Brush shape + size tests.
  - **Difficulty:** Low

- [x] `2.3.6` Status bar + canvas settings
  - **Goal:** Status bar: cursor position (X,Y), zoom level, current
    tool name, mode, unsaved indicator. Settings panel: canvas width ×
    height, font size, grid toggle, snap-to-grid.
  - **Touches:** `figby-rs/src/tui/status.rs`
  - **Success:** Status updates on cursor move. Settings panel changes
    canvas dimensions.
  - **Tests:** Status bar update tests.
  - **Difficulty:** Low

- [x] `2.3.7` Phase merge: release/2.3 → main
  - **Difficulty:** Low

---

## Phase 2.4 — Drawing Tools

- [x] `2.4.1` Brush tool
  - **Goal:** Paint characters onto canvas at cursor. Uses active brush
    shape and size. Applies active foreground color to each char cell.
    Continuous stroke on click+drag.
  - **Touches:** `figby-rs/src/tui/tools/brush.rs`
  - **Success:** Click places char. Drag draws line. Brush shape respected.
  - **Tests:** Brush stroke pattern tests.
  - **Difficulty:** Medium

- [ ] `2.4.2` Eraser tool
  - **Goal:** Erases characters (sets to space/transparent). Same brush
    shape/size respect as brush.
  - **Touches:** `figby-rs/src/tui/tools/eraser.rs`
  - **Success:** Eraser removes chars within brush shape.
  - **Tests:** Eraser shape tests.
  - **Difficulty:** Low

- [ ] `2.4.3` Line tool
  - **Goal:** Click start point, drag to end point. Draws straight line
    using Bresenham. Uses active brush shape. Preview line while dragging.
  - **Touches:** `figby-rs/src/tui/tools/line.rs`
  - **Success:** Straight line drawn between two points.
  - **Tests:** Horizontal, vertical, diagonal line tests.
  - **Difficulty:** Medium

- [ ] `2.4.4` Fill / flood fill tool
  - **Goal:** Click contiguous region of same character → replace all
    with active brush char. Boundary-aware (stops at different chars).
  - **Touches:** `figby-rs/src/tui/tools/fill.rs`
  - **Success:** Flood fill replaces contiguous region.
  - **Tests:** Fill on bounded and unbounded regions.
  - **Difficulty:** Medium

- [ ] `2.4.5` Selection tools: marquee, lasso, circle, polygon
  - **Goal:** Marquee: click-drag rectangle selection. Lasso: freehand
    selection. Circle: click-drag center-to-edge. Polygon: click points,
    enter to close. Dashed border overlay. Selection can be moved, copied,
    cut, deleted.
  - **Touches:** `figby-rs/src/tui/tools/selection.rs`
  - **Success:** All selection shapes work. Selection move/copy/cut/deleted.
  - **Tests:** Selection boundary tests per shape.
  - **Difficulty:** High

- [ ] `2.4.6` Eyedropper tool
  - **Goal:** Click a cell → set active foreground color to that cell's
    color, active brush char to that cell's character.
  - **Touches:** `figby-rs/src/tui/tools/eyedropper.rs`
  - **Success:** Color + char sampled from canvas.
  - **Tests:** Sample color + char after drawing.
  - **Difficulty:** Low

- [ ] `2.4.7` Spray paint brush
  - **Goal:** Stochastic spray within brush radius. Density slider.
    Characters scattered randomly within circle area.
  - **Touches:** `figby-rs/src/tui/tools/spray.rs`
  - **Success:** Spray pattern scatters chars within radius.
  - **Tests:** Spray density distribution check.
  - **Difficulty:** Medium

- [ ] `2.4.8` Phase merge: release/2.4 → main
  - **Difficulty:** Low

---

## Phase 2.5 — Font Editor Mode

- [ ] `2.5.1` Font mode scaffold: glyph grid overview
  - **Goal:** Font Editor mode. Main view: grid of all 102 required
    FIGcharacters (32-126 + Deutsch 196,214,220,228,246,252,223) plus
    codetagged chars. Each cell shows mini preview. Click to edit.
    Search/filter by char code or value.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`
  - **Success:** All chars displayed in grid. Click opens char in canvas.
  - **Tests:** Glyph grid renders all required chars.
  - **Difficulty:** Medium

- [ ] `2.5.2` Per-character canvas editing with drawing tools
  - **Goal:** Selected FIGcharacter opens in canvas. All Phase 2.4
    drawing tools available (brush, eraser, line, fill, selection, etc.).
    Canvas grid = character rows × sub-character columns. Changes update
    the char in real time.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`
  - **Success:** Drawing tools modify FIGcharacter. Undo/redo per char.
  - **Tests:** Edit char via brush, verify FIGcharacter data changed.
  - **Difficulty:** Medium

- [ ] `2.5.3` FIGfont header / layout editor
  - **Goal:** Panel for editing font-level properties: hardblank char,
    height, baseline, max_length, full_layout bitflags, print direction,
    comment lines. Validation (height≥1, baseline≤height).
  - **Touches:** `figby-rs/src/tui/font_editor.rs`
  - **Success:** Header fields editable. Invalid values rejected.
  - **Tests:** Header edit round-trip.
  - **Difficulty:** Low

- [ ] `2.5.4` Smushing rule configuration
  - **Goal:** Visual toggle grid for smushing rule bits: equal char,
    underscore, hierarchy, pair, big X, hardblank. Preview: render two
    sample chars with current rules to show effect.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`
  - **Success:** Toggling rules updates preview instantly.
  - **Tests:** Rule toggle changes output.
  - **Difficulty:** Medium

- [ ] `2.5.5` Add/remove codetagged characters
  - **Goal:** Insert new character by code. Delete existing. Bulk copy
    from one char code to another. Missing char (code 0) fallback editing.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`
  - **Success:** New chars added to font. Deleted chars fall back to
    missing-char.
  - **Tests:** Add + remove + copy char tests.
  - **Difficulty:** Medium

- [ ] `2.5.6` Font-level transform tools
  - **Goal:** Resize entire font (change height, reflow all chars).
    Italicize (shift rows). Bold (duplicate columns). Mirror/flip all
    glyphs. Copy glyph from another font. Rename font.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`
  - **Success:** Transform applies to all glyphs consistently.
  - **Tests:** Resize + bold + mirror across all chars.
  - **Difficulty:** High

- [ ] `2.5.7` Phase merge: release/2.5 → main
  - **Difficulty:** Low

---

## Phase 2.6 — Image Editor Mode

- [ ] `2.6.1` Image import + canvas display
  - **Goal:** Image Editor mode. Import image via path or `rascii_art`.
    Display as ASCII char grid on canvas. Color mode renders ANSI colors
    per cell. Grayscale mode renders luminance chars.
  - **Touches:** `figby-rs/src/tui/image_editor.rs`, `figby-rs/src/image_input.rs`
  - **Success:** Image appears on canvas as ASCII art with correct colors.
  - **Tests:** Load known image, verify canvas output matches CLI output.
  - **Difficulty:** Medium

- [ ] `2.6.2` Text tool with FIGlet font overlay
  - **Goal:** Text tool in toolbox. Click on canvas → type text →
    render that text using selected FIGlet font at cursor position.
    Font preview dropdown in tool options. Text color, size, justification.
  - **Touches:** `figby-rs/src/tui/tools/text.rs`
  - **Success:** Text renders on canvas using FIGlet font at cursor.
  - **Tests:** Place text, verify FIGlet output in canvas buffer.
  - **Difficulty:** High

- [ ] `2.6.3` Text tool advanced: selection + transform
  - **Goal:** Placed text blocks are selectable (marquee around block).
    Move, scale, rotate (90° steps). Re-edit text content. Delete block.
    Multiple text blocks on same canvas.
  - **Touches:** `figby-rs/src/tui/tools/text.rs`
  - **Success:** Text blocks selectable, movable, resizable.
  - **Tests:** Move + rescale text block.
  - **Difficulty:** Medium

- [ ] `2.6.4` Image adjustments
  - **Goal:** Brightness/contrast sliders. Threshold (for braille mode).
    Dither toggle. Invert colors. Resize/re-sample image. All adjustments
    update canvas in real time.
  - **Touches:** `figby-rs/src/tui/image_editor.rs`
  - **Success:** Sliders modify image output. Reset to original.
  - **Tests:** Adjust brightness, verify pixel values change.
  - **Difficulty:** Medium

- [ ] `2.6.5` Phase merge: release/2.6 → main
  - **Difficulty:** Low

---

## Phase 2.7 — File Operations & Persistence

- [ ] `2.7.1` Save / Save As
  - **Goal:** Save current font as `.flf` file. Save As dialog (file
    browser widget). Auto-save timer option. Untitled→prompt on first
    save. Unsaved indicator in status bar.
  - **Touches:** `figby-rs/src/tui/file_ops.rs`
  - **Success:** Font saved as valid `.flf`. Reloadable.
  - **Tests:** Save then load, verify byte-identical.
  - **Difficulty:** Medium

- [ ] `2.7.2` Open / recent files
  - **Goal:** Open `.flf` file via file browser. Recent files list in
    menu. Drag-and-drop file path entry. File type filter (`.flf`, `.tlf`).
  - **Touches:** `figby-rs/src/tui/file_ops.rs`
  - **Success:** Font loaded into editor. Recent files persisted.
  - **Tests:** Open known font, verify all glyphs loaded.
  - **Difficulty:** Medium

- [ ] `2.7.3` Copy / duplicate font
  - **Goal:** Duplicate current font in editor (new untitled copy).
    Copy glyphs between fonts (copy from one FIGfont to current).
    Import glyphs from another `.flf` file.
  - **Touches:** `figby-rs/src/tui/font_editor.rs`
  - **Success:** Duplicate creates independent copy. Import merges glyphs.
  - **Tests:** Duplicate, edit one, verify other unchanged.
  - **Difficulty:** Low

- [ ] `2.7.4` Export: PNG, TXT, GIF
  - **Goal:** Export canvas/current preview as PNG (rasterized ASCII),
    TXT (raw ASCII text), GIF (animated if multiple frames). Color
    mode preserved in PNG/GIF. Font selection for export.
  - **Touches:** `figby-rs/src/tui/export.rs`, `figby-rs/src/output.rs`
  - **Success:** Exported files match canvas appearance.
  - **Tests:** Export → re-import, verify content preserved.
  - **Difficulty:** Medium

- [ ] `2.7.5` Config file
  - **Goal:** `~/.config/figby/config.toml`: default font, output width,
    color mode, TUI preferences (theme, recent files, brush defaults).
  - **Touches:** `figby-rs/src/config.rs`
  - **Success:** Config parsed. CLI flags override config values.
  - **Tests:** Config parse + override hierarchy tests.
  - **Difficulty:** Low

- [ ] `2.7.6` Undo/redo system
  - **Goal:** Global undo/redo stack for all editing actions. Ctrl+Z /
    Ctrl+Shift+Z. Undo history panel. Configurable undo limit (default 50).
  - **Touches:** `figby-rs/src/tui/undo.rs`
  - **Success:** Every action undoable. Undo stack persists within session.
  - **Tests:** Multiple undo/redo cycles, verify state consistency.
  - **Difficulty:** Medium

- [ ] `2.7.7` Phase merge: release/2.7 → main
  - **Difficulty:** Low

---

## Phase 2.8 — Layers & Blending

- [ ] `2.8.1` Layer system
  - **Goal:** Layer panel: list of layers, visibility toggle, lock toggle,
    opacity slider, drag-to-reorder. New/delete/duplicate/merge layers.
    Each layer is an independent ASCII buffer.
  - **Touches:** `figby-rs/src/tui/layers.rs`
  - **Success:** Layers render stacked. Layer operations work.
  - **Tests:** Create, delete, reorder, merge layers.
  - **Difficulty:** High

- [ ] `2.8.2` Blending modes
  - **Goal:** Per-layer blend mode: Normal, Multiply, Overlay, Screen,
    Add, Subtract. Render composited output in real time. Preview
    thumbnail per layer showing blend effect.
  - **Touches:** `figby-rs/src/tui/layers.rs`
  - **Success:** Blend modes produce correct composed output.
  - **Tests:** Multiply + Overlay blend with known test patterns.
  - **Difficulty:** High

- [ ] `2.8.3` Layer groups + masks
  - **Goal:** Group layers into folders. Layer mask: paint on mask to
    hide/reveal parts of layer. Mask thumbnail in layer panel.
  - **Touches:** `figby-rs/src/tui/layers.rs`
  - **Success:** Groups collapsible. Mask hides painted areas.
  - **Tests:** Group create, mask paint, verify composited result.
  - **Difficulty:** Medium

- [ ] `2.8.4` Export with layers
  - **Goal:** Export flattened composite. Export individual layers as
    separate files. Export with transparency (space = transparent).
  - **Touches:** `figby-rs/src/tui/export.rs`
  - **Success:** Flattened export matches canvas. Layer exports correct.
  - **Tests:** Export composite vs manual layer merge.
  - **Difficulty:** Low

- [ ] `2.8.5` Phase merge: release/2.8 → main
  - **Difficulty:** Low

---

## Phase 2.9 — Animation Timeline

- [ ] `2.9.1` Frame management
  - **Goal:** Timeline panel: frame thumbnails, add/delete/duplicate/
    reorder frames. Each frame stores full layer state. Onion skinning
    (semi-transparent overlay of prev/next frame).
  - **Touches:** `figby-rs/src/tui/timeline.rs`
  - **Success:** Frames addable, reorderable. Onion skin overlay renders.
  - **Tests:** Create frames, switch between them, verify state isolation.
  - **Difficulty:** High

- [ ] `2.9.2` Keyframing
  - **Goal:** Keyframeable properties per layer: position offset, opacity,
    blend mode. Keyframe markers on timeline. Interpolation between
    keyframes (linear). Keyframe editor panel.
  - **Touches:** `figby-rs/src/tui/timeline.rs`
  - **Success:** Keyframes set. Playback interpolates between them.
  - **Tests:** Set keyframes, play, verify interpolation.
  - **Difficulty:** High

- [ ] `2.9.3` Tweening
  - **Goal:** Auto-tween: select start/end keyframes, generate
    intermediate frames. Easing functions: linear, ease-in, ease-out,
    bounce. Preview tween before committing.
  - **Touches:** `figby-rs/src/tui/timeline.rs`
  - **Success:** Tween generates intermediate frames with correct easing.
  - **Tests:** Tween between known keyframes, verify frame sequence.
  - **Difficulty:** Medium

- [ ] `2.9.4` GIF export from timeline
  - **Goal:** Render animation timeline to animated GIF. Frame delay per
    frame or global FPS setting. Loop count. Preview playback in TUI.
  - **Touches:** `figby-rs/src/tui/export.rs`
  - **Success:** GIF matches timeline playback.
  - **Tests:** Export GIF, verify frame count + timing.
  - **Difficulty:** Medium

- [ ] `2.9.5` Phase merge: release/2.9 → main
  - **Difficulty:** Low

---

## Phase 2.10 — Major Release

- [ ] `2.10.1` Full regression against C FIGlet 2.2.5
  - **Goal:** All FIGlet features produce identical output. Image/TUI/
    animation verified via manual review.
  - **Touches:** Test infrastructure
  - **Success:** 100% FIGlet output compatibility.
  - **Difficulty:** Medium

- [ ] `2.10.2` v2 major milestone RC — human sign-off
  - **Goal:** RC for v2.0.0. Ralph halts. Human reviews.
  - **Touches:** RC branch, annotated tag
  - **Success:** `rc/2.0.0-rc.1` created. Human merges.
  - **Difficulty:** Low
  - **Model:** Human
