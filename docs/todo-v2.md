# Feiglet v2 — Image Input & TUI Extensions

Milestone goal: Extend Feiglet beyond FIGlet 2.2.5 feature set with image-to-ASCII
conversion, system font → FIGfont creation, interactive TUI preview, and rich
output formats (PNG, GIF, colored ANSI). Inspired by
[vendor_forks/ascii-image-converter](../vendor_forks/ascii-image-converter).

---

## Phase 2.1 — Image-to-ASCII Pipeline

- [ ] `2.1.1` Image loading crate + grayscale conversion
  - **Goal:** Add `image` crate dependency. Load JPEG, PNG, BMP, WEBP. Convert
    to grayscale luminance matrix. Handle pixel aspect ratio.
  - **Touches:** `feiglet-rs/Cargo.toml`, `feiglet-rs/src/image_input.rs`
  - **Success:** Image loads, pixels normalized to 0-255 luminance.
  - **Tests:** Load each supported format. Verify luminance values.
  - **Difficulty:** Medium

- [ ] `2.1.2` Luminance-to-ASCII character mapping
  - **Goal:** Map grayscale pixel grid to ASCII char grid. Configurable char
    map (default: ` .-:=+*#%@`). Output dimensions in character cells.
    Bilinear resize to fit terminal width.
  - **Touches:** `feiglet-rs/src/image_input.rs`
  - **Success:** Image renders as ASCII art. Custom map works.
  - **Tests:** Known-image→expected-ASCII output tests.
  - **Difficulty:** Medium

- [ ] `2.1.3` Colored ASCII output (24-bit ANSI)
  - **Goal:** Emit 24-bit ANSI escape codes per char preserving original
    pixel color. Grayscale flag forces monochrome luminance. Negative flag
    inverts colors.
  - **Touches:** `feiglet-rs/src/image_input.rs` — color path
  - **Success:** Colored ASCII renders in terminals (kitty, iTerm2, etc.).
  - **Tests:** Color output escape code correctness.
  - **Difficulty:** Medium

- [ ] `2.1.4` Braille art mode
  - **Goal:** Map 2×4 pixel blocks to Unicode braille characters (U+2800–U+28FF).
    Threshold-based dot activation. Optional Floyd-Steinberg dithering.
  - **Touches:** `feiglet-rs/src/image_input.rs` — braille path
  - **Success:** Braille art renders alongside ASCII mode.
  - **Tests:** Known braille patterns match expected output.
  - **Difficulty:** Medium

- [ ] `2.1.5` Image CLI integration
  - **Goal:** New CLI flag `--image` / `-i` toggles image→ASCII mode.
    `--map` for custom char sets. `--braille` / `-b` for braille mode.
    `--color` / `--grayscale` / `--negative`. `--dither` for dithering.
    `--width` / `--height` / `--dimensions` for output sizing.
    `--flipX` / `--flipY`. Multiple image paths + URLs supported.
  - **Touches:** `feiglet-rs/src/main.rs` — CLI flags
  - **Success:** All image flags parsed. Image mode and FIGlet mode coexist.
  - **Tests:** Flag parse tests. Integration test with known image.
  - **Difficulty:** Low

- [ ] `2.1.6` Phase merge: release/2.1 → main
  - **Difficulty:** Low

---

## Phase 2.2 — System Font → FIGfont Creation

- [ ] `2.2.1` System font enumeration via font-kit
  - **Goal:** Use `font-kit` to enumerate installed system fonts. List all
    available families + styles. Filter by monospace (for FIGfont suitability).
  - **Touches:** `feiglet-rs/src/font_gen.rs`, `Cargo.toml` (enable `font-kit`)
  - **Success:** System fonts enumerated. Monospace filter works.
  - **Tests:** Font listing test.
  - **Difficulty:** Low

- [ ] `2.2.2` Glyph rasterization → FIGcharacter rows
  - **Goal:** For each char code, rasterize glyph at a target cell size.
    Convert rasterized bitmap to FIGcharacter rows (sub-character strings).
    Handle variable-width glyphs, baseline alignment.
  - **Touches:** `feiglet-rs/src/font_gen.rs`
  - **Success:** Rendered FIGcharacter matches glyph shape at cell resolution.
  - **Tests:** Known font→known FIGcharacter output.
  - **Difficulty:** High

- [ ] `2.2.3` FIGfont header generation from font metrics
  - **Goal:** Build complete FIGfont header from font metrics: hardblank
    (use space or custom), height (cell height in lines), baseline,
    max_length, full_layout. Set old_layout to 0 (full-size / no smush)
    as default for generated fonts.
  - **Touches:** `feiglet-rs/src/font_gen.rs`
  - **Success:** Generated `.flf` file loads in Feiglet and renders identically
    to the original system font at the same point size.
  - **Tests:** Round-trip: generate .flf, parse it, compare rendered glyphs.
  - **Difficulty:** Medium

- [ ] `2.2.4` CLI command: `feiglet --create-font`
  - **Goal:** New flag `--create-font <name>` generates a FIGfont `.flf` file
    from a system font name. Optional `--font-size` for cell resolution.
    Output written to stdout or `--output` path.
  - **Touches:** `feiglet-rs/src/main.rs`, `feiglet-rs/src/font_gen.rs`
  - **Success:** System font exported as valid FIGfont. Loadable by FIGlet/Feiglet.
  - **Tests:** Generate font, load it, render known text. Compare with C FIGlet.
  - **Difficulty:** Low

- [ ] `2.2.5` Phase merge: release/2.2 → main
  - **Difficulty:** Low

---

## Phase 2.3 — TUI Interactive Mode

- [ ] `2.3.1` TUI scaffold with ratatui
  - **Goal:** Interactive terminal UI with split panes: text input, font
    selector, parameter controls, live preview pane. Event loop for keyboard
    input.
  - **Touches:** `feiglet-rs/Cargo.toml` (enable `ratatui`), `feiglet-rs/src/tui.rs`
  - **Success:** TUI launches. Text input editable. Font list scrollable.
  - **Tests:** Smoke test: TUI renders without panic.
  - **Difficulty:** Medium

- [ ] `2.3.2` Live FIGlet preview pane
  - **Goal:** Re-render FIGlet output on every keystroke. Show current font,
    width, justification, kerning/smushing mode. Respect terminal color
    for ANSI output.
  - **Touches:** `feiglet-rs/src/tui.rs`
  - **Success:** Typing updates preview in real time. Font switch re-renders.
  - **Tests:** Integration test rendering known input at various widths.
  - **Difficulty:** Medium

- [ ] `2.3.3` Image import in TUI
  - **Goal:** Drag-and-drop image path entry opens image-to-ASCII view in
    preview pane. Toggle between FIGlet mode and image mode.
  - **Touches:** `feiglet-rs/src/tui.rs`, `feiglet-rs/src/image_input.rs`
  - **Success:** Image renders in TUI. Switchable to FIGlet mode.
  - **Tests:** Image load in TUI context.
  - **Difficulty:** Medium

- [ ] `2.3.4` Output export from TUI
  - **Goal:** Save current preview as `.flf` font (if system font), as
    `.txt` ASCII art, as `.png` image. Color/greyscale/negative options
    in TUI sidebar.
  - **Touches:** `feiglet-rs/src/tui.rs` — export commands
  - **Success:** Export produces correct file. PNG has proper colors.
  - **Tests:** Export all formats. Verify file contents.
  - **Difficulty:** Low

- [ ] `2.3.5` Phase merge: release/2.3 → main
  - **Difficulty:** Low

---

## Phase 2.4 — Output Formats & Polish

- [ ] `2.4.1` PNG/SVG output for FIGlet text
  - **Goal:** Render FIGlet output as PNG image (rasterized) or SVG (vector).
    Use current font glyphs. Respect width/justification.
  - **Touches:** `feiglet-rs/Cargo.toml` (add `image`), `feiglet-rs/src/output.rs`
  - **Success:** PNG and SVG files match terminal output.
  - **Tests:** Render known text, compare PNG/SVG to reference.
  - **Difficulty:** Medium

- [ ] `2.4.2` Animated GIF output
  - **Goal:** Generate animated GIF cycling through different fonts or
    smushing modes. Configurable frame delay. Inspired by
    ascii-image-converter `--save-gif`.
  - **Touches:** `feiglet-rs/Cargo.toml` (add `gif`), `feiglet-rs/src/output.rs`
  - **Success:** GIF renders and animates in browser/image viewer.
  - **Tests:** GIF frame count and timing verified.
  - **Difficulty:** Medium

- [ ] `2.4.3` Config file support
  - **Goal:** `~/.config/feiglet/config.toml` for persistent defaults:
    preferred font, output width, color mode, TUI preferences.
  - **Touches:** `feiglet-rs/src/config.rs`
  - **Success:** Config parsed. CLI flags override config values.
  - **Tests:** Config parse tests. Override hierarchy tests.
  - **Difficulty:** Low

- [ ] `2.4.4` Performance tuning + large output optimization
  - **Goal:** Profile render pipeline. Optimize hot loops. Support
    very wide output (200+ chars) without frame drops in TUI.
  - **Touches:** `feiglet-rs/src/render.rs`, `feiglet-rs/src/tui.rs`
  - **Success:** 200-char-wide text renders at 60+ fps in TUI.
  - **Tests:** Benchmark suite; no regressions.
  - **Difficulty:** Medium

- [ ] `2.4.5` Phase merge: release/2.4 → main
  - **Difficulty:** Low

---

## Phase 2.5 — Major Release

- [ ] `2.5.1` End-to-end verification against C FIGlet 2.2.5
  - **Goal:** Full regression: all FIGlet features produce identical output.
    Image/TUI features verified via manual review.
  - **Touches:** Test infrastructure
  - **Success:** 100% FIGlet output compatibility. Image/TUI stable.
  - **Difficulty:** Medium

- [ ] `2.5.2` v2 major milestone RC — human sign-off
  - **Goal:** Prepare RC for v2.0.0. Ralph halts. Human reviews.
  - **Touches:** RC branch, annotated tag
  - **Success:** `rc/2.0.0-rc.1` created. Human merges.
  - **Difficulty:** Low
  - **Model:** Human
